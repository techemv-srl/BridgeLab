import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

vi.mock('$lib/ipc/database', () => ({
	loadSession: vi.fn(async () => []),
	saveSession: vi.fn(async () => undefined),
	clearSession: vi.fn(async () => undefined),
	setExitSnapshot: vi.fn(async () => undefined),
	loadRecoveryTabs: vi.fn(async () => []),
	clearRecoveryTabs: vi.fn(async () => undefined),
}));

import {
	loadSession, saveSession, clearSession, setExitSnapshot, loadRecoveryTabs, clearRecoveryTabs,
} from '$lib/ipc/database';
import { sessionStore, EXIT_SNAPSHOT_DELAY_MS } from './session.svelte';
import { messageStore } from './messages.svelte';

const savedTab = (label: string, active = false) => ({
	tab_order: 0, label, file_path: null, content: `MSH|${label}`,
	is_modified: false, is_active: active, cursor_line: 1, cursor_column: 1,
});

beforeEach(() => {
	messageStore.closeAllTabs();
	sessionStore.restoreEnabled = true;
	sessionStore.startupComplete = false;
	vi.mocked(loadSession).mockReset().mockResolvedValue([]);
	vi.mocked(saveSession).mockReset().mockResolvedValue(undefined);
});

afterEach(() => {
	vi.useRealTimers();
});

describe('restoreFromDisk', () => {
	it('restores tabs and re-parses each one', async () => {
		vi.mocked(loadSession).mockResolvedValue([savedTab('a', true), { ...savedTab('b'), tab_order: 1 }]);
		const parsed: Array<[string, string]> = [];
		const restored = await sessionStore.restoreFromDisk((id, c) => parsed.push([id, c]));
		expect(restored).toBe(true);
		expect(messageStore.tabs).toHaveLength(2);
		// Each parse is bound to its own tab, not to the active one.
		expect(parsed).toEqual(messageStore.tabs.map((t) => [t.id, t.content]));
		expect(parsed.map(([, c]) => c)).toEqual(['MSH|a', 'MSH|b']);
	});

	it('skips when restore is disabled', async () => {
		sessionStore.restoreEnabled = false;
		vi.mocked(loadSession).mockResolvedValue([savedTab('a')]);
		expect(await sessionStore.restoreFromDisk(() => {})).toBe(false);
		expect(loadSession).not.toHaveBeenCalled();
	});

	it('never clobbers a tab the user created during startup', async () => {
		messageStore.newTab();
		vi.mocked(loadSession).mockResolvedValue([savedTab('old')]);
		expect(await sessionStore.restoreFromDisk(() => {})).toBe(false);
		expect(messageStore.tabs).toHaveLength(1);
		expect(messageStore.activeTab?.label).toBe('Untitled');
	});

	it('returns false for an empty persisted session', async () => {
		expect(await sessionStore.restoreFromDisk(() => {})).toBe(false);
		expect(messageStore.tabs).toHaveLength(0);
	});
});

