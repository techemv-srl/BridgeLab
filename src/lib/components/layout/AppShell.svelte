<script lang="ts">
	import type { TreeNode, ParseResult } from '$lib/types/hl7';
	import { parseMessage } from '$lib/ipc/parser';
	import { saveTarget } from '$lib/save-target';
	import { hl7ToolTarget } from '$lib/tool-target';
	import { getPreference, setPreference } from '$lib/ipc/database';
	import { validateMessage, parseFhirMessage } from '$lib/ipc/validation';
	import { getMessageTruncatedText, exportAsJson, exportAsCsv } from '$lib/ipc/anonymization';
	import type { ValidationIssue, ValidationReport } from '$lib/ipc/validation';
	import { t, setLocale, subscribeLocale, type Locale } from '$lib/i18n';
	import { messageStore, type MessageTab } from '$lib/stores/messages.svelte';
	import { editorOptionsStore } from '$lib/stores/editor-options.svelte';
	import { sessionStore } from '$lib/stores/session.svelte';
	import { fileOpsStore } from '$lib/stores/file-ops.svelte';
	import { shortcutStore, shortcutCapture, matchesKeys, displayKeys } from '$lib/stores/shortcuts.svelte';
	import { dialogStore } from '$lib/stores/dialog.svelte';
	import { parseUpgradeError } from '$lib/ipc/licensing';
	import { openPricing } from '$lib/licensing/pricing';
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
	import LicenseNoticeBanner from '$lib/components/licensing/LicenseNoticeBanner.svelte';
	import UpdateBanner from '$lib/components/layout/UpdateBanner.svelte';
	import { isNewerVersion, fetchLatestRelease } from '$lib/updates';
	import FhirPathPanel from '$lib/components/fhirpath/FhirPathPanel.svelte';
	import SegmentGridPanel from '$lib/components/tree/SegmentGridPanel.svelte';
	import type { TestCase } from '$lib/ipc/testcases';
	import { checkLicense, type LicenseStatus } from '$lib/ipc/licensing';
	import type { MessageTemplate } from '$lib/ipc/templates';
	import { sampleTabLabel, type Sample } from '$lib/ipc/samples';
	import {
		splitLines, segmentOfLine, lineOfSegment, separatorsOf, fieldAtColumn, fieldRange,
		segmentSkeleton, insertionLine, type FieldTarget,
	} from '$lib/hl7/segment-lines';

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
	let showFhirRules = $state(false);
	let showFhirPackages = $state(false);
	let showCompare = $state(false);
	let showBatch = $state(false);
	let showBatchAnon = $state(false);
	let showGenerate = $state(false);
	let showActivation = $state(false);
	let showTemplates = $state(false);
	let showSamples = $state(false);
	let showBundleVisualizer = $state(false);
	let showFhirPath = $state(false);
	let showSegmentGrid = $state(false);
	let segmentGridRequest = $state<{ segment: string; stamp: number } | null>(null);
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
	type BottomPanelId = 'validation' | 'communication' | 'fhirpath' | 'segments';
	let activeBottomPanel = $state<BottomPanelId>('validation');
	let openBottomPanels = $derived.by<BottomPanelId[]>(() => {
		const out: BottomPanelId[] = [];
		if (showValidation && validationReport) out.push('validation');
		if (showCommunication) out.push('communication');
		// FHIRPath evaluates FHIR only: on an HL7 v2 tab it answered
		// "Message not found". The panel comes back on a FHIR tab.
		if (showFhirPath && activeTab?.parseResult?.format?.startsWith('FHIR')) out.push('fhirpath');
		if (showSegmentGrid && activeTab?.parseResult?.format === 'HL7v2') out.push('segments');
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
		else if (activeBottomPanel === 'segments') showSegmentGrid = false;
		else showFhirPath = false;
	}

	/** Toggle a bottom panel; opening one also brings its tab to the front. */
	function toggleBottomPanel(id: BottomPanelId) {
		if (id === 'validation') showValidation = !showValidation;
		else if (id === 'communication') showCommunication = !showCommunication;
		else if (id === 'segments') showSegmentGrid = !showSegmentGrid;
		else showFhirPath = !showFhirPath;
		const nowOpen =
			(id === 'validation' && showValidation) ||
			(id === 'communication' && showCommunication) ||
			(id === 'segments' && showSegmentGrid) ||
			(id === 'fhirpath' && showFhirPath);
		if (nowOpen) activeBottomPanel = id;
	}

	// Validation state
	let validationReport = $state<ValidationReport | null>(null);
	/** The text the report was made from: a later edit makes it stale. */
	let validatedContent = $state<string | null>(null);
	let validationStale = $derived(
		validationReport !== null && validatedContent !== null && messageStore.activeTab?.content !== validatedContent,
	);

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
				// Off: also clears tabs a version that did not delete them left.
				if (savedRestore === 'false') await sessionStore.setRestoreEnabled(false);
				await fileOpsStore.refreshRecent();
				await editorOptionsStore.loadFromPrefs();

				// On/off choices saved before 1.9.0 as plugin_enabled:<id> preferences;
				// the backend copies them into the plugins folder's state file.
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

				// Notepad++-style tab restore (skip conditions live in the store).
				// An unedited file tab shows its file as it is now; the others
				// parse their stored text. Files gone since are reported once.
				const missing: string[] = [];
				const adopted: Promise<void>[] = [];
				// Only the active tab is loaded now; the others when first shown
				// (see the effect on activeTabId). Reading and parsing ten large
				// files at launch made the start slow and the memory peak high.
				sessionRestored = await sessionStore.restoreFromDisk((tabId, content) => {
					if (tabId === messageStore.activeTabId) {
						adopted.push(loadRestoredTab(tabId, content, missing));
					} else {
						deferredRestore.set(tabId, content);
						adopted.push(fileOpsStore.checkRestoredFile(tabId, missing));
					}
				});
				void Promise.all(adopted).then(async () => {
					if (missing.length > 0) {
						await dialogStore.warning(t('file.missingAtRestore', { paths: missing.join('\n') }), t('file.notFoundTitle'));
					}
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
				// Listen first, then drain: the backend queues files until the
				// drain and emits them afterwards, so none falls in between.
				const { getLaunchFiles } = await import('$lib/ipc/parser');
				const { listen } = await import('@tauri-apps/api/event');
				await listen<string[]>('app://open-files', (e) => {
					void (async () => {
						for (const p of e.payload) {
							await fileOpsStore.openPath(p, suppressAutoParse);
						}
					})();
				});
				for (const p of await getLaunchFiles()) {
					await fileOpsStore.openPath(p, suppressAutoParse);
				}
			} catch { /* web mode */ }

			// Back in the app after working elsewhere: say which open files
			// another program changed or deleted meanwhile.
			try {
				const { getCurrentWindow } = await import('@tauri-apps/api/window');
				await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
					if (focused) void fileOpsStore.checkAllTabs();
				});
			} catch { /* web mode */ }

			// Closing the window: write the session now (the autosave is
			// debounced, and the last keystrokes were lost), and without
			// session restore ask before unsaved work is dropped for good.
			try {
				const { getCurrentWindow } = await import('@tauri-apps/api/window');
				await getCurrentWindow().onCloseRequested(async (event) => {
					const saved = await sessionStore.flush();
					if (sessionStore.restoreEnabled && !saved && dirtyTabs(messageStore.tabs).length > 0) {
						// The session could not be written (disk full...): the
						// unsaved tabs would not come back. Ask.
						if (!(await dialogStore.confirm(t('dialog.sessionSaveFailed'), t('dialog.unsavedTitle')))) {
							event.preventDefault();
						}
						return;
					}
					if (!sessionStore.restoreEnabled && !(await confirmDiscard(messageStore.tabs))) {
						event.preventDefault();
					}
				});
			} catch { /* web mode */ }

			try {
				licenseStatus = await checkLicense();
			await shortcutStore.loadFromPrefs();
			} catch {
				// License check failed - treat as trial
			}

			// The backend silently refreshes online-activated licenses near
			// expiry; reload the status so banner and dialog show the new
			// expiry without a restart.
			try {
				const { listen } = await import('@tauri-apps/api/event');
				await listen('license://refreshed', () => {
					void (async () => {
						try { licenseStatus = await checkLicense(); } catch { /* keep old */ }
					})();
				});
			} catch { /* web mode */ }
		})();
	});

	// Session autosave: persist open tabs whenever they change, debounced.
	// Not before startup (restore included) has finished.
	$effect(() => {
		if (!appInitialized || typeof window === 'undefined') return;
		if (!sessionStore.restoreEnabled || !sessionStore.startupComplete) return;
		// Track tabs + active id as dependencies
		void messageStore.tabs;
		void messageStore.activeTabId;
		for (const t of messageStore.tabs) {
			void t.content;
			void t.label;
			void t.filePath;
			void t.isModified;
			void t.cursorLine;
			void t.cursorColumn;
		}
		sessionStore.scheduleAutosave();
	});

	function handleTestCaseLoaded(tc: TestCase) {
		showTestCases = false;
		messageStore.newTab();
		const newTab = messageStore.activeTab;
		if (newTab) {
			suppressAutoParse();
			messageStore.updateContent(newTab.id, tc.content);
			newTab.label = tc.name;
			autoParse(tc.content);
		}
	}

	function handleSampleSelected(sample: Sample) {
		showSamples = false;
		handleOpenGenerated(sample.content, sampleTabLabel(sample));
	}

	function handleTemplateSelected(template: MessageTemplate) {
		showTemplates = false;
		// Open in new tab
		messageStore.newTab();
		const newTab = messageStore.activeTab;
		if (newTab) {
			suppressAutoParse();
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

	/**
	 * Help → Check for updates: the same GitHub releases API request as the
	 * daily check (compare versions, then open the release page to
	 * download). The Tauri updater (signed artifacts, latest.json, in-app
	 * install) is not queried until release signing is configured: until
	 * then its request to github.com could only fail, and the privacy page
	 * promises the one request to api.github.com.
	 */
	async function handleCheckUpdates() {
		let current = '';
		try {
			const { getVersion } = await import('@tauri-apps/api/app');
			current = await getVersion();
		} catch { /* web mode */ }

		try {
			const { version: latest, url } = await fetchLatestRelease();
			if (current && isNewerVersion(latest, current)) {
				const go = await dialogStore.confirm(
					t('update.available', { version: latest, current }),
					t('update.title'),
				);
				if (go) await openExternal(url);
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

	/** Unsaved work in these tabs. Only an untitled tab left empty has
	 *  nothing to lose; a file emptied by the user is an unsaved change. */
	function dirtyTabs(tabs: typeof messageStore.tabs) {
		return tabs.filter((tab) => tab.isModified && (tab.filePath !== null || tab.content.trim() !== ''));
	}

	/** Ask before discarding unsaved changes. True when it is fine to close. */
	async function confirmDiscard(tabs: typeof messageStore.tabs): Promise<boolean> {
		const dirty = dirtyTabs(tabs);
		if (dirty.length === 0) return true;
		const names = dirty.map((tab) => tab.label).join(', ');
		return dialogStore.confirm(t('dialog.discardTabs', { names }), t('dialog.unsavedTitle'));
	}

	async function handleCloseTab(tabId?: string) {
		const id = tabId ?? messageStore.activeTabId;
		const tab = messageStore.tabs.find((x) => x.id === id);
		if (!id || !tab) return;
		if (!(await confirmDiscard([tab]))) return;
		messageStore.closeTab(id);
	}

	async function handleCloseOthers(keepId: string) {
		const others = messageStore.tabs.filter((tab) => tab.id !== keepId);
		if (!(await confirmDiscard(others))) return;
		messageStore.closeOtherTabs(keepId);
	}

	async function handleCloseAllTabs() {
		if (!(await confirmDiscard(messageStore.tabs))) return;
		messageStore.closeAllTabs();
		messageStore.newTab();
	}

	/** Parse `content` into tab `tabId` — not into whichever tab is active
	 *  when the IPC returns (session restore parses every tab at once). */
	/** Restored tabs not loaded yet, with their stored text. */
	const deferredRestore = new Map<string, string>();

	function loadRestoredTab(tabId: string, content: string, missing: string[] = []): Promise<void> {
		return fileOpsStore.adoptRestoredTab(tabId, missing).then((reloaded) => {
			if (!reloaded) return parseIntoTab(tabId, content);
		});
	}

	$effect(() => {
		const id = messageStore.activeTabId;
		if (!id || !deferredRestore.has(id)) return;
		const content = deferredRestore.get(id)!;
		deferredRestore.delete(id);
		void loadRestoredTab(id, content);
	});

	async function parseIntoTab(tabId: string, content: string) {
		const trimmed = content.trim();
		try {
			if (looksLikeHl7(trimmed)) {
				messageStore.updateParseResult(tabId, await parseMessage(content));
			} else if (looksLikeFhir(trimmed)) {
				messageStore.updateParseResult(tabId, await parseFhirMessage(content));
			}
		} catch { /* not parseable yet: the tab shows it unparsed */ }
	}

	// --- Editor operations ---

	let skipNextAutoParse = false;
	/** One debounce timer per tab: an edit in one tab followed by a switch
	 *  (or a paste into a new tab) must still get its own tab parsed. */
	const autoParseTimers = new Map<string, ReturnType<typeof setTimeout>>();

	function cancelAutoParse(tabId: string) {
		const timer = autoParseTimers.get(tabId);
		if (timer) clearTimeout(timer);
		autoParseTimers.delete(tabId);
	}

	/** Text the HL7 parser takes: a header (any field separator) after
	 *  what it skips (BOM, blank lines, an MLLP start byte). */
	function looksLikeHl7(trimmed: string): boolean {
		return /^(MSH|FHS|BHS)/.test(trimmed);
	}

	function looksLikeFhir(trimmed: string): boolean {
		return (trimmed.startsWith('{') && trimmed.includes('"resourceType"')) || trimmed.startsWith('<');
	}

	async function handleContentChange(value: string) {
		const tabId = messageStore.activeTabId;
		if (!tabId) return;

		// Always save the current editor text to the tab - even if autoparse should be skipped.
		// Previously this return was above updateContent, causing user edits to be lost.
		messageStore.updateContent(tabId, value);

		// Skip auto-parse if content was just set by file open / parse action
		if (skipNextAutoParse) {
			skipNextAutoParse = false;
			return;
		}

		// Debounced auto-parse, after the delay set in Settings → Parser;
		// off there, the message is parsed by Validate / Re-parse only.
		cancelAutoParse(tabId);
		if (!editorOptionsStore.autoParse) return;
		autoParseTimers.set(tabId, setTimeout(() => {
			autoParseTimers.delete(tabId);
			void autoParse(value, tabId);
		}, editorOptionsStore.autoParseDelay));
	}

	/** Parse `value` into `tabId` (default: the active tab), if it is still
	 *  that tab's text when the parse returns. */
	async function autoParse(value: string, tabId = messageStore.activeTabId) {
		if (!tabId) return;
		// The parse answers for this tab and this text only: a slower,
		// older parse (a debounced edit overtaken by a paste) or a tab
		// switch in the meantime must not overwrite a newer result.
		const current = () => messageStore.tabs.find((t) => t.id === tabId)?.content === value;
		if (!value || value.length < 10) {
			messageStore.markParseStale(tabId, t('status.parseStaleEmpty'));
			return;
		}
		const trimmed = value.trim();
		try {
			if (looksLikeHl7(trimmed)) {
				const result = await parseMessage(value);
				// Background parse while user is typing: update parseResult only,
				// do NOT replace editor content (would reset cursor to 1:1).
				if (current()) {
					messageStore.updateParseResult(tabId, result);
					await revalidateAfterParse(tabId, value, result);
				}
			} else if (looksLikeFhir(trimmed)) {
				const result = await parseFhirMessage(value);
				if (current()) {
					messageStore.updateParseResult(tabId, result);
					await revalidateAfterParse(tabId, value);
				}
			} else if (current()) {
				messageStore.markParseStale(tabId, t('status.parseStaleUnknown'));
			}
		} catch (e) {
			// Not valid yet: keep the last parse, but say it is out of date
			// (the tree, status bar and FHIRPath would describe old text).
			if (current()) messageStore.markParseStale(tabId, String(e));
		}
	}

	/** A validation report on screen follows the text: re-run it when the
	 *  tab it belongs to is parsed again (the counts and the issue list
	 *  used to keep describing segments that had been deleted). */
	async function revalidateAfterParse(tabId: string, value: string, hl7?: ParseResult) {
		if (!validationReport || messageStore.activeTabId !== tabId || validatedContent === value) return;
		const tab = messageStore.activeTab;
		if (!tab || tab.content !== value) return;
		if (!hl7) {
			await runValidation(tab);
			return;
		}
		// The message was just parsed: validate that parse, not a second one.
		try {
			const report = await validateMessage(hl7.message_id);
			if (messageStore.activeTabId === tabId && tab.content === value) {
				validationReport = report;
				validatedContent = value;
			}
		} catch { /* keep the old report, marked stale */ }
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
		await runValidation(activeTab);
	}

	/** Parse `tab`'s text afresh, validate it and show the report. */
	async function runValidation(tab: MessageTab) {
		const content = tab.content;
		const trimmed = content.trim();
		const done = (report: ValidationReport) => {
			// Another tab since: this report is not for the one on screen.
			if (messageStore.activeTabId !== tab.id) return;
			validationReport = report;
			validatedContent = content;
		};

		// FHIR branch (JSON or XML — the backend routes each to its parser
		// and runs the same rule set on both)
		if (looksLikeFhir(trimmed)) {
			try {
				const result = await parseFhirMessage(trimmed);
				if (tab.content === content) messageStore.updateParseResult(tab.id, result);
				// Real FHIR validation (resourceType, id, per-resource field
				// rules) — mapped into the panel's HL7-shaped report, using
				// the JSON path where a segment reference would go.
				const { validateFhir } = await import('$lib/ipc/validation');
				const fhirReport = await validateFhir(trimmed);
				// Say when conformance was actually checked: a clean report
				// with no profile applied is not the same reassurance.
				const profileNote = fhirReport.profiles_applied
					? [{
						severity: 'info' as const,
						rule_id: 'FHIR',
						segment_idx: null,
						segment_type: null,
						field_position: null,
						message: t('fhir.profilesApplied'),
					}]
					: [];
				done({
					issues: [...profileNote, ...fhirReport.issues.map((i) => ({
						severity: (['error', 'warning', 'info'].includes(i.severity)
							? i.severity
							: 'info') as 'error' | 'warning' | 'info',
						rule_id: i.rule_id || 'FHIR',
						segment_idx: null,
						segment_type: i.path || null,
						field_position: null,
						message: i.message,
					}))],
					error_count: fhirReport.error_count,
					warning_count: fhirReport.warning_count,
					info_count: fhirReport.info_count + profileNote.length,
				});
			} catch (e) {
				done(buildSyntheticReport(content, String(e)));
			}
			return;
		}

		// HL7 v2 branch: try to parse fresh
		try {
			const result = await parseMessage(content);
			if (tab.content === content) messageStore.updateParseResult(tab.id, result);
			try {
				done(await validateMessage(result.message_id));
			} catch (ve) {
				console.error('Validation IPC error:', ve);
				done(buildSyntheticReport(content, String(ve)));
			}
		} catch (e) {
			// Parse failed - produce a detailed synthetic report explaining why
			console.error('Parse error:', e);
			done(buildSyntheticReport(content, String(e)));
		}
	}

	/** Build a synthetic validation report when parsing fails. */
	function buildSyntheticReport(content: string, parseError: string): ValidationReport {
		const issues: ValidationIssue[] = [];
		// The first line with something on it, as the parser sees it: blank
		// lines before MSH are skipped there too.
		const firstLine = content.replace(/^[\ufeff\s\x1c]+/, '').split(/[\r\n]/)[0] ?? '';
		const firstSegType = firstLine.substring(0, 3);

		if (firstLine.length < 8) {
			issues.push({
				severity: 'error', rule_id: 'STRUCT-001',
				segment_idx: null, segment_type: null, field_position: null,
				message: t('val.tooShort'),
			});
		} else if (!/^(MSH|FHS|BHS)/.test(firstLine)) {
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

	/** An issue row: select its segment (and field) in the editor and the tree. */
	function handleValidationIssueClick(issue: ValidationIssue) {
		if (issue.segment_idx === null || issue.segment_idx === undefined) return;
		const target: FieldTarget = { field: issue.field_position ?? 0, repetition: null, component: null };
		handleTreeNavigateToEditor(issue.segment_idx, target);
		if (activeTab?.parseResult) {
			treeNavigation = { tabId: activeTab.id, segmentIdx: issue.segment_idx, target, stamp: Date.now() };
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

	function handleNodeSelect(node: TreeNode | null) {
		selectedTreeNode = node;
	}

	// A tree node belongs to the tab it was clicked in: drop the selection
	// whenever the active tab changes, otherwise the Field Inspector keeps
	// showing e.g. "MSH (0)" from an HL7 tab on top of a FHIR document.
	$effect(() => {
		void messageStore.activeTabId;
		selectedTreeNode = null;
		// The tree is rebuilt for the other tab: a jump asked of the
		// previous one must not replay there.
		treeNavigation = null;
	});

	/** Derive the segment type code (e.g. "PID") for the currently selected tree node. */
	let selectedSegmentType = $derived.by<string | null>(() => {
		if (!selectedTreeNode || !activeTab?.parseResult) return null;
		// A greyed segment of the standard structure: "ghost.OBX[.f5]".
		const ghost = selectedTreeNode.id.match(/^ghost\.([A-Z][A-Z0-9]{2})(?:\.|$)/);
		if (ghost) return ghost[1];
		const parts = selectedTreeNode.id.split('.');
		const segPart = parts.find((p) => p.startsWith('seg'));
		if (!segPart) return null;
		const segIdx = parseInt(segPart.slice(3));
		const segNode = activeTab.parseResult.tree_roots[segIdx];
		if (!segNode) return null;
		// Segment label is "MSH (0)" / "PID (1)" — take the 3-char code
		// ("PIDX (2)" has none: it is no PID).
		const m = segNode.label.match(/^([A-Z][A-Z0-9]{2}) /);
		return m ? m[1] : null;
	});

	function handleFieldExpand(content: string) {
		expandedFieldContent = content;
	}

	/** The editor component: folds long runs visually, expands on request. */
	let editorRef = $state<ReturnType<typeof MonacoEditor> | undefined>(undefined);
	/** Folded runs the editor currently shows (status-bar badge). */
	let foldCount = $state(0);

	function handleExpandAll() {
		editorRef?.expandAllFolds();
	}

	/** Show Segment in Tree: select what is under the caret (segment,
	 *  field, repetition, component) in the tree. */
	function handleEditorNavigateSegment(lineNumber: number, column: number) {
		showTree = true;
		const text = activeTab?.content;
		if (!text || !activeTab?.parseResult) return;
		const lines = splitLines(text);
		// Segment N is not line N + 1 when there are blank lines.
		const segIdx = segmentOfLine(lines, lineNumber - 1);
		if (segIdx === null) return;
		const target = fieldAtColumn(lines[lineNumber - 1], column, separatorsOf(lines), segIdx === 0);
		treeNavigation = {
			tabId: activeTab.id,
			segmentIdx: segIdx,
			target: target && target.field > 0 ? target : null,
			stamp: Date.now(),  // stamp to force re-trigger even on same target
		};
	}

	/** Tree navigation request from editor: segment index and optional field target */
	let treeNavigation = $state<{ tabId: string; segmentIdx: number; target: FieldTarget | null; stamp: number } | null>(null);

	/** Editor navigation request from tree: scrolls Monaco to a specific position */
	let editorNavigation = $state<{ line: number; column: number; selectionLength: number; stamp: number } | null>(null);

	/** Handle a tree node requesting to show its position in the editor */
	/** Open the segment grid on one segment type (tree context menu). */
	function showSegmentInGrid(segmentType: string) {
		segmentGridRequest = { segment: segmentType, stamp: Date.now() };
		showSegmentGrid = true;
		activeBottomPanel = 'segments';
	}

	/** Select a segment, field, repetition or component in the editor. */
	function handleTreeNavigateToEditor(segmentIdx: number, target: FieldTarget) {
		if (!activeTab) return;
		const lines = splitLines(activeTab.content);
		// Blank lines are no segments: count segments, not lines.
		const lineIdx = lineOfSegment(lines, segmentIdx);
		if (lineIdx === null) return;
		// The separators the message declares in MSH-1/MSH-2, not the usual
		// '|' and '^': the parser honours them, so navigation must too.
		const { column, length } = fieldRange(lines[lineIdx], target, separatorsOf(lines), segmentIdx === 0);
		editorNavigation = { line: lineIdx + 1, column, selectionLength: length, stamp: Date.now() };
	}

	/** Insert a skeleton for a standard segment (from a ghost row in the
	 *  structure tree) into the editor at its standard position. */
	async function handleInsertSegment(code: string, afterSegmentIdx: number | null) {
		const tab = messageStore.activeTab;
		if (!tab) return;
		let pipes = 1;
		try {
			const { getSegmentSchema } = await import('$lib/ipc/tables');
			const schema = await getSegmentSchema(code, tab.parseResult?.version ?? '2.5');
			if (schema) {
				// Enough separators to reach the last required field, so the
				// mandatory slots are visible; at least one.
				const lastRequired = schema.fields.filter((f) => f.required).map((f) => f.position);
				pipes = Math.max(1, ...lastRequired);
			}
		} catch { /* skeleton with a single separator */ }
		const content = tab.content;
		const lines = splitLines(content);
		// With the message's own separators: a '#'-delimited message got '|'.
		const skeleton = segmentSkeleton(code, pipes, separatorsOf(lines));
		const at = insertionLine(lines, afterSegmentIdx, code);
		// Through the editor, as one undoable step: replacing the text
		// wholesale cleared the undo history, edits before it included.
		const viaEditor = messageStore.activeTabId === tab.id && (editorRef?.insertLine(at, skeleton) ?? false);
		if (!viaEditor) {
			const eol = content.includes('\r\n') ? '\r\n' : content.includes('\r') ? '\r' : '\n';
			lines.splice(at, 0, skeleton);
			messageStore.updateContent(tab.id, lines.join(eol));
		}
		const updated = tab.content;
		// Parse bound to THIS tab id — autoParse resolves the active tab when
		// the IPC returns, and the user may have switched tabs meanwhile.
		cancelAutoParse(tab.id);
		try {
			const result = await parseMessage(updated);
			if (tab.content === updated) messageStore.updateParseResult(tab.id, result);
		} catch { /* leave unparsed */ }
	}

	// --- View operations ---

	function handleToggleTree() {
		showTree = !showTree;
	}

	// --- Upgrade prompt helper ---

	async function handleUpgradeError(err: unknown): Promise<boolean> {
		const upgrade = parseUpgradeError(err);
		if (upgrade) {
			// Offer the way to buy right where the need shows up.
			const buy = await dialogStore.show({
				kind: 'info',
				message: t('upgrade.required', { tier: upgrade.tier }),
				okLabel: t('upgrade.seePrices'),
				cancelLabel: t('modal.close'),
				showCancel: true,
			});
			if (buy) await openPricing('upgrade_prompt');
			return true;
		}
		return false;
	}

	// --- Anonymization / Copy / Export ---

	/** The HL7 v2 message a Tools command works on: the editor text of the
	 *  tab the command started on, parsed now (see hl7ToolTarget). Says why
	 *  not when there is none, and returns null. */
	async function requireHl7(): Promise<{ tabId: string; result: ParseResult } | null> {
		const tab = activeTab;
		if (tab) cancelAutoParse(tab.id);
		const target = await hl7ToolTarget(tab ? { id: tab.id, content: tab.content } : null, {
			parse: parseMessage,
			current: () => {
				const now = messageStore.activeTab;
				return now ? { id: now.id, content: now.content } : null;
			},
			looksLikeHl7,
			looksLikeFhir,
		});
		if (target.ok) {
			messageStore.updateParseResult(target.tabId, target.result);
			return { tabId: target.tabId, result: target.result };
		}
		switch (target.reason) {
			case 'empty': await dialogStore.warning(t('dialog.parseFirst')); break;
			case 'fhir': await dialogStore.warning(t('dialog.hl7Only')); break;
			case 'notHl7': await dialogStore.warning(t('dialog.notHl7Tool')); break;
			case 'parseFailed': await dialogStore.warning(t('dialog.parseFailedTool', { error: target.error })); break;
			case 'changed': break; // the user moved on: run the command again
		}
		return null;
	}

	async function handleShowAnonymize() {
		const target = await requireHl7();
		// The dialog works on the active tab: it is the one just parsed.
		if (target && messageStore.activeTabId === target.tabId) showAnonymize = true;
	}

	/** Opening FHIRPath on a tab that is not FHIR says why nothing opens;
	 *  closing it works anywhere. */
	async function handleToggleFhirPath() {
		if (!showFhirPath && !activeTab?.parseResult?.format?.startsWith('FHIR')) {
			await dialogStore.warning(t(activeTab?.parseResult ? 'dialog.fhirPathFhirOnly' : 'dialog.parseFirst'));
			return;
		}
		toggleBottomPanel('fhirpath');
	}

	async function handleShowBundleVisualizer() {
		if (!activeTab?.parseResult?.format?.startsWith('FHIR')) {
			await dialogStore.warning(t('dialog.fhirOnly'));
			return;
		}
		showBundleVisualizer = true;
	}

	function handleAnonymized(text: string) {
		showAnonymize = false;
		// Open anonymized text in a new tab
		messageStore.newTab();
		const newTab = messageStore.activeTab;
		if (newTab) {
			messageStore.updateContent(newTab.id, text);
			newTab.label = t('tab.anonymized');
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
					suppressAutoParse();
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
		// Restored tabs not shown yet hold no text (a large file is read
		// back when first shown): load them, or one side of the diff would
		// be empty without a hint.
		const pending = [...deferredRestore];
		deferredRestore.clear();
		await Promise.all(pending.map(([id, content]) => loadRestoredTab(id, content)));
		showCompare = true;
	}

	async function handleCopyFull() {
		// The tab content is the full text, edits included.
		if (!activeTab?.content) {
			await dialogStore.warning(t('dialog.noMessage'));
			return;
		}
		try {
			await navigator.clipboard.writeText(activeTab.content);
		} catch { /* no clipboard in this context */ }
	}

	async function handleCopyTruncated() {
		const target = await requireHl7();
		if (!target) return;
		try {
			const text = await getMessageTruncatedText(target.result.message_id, 100);
			await navigator.clipboard.writeText(text);
		} catch {
			// web mode fallback
		}
	}

	async function handleExportJson() {
		await exportStructured('json');
	}

	async function handleExportCsv() {
		await exportStructured('csv');
	}

	/** Tools → Export JSON / CSV: ask where, then write. A blob download
	 *  wrote the file wherever the webview chose (or nowhere), silently. */
	async function exportStructured(kind: 'json' | 'csv') {
		const target = await requireHl7();
		if (!target) return;
		const messageId = target.result.message_id;
		const label = messageStore.tabs.find((x) => x.id === target.tabId)?.label;
		const base = (label || 'message').replace(/\.[^.]*$/, '');
		try {
			const text = kind === 'json' ? await exportAsJson(messageId) : await exportAsCsv(messageId);
			const { save } = await import('@tauri-apps/plugin-dialog');
			const path = await save({
				title: t(kind === 'json' ? 'menu.tools.exportJson' : 'menu.tools.exportCsv'),
				defaultPath: `${base}.${kind}`,
				filters: [{ name: kind.toUpperCase(), extensions: [kind] }],
			});
			const target = path ? await saveTarget(path, kind) : null;
			if (!target) return;
			const { writeTextFile } = await import('@tauri-apps/plugin-fs');
			await writeTextFile(target, text);
		} catch (e) {
			if (!(await handleUpgradeError(e))) {
				await dialogStore.error(t('dialog.exportFailed'), undefined, String(e));
			}
		}
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
					messageStore.openMessage(result, null, text);
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
		// A paste that lands in any text control (the editor, a search box,
		// the MLLP host, the licence key, FHIRPath...) belongs to that
		// control: this window-level fallback used to replace the whole
		// message with it.
		const inTextControl = (el: EventTarget | null) => el instanceof Element && !!el.closest(
			'input, textarea, select, [contenteditable=""], [contenteditable="true"], .editor-container, .monaco-editor, [role="dialog"], .modal, .modal-overlay',
		);
		if (inTextControl(e.target) || inTextControl(document.activeElement)) return;

		const text = e.clipboardData?.getData('text/plain');
		if (!text) return;
		e.preventDefault();
		// Paste-to-start: into the empty active tab, otherwise into a NEW
		// tab. Never over an existing message, which has no undo here.
		const active = messageStore.activeTab;
		if (!active || active.content.trim() !== '') messageStore.newTab();
		if (!messageStore.activeTabId) return;
		const pastedInto = messageStore.activeTabId;
		cancelAutoParse(pastedInto);
		messageStore.updateContent(pastedInto, text);

		// Trigger auto-parse
		await autoParse(text, pastedInto);
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
		'view.toggleFhirPath': () => { void handleToggleFhirPath(); },
		'view.toggleSegmentGrid': () => toggleBottomPanel('segments'),
		'tools.reparse': () => handleParse(),
		'tools.validate': () => handleValidate(),
	};

	/**
	 * Edit menu. The menu does not take the focus, so the command goes to
	 * what the user was working in: the editor through its own API, a text
	 * field through execCommand (paste through the clipboard API, since
	 * script-issued 'paste' is blocked).
	 */
	async function handleEditCommand(cmd: 'undo' | 'redo' | 'cut' | 'copy' | 'paste') {
		const el = document.activeElement;
		const field = el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement ? el : null;
		let ok = true;
		if (field && !editorRef?.hasTextFocus()) {
			if (cmd !== 'paste') {
				ok = document.execCommand(cmd);
			} else {
				try {
					const text = await navigator.clipboard.readText();
					field.focus();
					ok = document.execCommand('insertText', false, text);
				} catch {
					ok = false;
				}
			}
		} else if (editorRef && activeTab) {
			ok = await editorRef.runEditCommand(cmd);
		}
		if (!ok && (cmd === 'cut' || cmd === 'copy' || cmd === 'paste')) {
			const key = { cut: 'Ctrl+X', copy: 'Ctrl+C', paste: 'Ctrl+V' }[cmd];
			await dialogStore.info(t('dialog.clipboardBlocked', { keys: displayKeys(key) }));
		}
	}

	/**
	 * App shortcuts, in the capture phase: they must reach the app before
	 * the editor, which otherwise keeps keys it also binds (Ctrl+L, Ctrl+K)
	 * for itself. A binding the user gives an editor key wins over the
	 * editor; the shortcut editor says so when the binding is made.
	 */
	function handleKeydown(e: KeyboardEvent) {
		// Stand down while the ShortcutsEditor is capturing a combo — pressing
		// Ctrl+O to *assign* it must not also open the file picker.
		if (shortcutCapture.active) return;
		if (e.key === 'F1') {
			e.preventDefault();
			e.stopPropagation();
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
				e.stopPropagation();
				action();
				return;
			}
		}
	}

	/**
	 * Browser keys that would act on the page itself: on Windows, WebView2
	 * reloads on Ctrl+R (losing panels, and every tab when session restore
	 * is off) and prints on Ctrl+P. Runs in the bubble phase, so a key the
	 * editor or a field used (Ctrl+F in the editor opens its Find) is left
	 * alone; the rest are cancelled.
	 */
	function blockBrowserKeys(e: KeyboardEvent) {
		if (e.defaultPrevented) return;
		const ctrl = e.ctrlKey || e.metaKey;
		const k = e.key.toLowerCase();
		const reload = e.key === 'F5' || (ctrl && k === 'r');
		const pageAction = ctrl && !e.altKey && !e.shiftKey && ['p', 'f', 'g', 'u', 'j', 'h'].includes(k);
		const caretBrowsing = e.key === 'F7' || e.key === 'F3';
		if (reload || pageAction || caretBrowsing) e.preventDefault();
	}
</script>

<svelte:window
	onkeydowncapture={handleKeydown}
	onkeydown={blockBrowserKeys}
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
	<!-- Non-blocking notice pushed by the license server (dismiss clears it) -->
	<LicenseNoticeBanner />
	<UpdateBanner />

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
		onShowSamples={() => { showSamples = true; }}
		onShowTestCases={() => { showTestCases = true; }}
		onParse={handleParse}
		onValidate={handleValidate}
		onToggleValidation={() => toggleBottomPanel('validation')}
		onToggleCommunication={() => toggleBottomPanel('communication')}
		onAnonymize={handleShowAnonymize}
		onShowBundleVisualizer={handleShowBundleVisualizer}
		onToggleFhirPath={() => { void handleToggleFhirPath(); }}
		onToggleSegmentGrid={() => toggleBottomPanel('segments')}
		onCopyFull={handleCopyFull}
		onCopyTruncated={handleCopyTruncated}
		onExportJson={handleExportJson}
		onExportXsd={() => { showSchemaExport = true; }}
		onShowFhirRules={() => { showFhirRules = true; }}
		onShowFhirPackages={() => { showFhirPackages = true; }}
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
		onEditCommand={handleEditCommand}
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
						<!-- One tree per tab: re-parses of the same tab keep what is
						     expanded and selected, another tab starts fresh. -->
						{#key activeTab.id}
						<MessageTree
							messageId={activeTab.parseResult.message_id}
							roots={activeTab.parseResult.tree_roots}
							version={activeTab.parseResult.version}
							messageType={activeTab.parseResult.message_type}
							format={activeTab.parseResult.format}
							showSchemaFields={showSchemaFields}
							onNodeSelect={handleNodeSelect}
							onFieldExpand={handleFieldExpand}
							navigateTo={treeNavigation?.tabId === activeTab.id ? treeNavigation : null}
							onNavigateToEditor={handleTreeNavigateToEditor}
							onInsertSegment={handleInsertSegment}
							onShowInGrid={showSegmentInGrid}
						/>
						{/key}
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
							title={tr('common.dragResize')}
						></div>
						<div class="inspector-wrapper" style="height: {inspectorHeight}px">
							<FieldInspector
								messageId={activeTab.parseResult.message_id}
								version={activeTab.parseResult.version}
								format={activeTab.parseResult.format}
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
						{#if shortcutStore.get('file.open')}
							<p class="shortcut-hint">{tr('tree.shortcutHint', { keys: displayKeys(shortcutStore.get('file.open')) })}</p>
						{/if}
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
				onCloseOthers={(id) => handleCloseOthers(id)}
				onNewTab={handleNewTab}
			/>

			<!-- Editor -->
			<div class="editor-area">
				{#if activeTab}
					<MonacoEditor
						bind:this={editorRef}
						content={activeTab.content}
						docKey={activeTab.id}
						openDocs={messageStore.tabs.map((t) => t.id)}
						cursorLine={activeTab.cursorLine}
						cursorColumn={activeTab.cursorColumn}
						theme={theme === 'light' ? 'bridgelab-light' : 'bridgelab-dark'}
						language={activeTab.parseResult?.format?.startsWith('FHIR JSON') ? 'json'
							: activeTab.parseResult?.format?.startsWith('FHIR XML') ? 'xml'
							: 'hl7v2'}
						options={editorOptionsStore.options}
						onContentChange={handleContentChange}
						onCursorChange={handleCursorChange}
						onNavigateToSegment={handleEditorNavigateSegment}
						foldThreshold={editorOptionsStore.foldThreshold}
						onFoldCountChange={(n) => { foldCount = n; }}
						onCopyFullMessage={handleCopyFull}
						onCopyTruncatedMessage={handleCopyTruncated}
						navigation={editorNavigation}
					/>
				{:else if sessionStore.startupComplete}
					<WelcomeScreen
						onOpenFile={handleOpenFile}
						onNewFromTemplate={() => { showTemplates = true; }}
						onShowSamples={() => { showSamples = true; }}
						onShowTestCases={() => { showTestCases = true; }}
						onShowHelp={() => { showHelp = true; }}
						onNewTab={handleNewTab}
						onOpenRecentFile={handleOpenRecentFile}
						onShowGenerate={() => { showGenerate = true; }}
						onShowBatchAnonymize={() => { showBatchAnon = true; }}
						onShowCommunication={() => toggleBottomPanel('communication')}
						onShowSchemaExport={() => { showSchemaExport = true; }}
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
								stale={validationStale}
								onIssueClick={handleValidationIssueClick}
							/>
						</div>
					{/if}
					{#if showCommunication}
						<div class="panel-body" class:hidden-panel={activeBottomPanel !== 'communication'}>
							<CommunicationPanel
								currentMessage={activeTab?.content ?? ''}
								activeTabLabel={activeTab?.label ?? ''}
								activeTabCharset={activeTab?.charset ?? null}
								onMessageReceived={(content) => {
									// Open each incoming MLLP message in a fresh tab so the
									// user does not lose the message currently in the editor.
									messageStore.newTab();
									const t = messageStore.activeTab;
									if (t) {
										messageStore.updateContent(t.id, content);
										const ts = new Date().toLocaleTimeString();
										t.label = tr('tab.inbox', { time: ts });
									}
								}}
								onOpenGenerated={handleOpenGenerated}
							/>
						</div>
					{/if}
					{#if showFhirPath && activeTab?.parseResult?.format?.startsWith('FHIR')}
						<div class="panel-body" class:hidden-panel={activeBottomPanel !== 'fhirpath'}>
							<FhirPathPanel messageId={activeTab.parseResult.message_id} stale={activeTab.parseStale ?? null} />
						</div>
					{/if}
					{#if showSegmentGrid && activeTab?.parseResult?.format === 'HL7v2'}
						<div class="panel-body" class:hidden-panel={activeBottomPanel !== 'segments'}>
							<SegmentGridPanel
								messageId={activeTab.parseResult.message_id}
								request={segmentGridRequest}
								onNavigate={(segIdx, pos) => handleTreeNavigateToEditor(segIdx, { field: pos, repetition: null, component: null })}
							/>
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
		bind:showSamples
		bind:showActivation
		bind:showSettings
		bind:showSchemaExport
		bind:showFhirRules
		bind:showFhirPackages
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
		onSampleSelected={handleSampleSelected}
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
		truncationCount={foldCount}
		cursorLine={activeTab?.cursorLine}
		cursorColumn={activeTab?.cursorColumn}
		isModified={activeTab?.isModified ?? false}
		parseStale={activeTab?.parseResult ? (activeTab?.parseStale ?? null) : null}
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
