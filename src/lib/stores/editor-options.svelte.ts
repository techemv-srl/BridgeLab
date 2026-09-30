import { getPreference } from '$lib/ipc/database';
import { clampSetting } from '$lib/settings-limits';
import type { EditorOptions } from '$lib/components/editor/monaco-options';

/**
 * Monaco editor options loaded from preferences. Until v0.2.5 these prefs
 * were saved by Settings but read by nobody — Monaco hardcoded everything.
 * Reloaded live when Settings persists a change (onEditorOptionsChange).
 */
class EditorOptionsStore {
	options = $state<EditorOptions>({});
	/** Fields longer than this are shown folded in the editor (Settings). */
	foldThreshold = $state(100);
	/** Parse the message in the background while typing (Settings → Parser). */
	autoParse = $state(true);
	/** How long after the last keystroke the background parse runs, in ms. */
	autoParseDelay = $state(500);

	async loadFromPrefs(): Promise<void> {
		try {
			const [fs, ff, ww, mm, ln, ts, rw, ss, bc] = await Promise.all([
				getPreference('editor_font_size'),
				getPreference('editor_font_family'),
				getPreference('editor_word_wrap'),
				getPreference('editor_minimap'),
				getPreference('editor_line_numbers'),
				getPreference('editor_tab_size'),
				getPreference('editor_render_whitespace'),
				getPreference('editor_smooth_scrolling'),
				getPreference('editor_bracket_colors'),
			]);
			const [wsg, occ, lk, sticky] = await Promise.all([
				getPreference('editor_word_suggestions'),
				getPreference('editor_occurrences'),
				getPreference('editor_links'),
				getPreference('editor_sticky_scroll'),
			]);
			const [tt, ap, apd] = await Promise.all([
				getPreference('truncation_threshold'),
				getPreference('auto_parse'),
				getPreference('auto_parse_delay'),
			]);
			// 0 means "never fold": keep it, fall back only on garbage.
			if (tt !== null) this.foldThreshold = clampSetting('foldThreshold', tt);
			this.autoParse = ap !== 'false';
			this.autoParseDelay = clampSetting('autoParseDelay', apd);
			this.options = {
				...(fs && { fontSize: clampSetting('fontSize', fs) }),
				...(ff && { fontFamily: ff }),
				...(ww && { wordWrap: ww as 'on' | 'off' | 'wordWrapColumn' | 'bounded' }),
				...(mm !== null && { minimap: mm !== 'false' }),
				...(ln !== null && { lineNumbers: ln !== 'false' }),
				...(ts && { tabSize: clampSetting('tabSize', ts) }),
				...(rw && { renderWhitespace: rw as 'none' | 'boundary' | 'all' }),
				...(ss !== null && { smoothScrolling: ss !== 'false' }),
				...(bc !== null && { bracketPairColorization: bc === 'true' }),
				...(wsg && ['off', 'currentDocument', 'allDocuments'].includes(wsg) && { wordSuggestions: wsg as 'off' | 'currentDocument' | 'allDocuments' }),
				...(occ !== null && { occurrencesHighlight: occ !== 'false' }),
				...(lk !== null && { links: lk !== 'false' }),
				...(sticky !== null && { stickyScroll: sticky === 'true' }),
			};
		} catch { /* web mode */ }
	}
}

export const editorOptionsStore = new EditorOptionsStore();
