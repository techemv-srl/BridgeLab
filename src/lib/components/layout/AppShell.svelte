<script lang="ts">
	import { /* onMount not used - resolves to server no-op */ } from 'svelte';
	import type { TreeNode, ParseResult } from '$lib/types/hl7';
	import { parseMessage, openFile, getFieldContent, getTreeChildren } from '$lib/ipc/parser';
	import { getRecentFiles, addRecentFile, clearRecentFiles, getPreference, setPreference } from '$lib/ipc/database';
	import { validateMessage, parseFhirMessage } from '$lib/ipc/validation';
	import { getMessageFullText, getMessageTruncatedText, exportAsJson, exportAsCsv } from '$lib/ipc/anonymization';
	import type { RecentFile } from '$lib/ipc/database';
	import type { ValidationIssue, ValidationReport } from '$lib/ipc/validation';
	import { t, setLocale, subscribeLocale, type Locale } from '$lib/i18n';
	import { messageStore, type MessageTab } from '$lib/stores/messages.svelte';
	import MonacoEditor from '$lib/components/editor/MonacoEditor.svelte';
	import MessageTree from '$lib/components/tree/MessageTree.svelte';
	import EditorTabs from '$lib/components/editor/EditorTabs.svelte';
	import MenuBar from '$lib/components/layout/MenuBar.svelte';
	import StatusBar from '$lib/components/layout/StatusBar.svelte';
	import ValidationPanel from '$lib/components/validation/ValidationPanel.svelte';
	import CommunicationPanel from '$lib/components/communication/CommunicationPanel.svelte';
	import AnonymizeDialog from '$lib/components/anonymization/AnonymizeDialog.svelte';
	import SettingsModal from '$lib/components/layout/SettingsModal.svelte';
	import TrialBanner from '$lib/components/licensing/TrialBanner.svelte';
	import ActivationDialog from '$lib/components/licensing/ActivationDialog.svelte';
	import { checkLicense, type LicenseStatus } from '$lib/ipc/licensing';

	// UI state
	let treeWidth = $state(350);
	let draggingTarget = $state<'tree' | 'bottom' | null>(null);
	let isDragging = $derived(draggingTarget !== null);
	let showTree = $state(true);
	let showValidation = $state(false);
	let showCommunication = $state(false);
	let bottomPanelHeight = $state(220);
	let expandedFieldContent = $state<string | null>(null);
	let showAbout = $state(false);
	let showAnonymize = $state(false);
	let showSettings = $state(false);
	let showActivation = $state(false);
	let licenseStatus = $state<LicenseStatus | null>(null);
	let recentFiles = $state<RecentFile[]>([]);
	let theme = $state('dark');
	let localeVersion = $state(0);

	// Subscribe to locale changes to force re-render
	if (typeof window !== 'undefined') {
		subscribeLocale(() => { localeVersion++; });
	}

	// Reactive translate function
	function tr(key: string, params?: Record<string, string | number>): string {
		// Reading localeVersion makes this reactive
		void localeVersion;
		return t(key, params);
	}

	// Validation state
	let validationReport = $state<ValidationReport | null>(null);

	// Reactive references to the active tab
	let activeTab = $derived(messageStore.activeTab);

	// Initialize app (using $effect instead of onMount which is a server no-op)
	let appInitialized = false;
	$effect(() => {
		if (appInitialized || typeof window === 'undefined') return;
		appInitialized = true;

		// Start with one empty tab immediately
		if (messageStore.tabs.length === 0) {
			messageStore.newTab();
		}

		// Load preferences and check license async
		(async () => {
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
			try {
				licenseStatus = await checkLicense();
			} catch {
				// License check failed - treat as trial
			}
		})();
	});

	async function handleCheckUpdates() {
		try {
			const { checkForUpdates, installUpdate } = await import('$lib/ipc/updater');
			const info = await checkForUpdates();
			if (info && info.available) {
				const ok = confirm(`Update available: v${info.version}\n\nCurrent: v${info.currentVersion}\n\n${info.notes}\n\nInstall now? The app will restart.`);
				if (ok) {
					await installUpdate(info.update);
				}
			} else {
				alert('You are running the latest version.');
			}
		} catch (e) {
			console.error('Update check failed:', e);
			alert('Could not check for updates. Please check your internet connection.');
		}
	}

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
				const path = typeof selected === 'string' ? selected : (selected as any).path ?? String(selected);
				console.log('[BridgeLab] Opening file:', path);
				const result = await openFile(path);
				console.log('[BridgeLab] Parse result:', result.message_type, result.format, result.segment_count, 'segments');
				skipNextAutoParse = true;
				messageStore.openMessage(result, path, result.truncated_text);
				// Track recent file
				const filename = path.split('/').pop()?.split('\\').pop() ?? '';
				try {
					await addRecentFile(path, filename, result.message_type, result.version, result.file_size_bytes);
					recentFiles = await getRecentFiles(20);
				} catch {
					// DB might not be available
				}
			}
		} catch (e) {
			console.error('[BridgeLab] Failed to open file:', e);
			// Show error in a new tab so user sees something
			const errMsg = String(e);
			if (messageStore.activeTabId) {
				messageStore.updateContent(messageStore.activeTabId, `Error opening file:\n${errMsg}`);
			}
		}
	}

	async function handleOpenRecentFile(path: string) {
		try {
			const result = await openFile(path);
			skipNextAutoParse = true;
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

	let skipNextAutoParse = false;
	let autoParseTimer: ReturnType<typeof setTimeout> | null = null;

	async function handleContentChange(value: string) {
		if (!messageStore.activeTabId) return;

		// Skip auto-parse if content was just set by file open / parse action
		if (skipNextAutoParse) {
			skipNextAutoParse = false;
			return;
		}

		messageStore.updateContent(messageStore.activeTabId, value);

		// Debounced auto-parse (500ms after user stops typing/pasting)
		if (autoParseTimer) clearTimeout(autoParseTimer);
		autoParseTimer = setTimeout(() => autoParse(value), 500);
	}

	async function autoParse(value: string) {
		if (!messageStore.activeTabId || !value || value.length < 10) return;
		const trimmed = value.trim();
		try {
			if (trimmed.startsWith('MSH|')) {
				const result = await parseMessage(value);
				skipNextAutoParse = true;
				messageStore.updateParseResult(messageStore.activeTabId!, result, result.truncated_text);
			} else if (trimmed.startsWith('{') && trimmed.includes('"resourceType"')) {
				const result = await parseFhirMessage(value);
				skipNextAutoParse = true;
				messageStore.updateParseResult(messageStore.activeTabId!, result, result.truncated_text);
			}
		} catch {
			// Not valid yet, ignore
		}
	}

	async function handleParse() {
		if (!activeTab?.content?.trim()) return;
		const content = activeTab.content.trim();
		try {
			let result: ParseResult;
			// Detect format
			if (content.startsWith('{') && content.includes('"resourceType"')) {
				result = await parseFhirMessage(content);
			} else {
				result = await parseMessage(content);
			}
			skipNextAutoParse = true;
			messageStore.updateParseResult(activeTab.id, result, result.truncated_text);
		} catch (e) {
			console.error('Parse error:', e);
		}
	}

	async function handleValidate() {
		if (!activeTab?.parseResult) return;
		try {
			if (activeTab.parseResult.format === 'HL7v2') {
				const report = await validateMessage(activeTab.parseResult.message_id);
				validationReport = report;
			}
			showValidation = true;
		} catch (e) {
			console.error('Validation error:', e);
		}
	}

	function handleValidationIssueClick(issue: ValidationIssue) {
		// Navigate to the segment in the tree/editor
		if (issue.segment_idx !== null && issue.segment_idx !== undefined) {
			// Scroll editor to the segment line (segments are 1-indexed lines)
			// The segment index maps roughly to line numbers in the truncated text
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

	/** Handle expand truncated: replace truncated text inline with full content.
	 *  fieldPositionStr is the HL7 field position number determined by counting pipes. */
	async function handleEditorExpandTruncated(lineNumber: number, fieldPositionStr: string) {
		if (!activeTab?.parseResult) return;
		const segIdx = lineNumber - 1;
		const msgId = activeTab.parseResult.message_id;
		const fieldPosition = parseInt(fieldPositionStr) || 0;

		try {
			const { expandFieldInline } = await import('$lib/ipc/parser');
			const expandedText = await expandFieldInline(msgId, segIdx, fieldPosition);
			if (messageStore.activeTabId) {
				skipNextAutoParse = true;
				messageStore.updateContent(messageStore.activeTabId, expandedText);
			}
		} catch (e) {
			console.error('Failed to expand field:', e);
		}
	}

	/** Expand ALL truncated fields inline */
	async function handleExpandAll() {
		if (!activeTab?.parseResult) return;
		try {
			const { expandAllFields } = await import('$lib/ipc/parser');
			const fullText = await expandAllFields(activeTab.parseResult.message_id);
			if (messageStore.activeTabId) {
				skipNextAutoParse = true;
				messageStore.updateContent(messageStore.activeTabId, fullText);
			}
		} catch (e) {
			console.error('Failed to expand all fields:', e);
		}
	}

	/** Re-truncate all expanded fields */
	async function handleCollapseAll() {
		if (!activeTab?.parseResult) return;
		try {
			const { collapseAllFields } = await import('$lib/ipc/parser');
			const truncatedText = await collapseAllFields(activeTab.parseResult.message_id);
			if (messageStore.activeTabId) {
				skipNextAutoParse = true;
				messageStore.updateContent(messageStore.activeTabId, truncatedText);
			}
		} catch (e) {
			console.error('Failed to collapse fields:', e);
		}
	}

	/** Handle "Show in Tree" - expand tree panel and select the segment */
	function handleEditorNavigateSegment(lineNumber: number, _segmentType: string) {
		showTree = true;
		// Trigger tree to expand/select segment at this index
		const segIdx = lineNumber - 1;
		if (activeTab?.parseResult) {
			selectedTreeSegmentIdx = segIdx;
		}
	}

	let selectedTreeSegmentIdx = $state<number | null>(null);

	// --- View operations ---

	function handleToggleTree() {
		showTree = !showTree;
	}

	// --- Anonymization / Copy / Export ---

	function handleShowAnonymize() {
		if (activeTab?.parseResult) showAnonymize = true;
	}

	function handleAnonymized(text: string) {
		showAnonymize = false;
		// Open anonymized text in a new tab
		messageStore.newTab();
		const newTab = messageStore.activeTab;
		if (newTab) {
			messageStore.updateContent(newTab.id, text);
			newTab.label = 'Anonymized';
		}
	}

	async function handleCopyFull() {
		if (!activeTab?.parseResult) return;
		try {
			const text = await getMessageFullText(activeTab.parseResult.message_id);
			await navigator.clipboard.writeText(text);
		} catch { /* fallback: copy editor content */
			if (activeTab?.content) await navigator.clipboard.writeText(activeTab.content);
		}
	}

	async function handleCopyTruncated() {
		if (!activeTab?.parseResult) return;
		try {
			const text = await getMessageTruncatedText(activeTab.parseResult.message_id, 100);
			await navigator.clipboard.writeText(text);
		} catch {
			// web mode fallback
		}
	}

	async function handleExportJson() {
		if (!activeTab?.parseResult) return;
		try {
			const json = await exportAsJson(activeTab.parseResult.message_id);
			downloadFile(json, `${activeTab.label || 'message'}.json`, 'application/json');
		} catch (e) { console.error('Export JSON failed:', e); }
	}

	async function handleExportCsv() {
		if (!activeTab?.parseResult) return;
		try {
			const csv = await exportAsCsv(activeTab.parseResult.message_id);
			downloadFile(csv, `${activeTab.label || 'message'}.csv`, 'text/csv');
		} catch (e) { console.error('Export CSV failed:', e); }
	}

	function downloadFile(content: string, filename: string, mimeType: string) {
		const blob = new Blob([content], { type: mimeType });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = filename;
		a.click();
		URL.revokeObjectURL(url);
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
		draggingTarget = 'tree';
		e.preventDefault();
	}

	function startBottomDrag(e: MouseEvent) {
		draggingTarget = 'bottom';
		e.preventDefault();
	}

	function handleMouseMove(e: MouseEvent) {
		if (draggingTarget === 'tree') {
			treeWidth = Math.max(200, Math.min(600, e.clientX));
		} else if (draggingTarget === 'bottom') {
			const windowHeight = window.innerHeight;
			const newHeight = windowHeight - e.clientY - 24; // 24 = status bar height
			bottomPanelHeight = Math.max(100, Math.min(windowHeight * 0.7, newHeight));
		}
	}

	async function stopDrag() {
		if (draggingTarget === 'tree') {
			try { await setPreference('tree_width', String(treeWidth)); } catch { /* web mode */ }
		}
		draggingTarget = null;
	}

	// --- Paste handler (fallback for when Monaco doesn't have focus) ---

	async function handlePaste(e: ClipboardEvent) {
		// Only intercept if Monaco doesn't have focus
		const activeEl = document.activeElement;
		const isMonacoFocused = activeEl?.closest('.editor-container') ||
			activeEl?.classList.contains('monaco-editor') ||
			activeEl?.closest('.monaco-editor');

		if (isMonacoFocused) return; // Let Monaco handle it

		const text = e.clipboardData?.getData('text/plain');
		if (!text || !messageStore.activeTabId) return;

		e.preventDefault();
		messageStore.updateContent(messageStore.activeTabId, text);

		// Trigger auto-parse
		await autoParse(text);
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
		else if (e.key === 'F6') { e.preventDefault(); handleValidate(); }
		else if (ctrl && e.key === 'j') { e.preventDefault(); showValidation = !showValidation; }
		else if (ctrl && e.key === 'k') { e.preventDefault(); showCommunication = !showCommunication; }
		else if (ctrl && e.key === ',') { e.preventDefault(); showSettings = true; }
	}
</script>

<svelte:window
	onkeydown={handleKeydown}
	onmousemove={handleMouseMove}
	onmouseup={stopDrag}
	onpaste={handlePaste}
/>

<div
	class="app-shell"
	ondragover={handleDragOver}
	ondrop={handleDrop}
	role="application"
>
	<!-- Trial/License Banner -->
	{#if licenseStatus}
		<TrialBanner status={licenseStatus} onActivate={() => { showActivation = true; }} />
	{/if}

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
		onValidate={handleValidate}
		onToggleValidation={() => { showValidation = !showValidation; }}
		onToggleCommunication={() => { showCommunication = !showCommunication; }}
		onAnonymize={handleShowAnonymize}
		onCopyFull={handleCopyFull}
		onCopyTruncated={handleCopyTruncated}
		onExportJson={handleExportJson}
		onExportCsv={handleExportCsv}
		onToggleTree={handleToggleTree}
		onSetTheme={handleSetTheme}
		onSetLanguage={handleSetLanguage}
		onShowSettings={() => { showSettings = true; }}
		onCheckUpdates={handleCheckUpdates}
		onShowAbout={() => { showAbout = true; }}
	/>

	<!-- Main content area -->
	<div class="main-content">
		<!-- Tree panel -->
		{#if showTree}
			<div class="tree-panel" style="width: {treeWidth}px">
				{#if activeTab?.parseResult}
					<div class="panel-header">
						<span>{tr('tree.header')}</span>
						<span class="panel-badge">{activeTab.parseResult.segment_count}</span>
					</div>
					<MessageTree
						messageId={activeTab.parseResult.message_id}
						roots={activeTab.parseResult.tree_roots}
						onNodeSelect={handleNodeSelect}
						onFieldExpand={handleFieldExpand}
						navigateToSegmentIdx={selectedTreeSegmentIdx}
					/>
				{:else}
					<div class="panel-header">
						<span>{tr('tree.header')}</span>
					</div>
					<div class="panel-empty">
						<p>{tr('tree.empty')}</p>
						<p class="shortcut-hint">{tr('tree.shortcutHint')}</p>
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
					<MonacoEditor
						content={activeTab.content}
						theme={theme === 'light' ? 'bridgelab-light' : 'bridgelab-dark'}
						onContentChange={handleContentChange}
						onCursorChange={handleCursorChange}
						onExpandTruncated={handleEditorExpandTruncated}
						onExpandAll={handleExpandAll}
						onNavigateToSegment={handleEditorNavigateSegment}
						onCollapseAll={handleCollapseAll}
						onCopyFullMessage={handleCopyFull}
						onCopyTruncatedMessage={handleCopyTruncated}
					/>
				{:else}
					<div class="editor-empty">
						<p>{tr('tree.empty')}</p>
					</div>
				{/if}
			</div>

			<!-- Bottom Panels (Validation / Communication) -->
			{#if (showValidation && validationReport) || showCommunication}
				<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
				<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
				<div
					class="bottom-splitter"
					onmousedown={startBottomDrag}
					role="separator"
					tabindex={0}
				></div>
			{/if}

			{#if showValidation && validationReport}
				<div class="bottom-panel" style="height: {bottomPanelHeight}px">
					<div class="panel-header">
						<span>Validation</span>
						<button class="panel-close" onclick={() => { showValidation = false; }}>&times;</button>
					</div>
					<ValidationPanel
						issues={validationReport.issues}
						errorCount={validationReport.error_count}
						warningCount={validationReport.warning_count}
						infoCount={validationReport.info_count}
						onIssueClick={handleValidationIssueClick}
					/>
				</div>
			{/if}

			{#if showCommunication}
				<div class="bottom-panel" style="height: {bottomPanelHeight}px">
					<div class="panel-header">
						<span>Communication</span>
						<button class="panel-close" onclick={() => { showCommunication = false; }}>&times;</button>
					</div>
					<CommunicationPanel
						currentMessage={activeTab?.content ?? ''}
						activeTabLabel={activeTab?.label ?? ''}
						onMessageReceived={(content) => {
							if (messageStore.activeTabId) {
								messageStore.updateContent(messageStore.activeTabId, content);
							}
						}}
					/>
				</div>
			{/if}
		</div>
	</div>

	<!-- Expanded field modal -->
	{#if expandedFieldContent !== null}
		<div class="modal-overlay" onclick={closeExpandedField} role="presentation">
			<!-- svelte-ignore a11y_interactive_supports_focus -->
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="modal" onclick={(e) => e.stopPropagation()} role="dialog">
				<div class="modal-header">
					<span>{tr('modal.fullContent')}</span>
					<button class="modal-close" onclick={closeExpandedField}>&times;</button>
				</div>
				<div class="modal-body">
					<pre>{expandedFieldContent}</pre>
				</div>
				<div class="modal-footer">
					<button class="btn" onclick={() => { navigator.clipboard.writeText(expandedFieldContent!); }}>
						{tr('modal.copy')}
					</button>
					<span class="modal-info">{tr('modal.characters', { count: expandedFieldContent.length.toLocaleString() })}</span>
				</div>
			</div>
		</div>
	{/if}

	<!-- Anonymize dialog -->
	{#if showAnonymize && activeTab?.parseResult}
		<div class="modal-overlay" onclick={() => { showAnonymize = false; }} role="presentation">
			<!-- svelte-ignore a11y_interactive_supports_focus -->
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="modal" onclick={(e) => e.stopPropagation()} role="dialog">
				<AnonymizeDialog
					messageId={activeTab.parseResult.message_id}
					onAnonymized={handleAnonymized}
					onClose={() => { showAnonymize = false; }}
				/>
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
					<span>{tr('about.title')}</span>
					<button class="modal-close" onclick={() => { showAbout = false; }}>&times;</button>
				</div>
				<div class="modal-body about-body">
					<div class="about-title">{tr('app.title')}</div>
					<div class="about-subtitle">{tr('app.subtitle')}</div>
					<div class="about-version">{tr('about.version', { version: '0.2.0' })}</div>
					<p class="about-desc">{tr('about.description')}</p>
					<p class="about-license">{tr('about.license')}</p>
					<p class="about-copyright">{tr('about.copyright', { year: new Date().getFullYear().toString() })}</p>
				</div>
			</div>
		</div>
	{/if}

	<!-- License Activation modal -->
	{#if showActivation && licenseStatus}
		<div class="modal-overlay" onclick={() => { showActivation = false; }} role="presentation">
			<!-- svelte-ignore a11y_interactive_supports_focus -->
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="modal modal-lg" onclick={(e) => e.stopPropagation()} role="dialog">
				<ActivationDialog
					currentStatus={licenseStatus}
					onClose={() => { showActivation = false; }}
					onStatusChange={(s) => { licenseStatus = s; }}
				/>
			</div>
		</div>
	{/if}

	<!-- Settings modal -->
	{#if showSettings}
		<div class="modal-overlay" onclick={() => { showSettings = false; }} role="presentation">
			<!-- svelte-ignore a11y_interactive_supports_focus -->
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="modal modal-lg" onclick={(e) => e.stopPropagation()} role="dialog">
				<SettingsModal
					{theme}
					onClose={() => { showSettings = false; }}
					onThemeChange={handleSetTheme}
				/>
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

	.bottom-splitter {
		height: 4px;
		cursor: ns-resize;
		background-color: var(--color-border);
		flex-shrink: 0;
	}

	.bottom-splitter:hover {
		background-color: var(--color-accent);
	}

	.bottom-panel {
		display: flex;
		flex-direction: column;
		flex-shrink: 0;
		border-top: 1px solid var(--color-border);
		overflow: hidden;
	}

	.panel-close {
		background: none;
		border: none;
		color: var(--color-text-secondary);
		cursor: pointer;
		font-size: 14px;
		line-height: 1;
		padding: 0 4px;
	}

	.panel-close:hover {
		color: var(--color-error);
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

	.modal-lg {
		width: 700px;
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

	.about-copyright {
		font-size: 10px;
		color: var(--color-text-secondary);
		margin-top: 12px;
		opacity: 0.7;
	}
</style>
