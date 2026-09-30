import { describe, it, expect, beforeEach, vi } from 'vitest';

const savedFileResult = { path: '', bytes_written: 0 };

vi.mock('$lib/ipc/parser', () => ({
	openFile: vi.fn(),
	saveFile: vi.fn(async () => savedFileResult),
	fileStat: vi.fn(async () => null),
	canonicalPath: vi.fn(async () => null),
}));
vi.mock('$lib/ipc/database', () => ({
	getRecentFiles: vi.fn(async () => []),
	addRecentFile: vi.fn(async () => undefined),
	removeRecentFile: vi.fn(async () => undefined),
	clearRecentFiles: vi.fn(async () => undefined),
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({
	open: vi.fn(async () => null),
	save: vi.fn(async () => null),
}));

import { openFile, saveFile, fileStat, canonicalPath } from '$lib/ipc/parser';
import { getRecentFiles, addRecentFile, removeRecentFile, clearRecentFiles } from '$lib/ipc/database';
import { open, save } from '@tauri-apps/plugin-dialog';
import { fileOpsStore, sameStamp } from './file-ops.svelte';
import { messageStore } from './messages.svelte';
import { dialogStore } from './dialog.svelte';

const parseResult = (over: Record<string, unknown> = {}) => ({
	message_id: 'm1',
	message_type: 'ADT^A01',
	version: '2.5',
	format: 'HL7 v2',
	segment_count: 3,
	file_size_bytes: 120,
	truncated_text: 'MSH|truncated',
	tree_roots: [],
	...over,
}) as any;

const v1 = { modified_ms: 1000, size: 10 };
const v2 = { modified_ms: 2000, size: 12 };

beforeEach(() => {
	messageStore.closeAllTabs();
	dialogStore.close(false);
	fileOpsStore.recentFiles = [];
	vi.mocked(openFile).mockReset().mockResolvedValue(parseResult());
	vi.mocked(saveFile).mockReset().mockResolvedValue(savedFileResult);
	vi.mocked(fileStat).mockReset().mockResolvedValue(v1);
	vi.mocked(canonicalPath).mockReset().mockResolvedValue(null);
	vi.mocked(removeRecentFile).mockReset().mockResolvedValue(undefined);
	vi.mocked(getRecentFiles).mockReset().mockResolvedValue([]);
	vi.mocked(addRecentFile).mockReset().mockResolvedValue(undefined);
	vi.mocked(clearRecentFiles).mockReset().mockResolvedValue(undefined);
	vi.mocked(open).mockReset().mockResolvedValue(null);
	vi.mocked(save).mockReset().mockResolvedValue(null);
});

describe('openPath', () => {
	it('opens the parsed file in a tab with truncated text and mutes auto-parse', async () => {
		const suppress = vi.fn();
		await fileOpsStore.openPath('/data/adt.hl7', suppress);
		expect(suppress).toHaveBeenCalledOnce();
		expect(messageStore.activeTab?.filePath).toBe('/data/adt.hl7');
		expect(messageStore.activeTab?.content).toBe('MSH|truncated');
		expect(addRecentFile).toHaveBeenCalledWith('/data/adt.hl7', 'adt.hl7', 'ADT^A01', '2.5', 120);
	});

	it('shows a visible error dialog on failure, without creating a tab', async () => {
		vi.mocked(openFile).mockRejectedValue(new Error('gone'));
		const p = fileOpsStore.openPath('/missing.hl7', () => {});
		await vi.waitFor(() => expect(dialogStore.active).not.toBeNull());
		expect(dialogStore.active?.kind).toBe('error');
		expect(dialogStore.active?.details).toContain('gone');
		dialogStore.close(true);
		await p;
		expect(messageStore.tabs).toHaveLength(0);
	});
});

describe('openPath: one file, one tab', () => {
	it('recognises the same file reached through a symlinked folder', async () => {
		vi.mocked(canonicalPath).mockResolvedValue('/real/b/m.hl7');
		await fileOpsStore.openPath('/real/b/m.hl7', () => {});
		await fileOpsStore.openPath('/link/m.hl7', () => {});
		expect(messageStore.tabs).toHaveLength(1);
		expect(openFile).toHaveBeenCalledTimes(1);
	});

	it('opens a file that does not parse as text, and says why', async () => {
		vi.mocked(openFile).mockResolvedValue(parseResult({
			message_id: '', full_text: 'XYZ|garbage', parse_error: 'Message does not start with MSH', source_charset: 'UTF-16LE',
		}));
		const p = fileOpsStore.openPath('/data/odd.hl7', () => {});
		await vi.waitFor(() => expect(dialogStore.active).not.toBeNull());
		expect(dialogStore.active?.kind).toBe('warning');
		expect(dialogStore.active?.message).toContain('does not start with MSH');
		dialogStore.close(true);
		await p;
		expect(messageStore.activeTab?.content).toBe('XYZ|garbage');
		expect(messageStore.activeTab?.parseResult).toBeNull();
		expect(messageStore.activeTab?.charset).toBe('UTF-16LE');
	});

	it('keeps no second copy of the text in the parse result', async () => {
		vi.mocked(openFile).mockResolvedValue(parseResult({ full_text: 'MSH|full' }));
		await fileOpsStore.openPath('/data/full.hl7', () => {});
		expect(messageStore.activeTab?.content).toBe('MSH|full');
		expect(messageStore.activeTab?.parseResult?.full_text).toBeUndefined();
		expect(messageStore.activeTab?.parseResult?.truncated_text).toBe('');
	});
});

describe('openFromDialog', () => {
	it('does nothing when the picker is cancelled', async () => {
		await fileOpsStore.openFromDialog(() => {});
		expect(openFile).not.toHaveBeenCalled();
		expect(messageStore.tabs).toHaveLength(0);
	});

	it('opens the picked file and refreshes recents', async () => {
		vi.mocked(open).mockResolvedValue('/picked/oru.hl7');
		vi.mocked(getRecentFiles).mockResolvedValue([
			{ path: '/picked/oru.hl7', filename: 'oru.hl7' } as any,
		]);
		await fileOpsStore.openFromDialog(() => {});
		expect(messageStore.activeTab?.filePath).toBe('/picked/oru.hl7');
		expect(fileOpsStore.recentFiles).toHaveLength(1);
	});

	it('reports open failures in a visible dialog — even with zero tabs', async () => {
		// Regression: from the welcome screen (no tabs) failures used to be
		// written into the active tab, i.e. nowhere — "the button does nothing".
		vi.mocked(open).mockResolvedValue('/broken.hl7');
		vi.mocked(openFile).mockRejectedValue(new Error('corrupt'));
		const p = fileOpsStore.openFromDialog(() => {});
		await vi.waitFor(() => expect(dialogStore.active).not.toBeNull());
		expect(dialogStore.active?.kind).toBe('error');
		expect(dialogStore.active?.details).toContain('corrupt');
		dialogStore.close(true);
		await p;
	});
});

describe('saveActive', () => {
	it('is a no-op without an active tab', async () => {
		await fileOpsStore.saveActive();
		expect(saveFile).not.toHaveBeenCalled();
	});

	it('saves the current editor text to the existing path', async () => {
		const id = messageStore.newTab();
		messageStore.markSaved(id, '/work/msg.hl7');
		messageStore.updateContent(id, 'MSH|edited');
		await fileOpsStore.saveActive();
		expect(saveFile).toHaveBeenCalledWith({ path: '/work/msg.hl7', content: 'MSH|edited' });
		expect(messageStore.activeTab?.isModified).toBe(false);
	});

	it('falls back to Save As for untitled tabs', async () => {
		messageStore.newTab();
		await fileOpsStore.saveActive();
		expect(save).toHaveBeenCalled();
		expect(saveFile).not.toHaveBeenCalled(); // dialog was cancelled
	});

	it('surfaces save failures in an error dialog', async () => {
		const id = messageStore.newTab();
		messageStore.markSaved(id, '/work/msg.hl7');
		vi.mocked(saveFile).mockRejectedValue(new Error('disk full'));
		const p = fileOpsStore.saveActive();
		await vi.waitFor(() => expect(dialogStore.active).not.toBeNull());
		expect(dialogStore.active?.kind).toBe('error');
		expect(dialogStore.active?.details).toContain('disk full');
		dialogStore.close(true);
		await p;
	});
});

describe('saveActiveAs', () => {
	it('adopts the chosen path and updates the tab label', async () => {
		const id = messageStore.newTab();
		messageStore.updateContent(id, 'MSH|new');
		vi.mocked(save).mockResolvedValue('/exports/final.hl7');
		await fileOpsStore.saveActiveAs();
		expect(saveFile).toHaveBeenCalledWith({ path: '/exports/final.hl7', content: 'MSH|new' });
		expect(messageStore.activeTab?.filePath).toBe('/exports/final.hl7');
		expect(messageStore.activeTab?.label).toBe('final.hl7');
	});

	it('keeps the charset a legacy file was opened in', async () => {
		messageStore.openMessage(
			parseResult({ source_charset: 'windows-1252' }) as never,
			'/data/latin1.hl7',
			'MSH|x',
		);
		vi.mocked(save).mockResolvedValue('/exports/copy.hl7');
		await fileOpsStore.saveActiveAs();
		expect(saveFile).toHaveBeenCalledWith({ path: '/exports/copy.hl7', content: 'MSH|x', charset: 'windows-1252' });
	});
});

describe('recent files', () => {
	it('refreshRecent pulls the list from the backend', async () => {
		vi.mocked(getRecentFiles).mockResolvedValue([{ path: '/a.hl7' } as any]);
		await fileOpsStore.refreshRecent();
		expect(fileOpsStore.recentFiles).toHaveLength(1);
	});

	it('clearRecent empties the list', async () => {
		fileOpsStore.recentFiles = [{ path: '/a.hl7' } as any];
		await fileOpsStore.clearRecent();
		expect(clearRecentFiles).toHaveBeenCalled();
		expect(fileOpsStore.recentFiles).toEqual([]);
	});
});

describe('files changed by another program', () => {
	it('compares stamps by time and size', () => {
		expect(sameStamp(v1, { ...v1 })).toBe(true);
		expect(sameStamp(v1, v2)).toBe(false);
		expect(sameStamp(null, null)).toBe(true);
		expect(sameStamp(v1, null)).toBe(false);
	});

	it('asks before Save overwrites a file changed since it was opened', async () => {
		await fileOpsStore.openPath('/data/adt.hl7', () => {});
		messageStore.updateContent(messageStore.activeTabId!, 'MSH|mine');
		vi.mocked(fileStat).mockResolvedValue(v2);
		const p = fileOpsStore.saveActive();
		await vi.waitFor(() => expect(dialogStore.active?.kind).toBe('confirm'));
		dialogStore.close(false);
		await p;
		expect(saveFile).not.toHaveBeenCalled();
		expect(messageStore.activeTab?.isModified).toBe(true);
	});

	it('saves without asking when the file is as it was', async () => {
		await fileOpsStore.openPath('/data/adt.hl7', () => {});
		messageStore.updateContent(messageStore.activeTabId!, 'MSH|mine');
		await fileOpsStore.saveActive();
		expect(dialogStore.active).toBeNull();
		expect(saveFile).toHaveBeenCalledOnce();
	});

	it('reopening a changed file offers the new version, once', async () => {
		await fileOpsStore.openPath('/data/adt.hl7', () => {});
		vi.mocked(fileStat).mockResolvedValue(v2);
		vi.mocked(openFile).mockResolvedValue(parseResult({ full_text: 'MSH|new' }));
		const p = fileOpsStore.openPath('/data/../data/adt.hl7', () => {});
		await vi.waitFor(() => expect(dialogStore.active?.kind).toBe('confirm'));
		dialogStore.close(true);
		await p;
		expect(messageStore.tabs).toHaveLength(1);
		expect(messageStore.activeTab?.content).toBe('MSH|new');
		expect(messageStore.activeTab?.isModified).toBe(false);
		// The same version again is not reported a second time.
		await fileOpsStore.checkAllTabs();
		expect(dialogStore.active).toBeNull();
	});

	it('a missing Recent Files entry can be removed', async () => {
		fileOpsStore.recentFiles = [{ path: '/gone/m.hl7', filename: 'm.hl7' } as any];
		vi.mocked(openFile).mockRejectedValue(new Error('No such file or directory (os error 2)'));
		vi.mocked(fileStat).mockResolvedValue(null);
		const p = fileOpsStore.openPath('/gone/m.hl7', () => {});
		await vi.waitFor(() => expect(dialogStore.active?.kind).toBe('confirm'));
		expect(dialogStore.active?.message).toContain('/gone/m.hl7');
		dialogStore.close(true);
		await p;
		expect(removeRecentFile).toHaveBeenCalledWith('/gone/m.hl7');
	});

	it('a restored, unedited file tab shows the file as it is now', async () => {
		messageStore.restoreSession([{
			tab_order: 0, label: 'a.hl7', file_path: '/data/a.hl7', content: 'MSH|old',
			is_modified: false, is_active: true, cursor_line: 1, cursor_column: 1,
		}]);
		vi.mocked(openFile).mockResolvedValue(parseResult({ full_text: 'MSH|disk' }));
		const missing: string[] = [];
		expect(await fileOpsStore.adoptRestoredTab(messageStore.activeTabId!, missing)).toBe(true);
		expect(messageStore.activeTab?.content).toBe('MSH|disk');

		vi.mocked(fileStat).mockResolvedValue(null);
		messageStore.restoreSession([{
			tab_order: 0, label: 'b.hl7', file_path: '/data/b.hl7', content: 'MSH|kept',
			is_modified: false, is_active: true, cursor_line: 1, cursor_column: 1,
		}]);
		expect(await fileOpsStore.adoptRestoredTab(messageStore.activeTabId!, missing)).toBe(false);
		expect(missing).toEqual(['/data/b.hl7']);
		expect(messageStore.activeTab?.content).toBe('MSH|kept');
	});

	it('the first save of a tab restored with edits asks, since the file may have changed meanwhile', async () => {
		messageStore.restoreSession([{
			tab_order: 0, label: 'c.hl7', file_path: '/data/c.hl7', content: 'MSH|edited',
			is_modified: true, is_active: true, cursor_line: 1, cursor_column: 1,
		}]);
		expect(await fileOpsStore.adoptRestoredTab(messageStore.activeTabId!, [])).toBe(false);
		const p = fileOpsStore.saveActive();
		await vi.waitFor(() => expect(dialogStore.active).not.toBeNull());
		dialogStore.close(false);
		await p;
		expect(saveFile).not.toHaveBeenCalled();
		const again = fileOpsStore.saveActive();
		await vi.waitFor(() => expect(dialogStore.active).not.toBeNull());
		dialogStore.close(true);
		await again;
		expect(saveFile).toHaveBeenCalledTimes(1);
		// Saved: the tab now knows the file, and the next save does not ask.
		await fileOpsStore.saveActive();
		expect(saveFile).toHaveBeenCalledTimes(2);
	});
});
