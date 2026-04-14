<script lang="ts">
	import type { TreeNode, ParseResult } from '$lib/types/hl7';
	import { parseMessage } from '$lib/ipc/parser';
	import MonacoEditor from '$lib/components/editor/MonacoEditor.svelte';
	import MessageTree from '$lib/components/tree/MessageTree.svelte';
	import StatusBar from '$lib/components/layout/StatusBar.svelte';

	// State
	let parseResult = $state<ParseResult | null>(null);
	let editorContent = $state('');
	let cursorLine = $state(1);
	let cursorColumn = $state(1);
	let expandedFieldContent = $state<string | null>(null);

	// Splitter state
	let treeWidth = $state(350);
	let isDragging = $state(false);

	/** Handle pasting HL7 messages into the editor */
	async function handleContentChange(value: string) {
		editorContent = value;
		// Auto-parse if content looks like HL7
		if (value.startsWith('MSH|') && value.length > 10) {
			try {
				parseResult = await parseMessage(value);
			} catch {
				// Not a valid HL7 message yet, ignore
			}
		}
	}

	/** Handle file open via dialog */
	async function handleOpenFile() {
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const selected = await open({
				multiple: false,
				filters: [
					{ name: 'HL7 Messages', extensions: ['hl7', 'txt', 'msg'] },
					{ name: 'FHIR Resources', extensions: ['json', 'xml'] },
					{ name: 'All Files', extensions: ['*'] },
				],
			});

			if (selected) {
				const { openFile } = await import('$lib/ipc/parser');
				const path = typeof selected === 'string' ? selected : selected.path;
				parseResult = await openFile(path);
				editorContent = parseResult.truncated_text;
			}
		} catch (e) {
			console.error('Failed to open file:', e);
		}
	}

	/** Parse content from button click */
	async function handleParse() {
		if (!editorContent.trim()) return;
		try {
			parseResult = await parseMessage(editorContent);
			editorContent = parseResult.truncated_text;
		} catch (e) {
			console.error('Parse error:', e);
		}
	}

	/** Handle tree node selection */
	function handleNodeSelect(node: TreeNode) {
		// Find the line in the editor for this segment
		if (node.node_type === 'segment') {
			const segIdx = parseInt(node.id.replace('seg', ''));
			// Lines are 1-based, segments are 0-based
			const monacoEditor = document.querySelector('.editor-container');
			if (monacoEditor) {
				// Reveal line in editor
			}
		}
	}

	/** Handle expanding truncated fields */
	function handleFieldExpand(content: string) {
		expandedFieldContent = content;
	}

	/** Close expanded field modal */
	function closeExpandedField() {
		expandedFieldContent = null;
	}

	/** Splitter drag handlers */
	function startDrag(e: MouseEvent) {
		isDragging = true;
		e.preventDefault();
	}

	function handleMouseMove(e: MouseEvent) {
		if (!isDragging) return;
		const newWidth = Math.max(200, Math.min(600, e.clientX));
		treeWidth = newWidth;
	}

	function stopDrag() {
		isDragging = false;
	}

	/** Handle keyboard shortcut for open file */
	function handleKeydown(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && e.key === 'o') {
			e.preventDefault();
			handleOpenFile();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} onmousemove={handleMouseMove} onmouseup={stopDrag} />

