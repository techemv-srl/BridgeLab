import { describe, it, expect, beforeEach, vi } from 'vitest';

const savedFileResult = { path: '', bytes_written: 0 };

vi.mock('$lib/ipc/parser', () => ({
	openFile: vi.fn(),
	saveFile: vi.fn(async () => savedFileResult),
}));
vi.mock('$lib/ipc/database', () => ({
	getRecentFiles: vi.fn(async () => []),
	addRecentFile: vi.fn(async () => undefined),
	clearRecentFiles: vi.fn(async () => undefined),
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({
	open: vi.fn(async () => null),
	save: vi.fn(async () => null),
}));

import { openFile, saveFile } from '$lib/ipc/parser';
import { getRecentFiles, addRecentFile, clearRecentFiles } from '$lib/ipc/database';
import { open, save } from '@tauri-apps/plugin-dialog';
import { fileOpsStore } from './file-ops.svelte';
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

beforeEach(() => {
	messageStore.closeAllTabs();
	dialogStore.close(false);
	fileOpsStore.recentFiles = [];
	vi.mocked(openFile).mockReset().mockResolvedValue(parseResult());
	vi.mocked(saveFile).mockReset().mockResolvedValue(savedFileResult);
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

	it('swallows failures without creating a tab', async () => {
		vi.mocked(openFile).mockRejectedValue(new Error('gone'));
		await fileOpsStore.openPath('/missing.hl7', () => {});
		expect(messageStore.tabs).toHaveLength(0);
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

	it('reports open failures inside the active tab', async () => {
		messageStore.newTab();
		vi.mocked(open).mockResolvedValue('/broken.hl7');
		vi.mocked(openFile).mockRejectedValue(new Error('corrupt'));
		await fileOpsStore.openFromDialog(() => {});
		expect(messageStore.activeTab?.content).toContain('Error opening file:');
		expect(messageStore.activeTab?.content).toContain('corrupt');
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
