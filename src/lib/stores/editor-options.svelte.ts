import { getPreference } from '$lib/ipc/database';
import type { EditorOptions } from '$lib/components/editor/MonacoEditor.svelte';

/**
 * Monaco editor options loaded from preferences. Until v0.2.5 these prefs
 * were saved by Settings but read by nobody — Monaco hardcoded everything.
 * Reloaded live when Settings persists a change (onEditorOptionsChange).
 */
class EditorOptionsStore {
	options = $state<EditorOptions>({});

	async loadFromPrefs(): Promise<void> {
		try {
			const [fs, ff, ww, mm, ln, ts, rw] = await Promise.all([
				getPreference('editor_font_size'),
				getPreference('editor_font_family'),
				getPreference('editor_word_wrap'),
				getPreference('editor_minimap'),
				getPreference('editor_line_numbers'),
				getPreference('editor_tab_size'),
				getPreference('editor_render_whitespace'),
			]);
			this.options = {
				...(fs && { fontSize: parseInt(fs) || 13 }),
				...(ff && { fontFamily: ff }),
				...(ww && { wordWrap: ww as 'on' | 'off' | 'wordWrapColumn' | 'bounded' }),
				...(mm !== null && { minimap: mm !== 'false' }),
				...(ln !== null && { lineNumbers: ln !== 'false' }),
				...(ts && { tabSize: parseInt(ts) || 4 }),
				...(rw && { renderWhitespace: rw as 'none' | 'boundary' | 'all' }),
			};
		} catch { /* web mode */ }
	}
}

export const editorOptionsStore = new EditorOptionsStore();
