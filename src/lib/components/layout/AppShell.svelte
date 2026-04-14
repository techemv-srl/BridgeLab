<script lang="ts">
	import { onMount } from 'svelte';
	import type { TreeNode, ParseResult } from '$lib/types/hl7';
	import { parseMessage, openFile, getFieldContent } from '$lib/ipc/parser';
	import { getRecentFiles, addRecentFile, clearRecentFiles, getPreference, setPreference } from '$lib/ipc/database';
	import type { RecentFile } from '$lib/ipc/database';
	import { t, setLocale, type Locale } from '$lib/i18n';
	import { messageStore, type MessageTab } from '$lib/stores/messages.svelte';
	import MonacoEditor from '$lib/components/editor/MonacoEditor.svelte';
	import MessageTree from '$lib/components/tree/MessageTree.svelte';
	import EditorTabs from '$lib/components/editor/EditorTabs.svelte';
	import MenuBar from '$lib/components/layout/MenuBar.svelte';
	import StatusBar from '$lib/components/layout/StatusBar.svelte';

	// UI state
	let treeWidth = $state(350);
	let isDragging = $state(false);
	let showTree = $state(true);
	let expandedFieldContent = $state<string | null>(null);
	let showAbout = $state(false);
	let recentFiles = $state<RecentFile[]>([]);
	let theme = $state('dark');

	// Reactive references to the active tab
	let activeTab = $derived(messageStore.activeTab);

	onMount(async () => {
		// Load preferences (wrapped in try/catch for web-only dev mode)
		try {
			const savedTheme = await getPreference('theme');
			if (savedTheme) {
				theme = savedTheme;
				applyTheme(savedTheme);
			}
			const savedLang = await getPreference('language');
			if (savedLang) setLocale(savedLang as Locale);
			const savedTreeWidth = await getPreference('tree_width');
			if (savedTreeWidth) treeWidth = parseInt(savedTreeWidth) || 350;
			recentFiles = await getRecentFiles(20);
		} catch {
			// Running in web-only mode without Tauri backend
		}

		// Start with one empty tab
		if (messageStore.tabs.length === 0) {
			messageStore.newTab();
		}
	});

	function applyTheme(t: string) {
		document.documentElement.setAttribute('data-theme', t);
	}

	// --- File operations ---

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
				const path = typeof selected === 'string' ? selected : selected.path;
				const result = await openFile(path);
				messageStore.openMessage(result, path, result.truncated_text);
				// Track recent file
				const filename = path.split('/').pop()?.split('\\').pop() ?? '';
				await addRecentFile(path, filename, result.message_type, result.version, result.file_size_bytes);
				recentFiles = await getRecentFiles(20);
			}
		} catch (e) {
			console.error('Failed to open file:', e);
		}
	}

	async function handleOpenRecentFile(path: string) {
		try {
			const result = await openFile(path);
			messageStore.openMessage(result, path, result.truncated_text);
			const filename = path.split('/').pop()?.split('\\').pop() ?? '';
			await addRecentFile(path, filename, result.message_type, result.version, result.file_size_bytes);
			recentFiles = await getRecentFiles(20);
		} catch (e) {
			console.error('Failed to open recent file:', e);
		}
	}

	async function handleSave() {
		if (!activeTab?.filePath || !activeTab?.parseResult) return;
		try {
			const { saveFile } = await import('$lib/ipc/parser');
			await saveFile(activeTab.parseResult.message_id, activeTab.filePath);
			messageStore.markSaved(activeTab.id);
		} catch (e) {
			console.error('Save failed:', e);
		}
	}

	async function handleSaveAs() {
		if (!activeTab?.parseResult) return;
		try {
			const { save } = await import('@tauri-apps/plugin-dialog');
			const path = await save({
				filters: [
					{ name: 'HL7 Messages', extensions: ['hl7'] },
					{ name: 'All Files', extensions: ['*'] },
				],
			});
			if (path) {
				const { saveFile } = await import('$lib/ipc/parser');
				await saveFile(activeTab.parseResult.message_id, path);
				messageStore.markSaved(activeTab.id, path);
			}
		} catch (e) {
			console.error('Save As failed:', e);
		}
	}

	async function handleClearRecent() {
		try {
			await clearRecentFiles();
			recentFiles = [];
		} catch {
			// ignore in web mode
		}
	}

	// --- Tab operations ---

	function handleNewTab() {
		messageStore.newTab();
	}

	function handleCloseTab(tabId?: string) {
		const id = tabId ?? messageStore.activeTabId;
		if (id) messageStore.closeTab(id);
	}

	function handleCloseAllTabs() {
		messageStore.closeAllTabs();
		messageStore.newTab();
	}

	// --- Editor operations ---

	async function handleContentChange(value: string) {
		if (!messageStore.activeTabId) return;
		messageStore.updateContent(messageStore.activeTabId, value);
		// Auto-parse if content looks like HL7
		if (value.startsWith('MSH|') && value.length > 10) {
			try {
				const result = await parseMessage(value);
				messageStore.updateParseResult(messageStore.activeTabId, result, result.truncated_text);
			} catch {
				// Not a valid HL7 message yet
			}
		}
	}

	async function handleParse() {
		if (!activeTab?.content?.trim()) return;
		try {
			const result = await parseMessage(activeTab.content);
			messageStore.updateParseResult(activeTab.id, result, result.truncated_text);
		} catch (e) {
			console.error('Parse error:', e);
		}
	}

	function handleCursorChange(line: number, column: number) {
		if (messageStore.activeTabId) {
			messageStore.updateCursor(messageStore.activeTabId, line, column);
		}
	}

	// --- Tree operations ---

	function handleNodeSelect(node: TreeNode) {
		// Navigate editor to the corresponding segment line
		if (node.node_type === 'segment') {
			const segIdx = parseInt(node.id.replace('seg', ''));
			// Segments correspond to lines (1-based)
			// The editor will scroll to this line
			const editorEl = document.querySelector('.editor-container') as HTMLElement;
			if (editorEl) {
				// Monaco editor line reveal is handled via the MonacoEditor component
			}
		}
	}

	function handleFieldExpand(content: string) {
		expandedFieldContent = content;
	}

	function closeExpandedField() {
		expandedFieldContent = null;
	}

	// --- View operations ---

	function handleToggleTree() {
		showTree = !showTree;
	}

	async function handleSetTheme(newTheme: string) {
		theme = newTheme;
		applyTheme(newTheme);
		try { await setPreference('theme', newTheme); } catch { /* web mode */ }
	}

	async function handleSetLanguage(lang: string) {
		setLocale(lang as Locale);
		try { await setPreference('language', lang); } catch { /* web mode */ }
		// Force re-render by updating a reactive state
		theme = theme;
	}

	// --- Drag & Drop ---

	async function handleDragOver(e: DragEvent) {
		e.preventDefault();
		if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy';
	}

	async function handleDrop(e: DragEvent) {
		e.preventDefault();
		const files = e.dataTransfer?.files;
		if (!files || files.length === 0) return;

		for (const file of Array.from(files)) {
			// In Tauri, we need the file path, but web File API only gives name
			// Tauri's drag-drop gives us the path via the event
			try {
				// Try reading as text for paste-like behavior
				const text = await file.text();
				if (text.startsWith('MSH|')) {
					const result = await parseMessage(text, file.name);
					messageStore.openMessage(result, null, result.truncated_text);
					messageStore.tabs[messageStore.tabs.length - 1].label = file.name;
				}
			} catch {
				console.error('Failed to read dropped file:', file.name);
			}
		}
	}

	// --- Splitter ---

	function startDrag(e: MouseEvent) {
		isDragging = true;
		e.preventDefault();
	}

	function handleMouseMove(e: MouseEvent) {
		if (!isDragging) return;
		treeWidth = Math.max(200, Math.min(600, e.clientX));
	}

	async function stopDrag() {
		if (isDragging) {
			isDragging = false;
			try { await setPreference('tree_width', String(treeWidth)); } catch { /* web mode */ }
		}
	}

	// --- Keyboard shortcuts ---

	function handleKeydown(e: KeyboardEvent) {
		const ctrl = e.ctrlKey || e.metaKey;
		if (ctrl && e.key === 'o') { e.preventDefault(); handleOpenFile(); }
		else if (ctrl && e.key === 's' && e.shiftKey) { e.preventDefault(); handleSaveAs(); }
		else if (ctrl && e.key === 's') { e.preventDefault(); handleSave(); }
		else if (ctrl && e.key === 'w') { e.preventDefault(); handleCloseTab(); }
		else if (ctrl && e.key === 'b') { e.preventDefault(); handleToggleTree(); }
		else if (e.key === 'F5') { e.preventDefault(); handleParse(); }
	}
