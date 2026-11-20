import { messageStore } from './messages.svelte';

/** How soon after an edit the backend gets the latest tabs (see
 *  [`SessionStore.flushExitSnapshot`]): shorter than the autosave, since on
 *  a Windows shutdown it is all there is. */
export const EXIT_SNAPSHOT_DELAY_MS = 300;

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
		// A snapshot still waiting for its delay was made for the other
		// setting: it must not land after the one sent below (on, it would
		// put back as a recovery copy tabs the session already holds).
		this.cancelExitSnapshot();
		if (enabled) {
			// The backend now holds the whole session instead of the
			// unsaved tabs.
			await this.flushExitSnapshot();
			return;
		}
		if (this.saveTimer) {
			clearTimeout(this.saveTimer);
			this.saveTimer = null;
		}
		try {
			const { clearSession } = await import('$lib/ipc/database');
			await clearSession();
		} catch { /* web mode or backend unavailable */ }
		await this.flushExitSnapshot();
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
	 * Debounced autosave of the current tab set, and, sooner, the snapshot
	 * the backend keeps for the system ending the app (see
	 * [`flushExitSnapshot`]). Not before startup has finished: an early save
	 * of the still-empty tab set used to wipe the saved session before it
	 * was restored. With restore off nothing is written to disk.
	 */
	scheduleAutosave(delayMs = 800): void {
		if (!this.startupComplete) return;
		this.scheduleExitSnapshot();
		if (!this.restoreEnabled) return;
		if (this.saveTimer) clearTimeout(this.saveTimer);
		this.saveTimer = setTimeout(() => {
			this.saveTimer = null;
			void this.saveNow();
		}, delayMs);
	}

	private snapshotTimer: ReturnType<typeof setTimeout> | null = null;
	/** What was sent last, so an unchanged tab set is not sent again. */
	private lastSnapshotKey = '';
	private lastSnapshotContents: string[] = [];
	/** Set once the window closes the normal way: nothing to keep. */
	private closingNormally = false;

	private scheduleExitSnapshot(): void {
		this.cancelExitSnapshot();
		this.snapshotTimer = setTimeout(() => {
			this.snapshotTimer = null;
			void this.flushExitSnapshot();
		}, EXIT_SNAPSHOT_DELAY_MS);
	}

	private cancelExitSnapshot(): void {
		if (this.snapshotTimer) {
			clearTimeout(this.snapshotTimer);
			this.snapshotTimer = null;
		}
	}

	/**
	 * Hand the backend the latest tabs now, made for the setting as it is
	 * now: with restore on the whole session, with it off only the unsaved
	 * tabs. Held in memory, never on disk while the app runs. When the
	 * operating system ends the app without the window's close request (log
	 * out, shut down, the standard macOS Quit, Windows' end of session)
	 * nothing can ask the window any more, and on Windows and macOS not even
	 * for its latest text: the backend then writes this, once, as the
	 * session or as a recovery copy offered at the next start. Sent
	 * [`EXIT_SNAPSHOT_DELAY_MS`] after an edit, and at once when the window
	 * loses focus or is hidden (a shutdown from the Start menu or the Dock
	 * does that first).
	 */
	flushExitSnapshot(): Promise<boolean> {
		this.cancelExitSnapshot();
		if (!this.startupComplete) return Promise.resolve(false);
		return this.sendCurrentSnapshot();
	}

	private sendCurrentSnapshot(): Promise<boolean> {
		const session = this.restoreEnabled;
		return this.sendExitSnapshot(session ? messageStore.serializeSession() : unsavedTabs(), session);
	}

	/** Resolves true once the backend holds these tabs. */
	private sendExitSnapshot(tabs: SnapshotTab[], session: boolean, closing = false): Promise<boolean> {
		if (this.closingNormally && !closing) return Promise.resolve(false);
		// One after the other, so an older snapshot still on its way can
		// never land after a newer one, nor after the empty one sent when
		// the window closes.
		const sent = this.snapshotQueue.then(() => this.writeExitSnapshot(tabs, session));
		this.snapshotQueue = sent;
		return sent;
	}

	private snapshotQueue: Promise<boolean> = Promise.resolve(true);

	private async writeExitSnapshot(tabs: SnapshotTab[], session: boolean): Promise<boolean> {
		const key = JSON.stringify([session, tabs.map(({ content: _c, ...rest }) => rest)]);
		const contents = tabs.map((t) => t.content);
		if (key === this.lastSnapshotKey && contents.length === this.lastSnapshotContents.length
			&& contents.every((c, i) => c === this.lastSnapshotContents[i])) {
			return true;
		}
		try {
			const { setExitSnapshot } = await import('$lib/ipc/database');
			await setExitSnapshot(tabs, session);
			this.lastSnapshotKey = key;
			this.lastSnapshotContents = contents;
			return true;
		} catch {
			// web mode or backend unavailable
			return false;
		}
	}

	/**
	 * The window is closing the normal way, having asked about the unsaved
	 * tabs itself (or saved the session): the backend must keep nothing.
	 */
	async closeNormally(): Promise<void> {
		this.cancelExitSnapshot();
		this.closingNormally = true;
		await this.sendExitSnapshot([], false, true);
	}

	/** The recovery copy a run the system ended left, or none. */
	async loadRecovery(): Promise<SnapshotTab[]> {
		try {
			const { loadRecoveryTabs } = await import('$lib/ipc/database');
			return (await loadRecoveryTabs()) ?? [];
		} catch {
			return [];
		}
	}

	/** Delete the recovery copy. */
	async deleteRecovery(): Promise<void> {
		try {
			const { clearRecoveryTabs } = await import('$lib/ipc/database');
			await clearRecoveryTabs();
		} catch { /* web mode or backend unavailable */ }
	}

	/**
	 * After the user answered the recovery offer: delete the copy, but when
	 * its tabs were restored (already added to the open tabs), only once
	 * they are kept elsewhere. This runs during startup, before the
	 * snapshot and the autosave start: deleting the copy first left the
	 * restored tabs nowhere but in the window, lost if the app ended before
	 * the first snapshot. So here, startup or not: with restore on the
	 * session is written with them, with it off the backend takes the exit
	 * snapshot holding them. If that fails the copy stays, and is offered
	 * again at the next start.
	 */
	async finishRecovery(restored: boolean): Promise<void> {
		if (restored && !(await this.keepOpenTabs())) return;
		await this.deleteRecovery();
	}

	private async keepOpenTabs(): Promise<boolean> {
		const snapshot = await this.sendCurrentSnapshot();
		return this.restoreEnabled ? this.saveNow() : snapshot;
	}

	/** Saves still being written, and whether the last one failed. */
	private savesInFlight = 0;
	private lastSaveFailed = false;

	/** The latest edits may not be in the saved session yet: a save is
	 *  waiting for its debounce, is still being written, or the last one
	 *  failed. A reload then would lose them. */
	get autosavePending(): boolean {
		return this.saveTimer !== null || this.savesInFlight > 0 || this.lastSaveFailed;
	}

	/** Save the tab set now (window closing, or the system ending the
	 *  app): the debounce would lose the last edits. The backend's exit
	 *  snapshot is brought up to date too. Resolves false when the session
	 *  write failed, so the caller can keep the window open. */
	async flush(): Promise<boolean> {
		if (this.saveTimer) {
			clearTimeout(this.saveTimer);
			this.saveTimer = null;
		}
		if (!this.startupComplete) return true;
		await this.flushExitSnapshot();
		if (!this.restoreEnabled) return true;
		return this.saveNow();
	}

	/** What was saved last: autosave fires on every edit, tab switch and
	 *  cursor move, and an unchanged tab set is not written again. */
	private lastMeta = '';
	private lastContents: string[] = [];

	private async saveNow(): Promise<boolean> {
		this.savesInFlight++;
		try {
			const ok = await this.writeSession();
			this.lastSaveFailed = !ok;
			return ok;
		} finally {
			this.savesInFlight--;
		}
	}

	private async writeSession(): Promise<boolean> {
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

type SnapshotTab = ReturnType<typeof messageStore.serializeSession>[number];

/** The tabs whose changes are not saved to a file, as the session stores
 *  tabs: a modified file tab, or an untitled one with text. */
function unsavedTabs(): SnapshotTab[] {
	return messageStore
		.serializeSession()
		.filter((t) => t.is_modified && (t.file_path !== null || t.content.trim() !== ''));
}

export const sessionStore = new SessionStore();
