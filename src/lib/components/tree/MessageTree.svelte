<script lang="ts">
	import type { TreeNode } from '$lib/types/hl7';
	import { getTreeChildren, getFieldContent, searchMessage, type SearchHit } from '$lib/ipc/parser';
	import { getSegmentInfo } from '$lib/ipc/tables';
	import TreeNodeRow from './TreeNodeRow.svelte';
	import { t, subscribeLocale } from '$lib/i18n';
	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		messageId: string;
		roots: TreeNode[];
		onNodeSelect?: (node: TreeNode) => void;
		onFieldExpand?: (content: string) => void;
		/** Navigate to a specific segment and optionally a field within it. Stamp forces re-trigger. */
		navigateTo?: { segmentIdx: number; fieldPosition: number | null; stamp: number } | null;
		/** Callback to request the editor to navigate to the selected tree node */
		onNavigateToEditor?: (segmentIdx: number, fieldPosition: number | null, componentIdx: number | null) => void;
		/** HL7 version used to look up schema field definitions */
		version?: string;
		/** When true, inject placeholder rows for schema-defined fields that are absent from the message */
		showSchemaFields?: boolean;
		/** Parse format ("HL7v2", "FHIR JSON", "FHIR XML"). Search is HL7-only:
		 *  search_message looks up the HL7 store, FHIR resources live in a
		 *  separate store, so the box would always report no matches. */
		format?: string;
	}

	let {
		messageId,
		roots,
		onNodeSelect,
		onFieldExpand,
		navigateTo = null,
		onNavigateToEditor,
		version = '',
		showSchemaFields = false,
		format = 'HL7v2',
	}: Props = $props();

	const searchEnabled = $derived(format === 'HL7v2');

	type VNode = TreeNode & { _children?: TreeNode[]; _expanded?: boolean; _isPlaceholder?: boolean };

	// Flat list of visible nodes for virtual scrolling
	let visibleNodes = $state<VNode[]>([]);
	let selectedNodeId = $state<string | null>(null);

	// --- Search ---
	let searchQuery = $state('');
	let searchHits = $state<SearchHit[]>([]);
	let searchActive = $state(false);   // a completed search is being displayed
	let searchPending = $state(false);
	let activeHitIdx = $state(-1);
	let searchInputEl: HTMLInputElement | undefined = $state();
	let searchDebounce: ReturnType<typeof setTimeout> | null = null;
	// Monotonic token: a response is applied only if no newer search (or a
	// clear, or a message switch) started after it. Prevents a slow response
	// from repopulating results for a query no longer in the input.
	let searchToken = 0;

	// Debounced backend search. Searches the parsed message in the store, so
	// it finds fields even in segments the tree hasn't lazily expanded yet.
	$effect(() => {
		const q = searchQuery.trim();
		const msgId = messageId;
		if (searchDebounce) clearTimeout(searchDebounce);
		const token = ++searchToken;
		if (!q || !searchEnabled) {
			searchHits = [];
			searchActive = false;
			searchPending = false;
			activeHitIdx = -1;
			return;
		}
		searchPending = true;
		searchDebounce = setTimeout(async () => {
			let hits: SearchHit[] = [];
			try {
				hits = await searchMessage(msgId, q);
			} catch {
				hits = [];
			}
			if (token !== searchToken) return; // stale response
			searchHits = hits;
			searchActive = true;
			activeHitIdx = -1;
			searchPending = false;
		}, 250);
	});

	async function gotoHit(idx: number) {
		if (idx < 0 || idx >= searchHits.length) return;
		activeHitIdx = idx;
		const hit = searchHits[idx];
		await navigateToTarget(hit.segment_idx, hit.field_position);
		const node = visibleNodes.find((n) => n.id === hit.node_id)
			?? visibleNodes.find((n) => n.id === `seg${hit.segment_idx}`);
		if (node) onNodeSelect?.(node);
	}

	function nextHit() { if (searchHits.length) void gotoHit((activeHitIdx + 1) % searchHits.length); }
	function prevHit() { if (searchHits.length) void gotoHit((activeHitIdx - 1 + searchHits.length) % searchHits.length); }

	function clearSearch() {
		searchQuery = '';
		searchHits = [];
		searchActive = false;
		activeHitIdx = -1;
	}

	function handleSearchKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') { clearSearch(); (e.target as HTMLElement).blur(); }
		else if (e.key === 'Enter' && e.shiftKey) { e.preventDefault(); prevHit(); }
		else if (e.key === 'Enter') { e.preventDefault(); nextHit(); }
	}

	// Ctrl+F / Cmd+F while the tree has focus jumps to the search box.
	// Monaco keeps its own find widget for the raw-text panel.
	function handleTreeKeydown(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
			e.preventDefault();
			e.stopPropagation();
			searchInputEl?.focus();
			searchInputEl?.select();
		}
	}

	// Initialize with root nodes (also re-init when showSchemaFields toggles so
	// previously-expanded segments pick up / drop placeholder rows).
	$effect(() => {
		void showSchemaFields;
		visibleNodes = roots.map((r) => ({ ...r, _expanded: false }));
	});

	// Reset search when the displayed message changes — hits reference node
	// ids of the previous message.
	let lastMessageId = $state('');
	$effect(() => {
		if (messageId !== lastMessageId) {
			lastMessageId = messageId;
			clearSearch();
		}
	});

	// Track the last processed stamp to avoid duplicate navigation
	let lastNavStamp = $state(0);

	// Navigate to a specific segment + optional field when requested
	$effect(() => {
		if (!navigateTo || navigateTo.stamp === lastNavStamp) return;
		lastNavStamp = navigateTo.stamp;
		void navigateToTarget(navigateTo.segmentIdx, navigateTo.fieldPosition);
	});

	async function navigateToTarget(segmentIdx: number, fieldPosition: number | null) {
		const segId = `seg${segmentIdx}`;
		let segNode = visibleNodes.find((n) => n.id === segId);
		if (!segNode) return;

		// Expand segment if we need to reach a field
		if (fieldPosition !== null && !segNode._expanded) {
			await toggleNode(segNode);
			segNode = visibleNodes.find((n) => n.id === segId);
		}

		let targetId = segId;
		if (fieldPosition !== null && segNode) {
			const fieldId = `${segId}.f${fieldPosition}`;
			const fieldNode = visibleNodes.find((n) => n.id === fieldId);
			if (fieldNode) targetId = fieldId;
		}

		selectedNodeId = targetId;
		// Scroll the target into view
		requestAnimationFrame(() => {
			const el = document.querySelector(`[data-node-id="${CSS.escape(targetId)}"]`);
			el?.scrollIntoView({ behavior: 'smooth', block: 'center' });
		});
	}

	/** Extract segment-type code (e.g. "PID") from a segment node's label. */
	function segmentTypeFromNode(node: TreeNode): string | null {
		const m = node.label.match(/^([A-Z][A-Z0-9]{2})/);
		return m ? m[1] : null;
	}

	/**
	 * If showSchemaFields is on and the node being expanded is a segment, merge
	 * the real children with placeholder nodes for schema-defined fields that
	 * are absent from the actual message.
	 */
	async function mergeSchemaPlaceholders(segNode: VNode, realChildren: TreeNode[]): Promise<VNode[]> {
		if (!showSchemaFields || !version || segNode.node_type !== 'segment') {
			return realChildren.map((c) => ({ ...c, _expanded: false }));
		}
		const segType = segmentTypeFromNode(segNode);
		if (!segType) return realChildren.map((c) => ({ ...c, _expanded: false }));

		try {
			const info = await getSegmentInfo(segType, version);
			if (!info) return realChildren.map((c) => ({ ...c, _expanded: false }));

			const segId = segNode.id; // "seg{N}"
			const existing = new Set<number>();
			for (const c of realChildren) {
				const m = c.id.match(/\.f(\d+)$/);
				if (m) existing.add(parseInt(m[1]));
			}

			const placeholders: VNode[] = info.fields
				.filter((f) => !existing.has(f.position))
				.map((f) => ({
					id: `${segId}.f${f.position}`,
					label: `${segType}-${f.position} ${f.name}`,
					value_preview: '',
					node_type: 'field' as const,
					depth: segNode.depth + 1,
					has_children: false,
					is_truncated: false,
					child_count: 0,
					_expanded: false,
					_isPlaceholder: true,
				}));

			const merged: VNode[] = [
				...realChildren.map((c) => ({ ...c, _expanded: false })),
				...placeholders,
			];
			// Sort by field position
			merged.sort((a, b) => {
				const am = a.id.match(/\.f(\d+)$/);
				const bm = b.id.match(/\.f(\d+)$/);
				return (am ? parseInt(am[1]) : 0) - (bm ? parseInt(bm[1]) : 0);
			});
			return merged;
		} catch {
			return realChildren.map((c) => ({ ...c, _expanded: false }));
		}
	}

	async function toggleNode(node: VNode) {
		const idx = visibleNodes.findIndex((n) => n.id === node.id);
		if (idx === -1) return;

		if (node._expanded) {
			// Collapse: remove all children recursively
			const depth = node.depth;
			let removeCount = 0;
			for (let i = idx + 1; i < visibleNodes.length; i++) {
				if (visibleNodes[i].depth > depth) {
					removeCount++;
				} else {
					break;
				}
			}
			visibleNodes = [
				...visibleNodes.slice(0, idx),
				{ ...node, _expanded: false },
				...visibleNodes.slice(idx + 1 + removeCount),
			];
		} else {
			// Expand: fetch children and insert
			if (!node._children) {
				const children = await getTreeChildren(messageId, node.id);
				node._children = children;
			}
			const childNodes = await mergeSchemaPlaceholders(node, node._children!);
			visibleNodes = [
				...visibleNodes.slice(0, idx),
				{ ...node, _expanded: true },
				...childNodes,
				...visibleNodes.slice(idx + 1),
			];
		}
	}

	function selectNode(node: TreeNode) {
		selectedNodeId = node.id;
		onNodeSelect?.(node);
	}

	async function expandTruncated(node: TreeNode) {
		// Parse segment and field indices from node ID: "seg0.f5"
		const parts = node.id.split('.');
		if (parts.length < 2) return;

		const segIdx = parseInt(parts[0].replace('seg', ''));
		const fieldIdx = parseInt(parts[1].replace('f', ''));

		const content = await getFieldContent(messageId, segIdx, fieldIdx);
		onFieldExpand?.(content.full_text);
	}

	/** Parse a node id like "seg3", "seg3.f5", "seg3.f5.c2" into navigation parts. */
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

	function showInEditor(node: TreeNode) {
		const { segmentIdx, fieldPosition, componentIdx } = parseNodeId(node.id);
		if (segmentIdx === null) return;
		onNavigateToEditor?.(segmentIdx, fieldPosition, componentIdx);
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="tree-container" onkeydown={handleTreeKeydown}>
	{#if visibleNodes.length === 0}
		<div class="tree-empty">No message loaded</div>
	{:else}
		{#if searchEnabled}
		<div class="tree-search">
			<div class="search-row">
				<span class="search-icon">&#128269;</span>
				<input
					class="search-input"
					type="text"
					bind:this={searchInputEl}
					bind:value={searchQuery}
					placeholder={tr('tree.searchPlaceholder')}
					onkeydown={handleSearchKeydown}
					spellcheck="false"
				/>
				{#if searchQuery}
					<button class="search-clear" onclick={clearSearch} title={tr('tree.searchClear')}>&times;</button>
				{/if}
			</div>
			{#if searchActive && !searchPending}
				<div class="search-status">
					{#if searchHits.length === 0}
						<span class="no-results">{tr('tree.searchNoResults')}</span>
					{:else}
						<span>{tr('tree.searchResults', { count: searchHits.length })}</span>
						<span class="search-nav">
							<button class="nav-btn" onclick={prevHit} title="Shift+Enter">&#9650;</button>
							<button class="nav-btn" onclick={nextHit} title="Enter">&#9660;</button>
						</span>
					{/if}
				</div>
				{#if searchHits.length > 0}
					<div class="search-hits">
						{#each searchHits as hit, i (hit.node_id + ':' + i)}
							<button
								class="search-hit"
								class:active={i === activeHitIdx}
								onclick={() => gotoHit(i)}
							>
								<span class="hit-kind" class:kind-name={hit.match_kind === 'name'} class:kind-segment={hit.match_kind === 'segment'}>
									{hit.match_kind === 'value' ? '=' : hit.match_kind === 'name' ? 'Aa' : '§'}
								</span>
								<span class="hit-label">{hit.label}</span>
								{#if hit.snippet}<span class="hit-snippet">{hit.snippet}</span>{/if}
							</button>
						{/each}
					</div>
				{/if}
			{/if}
		</div>
		{/if}
		<div class="tree-list" role="tree">
			{#each visibleNodes as node (node.id)}
				<TreeNodeRow
					{node}
					isSelected={selectedNodeId === node.id}
					isExpanded={node._expanded ?? false}
					isPlaceholder={node._isPlaceholder ?? false}
					onToggle={() => toggleNode(node)}
					onSelect={() => selectNode(node)}
					onExpandTruncated={() => expandTruncated(node)}
					onShowInEditor={onNavigateToEditor && !node._isPlaceholder ? () => showInEditor(node) : undefined}
				/>
			{/each}
		</div>
	{/if}
</div>

<style>
	.tree-container {
		height: 100%;
		overflow-y: auto;
		overflow-x: hidden;
		font-family: 'JetBrains Mono', 'Fira Code', monospace;
		font-size: 12px;
		background-color: var(--color-bg-secondary);
	}

	.tree-empty {
		padding: 16px;
		color: var(--color-text-secondary);
		text-align: center;
		font-style: italic;
	}

	.tree-list {
		padding: 4px 0;
	}

	/* --- Search bar --- */
	.tree-search {
		position: sticky;
		top: 0;
		z-index: 10;
		background-color: var(--color-bg-secondary);
		border-bottom: 1px solid var(--color-border);
		padding: 4px 6px;
	}

	.search-row {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.search-icon {
		font-size: 11px;
		opacity: 0.6;
		flex-shrink: 0;
	}

	.search-input {
		flex: 1;
		min-width: 0;
		padding: 3px 6px;
		border: 1px solid var(--color-border);
		border-radius: 3px;
		background: var(--color-bg-tertiary);
		color: var(--color-text-primary);
		font-family: inherit;
		font-size: 11px;
	}

	.search-input:focus {
		outline: none;
		border-color: var(--color-accent);
	}

	.search-clear {
		background: none;
		border: none;
		color: var(--color-text-secondary);
		font-size: 14px;
		cursor: pointer;
		padding: 0 4px;
		flex-shrink: 0;
	}

	.search-clear:hover { color: var(--color-text-primary); }

	.search-status {
		display: flex;
		align-items: center;
		justify-content: space-between;
		font-size: 10px;
		color: var(--color-text-secondary);
		padding: 3px 2px 1px;
	}

	.no-results { font-style: italic; }

	.search-nav { display: flex; gap: 2px; }

	.nav-btn {
		background: none;
		border: 1px solid var(--color-border);
		border-radius: 3px;
		color: var(--color-text-secondary);
		font-size: 8px;
		cursor: pointer;
		padding: 1px 5px;
	}

	.nav-btn:hover { color: var(--color-text-primary); background: var(--color-bg-tertiary); }

	.search-hits {
		max-height: 180px;
		overflow-y: auto;
		margin-top: 3px;
		border-top: 1px solid var(--color-border);
	}

	.search-hit {
		display: flex;
		align-items: center;
		gap: 6px;
		width: 100%;
		padding: 2px 4px;
		background: none;
		border: none;
		border-left: 2px solid transparent;
		color: var(--color-text-primary);
		font-family: inherit;
		font-size: 11px;
		text-align: left;
		cursor: pointer;
		white-space: nowrap;
		overflow: hidden;
	}

	.search-hit:hover { background: var(--color-bg-tertiary); }

	.search-hit.active {
		background: var(--color-bg-tertiary);
		border-left-color: var(--color-accent);
	}

	.hit-kind {
		flex-shrink: 0;
		width: 18px;
		text-align: center;
		font-size: 9px;
		color: var(--color-accent);
		border: 1px solid var(--color-border);
		border-radius: 3px;
	}

	.hit-kind.kind-name { color: var(--color-field); }
	.hit-kind.kind-segment { color: var(--color-segment); }

	.hit-label {
		flex-shrink: 0;
		font-weight: 600;
		color: var(--color-field);
	}

	.hit-snippet {
		overflow: hidden;
		text-overflow: ellipsis;
		color: var(--color-text-secondary);
		opacity: 0.8;
	}
</style>