describe('scheduleAutosave', () => {
	beforeEach(() => {
		sessionStore.startupComplete = true;
	});

	it('does not save before startup has finished (it would wipe the saved session)', async () => {
		vi.useFakeTimers();
		sessionStore.startupComplete = false;
		sessionStore.scheduleAutosave(100);
		await vi.advanceTimersByTimeAsync(200);
		expect(saveSession).not.toHaveBeenCalled();
	});

	it('flush writes at once and cancels the pending debounce', async () => {
		vi.useFakeTimers();
		const id = messageStore.newTab();
		messageStore.updateContent(id, 'MSH|last-keystrokes');
		sessionStore.scheduleAutosave(800);
		expect(await sessionStore.flush()).toBe(true);
		expect(saveSession).toHaveBeenCalledTimes(1);
		expect(saveSession).toHaveBeenCalledWith(
			expect.arrayContaining([expect.objectContaining({ content: 'MSH|last-keystrokes' })]),
		);
		await vi.advanceTimersByTimeAsync(1000);
		expect(saveSession).toHaveBeenCalledTimes(1);
	});

	it('flush reports a failed write', async () => {
		vi.mocked(saveSession).mockRejectedValue(new Error('disk full'));
		messageStore.newTab();
		expect(await sessionStore.flush()).toBe(false);
	});

	it('debounces: only the last schedule within the window fires', async () => {
		vi.useFakeTimers();
		messageStore.newTab();
		sessionStore.scheduleAutosave(800);
		vi.advanceTimersByTime(400);
		sessionStore.scheduleAutosave(800);
		vi.advanceTimersByTime(799);
		expect(saveSession).not.toHaveBeenCalled();
		await vi.advanceTimersByTimeAsync(1);
		expect(saveSession).toHaveBeenCalledTimes(1);
	});

	it('persists the serialized tab set', async () => {
		vi.useFakeTimers();
		const id = messageStore.newTab();
		messageStore.updateContent(id, 'MSH|persist-me');
		sessionStore.scheduleAutosave(100);
		await vi.advanceTimersByTimeAsync(100);
		expect(saveSession).toHaveBeenCalledWith(
			expect.arrayContaining([expect.objectContaining({ content: 'MSH|persist-me' })]),
		);
	});

	it('reports a save still waiting for its debounce, and none once it has run', async () => {
		// What a reload of the page asks about: the newest edits are not in
		// the saved session until the debounced save has run.
		vi.useFakeTimers();
		messageStore.newTab();
		expect(sessionStore.autosavePending).toBe(false);
		sessionStore.scheduleAutosave(800);
		expect(sessionStore.autosavePending).toBe(true);
		await vi.advanceTimersByTimeAsync(800);
		expect(saveSession).toHaveBeenCalledTimes(1);
		expect(sessionStore.autosavePending).toBe(false);
		sessionStore.scheduleAutosave(800);
		await sessionStore.flush();
		expect(sessionStore.autosavePending).toBe(false);
		vi.useRealTimers();
	});

	it('still reports the edits pending while the save is being written, and after it failed', async () => {
		// The debounce has fired but the write has not finished (or failed):
		// a reload then would lose the newest edits all the same.
		let finish!: (v?: unknown) => void;
		vi.mocked(saveSession).mockImplementationOnce(() => new Promise((r) => { finish = r; }) as never);
		messageStore.newTab();
		messageStore.updateContent(messageStore.activeTabId!, 'MSH|in-flight');
		const writing = sessionStore.flush();
		await new Promise((r) => setTimeout(r, 0));
		expect(sessionStore.autosavePending).toBe(true);
		finish();
		expect(await writing).toBe(true);
		expect(sessionStore.autosavePending).toBe(false);

		vi.mocked(saveSession).mockRejectedValueOnce(new Error('disk full'));
		messageStore.updateContent(messageStore.activeTabId!, 'MSH|will-fail');
		expect(await sessionStore.flush()).toBe(false);
		expect(sessionStore.autosavePending).toBe(true);
		messageStore.updateContent(messageStore.activeTabId!, 'MSH|works-again');
		expect(await sessionStore.flush()).toBe(true);
		expect(sessionStore.autosavePending).toBe(false);
	});

	it('does nothing when restore is disabled', async () => {
		vi.useFakeTimers();
		sessionStore.restoreEnabled = false;
		sessionStore.scheduleAutosave(100);
		await vi.advanceTimersByTimeAsync(200);
		expect(saveSession).not.toHaveBeenCalled();
	});

	it('swallows backend failures silently (web mode)', async () => {
		vi.useFakeTimers();
		vi.mocked(saveSession).mockRejectedValue(new Error('no tauri'));
		messageStore.newTab();
		sessionStore.scheduleAutosave(100);
		await expect(vi.advanceTimersByTimeAsync(150)).resolves.not.toThrow();
	});
});

describe('setRestoreEnabled', () => {
	it('deletes the saved tabs when restore is turned off', async () => {
		vi.mocked(clearSession).mockClear();
		sessionStore.startupComplete = true;
		vi.useFakeTimers();
		sessionStore.scheduleAutosave(10);
		await sessionStore.setRestoreEnabled(false);
		expect(clearSession).toHaveBeenCalledOnce();
		await vi.advanceTimersByTimeAsync(50);
		expect(saveSession).not.toHaveBeenCalled();
		expect(await sessionStore.flush()).toBe(true);
		expect(saveSession).not.toHaveBeenCalled();
	});

	it('keeps them when restore is turned on', async () => {
		vi.mocked(clearSession).mockClear();
		await sessionStore.setRestoreEnabled(true);
		expect(clearSession).not.toHaveBeenCalled();
		expect(sessionStore.restoreEnabled).toBe(true);
	});
});

