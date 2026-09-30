import type { ParseResult, TreeNode } from '$lib/types/hl7';
import { t } from '$lib/i18n';
import { baseName, samePath } from '$lib/paths';
import { setLiveIds } from './message-gc';

/** What a tab last knew of its file on disk (see file_stat). */
export interface FileStamp {
	modified_ms: number;
	size: number;
}

/**
 * The parse result as a tab keeps it: without the texts it carries. The
 * tab's content is the text; a second copy of a 10 MB message (open_file's
 * full_text, a FHIR resource's truncated_text) stayed in memory for as
 * long as the tab was open.
 */
export function leanResult(result: ParseResult): ParseResult | null {
	// open_file's answer for a file that did not parse: no message.
	if (result.parse_error) return null;
	return { ...result, full_text: undefined, truncated_text: '' };
}

/** A single open message tab. */
export interface MessageTab {
	/** Unique tab ID */
	id: string;
	/** Display label (filename or "Untitled") */
	label: string;
	/** File path if opened from file, null if pasted */
	filePath: string | null;
	/** The file with symlinks resolved (canonical_path), when known: the
	 *  same file opened under another spelling is this tab. */
	realPath?: string | null;
	/** The current editor text content */
	content: string;
	/** Parse result from Rust (null if not yet parsed) */
	parseResult: ParseResult | null;
	/** Whether the content has been modified since last save/parse */
	isModified: boolean;
	/** Cursor position */
	cursorLine: number;
	cursorColumn: number;
	/** Charset of the file this tab was opened from, when it was not UTF-8 */
	charset?: string | null;
	/** The file as it was when opened or last saved: a different one on
	 *  disk means another program changed it. Undefined = not known. */
	diskStamp?: FileStamp;
	/** Restored from the session with unsaved edits: what the file looked
	 *  like when those edits were made is not known, so it may have changed
	 *  while the app was closed and the first save asks. */
	diskUnverified?: boolean;
	/** The changed file the user was already told about (null: deleted),
	 *  so the same change is not reported on every focus. */
	externalNotice?: FileStamp | null;
	/** The current text did not parse: `parseResult` (tree, status bar,
	 *  FHIRPath) still describes an earlier version. Null/undefined = the
	 *  parse result is current, as far as is known. */
	parseStale?: string | null;
}

/** Largest unedited file tab whose text the session stores itself. */
export const SESSION_INLINE_MAX = 64 * 1024;

/** Global message store using Svelte 5 runes. */
class MessageStore {
	tabs = $state<MessageTab[]>([]);
	activeTabId = $state<string | null>(null);

	private nextId = 1;

	/** Get the active tab. */
	get activeTab(): MessageTab | undefined {
		return this.tabs.find((t) => t.id === this.activeTabId);
	}

	/** Create a new empty tab. */
	newTab(): string {
		const id = `tab-${this.nextId++}`;
		const tab: MessageTab = {
			id,
			label: t('tab.untitled'),
			filePath: null,
			content: '',
			parseResult: null,
			isModified: false,
			cursorLine: 1,
			cursorColumn: 1,
		};
		this.tabs.push(tab);
		this.activeTabId = id;
		return id;
	}

	/** Open a message in a new tab. A result with `parse_error` opens it
	 *  unparsed (a file that is not a message BridgeLab can read). */
	openMessage(parseResult: ParseResult, filePath: string | null, content: string, realPath: string | null = null): string {
		// Check if file is already open
		if (filePath) {
			const existing = this.findByPath(filePath, realPath);
			if (existing) {
				this.activeTabId = existing.id;
				return existing.id;
			}
		}

		const id = `tab-${this.nextId++}`;
		const label = filePath ? baseName(filePath) : t('tab.untitled');
		const tab: MessageTab = {
			id,
			label,
			filePath,
			realPath,
			content,
			parseResult: leanResult(parseResult),
			isModified: false,
			cursorLine: 1,
			cursorColumn: 1,
			charset: parseResult.source_charset ?? null,
		};
		this.tabs.push(tab);
		this.activeTabId = id;
		return id;
	}

	/** The tab showing this file (however its path is written, or through
	 *  whichever symlink when `realPath` is known), if any. */
	findByPath(filePath: string, realPath: string | null = null): MessageTab | undefined {
		return this.tabs.find((t) => t.filePath !== null && samePath(t.filePath, filePath))
			?? (realPath ? this.tabs.find((t) => t.realPath && samePath(t.realPath, realPath)) : undefined);
	}

	/** Update the content of a tab. The same text again (the editor echoing
	 *  Expand All, say) is not an edit. */
	updateContent(tabId: string, content: string) {
		const tab = this.tabs.find((t) => t.id === tabId);
		if (tab && tab.content !== content) {
			tab.content = content;
			tab.isModified = true;
		}
	}

	/** Replace a tab's text with its file as now on disk (unmodified). */
	reloadFromDisk(tabId: string, content: string, parseResult: ParseResult, stamp: FileStamp | undefined) {
		const tab = this.tabs.find((t) => t.id === tabId);
		if (tab) {
			tab.content = content;
			tab.parseResult = leanResult(parseResult);
			tab.parseStale = null;
			tab.isModified = false;
			tab.charset = parseResult.source_charset ?? null;
			tab.diskStamp = stamp;
			tab.diskUnverified = false;
			tab.externalNotice = undefined;
		}
	}

