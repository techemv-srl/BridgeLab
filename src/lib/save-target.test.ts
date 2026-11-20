import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('$lib/ipc/parser', () => ({
	fileStat: vi.fn(async () => null),
	allowSaveWithExtension: vi.fn(async (picked: string, ext: string) => `${picked}.${ext}`),
}));

import { allowSaveWithExtension, fileStat } from '$lib/ipc/parser';
import { dialogStore } from '$lib/stores/dialog.svelte';
import { saveTarget } from './save-target';

beforeEach(() => {
	vi.restoreAllMocks();
	vi.mocked(fileStat).mockReset().mockResolvedValue(null);
	vi.mocked(allowSaveWithExtension).mockReset().mockImplementation(async (picked, ext) => `${picked}.${ext}`);
});

describe('saveTarget', () => {
	it('asks the backend to allow the name with the extension the Linux dialog left out', async () => {
		// The dialog only let the frontend write "/home/u/report": without
		// the grant, writeTextFile refused "/home/u/report.json".
		expect(await saveTarget('/home/u/report', 'json')).toBe('/home/u/report.json');
		expect(allowSaveWithExtension).toHaveBeenCalledWith('/home/u/report', 'json');
		expect(await saveTarget('/tmp/out', 'csv')).toBe('/tmp/out.csv');
		expect(allowSaveWithExtension).toHaveBeenLastCalledWith('/tmp/out', 'csv');
	});

	it('uses the path the backend allowed', async () => {
		vi.mocked(allowSaveWithExtension).mockResolvedValue('/real/report.xsd');
		expect(await saveTarget('/home/u/report', 'xsd')).toBe('/real/report.xsd');
	});

	it('needs no grant when the name already has the extension', async () => {
		expect(await saveTarget('/home/u/report.json', 'json')).toBe('/home/u/report.json');
		expect(allowSaveWithExtension).not.toHaveBeenCalled();
	});

	it('needs no grant when the backend writes the file (test-case pack)', async () => {
		expect(await saveTarget('/p/cases', 'bltests.json', { alsoAccept: ['json'], backendWrites: true })).toBe('/p/cases.bltests.json');
		expect(await saveTarget('/p/cases.json', 'bltests.json', { alsoAccept: ['json'], backendWrites: true })).toBe('/p/cases.json');
		expect(allowSaveWithExtension).not.toHaveBeenCalled();
	});

	it('asks before replacing an existing file, and grants nothing when declined', async () => {
		vi.mocked(fileStat).mockResolvedValue({ modified_ms: 1, size: 2 });
		const confirm = vi.spyOn(dialogStore, 'confirm').mockResolvedValue(false);
		expect(await saveTarget('/home/u/report', 'json')).toBeNull();
		expect(confirm).toHaveBeenCalledOnce();
		expect(allowSaveWithExtension).not.toHaveBeenCalled();

		confirm.mockResolvedValue(true);
		expect(await saveTarget('/home/u/report', 'json')).toBe('/home/u/report.json');
	});

	it('rejects when the backend refuses, so the caller shows why', async () => {
		vi.mocked(allowSaveWithExtension).mockRejectedValue('File error: /home/u/report already exists');
		await expect(saveTarget('/home/u/report', 'json')).rejects.toMatch('already exists');
	});
});
