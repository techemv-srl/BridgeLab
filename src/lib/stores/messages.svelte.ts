import type { ParseResult, TreeNode } from '$lib/types/hl7';

/** A single open message tab. */
export interface MessageTab {
	/** Unique tab ID */
	id: string;
	/** Display label (filename or "Untitled") */
	label: string;
	/** File path if opened from file, null if pasted */
	filePath: string | null;
	/** The current editor text content */
	content: string;
	/** Parse result from Rust (null if not yet parsed) */
	parseResult: ParseResult | null;
	/** Whether the content has been modified since last save/parse */
	isModified: boolean;
	/** Cursor position */
	cursorLine: number;
	cursorColumn: number;
}

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
			label: 'Untitled',
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

	/** Open a parsed message in a new tab. */
	openMessage(parseResult: ParseResult, filePath: string | null, content: string): string {
		// Check if file is already open
		if (filePath) {
			const existing = this.tabs.find((t) => t.filePath === filePath);
			if (existing) {
				this.activeTabId = existing.id;
				return existing.id;
			}
		}

		const id = `tab-${this.nextId++}`;
		const label = filePath ? filePath.split('/').pop()?.split('\\').pop() ?? 'Untitled' : 'Untitled';
		const tab: MessageTab = {
			id,
			label,
			filePath,
			content,
			parseResult,
			isModified: false,
			cursorLine: 1,
			cursorColumn: 1,
		};
		this.tabs.push(tab);
		this.activeTabId = id;
		return id;
	}

	/** Update the content of a tab. */
	updateContent(tabId: string, content: string) {
		const tab = this.tabs.find((t) => t.id === tabId);
		if (tab) {
			tab.content = content;
			tab.isModified = true;
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
			tab.parseResult = parseResult;
			if (truncatedText !== undefined) {
				tab.content = truncatedText;
			}
		}
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
				tab.label = filePath.split('/').pop()?.split('\\').pop() ?? tab.label;
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
			content: t.content,
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
				label: s.label || 'Untitled',
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
