import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

vi.mock('$lib/ipc/database', () => ({
	loadSession: vi.fn(async () => []),
	saveSession: vi.fn(async () => undefined),
	clearSession: vi.fn(async () => undefined),
}));

import { loadSession, saveSession, clearSession } from '$lib/ipc/database';
import { sessionStore } from './session.svelte';
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
