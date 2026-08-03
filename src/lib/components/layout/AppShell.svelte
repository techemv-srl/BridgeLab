<script lang="ts">
	import { /* onMount not used - resolves to server no-op */ } from 'svelte';
	import type { TreeNode, ParseResult } from '$lib/types/hl7';
	import { parseMessage } from '$lib/ipc/parser';
	import { getPreference, setPreference } from '$lib/ipc/database';
	import { validateMessage, parseFhirMessage } from '$lib/ipc/validation';
	import { getMessageFullText, getMessageTruncatedText, exportAsJson, exportAsCsv } from '$lib/ipc/anonymization';
	import type { ValidationIssue, ValidationReport } from '$lib/ipc/validation';
	import { t, setLocale, subscribeLocale, type Locale } from '$lib/i18n';
	import { messageStore, type MessageTab } from '$lib/stores/messages.svelte';
	import { editorOptionsStore } from '$lib/stores/editor-options.svelte';
	import { sessionStore } from '$lib/stores/session.svelte';
	import { fileOpsStore } from '$lib/stores/file-ops.svelte';
	import { shortcutStore, shortcutCapture, matchesKeys } from '$lib/stores/shortcuts.svelte';
	import { dialogStore } from '$lib/stores/dialog.svelte';
	import { parseUpgradeError } from '$lib/ipc/licensing';
	import AppDialog from '$lib/components/shared/AppDialog.svelte';
	import MonacoEditor from '$lib/components/editor/MonacoEditor.svelte';
	import MessageTree from '$lib/components/tree/MessageTree.svelte';
	import FieldInspector from '$lib/components/tree/FieldInspector.svelte';
	import WelcomeScreen from '$lib/components/layout/WelcomeScreen.svelte';
	import DialogHost from '$lib/components/layout/DialogHost.svelte';
	import EditorTabs from '$lib/components/editor/EditorTabs.svelte';
	import MenuBar from '$lib/components/layout/MenuBar.svelte';
	import StatusBar from '$lib/components/layout/StatusBar.svelte';
	import ValidationPanel from '$lib/components/validation/ValidationPanel.svelte';
	import CommunicationPanel from '$lib/components/communication/CommunicationPanel.svelte';
	import TrialBanner from '$lib/components/licensing/TrialBanner.svelte';
	import FhirPathPanel from '$lib/components/fhirpath/FhirPathPanel.svelte';
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
	let settingsSection = $state('editor');
	let showSchemaExport = $state(false);
	let showCompare = $state(false);
	let showBatch = $state(false);
	let showBatchAnon = $state(false);
	let showGenerate = $state(false);
	let showActivation = $state(false);
	let showTemplates = $state(false);
	let showBundleVisualizer = $state(false);
	let showFhirPath = $state(false);
	let showTestCases = $state(false);
	let showHelp = $state(false);
	let licenseStatus = $state<LicenseStatus | null>(null);
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

	// --- Bottom panel tabs ---
	// The open bottom panels share ONE resizable container and render as tabs.
	type BottomPanelId = 'validation' | 'communication' | 'fhirpath';
	let activeBottomPanel = $state<BottomPanelId>('validation');
	let openBottomPanels = $derived.by<BottomPanelId[]>(() => {
		const out: BottomPanelId[] = [];
		if (showValidation && validationReport) out.push('validation');
		if (showCommunication) out.push('communication');
		if (showFhirPath && activeTab?.parseResult) out.push('fhirpath');
		return out;
	});
	// Keep the active tab valid when its panel closes (fall back to the first
	// remaining one).
	$effect(() => {
		if (openBottomPanels.length > 0 && !openBottomPanels.includes(activeBottomPanel)) {
			activeBottomPanel = openBottomPanels[0];
		}
	});

	function closeActiveBottomPanel() {
		if (activeBottomPanel === 'validation') showValidation = false;
		else if (activeBottomPanel === 'communication') showCommunication = false;
		else showFhirPath = false;
	}

	/** Toggle a bottom panel; opening one also brings its tab to the front. */
	function toggleBottomPanel(id: BottomPanelId) {
		if (id === 'validation') showValidation = !showValidation;
		else if (id === 'communication') showCommunication = !showCommunication;
		else showFhirPath = !showFhirPath;
		const nowOpen =
			(id === 'validation' && showValidation) ||
			(id === 'communication' && showCommunication) ||
			(id === 'fhirpath' && showFhirPath);
		if (nowOpen) activeBottomPanel = id;
	}

	// Validation state
	let validationReport = $state<ValidationReport | null>(null);

	// The report is global while tabs are per-message: switching tab would
	// otherwise show (and open) the previous tab's results as if they were
	// the current tab's. Reset on switch; F6 re-validates the new tab.
	let lastValidatedTabId = $state<string | null>(null);
	$effect(() => {
		const id = messageStore.activeTabId;
		if (lastValidatedTabId !== null && id !== lastValidatedTabId) {
			validationReport = null;
			showValidation = false;
		}
		lastValidatedTabId = id;
	});

	// Reactive references to the active tab
	let activeTab = $derived(messageStore.activeTab);

	// Initialize app (using $effect instead of onMount which is a server no-op)
	let appInitialized = false;
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
				if (savedRestore !== null) sessionStore.restoreEnabled = savedRestore !== 'false';
				await fileOpsStore.refreshRecent();
				await editorOptionsStore.loadFromPrefs();

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

				// Notepad++-style tab restore (skip conditions live in the store)
				sessionRestored = await sessionStore.restoreFromDisk((content) => {
					void autoParse(content);
				});
			} catch {
				// Running in web-only mode without Tauri backend
			}

			// No session restored → leave zero tabs so the welcome screen
			// renders (first-run onboarding). Any welcome action, paste, or
			// the + button creates the first tab.
			void sessionRestored;
			sessionStore.startupComplete = true;

			// Files the app was launched with (double-clicked .hl7), plus
			// files forwarded by later launches (single-instance): both open
			// as tabs in this window.
			try {
				const { getLaunchFiles } = await import('$lib/ipc/parser');
				for (const p of await getLaunchFiles()) {
					await fileOpsStore.openPath(p, suppressAutoParse);
				}
				const { listen } = await import('@tauri-apps/api/event');
				await listen<string[]>('app://open-files', (e) => {
					void (async () => {
						for (const p of e.payload) {
							await fileOpsStore.openPath(p, suppressAutoParse);
						}
					})();
				});
			} catch { /* web mode */ }

			try {
				licenseStatus = await checkLicense();
			await shortcutStore.loadFromPrefs();
			} catch {
				// License check failed - treat as trial
			}
		})();
	});

	// Session autosave: persist open tabs whenever they change, debounced.
	$effect(() => {
		if (!appInitialized || typeof window === 'undefined') return;
		if (!sessionStore.restoreEnabled) return;
		// Track tabs + active id as dependencies
		void messageStore.tabs;
		void messageStore.activeTabId;
		for (const t of messageStore.tabs) {
			void t.content;
			void t.label;
			void t.filePath;
			void t.isModified;
		}
		sessionStore.scheduleAutosave();
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

	/** Open a URL in the OS browser (window.open is a no-op in the webview). */
	async function openExternal(url: string) {
		try {
			const { openUrl } = await import('@tauri-apps/plugin-opener');
			await openUrl(url);
		} catch {
			window.open(url, '_blank'); // web mode
		}
	}

	/** True when semver `a` is newer than `b` (numeric per-part compare). */
	function isNewerVersion(a: string, b: string): boolean {
		const pa = a.split('.').map((n) => parseInt(n) || 0);
		const pb = b.split('.').map((n) => parseInt(n) || 0);
		for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
			const d = (pa[i] ?? 0) - (pb[i] ?? 0);
			if (d !== 0) return d > 0;
		}
		return false;
	}

	/**
	 * Help → Check for updates. Preferred path: the Tauri updater (signed
	 * artifacts + latest.json) with in-app download and relaunch. Until
	 * artifact signing is configured on the release pipeline, that check
	 * errors out and we fall back to the GitHub releases API: compare
	 * versions and take the user to the release page to download.
	 */
	async function handleCheckUpdates() {
		let current = '';
		try {
			const { getVersion } = await import('@tauri-apps/api/app');
			current = await getVersion();
		} catch { /* web mode */ }

		try {
			const { check } = await import('@tauri-apps/plugin-updater');
			const update = await check();
			if (update) {
				const go = await dialogStore.confirm(
					t('update.available', { version: update.version, current }),
					t('update.title'),
				);
				if (go) {
					await update.downloadAndInstall();
					const restart = await dialogStore.confirm(t('update.restart'), t('update.title'));
					if (restart) {
						const { relaunch } = await import('@tauri-apps/plugin-process');
						await relaunch();
					}
				}
			} else {
				await dialogStore.info(t('update.upToDate', { current }), t('update.title'));
			}
			return;
		} catch {
			// Updater unavailable (unsigned artifacts) — GitHub fallback below.
		}

		try {
			const res = await fetch('https://api.github.com/repos/techemv-srl/BridgeLab/releases/latest');
			if (!res.ok) throw new Error(`GitHub API: HTTP ${res.status}`);
			const rel = await res.json();
			const latest = String(rel.tag_name ?? '').replace(/^v/, '');
			if (!latest) throw new Error('No release tag found');
			if (current && isNewerVersion(latest, current)) {
				const go = await dialogStore.confirm(
					t('update.available', { version: latest, current }),
					t('update.title'),
				);
				if (go) await openExternal(rel.html_url ?? 'https://github.com/techemv-srl/BridgeLab/releases');
			} else {
				await dialogStore.info(t('update.upToDate', { current: current || latest }), t('update.title'));
			}
		} catch (e) {
			await dialogStore.error(t('update.checkFailed'), t('update.title'), String(e));
		}
	}

	function applyTheme(t: string) {
		document.documentElement.setAttribute('data-theme', t);
	}

	// --- File operations (logic in fileOpsStore) ---

	// Monaco's programmatic content sync deliberately does not fire
	// onContentChange, so a bare one-shot flag would never be consumed and
	// would swallow the FIRST real user edit instead. Self-expire it after
	// the sync settles.
	let suppressExpiry: ReturnType<typeof setTimeout> | null = null;
	const suppressAutoParse = () => {
		skipNextAutoParse = true;
		if (suppressExpiry) clearTimeout(suppressExpiry);
		suppressExpiry = setTimeout(() => { skipNextAutoParse = false; }, 150);
	};

	async function handleOpenFile() {
		await fileOpsStore.openFromDialog(suppressAutoParse);
	}

	async function handleOpenRecentFile(path: string) {
		await fileOpsStore.openPath(path, suppressAutoParse);
	}

	async function handleSave() {
		await fileOpsStore.saveActive();
	}

	async function handleSaveAs() {
		await fileOpsStore.saveActiveAs();
	}

	async function handleClearRecent() {
		await fileOpsStore.clearRecent();
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
			} else if (
				(trimmed.startsWith('{') && trimmed.includes('"resourceType"')) ||
				trimmed.startsWith('<')
			) {
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
		activeBottomPanel = 'validation';
		const content = activeTab.content;
		const trimmed = content.trim();

		// FHIR branch (JSON or XML — the backend routes each to its parser
		// and runs the same rule set on both)
		if ((trimmed.startsWith('{') && trimmed.includes('"resourceType"')) || trimmed.startsWith('<')) {
			try {
				const result = await parseFhirMessage(trimmed);
				skipNextAutoParse = true;
				messageStore.updateParseResult(activeTab.id, result, result.truncated_text);
				// Real FHIR validation (resourceType, id, per-resource field
				// rules) — mapped into the panel's HL7-shaped report, using
				// the JSON path where a segment reference would go.
				const { validateFhir } = await import('$lib/ipc/validation');
				const fhirReport = await validateFhir(trimmed);
				validationReport = {
					issues: fhirReport.issues.map((i) => ({
						severity: (['error', 'warning', 'info'].includes(i.severity)
							? i.severity
							: 'info') as 'error' | 'warning' | 'info',
						rule_id: 'FHIR',
						segment_idx: null,
						segment_type: i.path || null,
						field_position: null,
						message: i.message,
					})),
					error_count: fhirReport.error_count,
					warning_count: fhirReport.warning_count,
					info_count: fhirReport.info_count,
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
			await dialogStore.info(t('upgrade.required', { tier: upgrade.tier }));
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

	/** Open a generated message in a new tab, parse bound to THAT tab id —
	 *  autoParse resolves the active tab after the IPC returns, and when
	 *  opening many messages in a loop that would misattribute results to
	 *  whichever tab ended up active. */
	function handleOpenGenerated(content: string, label: string) {
		messageStore.newTab();
		const tab = messageStore.activeTab;
		if (tab) {
			const tabId = tab.id;
			messageStore.updateContent(tabId, content);
			tab.label = label;
			void (async () => {
				try {
					const result = await parseMessage(content);
					skipNextAutoParse = true;
					messageStore.updateParseResult(tabId, result);
				} catch { /* leave unparsed */ }
			})();
		}
	}

	async function handleCompareMessages() {
		if (messageStore.tabs.length < 2) {
			await dialogStore.warning(t('diff.needTwoTabs'));
			return;
		}
		showCompare = true;
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
	//
	// In Tauri v2 the webview never receives HTML5 drops with files: the OS
	// drop is consumed by Tauri (dragDropEnabled defaults to true) and
	// surfaced as a native drag-drop event carrying real file PATHS. The
	// HTML handlers below only ever fire in web mode, where they remain as
	// a paste-like fallback.
	let dragDropHooked = false;
	$effect(() => {
		if (dragDropHooked || typeof window === 'undefined') return;
		dragDropHooked = true;
		(async () => {
			try {
				const { getCurrentWebview } = await import('@tauri-apps/api/webview');
				await getCurrentWebview().onDragDropEvent((event) => {
					const payload = event.payload;
					if (payload.type !== 'drop') return;
					const paths = payload.paths;
					void (async () => {
						// Sequential: keeps tab order = drop order and avoids
						// interleaved recent-list refreshes.
						for (const p of paths) {
							await fileOpsStore.openPath(p, suppressAutoParse);
						}
					})();
				});
				// AppShell lives for the whole app lifetime — no unlisten needed.
			} catch {
				// Web mode: the HTML5 handlers below do the work.
			}
		})();
	});

	async function handleDragOver(e: DragEvent) {
		e.preventDefault();
		if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy';
	}

	async function handleDrop(e: DragEvent) {
		e.preventDefault();
		const files = e.dataTransfer?.files;
		if (!files || files.length === 0) return;

		for (const file of Array.from(files)) {
			// Web mode only: the File API has no OS path, so read as text
			try {
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
		if (!text) return;
		// Paste with zero tabs (welcome screen): create the first tab so
		// paste-to-start works as the onboarding promises.
		if (!messageStore.activeTabId) messageStore.newTab();
		if (!messageStore.activeTabId) return;

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
		'view.toggleValidation': () => toggleBottomPanel('validation'),
		'view.toggleCommunication': () => toggleBottomPanel('communication'),
		'view.toggleFhirPath': () => toggleBottomPanel('fhirpath'),
		'tools.reparse': () => handleParse(),
		'tools.validate': () => handleValidate(),
	};

	function handleKeydown(e: KeyboardEvent) {
		// Stand down while the ShortcutsEditor is capturing a combo — pressing
		// Ctrl+O to *assign* it must not also open the file picker.
		if (shortcutCapture.active) return;
		if (e.key === 'F1') {
			e.preventDefault();
			showHelp = !showHelp;
			return;
		}
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
		recentFiles={fileOpsStore.recentFiles}
		{theme}
		{showTree}
		{showInspector}
		{showSchemaFields}
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
		onToggleValidation={() => toggleBottomPanel('validation')}
		onToggleCommunication={() => toggleBottomPanel('communication')}
		onAnonymize={handleShowAnonymize}
		onShowBundleVisualizer={() => { showBundleVisualizer = true; }}
		onToggleFhirPath={() => toggleBottomPanel('fhirpath')}
		onCopyFull={handleCopyFull}
		onCopyTruncated={handleCopyTruncated}
		onExportJson={handleExportJson}
		onExportXsd={() => { showSchemaExport = true; }}
		onExportCsv={handleExportCsv}
		onCompareMessages={handleCompareMessages}
		onBatchValidate={() => { showBatch = true; }}
		onBatchAnonymize={() => { showBatchAnon = true; }}
		onGenerateMessages={() => { showGenerate = true; }}
		onToggleTree={handleToggleTree}
		onToggleInspector={() => { showInspector = !showInspector; }}
		onToggleSchemaFields={() => { showSchemaFields = !showSchemaFields; }}
		onSetTheme={handleSetTheme}
		onSetLanguage={handleSetLanguage}
		onShowSettings={() => { settingsSection = 'editor'; showSettings = true; }}
		onShowShortcuts={() => { settingsSection = 'shortcuts'; showSettings = true; }}
		onCheckUpdates={handleCheckUpdates}
		onShowHelp={() => { showHelp = true; }}
		onShowActivation={() => { showActivation = true; }}
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
							messageType={activeTab.parseResult.message_type}
							format={activeTab.parseResult.format}
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
						language={activeTab.parseResult?.format?.startsWith('FHIR JSON') ? 'json'
							: activeTab.parseResult?.format?.startsWith('FHIR XML') ? 'xml'
							: 'hl7v2'}
						options={editorOptionsStore.options}
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
				{:else if sessionStore.startupComplete}
					<WelcomeScreen
						onOpenFile={handleOpenFile}
						onNewFromTemplate={() => { showTemplates = true; }}
						onShowTestCases={() => { showTestCases = true; }}
						onShowHelp={() => { showHelp = true; }}
						onNewTab={handleNewTab}
						onOpenRecentFile={handleOpenRecentFile}
					/>
				{/if}
			</div>

			<!-- Bottom panel: one shared, resizable container. Open panels are
			     TABS inside it (stacking them at full height pushed the lower
			     ones off-screen). Inactive panels stay mounted but hidden so
			     the Communication panel never loses its listener console. -->
			{#if openBottomPanels.length > 0}
				<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
				<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
				<div
					class="bottom-splitter"
					onmousedown={startBottomDrag}
					role="separator"
					tabindex={0}
				></div>
				<div class="bottom-panel" style="height: {bottomPanelHeight}px">
					<div class="panel-header panel-tab-bar">
						{#each openBottomPanels as p (p)}
							<button
								class="panel-tab"
								class:active={activeBottomPanel === p}
								onclick={() => { activeBottomPanel = p; }}
							>
								{tr(`panel.${p}`)}
							</button>
						{/each}
						<button class="panel-close" onclick={closeActiveBottomPanel}>&times;</button>
					</div>
					{#if showValidation && validationReport}
						<div class="panel-body" class:hidden-panel={activeBottomPanel !== 'validation'}>
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
						<div class="panel-body" class:hidden-panel={activeBottomPanel !== 'communication'}>
							<CommunicationPanel
								currentMessage={activeTab?.content ?? ''}
								activeTabLabel={activeTab?.label ?? ''}
								onMessageReceived={(content) => {
									// Open each incoming MLLP message in a fresh tab so the
									// user does not lose the message currently in the editor.
									messageStore.newTab();
									const t = messageStore.activeTab;
									if (t) {
										messageStore.updateContent(t.id, content);
										const ts = new Date().toLocaleTimeString();
										t.label = `Inbox ${ts}`;
									}
								}}
								onOpenGenerated={handleOpenGenerated}
							/>
						</div>
					{/if}
					{#if showFhirPath && activeTab?.parseResult}
						<div class="panel-body" class:hidden-panel={activeBottomPanel !== 'fhirpath'}>
							<FhirPathPanel messageId={activeTab.parseResult.message_id} />
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>

	<DialogHost
		bind:expandedFieldContent
		bind:showAnonymize
		bind:showAbout
		bind:showBundleVisualizer
		bind:showTestCases
		bind:showTemplates
		bind:showActivation
		bind:showSettings
		bind:showSchemaExport
		bind:showBatch
		bind:showBatchAnon
		bind:showGenerate
		bind:showCompare
		bind:showHelp
		bind:licenseStatus
		{settingsSection}
		{theme}
		onTestCaseLoaded={handleTestCaseLoaded}
		onTemplateSelected={handleTemplateSelected}
		onAnonymized={handleAnonymized}
		onSetTheme={handleSetTheme}
		onOpenRecentFile={(path) => { void handleOpenRecentFile(path); }}
		onOpenGenerated={handleOpenGenerated}
	/>

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
		isModified={activeTab?.isModified ?? false}
		errorCount={validationReport ? validationReport.error_count : null}
		warningCount={validationReport ? validationReport.warning_count : null}
		onShowValidation={() => { showValidation = true; activeBottomPanel = 'validation'; }}
		onExpandAll={handleExpandAll}
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

	.panel-tab-bar { gap: 2px; justify-content: flex-start; }
	.panel-tab {
		padding: 3px 12px;
		border: none;
		border-bottom: 2px solid transparent;
		background: none;
		color: var(--color-text-secondary);
		font-size: 11px;
		font-weight: 600;
		font-family: inherit;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		cursor: pointer;
	}
	.panel-tab:hover { color: var(--color-text-primary); }
	.panel-tab.active {
		color: var(--color-accent);
		border-bottom-color: var(--color-accent);
	}
	.panel-tab-bar .panel-close { margin-left: auto; }

	.panel-body {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		overflow: hidden;
	}
	.panel-body.hidden-panel { display: none; }

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

</style>
