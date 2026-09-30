/**
 * The editor settings BridgeLab stores (Settings → Editor) and how they map
 * onto Monaco's own options. Kept apart from the component so the mapping
 * can be tested.
 */
export interface EditorOptions {
	fontSize?: number;
	fontFamily?: string;
	wordWrap?: 'on' | 'off' | 'wordWrapColumn' | 'bounded';
	minimap?: boolean;
	lineNumbers?: boolean;
	tabSize?: number;
	renderWhitespace?: 'none' | 'boundary' | 'all';
	smoothScrolling?: boolean;
	cursorBlinking?: string;
	bracketPairColorization?: boolean;
	/** Word completions: from the edited document, every open one, or none. */
	wordSuggestions?: 'off' | 'currentDocument' | 'allDocuments';
	/** Highlight other occurrences of the word under the caret. */
	occurrencesHighlight?: boolean;
	/** Underline URLs and open them with Ctrl+click. */
	links?: boolean;
	/** Keep the enclosing block's first line pinned while scrolling. */
	stickyScroll?: boolean;
}

/** Monaco options for the settings that are set; unset ones keep the
 *  editor's defaults. */
export function toMonacoOptions(o: EditorOptions) {
	return {
		...(o.fontSize !== undefined && { fontSize: o.fontSize }),
		...(o.fontFamily !== undefined && { fontFamily: o.fontFamily }),
		...(o.wordWrap !== undefined && { wordWrap: o.wordWrap }),
		...(o.minimap !== undefined && { minimap: { enabled: o.minimap } }),
		...(o.lineNumbers !== undefined && { lineNumbers: (o.lineNumbers ? 'on' : 'off') as 'on' | 'off' }),
		...(o.tabSize !== undefined && { tabSize: o.tabSize }),
		...(o.renderWhitespace !== undefined && { renderWhitespace: o.renderWhitespace }),
		...(o.smoothScrolling !== undefined && { smoothScrolling: o.smoothScrolling }),
		...(o.cursorBlinking !== undefined && { cursorBlinking: o.cursorBlinking as never }),
		...(o.bracketPairColorization !== undefined && { bracketPairColorization: { enabled: o.bracketPairColorization } }),
		...(o.wordSuggestions !== undefined && { wordBasedSuggestions: o.wordSuggestions }),
		...(o.occurrencesHighlight !== undefined && { occurrencesHighlight: (o.occurrencesHighlight ? 'singleFile' : 'off') as 'singleFile' | 'off' }),
		...(o.links !== undefined && { links: o.links }),
		...(o.stickyScroll !== undefined && { stickyScroll: { enabled: o.stickyScroll } }),
	};
}
