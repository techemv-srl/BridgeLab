<script lang="ts">
	import type { TreeNode } from '$lib/types/hl7';
	import type { FieldInfo, SegmentInfo } from '$lib/ipc/tables';
	import { getFieldInfo, getSegmentInfo } from '$lib/ipc/tables';
	import { getFieldContent } from '$lib/ipc/parser';
	import { t, subscribeLocale } from '$lib/i18n';

	interface Props {
		messageId: string | null;
		version: string;
		/** The currently selected tree node (null if nothing selected) */
		selectedNode: TreeNode | null;
		/** Segment type for the segment containing the selected node (e.g., "PID") */
		segmentType: string | null;
		onViewFullValue?: (fullText: string) => void;
	}

	let { messageId, version, selectedNode, segmentType, onViewFullValue }: Props = $props();

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') {
		subscribeLocale(() => { localeVersion++; });
	}
	function tr(key: string): string {
		void localeVersion;
		return t(key);
	}

	let segmentInfo = $state<SegmentInfo | null>(null);
	let fieldInfo = $state<FieldInfo | null>(null);
	let schemaLookupDone = $state(false);

	// Parse node id to extract seg/field indices
	function parseNodeId(id: string): { segmentIdx: number | null; fieldPosition: number | null; componentIdx: number | null } {
		const parts = id.split('.');
		let segmentIdx: number | null = null;
		let fieldPosition: number | null = null;
		let componentIdx: number | null = null;
		for (const p of parts) {
			if (p.startsWith('seg')) segmentIdx = parseInt(p.slice(3));
			else if (p.startsWith('f')) fieldPosition = parseInt(p.slice(1));
			else if (p.startsWith('c')) componentIdx = parseInt(p.slice(1));
		}
		return { segmentIdx, fieldPosition, componentIdx };
	}

	let parsedId = $derived(selectedNode ? parseNodeId(selectedNode.id) : null);

	// Fetch schema info when selection changes
	$effect(() => {
		segmentInfo = null;
		fieldInfo = null;
		schemaLookupDone = false;

		if (!selectedNode || !segmentType) return;
		const p = parseNodeId(selectedNode.id);

		(async () => {
			try {
				if (p.fieldPosition !== null) {
					fieldInfo = await getFieldInfo(segmentType, p.fieldPosition, version);
				} else if (selectedNode.node_type === 'segment') {
					segmentInfo = await getSegmentInfo(segmentType, version);
				}
			} catch { /* IPC unavailable or segment unknown */ }
			finally { schemaLookupDone = true; }
		})();
	});

	// Current raw value length (from node preview if not truncated, else ask backend)
	let currentLength = $derived(() => {
		if (!selectedNode) return null;
		if (selectedNode.is_truncated) return null; // unknown without full fetch
		return selectedNode.value_preview?.length ?? 0;
	});

	async function handleViewFull() {
		if (!selectedNode || !messageId) return;
		const p = parseNodeId(selectedNode.id);
		if (p.segmentIdx === null || p.fieldPosition === null) return;
		try {
			const res = await getFieldContent(messageId, p.segmentIdx, p.fieldPosition);
			onViewFullValue?.(res.full_text);
		} catch (e) { console.error('Fetch full value failed:', e); }
	}
</script>

