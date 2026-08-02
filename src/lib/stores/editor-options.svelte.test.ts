import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('$lib/ipc/database', () => ({
	getPreference: vi.fn(async () => null),
}));

import { getPreference } from '$lib/ipc/database';
import { editorOptionsStore } from './editor-options.svelte';

function prefs(map: Record<string, string | null>) {
	vi.mocked(getPreference).mockImplementation(async (key: string) => map[key] ?? null);
}

beforeEach(() => {
	vi.mocked(getPreference).mockReset().mockResolvedValue(null);
	editorOptionsStore.options = {};
});

describe('loadFromPrefs', () => {
	it('leaves everything unset when no prefs are saved (Monaco defaults win)', async () => {
		await editorOptionsStore.loadFromPrefs();
		expect(editorOptionsStore.options).toEqual({});
	});

	it('maps every saved pref to its option', async () => {
		prefs({
			editor_font_size: '16',
			editor_font_family: 'Fira Code',
			editor_word_wrap: 'off',
			editor_minimap: 'false',
			editor_line_numbers: 'true',
			editor_tab_size: '2',
			editor_render_whitespace: 'boundary',
		});
		await editorOptionsStore.loadFromPrefs();
		expect(editorOptionsStore.options).toEqual({
			fontSize: 16,
			fontFamily: 'Fira Code',
			wordWrap: 'off',
			minimap: false,
			lineNumbers: true,
			tabSize: 2,
			renderWhitespace: 'boundary',
		});
	});

	it('falls back on unparsable numbers', async () => {
		prefs({ editor_font_size: 'huge', editor_tab_size: 'wide' });
		await editorOptionsStore.loadFromPrefs();
		expect(editorOptionsStore.options.fontSize).toBe(13);
		expect(editorOptionsStore.options.tabSize).toBe(4);
	});

	it('treats any non-"false" boolean pref as true', async () => {
		prefs({ editor_minimap: 'true', editor_line_numbers: 'yes' });
		await editorOptionsStore.loadFromPrefs();
		expect(editorOptionsStore.options.minimap).toBe(true);
		expect(editorOptionsStore.options.lineNumbers).toBe(true);
	});

	it('keeps the previous options when the backend is unavailable', async () => {
		editorOptionsStore.options = { fontSize: 15 };
		vi.mocked(getPreference).mockRejectedValue(new Error('no tauri'));
		await editorOptionsStore.loadFromPrefs();
		expect(editorOptionsStore.options).toEqual({ fontSize: 15 });
	});
});
