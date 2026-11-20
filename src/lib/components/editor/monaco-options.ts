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

/**
 * Whether a link the editor opens (Ctrl+click on a URL in the text,
 * "Follow link" in its hover) goes to the system's browser or mail
 * client: web and e-mail links do; anything else (`file:`, `command:`...)
 * opens nothing. Every link in the editor comes from the document, which
 * is not ours, and the editor would run a `command:` link from it.
 *
 * The editor's own way does not work in the app: it opens web links with
 * window.open, which the webview refuses, so the setting did nothing, and
 * sets the page's location for other schemes, which replaced the app.
 */
export function opensExternally(scheme: string): boolean {
	return ['http', 'https', 'mailto'].includes(scheme.toLowerCase());
}

/** The parts of the editor's API `routeEditorLinks` uses. */
interface LinkOpenerHost {
	editor: {
		registerLinkOpener(opener: {
			open(uri: { scheme: string; toString(skipEncoding?: boolean): string }): boolean;
		}): unknown;
	};
}

let linksRouted = false;

/** Send the editor's links to `open` (the app's external opener). The
 *  opener is global to every editor, so it is registered once. */
export function routeEditorLinks(monaco: LinkOpenerHost, open: (url: string) => void): void {
	if (linksRouted) return;
	linksRouted = true;
	monaco.editor.registerLinkOpener({
		open(uri) {
			// Written out as the editor itself would hand it on.
			if (opensExternally(uri.scheme)) open(encodeURI(uri.toString(true)));
			// Handled either way: the editor's own openers must not run.
			return true;
		},
	});
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
