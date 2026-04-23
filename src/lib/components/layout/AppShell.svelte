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
	import { shortcutStore, matchesKeys } from '$lib/stores/shortcuts.svelte';
	import { dialogStore } from '$lib/stores/dialog.svelte';
	import { parseUpgradeError } from '$lib/ipc/licensing';
	import AppDialog from '$lib/components/shared/AppDialog.svelte';
	import MonacoEditor from '$lib/components/editor/MonacoEditor.svelte';
	import MessageTree from '$lib/components/tree/MessageTree.svelte';
	import FieldInspector from '$lib/components/tree/FieldInspector.svelte';
	import EditorTabs from '$lib/components/editor/EditorTabs.svelte';
	import MenuBar from '$lib/components/layout/MenuBar.svelte';
	import StatusBar from '$lib/components/layout/StatusBar.svelte';
	import ValidationPanel from '$lib/components/validation/ValidationPanel.svelte';
	import CommunicationPanel from '$lib/components/communication/CommunicationPanel.svelte';
	import AnonymizeDialog from '$lib/components/anonymization/AnonymizeDialog.svelte';
	import SettingsModal from '$lib/components/layout/SettingsModal.svelte';
	import TrialBanner from '$lib/components/licensing/TrialBanner.svelte';
	import ActivationDialog from '$lib/components/licensing/ActivationDialog.svelte';
	import TemplateDialog from '$lib/components/templates/TemplateDialog.svelte';
	import BundleVisualizer from '$lib/components/bundle/BundleVisualizer.svelte';
	import FhirPathPanel from '$lib/components/fhirpath/FhirPathPanel.svelte';
	import TestCaseLibrary from '$lib/components/testcases/TestCaseLibrary.svelte';
	import type { TestCase } from '$lib/ipc/testcases';
	import { checkLicense, type LicenseStatus } from '$lib/ipc/licensing';
	import type { MessageTemplate } from '$lib/ipc/templates';

	// UI state
	let treeWidth = $state(350);
	let draggingTarget = $state<'tree' | 'bottom' | 'inspector' | null>(null);
	let isDragging = $derived(draggingTarget !== null);
	let showTree = $state(true);
	let showInspector = $state(true);
	let showSchemaFields = $state(false);
	let showValidation = $state(false);
	let showCommunication = $state(false);
	let bottomPanelHeight = $state(220);
	let inspectorHeight = $state(260);
	let expandedFieldContent = $state<string | null>(null);
	let showAbout = $state(false);
	let showAnonymize = $state(false);
	let showSettings = $state(false);
	let showActivation = $state(false);
	let showTemplates = $state(false);
	let showBundleVisualizer = $state(false);
	let showFhirPath = $state(false);
	let showTestCases = $state(false);
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
	let restoreSession = $state(true);
	$effect(() => {
		if (appInitialized || typeof window === 'undefined') return;
		appInitialized = true;

		// Load preferences and check license async. We intentionally do NOT
		// create the default "Untitled" tab synchronously here - if a previous
		// session exists we want to restore it instead.
		(async () => {
			let sessionRestored = false;
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
				const savedInspectorHeight = await getPreference('inspector_height');
				if (savedInspectorHeight) inspectorHeight = parseInt(savedInspectorHeight) || 260;
				const savedRestore = await getPreference('restore_session');
				if (savedRestore !== null) restoreSession = savedRestore !== 'false';
				recentFiles = await getRecentFiles(20);

				// Apply plugin enable/disable overrides (stored as plugin_enabled:<id>)
				try {
					const { getAllPreferences } = await import('$lib/ipc/database');
					const { applyPluginOverrides } = await import('$lib/ipc/plugins');
					const prefs = await getAllPreferences();
					const overrides: Record<string, boolean> = {};
					for (const p of prefs) {
						if (p.key.startsWith('plugin_enabled:')) {
							const id = p.key.slice('plugin_enabled:'.length);
							overrides[id] = p.value !== 'false';
						}
					}
					if (Object.keys(overrides).length > 0) {
						await applyPluginOverrides(overrides);
					}
				} catch { /* web mode */ }

				// Notepad++-style tab restore
				if (restoreSession) {
					const { loadSession } = await import('$lib/ipc/database');
					const sessionTabs = await loadSession();
					if (sessionTabs && sessionTabs.length > 0) {
						sessionRestored = messageStore.restoreSession(sessionTabs);
						// Re-parse any HL7/FHIR content so tree + inspector populate
						for (const tab of messageStore.tabs) {
							void autoParse(tab.content);
						}
					}
				}
			} catch {
				// Running in web-only mode without Tauri backend
			}

			// Fallback: empty tab if no session restored
			if (!sessionRestored && messageStore.tabs.length === 0) {
				messageStore.newTab();
			}

			try {
				licenseStatus = await checkLicense();
			await shortcutStore.loadFromPrefs();
			} catch {
				// License check failed - treat as trial
			}
		})();
	});

	// Session autosave: persist open tabs whenever they change, debounced.
	let sessionSaveTimer: ReturnType<typeof setTimeout> | null = null;
	$effect(() => {
		if (!appInitialized || typeof window === 'undefined') return;
		if (!restoreSession) return;
		// Track tabs + active id as dependencies
		void messageStore.tabs;
		void messageStore.activeTabId;
		for (const t of messageStore.tabs) {
			void t.content;
			void t.label;
			void t.filePath;
			void t.isModified;
		}
		if (sessionSaveTimer) clearTimeout(sessionSaveTimer);
		sessionSaveTimer = setTimeout(async () => {
			try {
				const { saveSession } = await import('$lib/ipc/database');
				await saveSession(messageStore.serializeSession());
			} catch {
				// web mode or backend unavailable - ignore
			}
		}, 800);
	});

	function handleTestCaseLoaded(tc: TestCase) {
		showTestCases = false;
		messageStore.newTab();
		const newTab = messageStore.activeTab;
		if (newTab) {
			skipNextAutoParse = true;
			messageStore.updateContent(newTab.id, tc.content);
			newTab.label = tc.name;
			autoParse(tc.content);
		}
	}

	function handleTemplateSelected(template: MessageTemplate) {
		showTemplates = false;
		// Open in new tab
		messageStore.newTab();
		const newTab = messageStore.activeTab;
		if (newTab) {
			skipNextAutoParse = true;
			messageStore.updateContent(newTab.id, template.content);
			newTab.label = template.name.split(' - ')[0] || template.name;
			// Trigger parse
			autoParse(template.content);
		}
	}

	function handleCheckUpdates() {
		window.open('https://github.com/techemv-srl/BridgeLab/releases', '_blank');
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
		if (!activeTab) return;
		// If tab has no file path (Untitled / from paste/template), fall back to Save As
		if (!activeTab.filePath) {
			await handleSaveAs();
			return;
		}
		try {
			const { saveFile } = await import('$lib/ipc/parser');
			await saveFile({
				path: activeTab.filePath,
				content: activeTab.content, // save current editor text, not the parsed store
			});
			messageStore.markSaved(activeTab.id);
			console.log('[BridgeLab] Saved to:', activeTab.filePath);
		} catch (e) {
			console.error('Save failed:', e);
			await dialogStore.error(t('dialog.saveFailed'), undefined, String(e));
		}
	}

	async function handleSaveAs() {
		if (!activeTab) return;
		try {
			const { save } = await import('@tauri-apps/plugin-dialog');
			const path = await save({
				defaultPath: activeTab.filePath ?? activeTab.label,
				filters: [
					{ name: 'HL7 Messages', extensions: ['hl7'] },
					{ name: 'All Files', extensions: ['*'] },
				],
			});
			if (path) {
				const { saveFile } = await import('$lib/ipc/parser');
				await saveFile({ path, content: activeTab.content });
				messageStore.markSaved(activeTab.id, path);
				console.log('[BridgeLab] Saved as:', path);
			}
		} catch (e) {
			console.error('Save As failed:', e);
			await dialogStore.error(t('dialog.saveAsFailed'), undefined, String(e));
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

		// Always save the current editor text to the tab - even if autoparse should be skipped.
		// Previously this return was above updateContent, causing user edits to be lost.
		messageStore.updateContent(messageStore.activeTabId, value);

		// Skip auto-parse if content was just set by file open / parse action
		if (skipNextAutoParse) {
			skipNextAutoParse = false;
			return;
		}

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
				// Background parse while user is typing: update parseResult only,
				// do NOT replace editor content (would reset cursor to 1:1).
				messageStore.updateParseResult(messageStore.activeTabId!, result);
			} else if (trimmed.startsWith('{') && trimmed.includes('"resourceType"')) {
				const result = await parseFhirMessage(value);
				messageStore.updateParseResult(messageStore.activeTabId!, result);
			}
		} catch {
			// Not valid yet, ignore
		}
	}

	/**
	 * Parse Message (F5): alias for Validate. We open the validation panel which
	 * always parses fresh and shows all issues (including parse errors). This
	 * avoids duplication between "parse" and "validate".
	 */
	async function handleParse() {
		await handleValidate();
	}

	/**
	 * Validate the current message. Always parses fresh from the editor content
	 * rather than relying on cached parseResult (which could be stale if the
	 * user edited the text but the parse failed).
	 */
	async function handleValidate() {
		if (!activeTab?.content?.trim()) {
			await dialogStore.warning(t('dialog.noMessageToValidate'));
			return;
		}
		showValidation = true;
		const content = activeTab.content;
		const trimmed = content.trim();

		// FHIR branch
		if (trimmed.startsWith('{') && trimmed.includes('"resourceType"')) {
			try {
				const result = await parseFhirMessage(trimmed);
				skipNextAutoParse = true;
				messageStore.updateParseResult(activeTab.id, result, result.truncated_text);
				// TODO: run FHIR-specific validation rules
				validationReport = {
					issues: [{
						severity: 'info',
						rule_id: 'FHIR-OK',
						segment_idx: null,
						segment_type: null,
						field_position: null,
						message: `FHIR ${result.message_type} parsed successfully`,
					}],
					error_count: 0, warning_count: 0, info_count: 1,
				};
			} catch (e) {
				validationReport = buildSyntheticReport(content, String(e));
			}
			return;
		}

		// HL7 v2 branch: try to parse fresh
		try {
			const result = await parseMessage(content);
			skipNextAutoParse = true;
			messageStore.updateParseResult(activeTab.id, result, result.truncated_text);
			try {
				validationReport = await validateMessage(result.message_id);
			} catch (ve) {
				console.error('Validation IPC error:', ve);
				validationReport = buildSyntheticReport(content, String(ve));
			}
		} catch (e) {
			// Parse failed - produce a detailed synthetic report explaining why
			console.error('Parse error:', e);
			validationReport = buildSyntheticReport(content, String(e));
		}
	}

	/** Build a synthetic validation report when parsing fails. */
	function buildSyntheticReport(content: string, parseError: string): ValidationReport {
		const issues: ValidationIssue[] = [];
		const firstLine = content.split(/[\r\n]/)[0] ?? '';
		const firstSegType = firstLine.substring(0, 3);

		if (firstLine.length < 8) {
			issues.push({
				severity: 'error', rule_id: 'STRUCT-001',
				segment_idx: null, segment_type: null, field_position: null,
				message: t('val.tooShort'),
			});
		} else if (!firstLine.startsWith('MSH|')) {
			issues.push({
				severity: 'error', rule_id: 'STRUCT-002',
				segment_idx: 0, segment_type: firstSegType || null, field_position: null,
				message: t('val.notMshStart', { found: firstSegType }),
			});
			issues.push({
				severity: 'info', rule_id: 'HINT-001',
				segment_idx: null, segment_type: null, field_position: null,
				message: t('val.parseFailedHint', { prefix: firstSegType }),
			});
		} else {
			issues.push({
				severity: 'error', rule_id: 'PARSE-001',
				segment_idx: null, segment_type: null, field_position: null,
				message: t('val.genericParseError', { error: parseError }),
			});
		}

		return {
			issues,
			error_count: issues.filter(i => i.severity === 'error').length,
			warning_count: issues.filter(i => i.severity === 'warning').length,
			info_count: issues.filter(i => i.severity === 'info').length,
		};
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

	/** Currently selected tree node (for Field Inspector) */
	let selectedTreeNode = $state<TreeNode | null>(null);

	function handleNodeSelect(node: TreeNode) {
		selectedTreeNode = node;
	}

	/** Derive the segment type code (e.g. "PID") for the currently selected tree node. */
	let selectedSegmentType = $derived.by<string | null>(() => {
		if (!selectedTreeNode || !activeTab?.parseResult) return null;
		const parts = selectedTreeNode.id.split('.');
		const segPart = parts.find((p) => p.startsWith('seg'));
		if (!segPart) return null;
		const segIdx = parseInt(segPart.slice(3));
		const segNode = activeTab.parseResult.tree_roots[segIdx];
		if (!segNode) return null;
		// Segment label is "MSH (0)" / "PID (1)" — take the 3-char code
		const m = segNode.label.match(/^([A-Z][A-Z0-9]{2})/);
		return m ? m[1] : null;
	});

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

	/** Handle "Show in Tree" - expand tree panel and select the segment + optional field */
	function handleEditorNavigateSegment(lineNumber: number, _segmentType: string, fieldPosition?: number) {
		showTree = true;
		const segIdx = lineNumber - 1;
		if (activeTab?.parseResult) {
			treeNavigation = {
				segmentIdx: segIdx,
				fieldPosition: fieldPosition ?? null,
				stamp: Date.now(),  // stamp to force re-trigger even on same target
			};
		}
	}

	/** Tree navigation request from editor: includes segment index and optional field position */
	let treeNavigation = $state<{ segmentIdx: number; fieldPosition: number | null; stamp: number } | null>(null);

	/** Editor navigation request from tree: scrolls Monaco to a specific position */
	let editorNavigation = $state<{ line: number; column: number; selectionLength: number; stamp: number } | null>(null);

	/** Handle a tree node requesting to show its position in the editor */
	function handleTreeNavigateToEditor(segmentIdx: number, fieldPosition: number | null, componentIdx: number | null) {
		if (!activeTab?.parseResult) return;
		const text = activeTab.content;
		const lines = text.split(/\r\n|\r|\n/);
		if (segmentIdx >= lines.length) return;

		const line = lines[segmentIdx];
		const lineNumber = segmentIdx + 1;
		let column = 1;
		let selectionLength = line.length;

		if (fieldPosition !== null && fieldPosition !== undefined) {
			// Find the start column of this field by counting pipes
			const isMsh = line.startsWith('MSH');
			let pipeIdx = 0;
			let cursor = 0;

			if (isMsh && fieldPosition === 1) {
				// MSH-1 is the field separator at position 4
				column = 4;
				selectionLength = 1;
			} else if (isMsh && fieldPosition === 2) {
				// MSH-2 is the encoding chars at position 5
				column = 5;
				selectionLength = 4;
			} else {
				// For non-MSH segments: pipes start counting after segment name
				// fieldPosition 1 = first field after first pipe
				// For MSH: fieldPosition 3 = third field after the encoding chars
				const targetPipe = isMsh ? fieldPosition - 1 : fieldPosition;
				while (cursor < line.length && pipeIdx < targetPipe) {
					if (line[cursor] === '|') pipeIdx++;
					cursor++;
				}
				column = cursor + 1;
				// Find the end of this field (next pipe or end of line)
				let end = cursor;
				while (end < line.length && line[end] !== '|') end++;
				selectionLength = Math.max(1, end - cursor);

				// Optionally narrow to component
				if (componentIdx !== null && componentIdx !== undefined && componentIdx > 0) {
					const fieldText = line.substring(cursor, end);
					const components = fieldText.split('^');
					if (componentIdx <= components.length) {
						let compStart = 0;
						for (let i = 0; i < componentIdx - 1; i++) {
							compStart += components[i].length + 1; // +1 for '^'
						}
						column = cursor + compStart + 1;
						selectionLength = Math.max(1, components[componentIdx - 1].length);
					}
				}
			}
		}

		editorNavigation = { line: lineNumber, column, selectionLength, stamp: Date.now() };
	}

	// --- View operations ---

	function handleToggleTree() {
		showTree = !showTree;
	}

	// --- Upgrade prompt helper ---

	async function handleUpgradeError(err: unknown): Promise<boolean> {
		const upgrade = parseUpgradeError(err);
		if (upgrade) {
			await dialogStore.info(
				`This feature requires a ${upgrade.tier} license.\n\n` +
				`Upgrade via Settings → Activation or contact info@techemv.it.`
			);
			return true;
		}
		return false;
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
		} catch (e) {
			if (!await handleUpgradeError(e)) console.error('Export JSON failed:', e);
		}
	}

	async function handleExportCsv() {
		if (!activeTab?.parseResult) return;
		try {
			const csv = await exportAsCsv(activeTab.parseResult.message_id);
			downloadFile(csv, `${activeTab.label || 'message'}.csv`, 'text/csv');
		} catch (e) {
			if (!await handleUpgradeError(e)) console.error('Export CSV failed:', e);
		}
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

	function startInspectorDrag(e: MouseEvent) {
		draggingTarget = 'inspector';
		e.preventDefault();
	}

	function handleMouseMove(e: MouseEvent) {
		if (draggingTarget === 'tree') {
			treeWidth = Math.max(200, Math.min(600, e.clientX));
		} else if (draggingTarget === 'bottom') {
			const windowHeight = window.innerHeight;
			const newHeight = windowHeight - e.clientY - 24; // 24 = status bar height
			bottomPanelHeight = Math.max(100, Math.min(windowHeight * 0.7, newHeight));
		} else if (draggingTarget === 'inspector') {
			const windowHeight = window.innerHeight;
			// Inspector is anchored to the bottom of the tree panel (above status bar)
			const newHeight = windowHeight - e.clientY - 24;
			inspectorHeight = Math.max(100, Math.min(windowHeight * 0.8, newHeight));
		}
	}

	async function stopDrag() {
		if (draggingTarget === 'tree') {
			try { await setPreference('tree_width', String(treeWidth)); } catch { /* web mode */ }
		} else if (draggingTarget === 'inspector') {
			try { await setPreference('inspector_height', String(inspectorHeight)); } catch { /* web mode */ }
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

	/** Action handlers mapped by shortcut id. */
	const shortcutActions: Record<string, () => void> = {
		'file.open': () => handleOpenFile(),
		'file.save': () => handleSave(),
		'file.saveAs': () => handleSaveAs(),
		'file.closeTab': () => handleCloseTab(),
		'file.newFromTemplate': () => { showTemplates = true; },
		'file.testCases': () => { showTestCases = true; },
		'edit.settings': () => { showSettings = true; },
		'view.toggleTree': () => handleToggleTree(),
		'view.toggleValidation': () => { showValidation = !showValidation; },
		'view.toggleCommunication': () => { showCommunication = !showCommunication; },
		'view.toggleFhirPath': () => { showFhirPath = !showFhirPath; },
		'tools.reparse': () => handleParse(),
		'tools.validate': () => handleValidate(),
	};

	function handleKeydown(e: KeyboardEvent) {
		// Always block F5 in Tauri WebView - it would reload the entire app and lose state
		if (e.key === 'F5') {
			e.preventDefault();
			// If user has F5 assigned to an action, run it; otherwise silently block
			const shortcut = Object.entries(shortcutActions).find(
				([id]) => shortcutStore.get(id) === 'F5'
			);
			if (shortcut) shortcut[1]();
			return;
		}
		// Iterate through shortcut store to find a match; respects user customization
		for (const [id, action] of Object.entries(shortcutActions)) {
			const keys = shortcutStore.get(id);
			if (keys && matchesKeys(e, keys)) {
				e.preventDefault();
				action();
				return;
			}
		}
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
		onNewFromTemplate={() => { showTemplates = true; }}
		onShowTestCases={() => { showTestCases = true; }}
		onParse={handleParse}
		onValidate={handleValidate}
		onToggleValidation={() => { showValidation = !showValidation; }}
		onToggleCommunication={() => { showCommunication = !showCommunication; }}
		onAnonymize={handleShowAnonymize}
		onShowBundleVisualizer={() => { showBundleVisualizer = true; }}
		onToggleFhirPath={() => { showFhirPath = !showFhirPath; }}
		onCopyFull={handleCopyFull}
		onCopyTruncated={handleCopyTruncated}
		onExportJson={handleExportJson}
		onExportCsv={handleExportCsv}
		onToggleTree={handleToggleTree}
		onToggleInspector={() => { showInspector = !showInspector; }}
		onToggleSchemaFields={() => { showSchemaFields = !showSchemaFields; }}
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
						<button
							class="inspector-toggle"
							class:active={showInspector}
							title={tr('inspector.title')}
							aria-label={tr('inspector.title')}
							onclick={() => { showInspector = !showInspector; }}
						>
							<svg width="14" height="14" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
								<rect x="1.5" y="2.5" width="13" height="11" rx="1.5" stroke="currentColor" stroke-width="1.2"/>
								<line x1="1.5" y1="7" x2="14.5" y2="7" stroke="currentColor" stroke-width="1.2"/>
								<line x1="4" y1="9.5" x2="12" y2="9.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
								<line x1="4" y1="11.5" x2="10" y2="11.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
							</svg>
						</button>
					</div>
					<div class="tree-scroll">
						<MessageTree
							messageId={activeTab.parseResult.message_id}
							roots={activeTab.parseResult.tree_roots}
							version={activeTab.parseResult.version}
							showSchemaFields={showSchemaFields}
							onNodeSelect={handleNodeSelect}
							onFieldExpand={handleFieldExpand}
							navigateTo={treeNavigation}
							onNavigateToEditor={handleTreeNavigateToEditor}
						/>
					</div>
					{#if showInspector}
						<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
						<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
						<div
							class="inspector-splitter"
							class:active={draggingTarget === 'inspector'}
							role="separator"
							tabindex={0}
							aria-orientation="horizontal"
							onmousedown={startInspectorDrag}
							title="Drag to resize"
						></div>
						<div class="inspector-wrapper" style="height: {inspectorHeight}px">
							<FieldInspector
								messageId={activeTab.parseResult.message_id}
								version={activeTab.parseResult.version}
								selectedNode={selectedTreeNode}
								segmentType={selectedSegmentType}
								onViewFullValue={(text) => { expandedFieldContent = text; }}
							/>
						</div>
					{/if}
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
						navigation={editorNavigation}
					/>
				{:else}
					<div class="editor-empty">
						<p>{tr('tree.empty')}</p>
					</div>
				{/if}
			</div>

			<!-- Bottom Panels (Validation / Communication) -->
			{#if (showValidation && validationReport) || showCommunication || showFhirPath}
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

			{#if showFhirPath && activeTab?.parseResult}
				<div class="bottom-panel" style="height: {bottomPanelHeight}px">
					<div class="panel-header">
						<span>FHIRPath</span>
						<button class="panel-close" onclick={() => { showFhirPath = false; }}>&times;</button>
					</div>
					<FhirPathPanel messageId={activeTab.parseResult.message_id} />
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
					<!-- Bridge logo -->
					<div class="about-logo" aria-hidden="true">
						<svg viewBox="0 0 120 50" width="120" height="50" fill="none" xmlns="http://www.w3.org/2000/svg">
							<defs>
								<linearGradient id="bridge-grad" x1="0%" y1="0%" x2="100%" y2="0%">
									<stop offset="0%" stop-color="var(--color-accent, #89b4fa)"/>
									<stop offset="100%" stop-color="var(--color-segment, #cba6f7)"/>
								</linearGradient>
							</defs>
							<path d="M10 32c15-24 75-24 100 0" stroke="url(#bridge-grad)" stroke-width="3.5" stroke-linecap="round"/>
							<line x1="28" y1="28" x2="28" y2="42" stroke="var(--color-accent, #89b4fa)" stroke-width="3" stroke-linecap="round"/>
							<line x1="48" y1="22" x2="48" y2="42" stroke="var(--color-accent, #89b4fa)" stroke-width="3" stroke-linecap="round"/>
							<line x1="72" y1="22" x2="72" y2="42" stroke="var(--color-accent, #89b4fa)" stroke-width="3" stroke-linecap="round"/>
							<line x1="92" y1="28" x2="92" y2="42" stroke="var(--color-accent, #89b4fa)" stroke-width="3" stroke-linecap="round"/>
							<rect x="8" y="42" width="104" height="4" rx="2" fill="url(#bridge-grad)"/>
						</svg>
					</div>

					<div class="about-title">{tr('app.title')}</div>
					<div class="about-subtitle">{tr('app.subtitle')}</div>
					<div class="about-version">{tr('about.version', { version: '0.1.0' })}</div>
					<p class="about-desc">{tr('about.description')}</p>
					<p class="about-license">{tr('about.license')}</p>

					<div class="about-company">
						<div class="about-company-name">TECHEMV SRL</div>
						<div class="about-contact">
							<a href="mailto:info@techemv.it">info@techemv.it</a>
							<span class="about-sep">&middot;</span>
							<a href="https://www.techemv.it" target="_blank" rel="noopener">www.techemv.it</a>
						</div>
					</div>

					<p class="about-copyright">{tr('about.copyright', { year: new Date().getFullYear().toString() })}</p>
				</div>
			</div>
		</div>
	{/if}

	<!-- FHIR Bundle Visualizer modal -->
	{#if showBundleVisualizer && activeTab?.parseResult}
		<div class="modal-overlay" onclick={() => { showBundleVisualizer = false; }} role="presentation">
			<!-- svelte-ignore a11y_interactive_supports_focus -->
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="modal modal-xl" onclick={(e) => e.stopPropagation()} role="dialog">
				<BundleVisualizer
					messageId={activeTab.parseResult.message_id}
					onClose={() => { showBundleVisualizer = false; }}
				/>
			</div>
		</div>
	{/if}

	<!-- Test Case Library modal -->
	{#if showTestCases}
		<div class="modal-overlay" onclick={() => { showTestCases = false; }} role="presentation">
			<!-- svelte-ignore a11y_interactive_supports_focus -->
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="modal modal-xl" onclick={(e) => e.stopPropagation()} role="dialog">
				<TestCaseLibrary
					currentContent={activeTab?.content ?? ''}
					currentLabel={activeTab?.label ?? ''}
					onLoad={handleTestCaseLoaded}
					onClose={() => { showTestCases = false; }}
				/>
			</div>
		</div>
	{/if}

	<!-- Template selection modal -->
	{#if showTemplates}
		<div class="modal-overlay" onclick={() => { showTemplates = false; }} role="presentation">
			<!-- svelte-ignore a11y_interactive_supports_focus -->
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="modal modal-lg" onclick={(e) => e.stopPropagation()} role="dialog">
				<TemplateDialog
					onSelect={handleTemplateSelected}
					onClose={() => { showTemplates = false; }}
				/>
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
					onShowActivation={() => { showSettings = false; showActivation = true; }}
				/>
			</div>
		</div>
	{/if}

	<!-- In-app dialog (replaces native alert/confirm) -->
	<AppDialog />

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

	.tree-scroll {
		flex: 1 1 auto;
		min-height: 0;
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	.tree-scroll :global(.tree-container) {
		flex: 1;
	}

	.inspector-wrapper {
		flex: 0 0 auto;
		min-height: 100px;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.inspector-splitter {
		flex: 0 0 auto;
		height: 4px;
		background-color: var(--color-border);
		cursor: row-resize;
		transition: background-color 0.15s;
	}

	.inspector-splitter:hover,
	.inspector-splitter.active {
		background-color: var(--color-accent);
	}

	.inspector-toggle {
		margin-left: auto;
		background: none;
		border: 1px solid transparent;
		color: var(--color-text-secondary);
		cursor: pointer;
		padding: 3px 5px;
		border-radius: 3px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		line-height: 1;
	}

	.inspector-toggle:hover {
		background-color: var(--color-bg-tertiary);
		color: var(--color-text-primary);
	}

	.inspector-toggle.active {
		color: var(--color-accent);
		border-color: var(--color-accent);
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

	.modal-xl {
		width: 1100px;
		max-width: 95%;
		height: 80vh;
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
		padding: 24px 20px;
	}

	.about-logo {
		margin-bottom: 16px;
		display: flex;
		justify-content: center;
	}

	.about-title {
		font-size: 26px;
		font-weight: 800;
		color: var(--color-accent);
		letter-spacing: -0.02em;
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
		margin-bottom: 16px;
	}

	.about-company {
		padding: 12px 0;
		border-top: 1px solid var(--color-border);
		margin-top: 8px;
	}

	.about-company-name {
		font-size: 13px;
		font-weight: 700;
		color: var(--color-text-primary);
		margin-bottom: 4px;
	}

	.about-contact {
		font-size: 12px;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
	}

	.about-contact a {
		color: var(--color-accent);
		text-decoration: none;
	}

	.about-contact a:hover {
		text-decoration: underline;
	}

	.about-sep {
		color: var(--color-text-secondary);
		opacity: 0.5;
	}

	.about-copyright {
		font-size: 10px;
		color: var(--color-text-secondary);
		margin-top: 12px;
		opacity: 0.7;
	}
</style>