<div class="inspector">
	<div class="panel-header">
		<span>{tr('inspector.title')}</span>
	</div>

	{#if !selectedNode}
		<div class="placeholder">{tr('inspector.noSelection')}</div>
	{:else}
		<div class="inspector-body">
			<!-- Position / label header -->
			<div class="item-header">
				{#if fieldInfo}
					{fieldInfo.segment_code}-{fieldInfo.position}
				{:else if segmentInfo}
					{segmentInfo.code}
				{:else}
					{selectedNode.label}
				{/if}
			</div>

			<!-- Schema-derived fields -->
			{#if fieldInfo}
				<dl class="kv">
					<dt>{tr('inspector.name')}</dt>
					<dd>{fieldInfo.name}</dd>

					<dt>{tr('inspector.dataType')}</dt>
					<dd><code>{fieldInfo.data_type}</code></dd>

					<dt>{tr('inspector.description')}</dt>
					<dd>{fieldInfo.description}</dd>

					<dt>{tr('inspector.required')}</dt>
					<dd class:yes={fieldInfo.required} class:no={!fieldInfo.required}>
						{fieldInfo.required ? tr('inspector.yes') : tr('inspector.no')}
					</dd>

					<dt>{tr('inspector.repeating')}</dt>
					<dd>{fieldInfo.repeating ? tr('inspector.yes') : tr('inspector.no')}</dd>

					{#if fieldInfo.max_length !== null}
						<dt>{tr('inspector.maxLength')}</dt>
						<dd>{fieldInfo.max_length}</dd>
					{/if}
				</dl>
			{:else if segmentInfo}
				<dl class="kv">
					<dt>{tr('inspector.name')}</dt>
					<dd>{segmentInfo.name}</dd>

					<dt>{tr('inspector.description')}</dt>
					<dd>{segmentInfo.description}</dd>

					<dt>{tr('inspector.dataType')}</dt>
					<dd>{segmentInfo.fields.length} fields</dd>
				</dl>
			{:else if schemaLookupDone}
				<div class="schema-unknown">{tr('inspector.schemaUnknown')}</div>
			{/if}

			<!-- Current value -->
			{#if selectedNode.value_preview || selectedNode.is_truncated}
				<div class="value-section">
					<div class="value-label">
						{tr('inspector.currentValue')}
						{#if selectedNode.is_truncated}
							<span class="badge-warn">{tr('inspector.truncated')}</span>
						{/if}
					</div>
					<pre class="value-box">{selectedNode.value_preview || ''}</pre>
					{#if selectedNode.is_truncated}
						<button class="view-full-btn" onclick={handleViewFull}>
							{tr('inspector.viewFull')}
						</button>
					{:else if currentLength() !== null}
						<div class="value-meta">{tr('inspector.currentLength')}: {currentLength()}</div>
					{/if}
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.inspector {
		display: flex;
		flex-direction: column;
		height: 100%;
		background-color: var(--color-bg-secondary);
		border-top: 1px solid var(--color-border);
		overflow: hidden;
	}

	.panel-header {
		padding: 6px 10px;
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: var(--color-text-secondary);
		background-color: var(--color-bg-tertiary);
		border-bottom: 1px solid var(--color-border);
		flex-shrink: 0;
	}

	.placeholder {
		padding: 16px;
		font-size: 12px;
		font-style: italic;
		color: var(--color-text-secondary);
		text-align: center;
	}

	.inspector-body {
		padding: 10px 12px;
		overflow-y: auto;
		font-size: 12px;
		flex: 1;
		min-height: 0;
	}

	.item-header {
		font-size: 13px;
		font-weight: 700;
		color: var(--color-accent);
		margin-bottom: 10px;
		font-family: 'JetBrains Mono', monospace;
	}

	.kv {
		display: grid;
		grid-template-columns: max-content 1fr;
		column-gap: 10px;
		row-gap: 4px;
		margin: 0 0 10px 0;
	}

	.kv dt {
		color: var(--color-text-secondary);
		font-weight: 500;
	}

	.kv dd {
		margin: 0;
		color: var(--color-text-primary);
		word-break: break-word;
	}

	.kv dd code {
		font-family: 'JetBrains Mono', monospace;
		background-color: var(--color-bg-tertiary);
		padding: 1px 5px;
		border-radius: 3px;
		font-size: 11px;
	}

	.kv dd.yes {
		color: var(--color-warn, #d68000);
		font-weight: 600;
	}

	.kv dd.no {
		color: var(--color-text-secondary);
	}

	.schema-unknown {
		font-style: italic;
		color: var(--color-text-secondary);
		padding: 6px 0;
		font-size: 11px;
	}

	.value-section {
		margin-top: 8px;
		padding-top: 10px;
		border-top: 1px dashed var(--color-border);
	}

	.value-label {
		font-weight: 600;
		color: var(--color-text-secondary);
		margin-bottom: 4px;
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.badge-warn {
		background-color: var(--color-error);
		color: white;
		padding: 0 6px;
		border-radius: 3px;
		font-size: 10px;
		font-weight: 600;
		text-transform: uppercase;
	}

	.value-box {
		background-color: var(--color-bg-primary);
		border: 1px solid var(--color-border);
		border-radius: 3px;
		padding: 6px 8px;
		margin: 0;
		font-family: 'JetBrains Mono', monospace;
		font-size: 11px;
		white-space: pre-wrap;
		word-break: break-all;
		max-height: 120px;
		overflow-y: auto;
	}

	.value-meta {
		font-size: 11px;
		color: var(--color-text-secondary);
		margin-top: 4px;
	}

	.view-full-btn {
		margin-top: 6px;
		background-color: var(--color-bg-tertiary);
		border: 1px solid var(--color-border);
		color: var(--color-text-primary);
		padding: 4px 10px;
		border-radius: 3px;
		cursor: pointer;
		font-size: 11px;
	}

	.view-full-btn:hover {
		background-color: var(--color-accent);
		color: var(--color-bg-primary);
	}
</style>