	/** Record what the tab's file on disk looks like now. */
	setDiskStamp(tabId: string, stamp: FileStamp | undefined) {
		const tab = this.tabs.find((t) => t.id === tabId);
		if (tab) {
			tab.diskStamp = stamp;
			tab.diskUnverified = false;
			tab.externalNotice = undefined;
		}
	}

	/**
	 * Update parse result for a tab.
	 * If truncatedText is provided, also replaces tab.content (use only for explicit
	 * user actions like open file / re-parse). When called from background auto-parse
	 * while the user is typing, omit truncatedText so the editor content/cursor is
	 * not disturbed.
	 */
	updateParseResult(tabId: string, parseResult: ParseResult, truncatedText?: string) {
		const tab = this.tabs.find((t) => t.id === tabId);
		if (tab) {
			tab.parseResult = leanResult(parseResult);
			tab.parseStale = null;
			if (truncatedText !== undefined) {
				tab.content = truncatedText;
			}
		}
	}

	/** The tab's current text failed to parse (`reason`): what was parsed
	 *  before is kept, marked out of date. Only meaningful when there is a
	 *  parse result to keep. */
	markParseStale(tabId: string, reason: string) {
		const tab = this.tabs.find((t) => t.id === tabId);
		if (tab?.parseResult) tab.parseStale = reason || 'parse failed';
	}

	/** Update cursor position for a tab. */
	updateCursor(tabId: string, line: number, column: number) {
		const tab = this.tabs.find((t) => t.id === tabId);
		if (tab) {
			tab.cursorLine = line;
			tab.cursorColumn = column;
		}
	}

	/** Mark a tab as saved. */
	markSaved(tabId: string, filePath?: string) {
		const tab = this.tabs.find((t) => t.id === tabId);
		if (tab) {
			tab.isModified = false;
			if (filePath) {
				tab.filePath = filePath;
				tab.label = baseName(filePath) || tab.label;
			}
		}
	}

	/** Close a tab. Returns the next active tab ID or null. */
	closeTab(tabId: string): string | null {
		const idx = this.tabs.findIndex((t) => t.id === tabId);
		if (idx === -1) return this.activeTabId;

		this.tabs.splice(idx, 1);

		if (this.activeTabId === tabId) {
			if (this.tabs.length === 0) {
				this.activeTabId = null;
			} else {
				// Activate the tab at the same position or the last one
				const newIdx = Math.min(idx, this.tabs.length - 1);
				this.activeTabId = this.tabs[newIdx].id;
			}
		}
		return this.activeTabId;
	}

	/** Close all tabs except the specified one. */
	closeOtherTabs(tabId: string) {
		this.tabs = this.tabs.filter((t) => t.id === tabId);
		this.activeTabId = tabId;
	}

	/** Close all tabs. */
	closeAllTabs() {
		this.tabs = [];
		this.activeTabId = null;
	}

	/** Set the active tab. */
	setActiveTab(tabId: string) {
		if (this.tabs.some((t) => t.id === tabId)) {
			this.activeTabId = tabId;
		}
	}

	/**
	 * Serialize the current session for persistence. Excludes parseResult
	 * (it is re-derived from content on restore).
	 */
	serializeSession(): Array<{
		tab_order: number;
		label: string;
		file_path: string | null;
		content: string;
		is_modified: boolean;
		is_active: boolean;
		cursor_line: number;
		cursor_column: number;
	}> {
		return this.tabs.map((t, idx) => ({
			tab_order: idx,
			label: t.label,
			file_path: t.filePath,
			// A large file tab without edits is read back from its file at
			// restore (see adoptRestoredTab): storing it too rewrote megabytes
			// on every autosave and doubled the time to start.
			content: t.filePath && !t.isModified && t.content.length > SESSION_INLINE_MAX ? '' : t.content,
			is_modified: t.isModified,
			is_active: t.id === this.activeTabId,
			cursor_line: t.cursorLine,
			cursor_column: t.cursorColumn,
		}));
	}

	/**
	 * Restore tabs from a persisted session. Skips empty sessions.
	 * Returns true if at least one tab was restored.
	 */
	restoreSession(
		tabs: Array<{
			tab_order: number;
			label: string;
			file_path: string | null;
			content: string;
			is_modified: boolean;
			is_active: boolean;
			cursor_line: number;
			cursor_column: number;
		}>,
	): boolean {
		if (!tabs || tabs.length === 0) return false;
		const sorted = [...tabs].sort((a, b) => a.tab_order - b.tab_order);
		this.tabs = [];
		let activeId: string | null = null;
		for (const s of sorted) {
			const id = `tab-${this.nextId++}`;
			const tab: MessageTab = {
				id,
				label: s.label || t('tab.untitled'),
				filePath: s.file_path,
				content: s.content,
				parseResult: null,
				isModified: s.is_modified,
				cursorLine: s.cursor_line,
				cursorColumn: s.cursor_column,
			};
			this.tabs.push(tab);
			if (s.is_active) activeId = id;
		}
		this.activeTabId = activeId ?? this.tabs[0]?.id ?? null;
		return true;
	}
}

/** Singleton message store. */
export const messageStore = new MessageStore();
setLiveIds(() => messageStore.tabs.map((t) => t.parseResult?.message_id));