<div class="app-shell">
	<!-- Toolbar -->
	<div class="toolbar">
		<div class="toolbar-left">
			<span class="app-title">BridgeLab</span>
			<span class="app-subtitle">HL7 made simple</span>
		</div>
		<div class="toolbar-actions">
			<button class="btn" onclick={handleOpenFile} title="Open File (Ctrl+O)">
				Open File
			</button>
			<button class="btn btn-primary" onclick={handleParse} title="Parse Message">
				Parse
			</button>
		</div>
	</div>

	<!-- Main content area -->
	<div class="main-content">
		<!-- Tree panel -->
		<div class="tree-panel" style="width: {treeWidth}px">
			{#if parseResult}
				<div class="panel-header">
					<span>Message Structure</span>
					<span class="panel-badge">{parseResult.segment_count}</span>
				</div>
				<MessageTree
					messageId={parseResult.message_id}
					roots={parseResult.tree_roots}
					onNodeSelect={handleNodeSelect}
					onFieldExpand={handleFieldExpand}
				/>
			{:else}
				<div class="panel-header">
					<span>Message Structure</span>
				</div>
				<div class="panel-empty">
					<p>Open an HL7 file or paste a message to begin.</p>
					<p class="shortcut-hint">Ctrl+O to open a file</p>
				</div>
			{/if}
		</div>

		<!-- Splitter -->
		<div
			class="splitter"
			class:active={isDragging}
			role="separator"
			tabindex={0}
			onmousedown={startDrag}
		></div>

		<!-- Editor panel -->
		<div class="editor-panel">
			<div class="panel-header">
				<span>
					{#if parseResult}
						{parseResult.message_type || 'Message'} - {parseResult.format}
					{:else}
						Editor
					{/if}
				</span>
			</div>
			<MonacoEditor
				bind:content={editorContent}
				onContentChange={handleContentChange}
				onCursorChange={(line, col) => { cursorLine = line; cursorColumn = col; }}
			/>
		</div>
	</div>

	<!-- Expanded field modal -->
	{#if expandedFieldContent !== null}
		<div class="modal-overlay" onclick={closeExpandedField} role="presentation">
			<div class="modal" onclick={(e) => e.stopPropagation()} role="dialog">
				<div class="modal-header">
					<span>Full Field Content</span>
					<button class="modal-close" onclick={closeExpandedField}>&times;</button>
				</div>
				<div class="modal-body">
					<pre>{expandedFieldContent}</pre>
				</div>
				<div class="modal-footer">
					<button class="btn" onclick={() => { navigator.clipboard.writeText(expandedFieldContent!); }}>
						Copy to Clipboard
					</button>
					<span class="modal-info">{expandedFieldContent.length.toLocaleString()} characters</span>
				</div>
			</div>
		</div>
	{/if}

	<!-- Status bar -->
	<StatusBar
		messageType={parseResult?.message_type}
		version={parseResult?.version}
		format={parseResult?.format}
		segmentCount={parseResult?.segment_count}
		fileSize={parseResult?.file_size_bytes}
		truncationCount={parseResult?.truncation_count}
		{cursorLine}
		{cursorColumn}
	/>
</div>

<style>
	.app-shell {
		display: flex;
		flex-direction: column;
		height: 100vh;
		overflow: hidden;
	}

	/* Toolbar */
	.toolbar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		height: 40px;
		padding: 0 12px;
		background-color: var(--color-bg-secondary);
		border-bottom: 1px solid var(--color-border);
		flex-shrink: 0;
	}

	.toolbar-left {
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	.app-title {
		font-weight: 700;
		font-size: 14px;
		color: var(--color-accent);
	}

	.app-subtitle {
		font-size: 11px;
		color: var(--color-text-secondary);
		font-style: italic;
	}

	.toolbar-actions {
		display: flex;
		gap: 8px;
	}

	.btn {
		padding: 4px 12px;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		background-color: var(--color-bg-tertiary);
		color: var(--color-text-primary);
		cursor: pointer;
		font-size: 12px;
		font-family: inherit;
		transition: background-color 0.15s;
	}

	.btn:hover {
		background-color: var(--color-border);
	}

	.btn-primary {
		background-color: var(--color-accent);
		color: var(--color-bg-primary);
		border-color: var(--color-accent);
	}

	.btn-primary:hover {
		background-color: var(--color-accent-hover);
	}

	/* Main content */
	.main-content {
		display: flex;
		flex: 1;
		overflow: hidden;
	}

	.tree-panel {
		display: flex;
		flex-direction: column;
		flex-shrink: 0;
		overflow: hidden;
	}

	.editor-panel {
		display: flex;
		flex-direction: column;
		flex: 1;
		overflow: hidden;
	}

	.panel-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		height: 28px;
		padding: 0 12px;
		background-color: var(--color-bg-tertiary);
		font-size: 11px;
		font-weight: 600;
		color: var(--color-text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		flex-shrink: 0;
	}

	.panel-badge {
		background-color: var(--color-accent);
		color: var(--color-bg-primary);
		font-size: 10px;
		padding: 1px 6px;
		border-radius: 8px;
	}

	.panel-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		flex: 1;
		color: var(--color-text-secondary);
		font-size: 13px;
		gap: 4px;
	}

	.shortcut-hint {
		font-size: 11px;
		opacity: 0.6;
	}

	/* Splitter */
	.splitter {
		width: 4px;
		cursor: col-resize;
		background-color: var(--color-border);
		flex-shrink: 0;
		transition: background-color 0.15s;
	}

	.splitter:hover,
	.splitter.active {
		background-color: var(--color-accent);
	}

	/* Modal */
	.modal-overlay {
		position: fixed;
		inset: 0;
		background-color: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 100;
	}

	.modal {
		background-color: var(--color-bg-secondary);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		width: 80%;
		max-width: 900px;
		max-height: 80vh;
		display: flex;
		flex-direction: column;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 12px 16px;
		border-bottom: 1px solid var(--color-border);
		font-weight: 600;
	}

	.modal-close {
		background: none;
		border: none;
		color: var(--color-text-secondary);
		cursor: pointer;
		font-size: 20px;
		line-height: 1;
	}

	.modal-body {
		flex: 1;
		overflow: auto;
		padding: 16px;
	}

	.modal-body pre {
		margin: 0;
		white-space: pre-wrap;
		word-break: break-all;
		font-family: 'JetBrains Mono', monospace;
		font-size: 12px;
		color: var(--color-text-primary);
	}

	.modal-footer {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 12px 16px;
		border-top: 1px solid var(--color-border);
	}

	.modal-info {
		font-size: 11px;
		color: var(--color-text-secondary);
	}
</style>
