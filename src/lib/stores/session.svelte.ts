import { messageStore } from './messages.svelte';

/**
 * Notepad++-style session persistence: restore the open tab set on startup,
 * autosave it (debounced) whenever tabs change. The dependency-tracking
 * $effects stay in AppShell — this store owns the state and the I/O.
 */
class SessionStore {
	/** Mirrors the `restore_session` preference; Settings updates it live. */
	restoreEnabled = $state(true);
	/**
	 * Welcome screen must not render (and accept input) until the async
	 * startup — including session restore — has finished, or a tab created
	 * meanwhile would be clobbered by restoreSession().
	 */
	startupComplete = $state(false);

	private saveTimer: ReturnType<typeof setTimeout> | null = null;

	/**
	 * Turn tab restore on or off. Off also deletes the saved tabs: they can
	 * hold patient data, and with restore off nothing would ever read (or
	 * overwrite) them again.
	 */
	async setRestoreEnabled(enabled: boolean): Promise<void> {
		this.restoreEnabled = enabled;
		if (enabled) return;
		if (this.saveTimer) {
			clearTimeout(this.saveTimer);
			this.saveTimer = null;
		}
		try {
			const { clearSession } = await import('$lib/ipc/database');
			await clearSession();
		} catch { /* web mode or backend unavailable */ }
	}

	/**
	 * Restore the persisted tab set. Skips if disabled or if the user already
	 * created a tab while startup I/O was in flight — restoreSession()
	 * replaces the whole tabs array and would discard their work.
	 * Returns true if at least one tab was restored.
	 */
	async restoreFromDisk(onTabRestored: (tabId: string, content: string) => void): Promise<boolean> {
		if (!this.restoreEnabled || messageStore.tabs.length !== 0) return false;
		const { loadSession } = await import('$lib/ipc/database');
		const sessionTabs = await loadSession();
		if (sessionTabs && sessionTabs.length > 0 && messageStore.tabs.length === 0) {
			const restored = messageStore.restoreSession(sessionTabs);
			// Re-parse any HL7/FHIR content so tree + inspector populate
			// Each result goes to its own tab: parsing "the active tab" gave
			// the active tab whichever parse finished last.
			for (const tab of messageStore.tabs) {
				onTabRestored(tab.id, tab.content);
			}
			return restored;
		}
		return false;
	}

	/**
	 * Debounced autosave of the current tab set. No-op when restore is off,
	 * and before startup has finished: an early save of the still-empty tab
	 * set used to wipe the saved session before it was restored.
	 */
	scheduleAutosave(delayMs = 800): void {
		if (!this.restoreEnabled || !this.startupComplete) return;
		if (this.saveTimer) clearTimeout(this.saveTimer);
		this.saveTimer = setTimeout(() => void this.saveNow(), delayMs);
	}

	/** Save the tab set now (window closing): the debounce would lose the
	 *  last edits. Resolves false when the write failed, so the caller can
	 *  keep the window open. */
	async flush(): Promise<boolean> {
		if (this.saveTimer) {
			clearTimeout(this.saveTimer);
			this.saveTimer = null;
		}
		if (!this.restoreEnabled || !this.startupComplete) return true;
		return this.saveNow();
	}

	/** What was saved last: autosave fires on every edit, tab switch and
	 *  cursor move, and an unchanged tab set is not written again. */
	private lastMeta = '';
	private lastContents: string[] = [];

	private async saveNow(): Promise<boolean> {
		try {
			const { saveSession } = await import('$lib/ipc/database');
			// Turned off meanwhile: the saved tabs were just deleted.
			if (!this.restoreEnabled) return true;
			const tabs = messageStore.serializeSession();
			const meta = JSON.stringify(tabs.map(({ content: _c, ...rest }) => rest));
			const contents = tabs.map((t) => t.content);
			if (meta === this.lastMeta && contents.length === this.lastContents.length && contents.every((c, i) => c === this.lastContents[i])) {
				return true;
			}
			await saveSession(tabs);
			this.lastMeta = meta;
			this.lastContents = contents;
			return true;
		} catch {
			// web mode or backend unavailable
			return false;
		}
	}
}

export const sessionStore = new SessionStore();
