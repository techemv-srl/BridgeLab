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
	 * Restore the persisted tab set. Skips if disabled or if the user already
	 * created a tab while startup I/O was in flight — restoreSession()
	 * replaces the whole tabs array and would discard their work.
	 * Returns true if at least one tab was restored.
	 */
	async restoreFromDisk(onTabRestored: (content: string) => void): Promise<boolean> {
		if (!this.restoreEnabled || messageStore.tabs.length !== 0) return false;
		const { loadSession } = await import('$lib/ipc/database');
		const sessionTabs = await loadSession();
		if (sessionTabs && sessionTabs.length > 0 && messageStore.tabs.length === 0) {
			const restored = messageStore.restoreSession(sessionTabs);
			// Re-parse any HL7/FHIR content so tree + inspector populate
			for (const tab of messageStore.tabs) {
				onTabRestored(tab.content);
			}
			return restored;
		}
		return false;
	}

	/** Debounced autosave of the current tab set. No-op when restore is off. */
	scheduleAutosave(delayMs = 800): void {
		if (!this.restoreEnabled) return;
		if (this.saveTimer) clearTimeout(this.saveTimer);
		this.saveTimer = setTimeout(async () => {
			try {
				const { saveSession } = await import('$lib/ipc/database');
				await saveSession(messageStore.serializeSession());
			} catch {
				// web mode or backend unavailable - ignore
			}
		}, delayMs);
	}
}

export const sessionStore = new SessionStore();