</script>

<svelte:window
	onkeydown={handleKeydown}
	onmousemove={handleMouseMove}
	onmouseup={stopDrag}
/>

<div
	class="app-shell"
	ondragover={handleDragOver}
	ondrop={handleDrop}
	role="application"
>
	<!-- Menu Bar -->
	<MenuBar
		{recentFiles}
		{theme}
		onOpenFile={handleOpenFile}
		onSave={handleSave}
		onSaveAs={handleSaveAs}
		onCloseTab={() => handleCloseTab()}
		onCloseAllTabs={handleCloseAllTabs}
		onClearRecent={handleClearRecent}
		onOpenRecentFile={handleOpenRecentFile}
		onParse={handleParse}
		onToggleTree={handleToggleTree}
		onSetTheme={handleSetTheme}
		onSetLanguage={handleSetLanguage}
		onShowAbout={() => { showAbout = true; }}
	/>

	<!-- Main content area -->
	<div class="main-content">
		<!-- Tree panel -->
		{#if showTree}
			<div class="tree-panel" style="width: {treeWidth}px">
				{#if activeTab?.parseResult}
					<div class="panel-header">
						<span>{t('tree.header')}</span>
						<span class="panel-badge">{activeTab.parseResult.segment_count}</span>
					</div>
					<MessageTree
						messageId={activeTab.parseResult.message_id}
						roots={activeTab.parseResult.tree_roots}
						onNodeSelect={handleNodeSelect}
						onFieldExpand={handleFieldExpand}
					/>
				{:else}
					<div class="panel-header">
						<span>{t('tree.header')}</span>
					</div>
					<div class="panel-empty">
						<p>{t('tree.empty')}</p>
						<p class="shortcut-hint">{t('tree.shortcutHint')}</p>
					</div>
				{/if}
			</div>

			<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
			<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
			<div
				class="splitter"
				class:active={isDragging}
				role="separator"
				tabindex={0}
				onmousedown={startDrag}
			></div>
		{/if}

		<!-- Editor panel -->
		<div class="editor-panel">
			<!-- Tabs -->
			<EditorTabs
				tabs={messageStore.tabs}
				activeTabId={messageStore.activeTabId}
				onSelectTab={(id) => messageStore.setActiveTab(id)}
				onCloseTab={(id) => handleCloseTab(id)}
				onNewTab={handleNewTab}
			/>

			<!-- Editor -->
			<div class="editor-area">
				{#if activeTab}
					{#key activeTab.id}
						<MonacoEditor
							content={activeTab.content}
							theme={theme === 'light' ? 'bridgelab-light' : 'bridgelab-dark'}
							onContentChange={handleContentChange}
							onCursorChange={handleCursorChange}
						/>
					{/key}
				{:else}
					<div class="editor-empty">
						<p>{t('tree.empty')}</p>
					</div>
				{/if}
			</div>
		</div>
	</div>

	<!-- Expanded field modal -->
	{#if expandedFieldContent !== null}
		<div class="modal-overlay" onclick={closeExpandedField} role="presentation">
			<!-- svelte-ignore a11y_interactive_supports_focus -->
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="modal" onclick={(e) => e.stopPropagation()} role="dialog">
				<div class="modal-header">
					<span>{t('modal.fullContent')}</span>
					<button class="modal-close" onclick={closeExpandedField}>&times;</button>
				</div>
				<div class="modal-body">
					<pre>{expandedFieldContent}</pre>
				</div>
				<div class="modal-footer">
					<button class="btn" onclick={() => { navigator.clipboard.writeText(expandedFieldContent!); }}>
						{t('modal.copy')}
					</button>
					<span class="modal-info">{t('modal.characters', { count: expandedFieldContent.length.toLocaleString() })}</span>
				</div>
			</div>
		</div>
	{/if}

	<!-- About dialog -->
	{#if showAbout}
		<div class="modal-overlay" onclick={() => { showAbout = false; }} role="presentation">
			<!-- svelte-ignore a11y_interactive_supports_focus -->
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="modal modal-small" onclick={(e) => e.stopPropagation()} role="dialog">
				<div class="modal-header">
					<span>{t('about.title')}</span>
					<button class="modal-close" onclick={() => { showAbout = false; }}>&times;</button>
				</div>
				<div class="modal-body about-body">
					<div class="about-title">{t('app.title')}</div>
					<div class="about-subtitle">{t('app.subtitle')}</div>
					<div class="about-version">{t('about.version', { version: '0.2.0' })}</div>
					<p class="about-desc">{t('about.description')}</p>
					<p class="about-license">{t('about.license')}</p>
				</div>
			</div>
		</div>
	{/if}

	<!-- Status bar -->
	<StatusBar
		messageType={activeTab?.parseResult?.message_type}
		version={activeTab?.parseResult?.version}
		format={activeTab?.parseResult?.format}
		segmentCount={activeTab?.parseResult?.segment_count}
		fileSize={activeTab?.parseResult?.file_size_bytes}
		truncationCount={activeTab?.parseResult?.truncation_count}
		cursorLine={activeTab?.cursorLine}
		cursorColumn={activeTab?.cursorColumn}
	/>
</div>

<style>
	.app-shell {
		display: flex;
		flex-direction: column;
		height: 100vh;
		overflow: hidden;
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

	.editor-area {
		flex: 1;
		overflow: hidden;
	}

	.editor-empty {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--color-text-secondary);
		font-size: 14px;
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

	/* Buttons */
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

	.modal-small {
		width: 400px;
		max-width: 90%;
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

	/* About dialog */
	.about-body {
		text-align: center;
	}

	.about-title {
		font-size: 24px;
		font-weight: 700;
		color: var(--color-accent);
	}

	.about-subtitle {
		font-size: 14px;
		color: var(--color-text-secondary);
		font-style: italic;
		margin-bottom: 8px;
	}

	.about-version {
		font-size: 12px;
		color: var(--color-text-secondary);
		margin-bottom: 16px;
	}

	.about-desc {
		font-size: 13px;
		color: var(--color-text-primary);
		margin-bottom: 8px;
	}

	.about-license {
		font-size: 11px;
		color: var(--color-text-secondary);
	}
</style>
