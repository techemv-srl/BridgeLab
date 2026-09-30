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
	editorOptionsStore.autoParse = true;
	editorOptionsStore.autoParseDelay = 500;
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

describe('parser and scrolling prefs', () => {
	it('reads auto-parse, its delay, smooth scrolling and bracket colours', async () => {
		prefs({
			auto_parse: 'false',
			auto_parse_delay: '2000',
			editor_smooth_scrolling: 'false',
			editor_bracket_colors: 'true',
		});
		await editorOptionsStore.loadFromPrefs();
		expect(editorOptionsStore.autoParse).toBe(false);
		expect(editorOptionsStore.autoParseDelay).toBe(2000);
		expect(editorOptionsStore.options.smoothScrolling).toBe(false);
		expect(editorOptionsStore.options.bracketPairColorization).toBe(true);
	});

	it('clamps out-of-range numbers saved by older versions', async () => {
		prefs({ editor_font_size: '400', editor_tab_size: 'null', auto_parse_delay: '5' });
		await editorOptionsStore.loadFromPrefs();
		expect(editorOptionsStore.options.fontSize).toBe(32);
		expect(editorOptionsStore.options.tabSize).toBe(4);
		expect(editorOptionsStore.autoParseDelay).toBe(100);
	});
});

describe('new editor settings', () => {
	it('loads word suggestions, occurrences, links and sticky scroll from the preferences', async () => {
		const prefs: Record<string, string> = {
			editor_word_suggestions: 'allDocuments',
			editor_occurrences: 'false',
			editor_links: 'false',
			editor_sticky_scroll: 'true',
		};
		vi.mocked(getPreference).mockImplementation(async (k: string) => prefs[k] ?? null);
		await editorOptionsStore.loadFromPrefs();
		expect(editorOptionsStore.options).toMatchObject({ wordSuggestions: 'allDocuments', occurrencesHighlight: false, links: false, stickyScroll: true });
		prefs.editor_word_suggestions = 'bogus';
		await editorOptionsStore.loadFromPrefs();
		expect(editorOptionsStore.options.wordSuggestions).toBeUndefined();
	});
});