describe('exit snapshot', () => {
	beforeEach(() => {
		sessionStore.startupComplete = true;
		sessionStore.restoreEnabled = false;
		vi.mocked(setExitSnapshot).mockReset().mockResolvedValue(undefined);
	});

	const lastCall = () => vi.mocked(setExitSnapshot).mock.calls.at(-1);
	const lastSnapshot = () => lastCall()?.[0];

	it('with restore off hands the backend only the unsaved tabs, soon after an edit, and never writes the session', async () => {
		vi.useFakeTimers();
		const dirty = messageStore.newTab();
		messageStore.updateContent(dirty, 'MSH|unsaved');
		messageStore.newTab(); // empty: nothing to lose
		sessionStore.scheduleAutosave();
		await vi.advanceTimersByTimeAsync(EXIT_SNAPSHOT_DELAY_MS - 50);
		expect(setExitSnapshot).not.toHaveBeenCalled();
		await vi.advanceTimersByTimeAsync(100);
		expect(lastSnapshot()?.map((t) => t.content)).toEqual(['MSH|unsaved']);
		expect(lastSnapshot()?.[0].is_modified).toBe(true);
		expect(lastCall()?.[1]).toBe(false);
		await vi.advanceTimersByTimeAsync(1000);
		expect(saveSession).not.toHaveBeenCalled();
	});

	it('with restore on hands over the whole session before the autosave writes it', async () => {
		vi.useFakeTimers();
		sessionStore.restoreEnabled = true;
		const a = messageStore.newTab();
		messageStore.updateContent(a, 'MSH|edited');
		messageStore.newTab();
		sessionStore.scheduleAutosave(800);
		await vi.advanceTimersByTimeAsync(EXIT_SNAPSHOT_DELAY_MS + 10);
		expect(lastCall()?.[1]).toBe(true);
		expect(lastSnapshot()?.map((t) => t.content)).toEqual(['MSH|edited', '']);
		expect(saveSession).not.toHaveBeenCalled();
		await vi.advanceTimersByTimeAsync(800);
		expect(saveSession).toHaveBeenCalledOnce();
	});

	it('is sent at once on request (the window losing focus or being hidden)', async () => {
		vi.useFakeTimers();
		const id = messageStore.newTab();
		messageStore.updateContent(id, 'MSH|before-shutdown');
		sessionStore.scheduleAutosave();
		await sessionStore.flushExitSnapshot();
		expect(lastSnapshot()?.[0].content).toBe('MSH|before-shutdown');
		vi.mocked(setExitSnapshot).mockClear();
		await vi.advanceTimersByTimeAsync(1000);
		expect(setExitSnapshot).not.toHaveBeenCalled(); // the pending one was cancelled
	});

	it('is not sent before startup has finished (it would replace the saved session)', async () => {
		vi.useFakeTimers();
		sessionStore.startupComplete = false;
		sessionStore.restoreEnabled = true;
		sessionStore.scheduleAutosave();
		await sessionStore.flushExitSnapshot();
		await vi.advanceTimersByTimeAsync(1000);
		expect(setExitSnapshot).not.toHaveBeenCalled();
	});

	it('does not send the same tabs twice', async () => {
		const id = messageStore.newTab();
		messageStore.updateContent(id, 'MSH|once');
		await sessionStore.flush();
		await sessionStore.flush();
		expect(vi.mocked(setExitSnapshot).mock.calls.filter((c) => c[0].length === 1)).toHaveLength(1);
	});

	it('flush sends the last edits at once, for the system ending the app', async () => {
		vi.useFakeTimers();
		const id = messageStore.newTab();
		messageStore.updateContent(id, 'MSH|last-keystrokes');
		sessionStore.scheduleAutosave(800);
		expect(await sessionStore.flush()).toBe(true);
		expect(lastSnapshot()?.[0].content).toBe('MSH|last-keystrokes');
		await vi.advanceTimersByTimeAsync(1000);
		expect(saveSession).not.toHaveBeenCalled();
	});

	it('turning restore on right after an edit leaves the session snapshot, not the old unsaved tabs', async () => {
		vi.useFakeTimers();
		const id = messageStore.newTab();
		messageStore.updateContent(id, 'MSH|edited-with-restore-off');
		sessionStore.scheduleAutosave(); // snapshot of the unsaved tabs pending
		await vi.advanceTimersByTimeAsync(100);
		await sessionStore.setRestoreEnabled(true);
		await vi.advanceTimersByTimeAsync(1000);
		// The pending snapshot did not fire after the switch: the backend
		// holds the session, which the next start restores once.
		expect(vi.mocked(setExitSnapshot).mock.calls.map((c) => c[1])).toEqual([true]);
		expect(lastSnapshot()?.[0].content).toBe('MSH|edited-with-restore-off');
	});

	it('turning restore off right after an edit leaves the unsaved tabs, not the old session', async () => {
		vi.useFakeTimers();
		sessionStore.restoreEnabled = true;
		const saved = messageStore.newTab();
		messageStore.updateContent(saved, 'MSH|x');
		messageStore.markSaved(saved, '/tmp/x.hl7');
		const dirty = messageStore.newTab();
		messageStore.updateContent(dirty, 'MSH|unsaved');
		sessionStore.scheduleAutosave();
		await vi.advanceTimersByTimeAsync(100);
		await sessionStore.setRestoreEnabled(false);
		await vi.advanceTimersByTimeAsync(1000);
		expect(vi.mocked(setExitSnapshot).mock.calls.map((c) => c[1])).toEqual([false]);
		expect(lastSnapshot()?.some((t) => t.content === 'MSH|unsaved')).toBe(true);
	});

	it('loads and deletes the recovery copy, and copes without a backend', async () => {
		vi.mocked(loadRecoveryTabs).mockResolvedValueOnce([{ ...savedTab('kept'), is_modified: true }]);
		expect((await sessionStore.loadRecovery()).map((t) => t.label)).toEqual(['kept']);
		await sessionStore.deleteRecovery();
		expect(clearRecoveryTabs).toHaveBeenCalledOnce();
		vi.mocked(loadRecoveryTabs).mockRejectedValueOnce(new Error('no tauri'));
		expect(await sessionStore.loadRecovery()).toEqual([]);
	});

	describe('finishRecovery', () => {
		const recovered = (content: string) => [{ ...savedTab('kept'), content, is_modified: true }];
		let order: string[];

		beforeEach(() => {
			// The offer is answered during startup, before snapshots and
			// the autosave start.
			sessionStore.startupComplete = false;
			order = [];
			vi.mocked(setExitSnapshot).mockImplementation(async () => { order.push('snapshot'); });
			vi.mocked(saveSession).mockImplementation(async () => { order.push('session'); });
			vi.mocked(clearRecoveryTabs).mockReset().mockImplementation(async () => { order.push('delete'); });
		});

		it('with restore off deletes the copy only after the backend holds the restored tabs', async () => {
			messageStore.appendRecoveredTabs(recovered('MSH|restored-off'));
			await sessionStore.finishRecovery(true);
			expect(order).toEqual(['snapshot', 'delete']);
			expect(lastCall()?.[1]).toBe(false);
			expect(lastSnapshot()?.map((t) => t.content)).toEqual(['MSH|restored-off']);
			expect(saveSession).not.toHaveBeenCalled();
		});

		it('with restore on deletes the copy only after the session is written with them', async () => {
			sessionStore.restoreEnabled = true;
			messageStore.appendRecoveredTabs(recovered('MSH|restored-on'));
			await sessionStore.finishRecovery(true);
			expect(order.at(-1)).toBe('delete');
			expect(order).toContain('session');
			expect(vi.mocked(saveSession).mock.calls.at(-1)?.[0].map((t) => t.content)).toEqual(['MSH|restored-on']);
		});

		it('keeps the copy when the restored tabs could not be kept', async () => {
			vi.mocked(setExitSnapshot).mockRejectedValue(new Error('backend gone'));
			messageStore.appendRecoveredTabs(recovered('MSH|not-kept-off'));
			await sessionStore.finishRecovery(true);
			expect(clearRecoveryTabs).not.toHaveBeenCalled();

			sessionStore.restoreEnabled = true;
			vi.mocked(saveSession).mockRejectedValue(new Error('disk full'));
			messageStore.updateContent(messageStore.tabs[0].id, 'MSH|not-kept-on');
			await sessionStore.finishRecovery(true);
			expect(clearRecoveryTabs).not.toHaveBeenCalled();
		});

		it('deletes the copy at once when the tabs were discarded', async () => {
			await sessionStore.finishRecovery(false);
			expect(order).toEqual(['delete']);
		});
	});

	// Last: a normal close is final for the store.
	it('a normal close empties the snapshot last, even with an older one still on its way', async () => {
		let release: () => void = () => {};
		vi.mocked(setExitSnapshot).mockImplementationOnce(() => new Promise<void>((r) => { release = r; }));
		const id = messageStore.newTab();
		messageStore.updateContent(id, 'MSH|discarded');
		const older = sessionStore.flush();
		const closing = sessionStore.closeNormally();
		await vi.waitFor(() => expect(setExitSnapshot).toHaveBeenCalledOnce());
		release();
		await Promise.all([older, closing]);
		expect(lastCall()).toEqual([[], false]);
		// Nothing more is sent once the window is closing.
		messageStore.updateContent(id, 'MSH|after');
		await sessionStore.flush();
		expect(lastCall()).toEqual([[], false]);
	});
});

describe('appendRecoveredTabs', () => {
	it('adds the recovered tabs after the open ones, unsaved, and shows the first', () => {
		const open = messageStore.newTab();
		const ids = messageStore.appendRecoveredTabs([
			{ ...savedTab('second'), tab_order: 1 },
			{ ...savedTab('first'), tab_order: 0, file_path: '/data/a.hl7' },
		]);
		expect(messageStore.tabs.map((t) => t.label)).toEqual(['Untitled', 'first', 'second']);
		expect(messageStore.tabs[0].id).toBe(open);
		expect(messageStore.activeTabId).toBe(ids[0]);
		expect(messageStore.tabs.slice(1).every((t) => t.isModified)).toBe(true);
		expect(messageStore.tabs[1].filePath).toBe('/data/a.hl7');
		expect(messageStore.tabs[1].content).toBe('MSH|first');
	});
});
