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
import { fileOpsStore, sameStamp, openRefusal, isWebLink } from './file-ops.svelte';
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

	it('says a file name that is not UTF-8 cannot be opened, not that the file is missing', async () => {
		vi.mocked(openFile).mockRejectedValue(new Error('No such file or directory (os error 2)'));
		vi.mocked(fileStat).mockResolvedValue(null);
		const p = fileOpsStore.openPath('/data/M�ller.hl7', () => {});
		await vi.waitFor(() => expect(dialogStore.active).not.toBeNull());
		expect(dialogStore.active?.kind).toBe('error');
		expect(dialogStore.active?.message).toContain('UTF-8');
		expect(dialogStore.active?.message).toContain('/data/M�ller.hl7');
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

	it("offers the document's own file type: FHIR JSON, FHIR XML or HL7", async () => {
		const offered = async (content: string, format: string | null) => {
			messageStore.closeAllTabs();
			const id = messageStore.newTab();
			messageStore.updateContent(id, content);
			if (format) messageStore.updateParseResult(id, parseResult({ format }));
			vi.mocked(save).mockClear();
			await fileOpsStore.saveActiveAs();
			return vi.mocked(save).mock.calls[0][0];
		};
		const json = await offered('{"resourceType":"Patient","id":"p1"}', 'FHIR JSON');
		expect(json).toEqual({
			defaultPath: 'Untitled.json',
			filters: [{ name: 'FHIR JSON', extensions: ['json'] }, { name: 'All Files', extensions: ['*'] }],
		});
		expect((await offered('<Patient xmlns="http://hl7.org/fhir"/>', 'FHIR XML'))?.filters?.[0]).toEqual({ name: 'FHIR XML', extensions: ['xml'] });
		expect((await offered('MSH|^~\\&|A', 'HL7v2'))?.defaultPath).toBe('Untitled.hl7');
		// Not parsed (yet): the text says what it is.
		expect((await offered('{"resourceType":"Patient"', null))?.defaultPath).toBe('Untitled.json');
	});

	it('adds the extension to a name typed without one (the Linux dialog does not)', async () => {
		const id = messageStore.newTab();
		messageStore.updateContent(id, '{"resourceType":"Patient","id":"p1"}');
		vi.mocked(save).mockResolvedValue('/home/u/patient');
		vi.mocked(fileStat).mockResolvedValue(null); // no patient.json there yet
		await fileOpsStore.saveActiveAs();
		expect(saveFile).toHaveBeenCalledWith({ path: '/home/u/patient.json', content: '{"resourceType":"Patient","id":"p1"}' });
		expect(messageStore.activeTab?.label).toBe('patient.json');
	});

	it('keeps a name typed with an extension, any extension', async () => {
		const id = messageStore.newTab();
		messageStore.updateContent(id, 'MSH|^~\\&|A');
		vi.mocked(save).mockResolvedValue('/home/u/msg.txt');
		await fileOpsStore.saveActiveAs();
		expect(saveFile).toHaveBeenCalledWith({ path: '/home/u/msg.txt', content: 'MSH|^~\\&|A' });
	});

	it('asks before the added extension replaces an existing file', async () => {
		const id = messageStore.newTab();
		messageStore.updateContent(id, 'MSH|^~\\&|A');
		vi.mocked(save).mockResolvedValue('/home/u/msg');
		vi.mocked(fileStat).mockImplementation(async (p: string) => (p === '/home/u/msg.hl7' ? v1 : null)); // msg.hl7 exists
		const confirm = vi.spyOn(dialogStore, 'confirm').mockResolvedValue(false);
		await fileOpsStore.saveActiveAs();
		expect(confirm).toHaveBeenCalledOnce();
		expect(confirm.mock.calls[0][0]).toContain('msg.hl7');
		expect(saveFile).not.toHaveBeenCalled();
		confirm.mockResolvedValue(true);
		await fileOpsStore.saveActiveAs();
		expect(saveFile).toHaveBeenCalledWith({ path: '/home/u/msg.hl7', content: 'MSH|^~\\&|A' });
		confirm.mockRestore();
	});

	it("saves over the tab's own file when it has no extension, and over an existing one picked in the dialog", async () => {
		messageStore.openMessage(parseResult() as never, '/data/message', 'MSH|x');
		vi.mocked(save).mockResolvedValue('/data/message');
		vi.mocked(fileStat).mockImplementation(async (p: string) => (p === '/data/message' ? v1 : null));
		await fileOpsStore.saveActiveAs();
		expect(saveFile).toHaveBeenCalledWith({ path: '/data/message', content: 'MSH|x', charset: null });
		expect(messageStore.activeTab?.filePath).toBe('/data/message');

		vi.mocked(saveFile).mockClear();
		vi.mocked(save).mockResolvedValue('/data/other');
		vi.mocked(fileStat).mockImplementation(async (p: string) => (p === '/data/other' ? v1 : null));
		await fileOpsStore.saveActiveAs();
		expect(saveFile).toHaveBeenCalledWith({ path: '/data/other', content: 'MSH|x', charset: null });
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

	it('a save during the read of a large restored tab waits for it, and never writes the empty placeholder', async () => {
		// Stored without its text: the session keeps '' for an unedited large file tab.
		messageStore.restoreSession([{
			tab_order: 0, label: 'big.hl7', file_path: '/data/big.hl7', content: '',
			is_modified: false, is_active: true, cursor_line: 1, cursor_column: 1,
		}]);
		const id = messageStore.activeTabId!;
		expect(messageStore.activeTab?.contentPending).toBe(true);
		let finishRead!: (r: unknown) => void;
		vi.mocked(openFile).mockReturnValue(new Promise((r) => { finishRead = r; }) as any);
		const load = messageStore.trackLoad(id, fileOpsStore.adoptRestoredTab(id, []));
		const saving = fileOpsStore.saveActive();
		await new Promise((r) => setTimeout(r, 0));
		expect(saveFile).not.toHaveBeenCalled();
		finishRead(parseResult({ full_text: 'MSH|the whole file' }));
		await load;
		await saving;
		expect(saveFile).toHaveBeenCalledTimes(1);
		expect(vi.mocked(saveFile).mock.calls[0][0].content).toBe('MSH|the whole file');
		expect(messageStore.activeTab?.contentPending).toBe(false);
	});

	it('a large restored tab whose file could not be read is not saved: the user is told why', async () => {
		vi.mocked(fileStat).mockResolvedValue(null);
		messageStore.restoreSession([{
			tab_order: 0, label: 'gone.hl7', file_path: '/data/gone.hl7', content: '',
			is_modified: false, is_active: true, cursor_line: 1, cursor_column: 1,
		}]);
		const id = messageStore.activeTabId!;
		await messageStore.trackLoad(id, fileOpsStore.adoptRestoredTab(id, []));
		const saving = fileOpsStore.saveActive();
		await vi.waitFor(() => expect(dialogStore.active).not.toBeNull());
		expect(dialogStore.active?.message).toContain('gone.hl7');
		dialogStore.close(true);
		await saving;
		const savingAs = fileOpsStore.saveActiveAs();
		await vi.waitFor(() => expect(dialogStore.active).not.toBeNull());
		dialogStore.close(true);
		await savingAs;
		expect(save).not.toHaveBeenCalled();
		expect(saveFile).not.toHaveBeenCalled();
	});
});

describe('a link dropped instead of a file', () => {
	it('is recognized; file paths are not', () => {
		expect(isWebLink('https://example.invalid/x.hl7')).toBe(true);
		expect(isWebLink('https:/example.invalid/x.hl7')).toBe(true);
		expect(isWebLink('ftp://host/a.hl7')).toBe(true);
		expect(isWebLink('/tmp/https:/x.hl7')).toBe(false);
		expect(isWebLink('C:/data/a.hl7')).toBe(false);
		expect(isWebLink('C:\\data\\a.hl7')).toBe(false);
		expect(isWebLink('\\\\server\\share\\a.hl7')).toBe(false);
	});

	it('is reported as a link, not as a missing file', async () => {
		const p = fileOpsStore.openPath('https://example.invalid/x.hl7', () => {});
		await vi.waitFor(() => expect(dialogStore.active?.kind).toBe('error'));
		expect(dialogStore.active?.message).toContain('https://example.invalid/x.hl7');
		expect(dialogStore.active?.message).not.toContain('not found');
		dialogStore.close(true);
		await p;
		expect(openFile).not.toHaveBeenCalled();
		expect(messageStore.tabs).toHaveLength(0);
	});
});

describe('files too large to open, and files that are not files', () => {
	const MB = 1024 * 1024;
	// open_file's errors, as the backend words them (fileio.rs).
	const asks = (path: string, size: number) =>
		new Error(`File error: large file, open only if confirmed: ${size} bytes, limit ${16 * MB} bytes: ${path}`);
	const refuses = (path: string, size: number) =>
		new Error(`File error: too large to open: ${size} bytes, limit ${64 * MB} bytes: ${path}`);
	const restored = (path: string, content = '') => messageStore.restoreSession([{
		tab_order: 0, label: path.split('/').pop()!, file_path: path, content,
		is_modified: false, is_active: true, cursor_line: 1, cursor_column: 1,
	}]);

	it('reads the refusal and its sizes from the error', () => {
		expect(openRefusal(asks('/a.hl7', 20 * MB))).toEqual({ kind: 'confirm', size: 20 * MB, limit: 16 * MB });
		expect(openRefusal(refuses('/a.hl7', 80 * MB))).toEqual({ kind: 'too_large', size: 80 * MB, limit: 64 * MB });
		expect(openRefusal('File error: not a regular file: /dev/zero')?.kind).toBe('not_regular');
		expect(openRefusal(new Error('No such file or directory (os error 2)'))).toBeNull();
	});

	it('asks before opening a large file, and opens it once the user agrees', async () => {
		vi.mocked(openFile).mockImplementation(async (_p, allowLarge) => {
			if (!allowLarge) throw asks('/logs/big.hl7', 20.5 * MB);
			return parseResult({ full_text: 'MSH|big' });
		});
		const p = fileOpsStore.openPath('/logs/big.hl7', () => {});
		await vi.waitFor(() => expect(dialogStore.active?.kind).toBe('confirm'));
		expect(dialogStore.active?.message).toContain('big.hl7 is 20.5 MB');
		expect(dialogStore.active?.message).toContain('over 16 MB');
		dialogStore.close(true);
		await p;
		expect(openFile).toHaveBeenLastCalledWith('/logs/big.hl7', true);
		expect(messageStore.activeTab?.content).toBe('MSH|big');
	});

	it('opens nothing when the user would rather not', async () => {
		vi.mocked(openFile).mockRejectedValue(asks('/logs/big.hl7', 20 * MB));
		const p = fileOpsStore.openPath('/logs/big.hl7', () => {});
		await vi.waitFor(() => expect(dialogStore.active?.kind).toBe('confirm'));
		dialogStore.close(false);
		await p;
		expect(openFile).toHaveBeenCalledTimes(1);
		expect(messageStore.tabs).toHaveLength(0);
		expect(addRecentFile).not.toHaveBeenCalled();
	});

	it('says why a file too large, or not a file, does not open', async () => {
		vi.mocked(openFile).mockRejectedValue(refuses('/logs/huge.hl7', 300 * MB));
		let p = fileOpsStore.openPath('/logs/huge.hl7', () => {});
		await vi.waitFor(() => expect(dialogStore.active?.kind).toBe('error'));
		expect(dialogStore.active?.message).toContain('huge.hl7 is 300 MB, larger than the 64 MB');
		dialogStore.close(true);
		await p;

		vi.mocked(openFile).mockRejectedValue(new Error('File error: not a regular file: /dev/zero'));
		p = fileOpsStore.openPath('/dev/zero', () => {});
		await vi.waitFor(() => expect(dialogStore.active?.kind).toBe('error'));
		expect(dialogStore.active?.message).toContain('not a regular file');
		expect(dialogStore.active?.message).toContain('/dev/zero');
		dialogStore.close(true);
		await p;
		expect(messageStore.tabs).toHaveLength(0);
	});

	it('a session tab whose file grew past the limit comes back empty, with the reason, and is not read again', async () => {
		// A small file, stored with its text, that is now 300 MB.
		restored('/logs/growing.hl7', 'MSH|small');
		const id = messageStore.activeTabId!;
		vi.mocked(openFile).mockRejectedValue(refuses('/logs/growing.hl7', 300 * MB));
		const load = fileOpsStore.adoptRestoredTab(id, []);
		await vi.waitFor(() => expect(dialogStore.active?.kind).toBe('warning'));
		expect(dialogStore.active?.message).toContain('growing.hl7 is 300 MB');
		dialogStore.close(true);
		// Handled: the caller does not parse the old text into the tab.
		expect(await load).toBe(true);
		const tab = messageStore.activeTab!;
		expect(tab.content).toBe('');
		expect(tab.contentPending).toBe(true);
		expect(tab.notLoaded).toBe(true);
		// The next session keeps it empty, so the next start does not read
		// the old text as the file's.
		expect(messageStore.serializeSession()[0].content).toBe('');
		// And Save cannot write the empty tab over the file.
		const saving = fileOpsStore.saveActive();
		await vi.waitFor(() => expect(dialogStore.active).not.toBeNull());
		dialogStore.close(true);
		await saving;
		expect(saveFile).not.toHaveBeenCalled();
	});

	it('a session tab on what is now a device comes back empty too', async () => {
		restored('/data/m.hl7');
		vi.mocked(openFile).mockRejectedValue(new Error('File error: not a regular file: /data/m.hl7'));
		const load = fileOpsStore.adoptRestoredTab(messageStore.activeTabId!, []);
		await vi.waitFor(() => expect(dialogStore.active?.kind).toBe('warning'));
		expect(dialogStore.active?.message).toContain('not a regular file');
		dialogStore.close(true);
		expect(await load).toBe(true);
		expect(messageStore.activeTab?.notLoaded).toBe(true);
	});

	it('a large session tab is loaded at start only if the user says so, and later by opening the file again', async () => {
		restored('/logs/big.hl7');
		const id = messageStore.activeTabId!;
		vi.mocked(openFile).mockImplementation(async (_p, allowLarge) => {
			if (!allowLarge) throw asks('/logs/big.hl7', 40 * MB);
			return parseResult({ full_text: 'MSH|big' });
		});
		const load = fileOpsStore.adoptRestoredTab(id, []);
		await vi.waitFor(() => expect(dialogStore.active?.kind).toBe('confirm'));
		expect(dialogStore.active?.message).toContain('was open when BridgeLab was last closed');
		dialogStore.close(false);
		expect(await load).toBe(true);
		expect(messageStore.activeTab?.notLoaded).toBe(true);
		expect(openFile).toHaveBeenCalledTimes(1);

		// File > Open on the same file loads it into that tab.
		const p = fileOpsStore.openPath('/logs/big.hl7', () => {});
		await vi.waitFor(() => expect(dialogStore.active?.kind).toBe('confirm'));
		dialogStore.close(true);
		await p;
		expect(messageStore.tabs).toHaveLength(1);
		expect(messageStore.activeTab?.content).toBe('MSH|big');
		expect(messageStore.activeTab?.contentPending).toBe(false);
		expect(messageStore.activeTab?.notLoaded).toBe(false);
	});
});
