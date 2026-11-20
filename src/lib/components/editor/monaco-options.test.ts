import { describe, it, expect, vi } from 'vitest';
import { opensExternally, routeEditorLinks, toMonacoOptions } from './monaco-options';

describe('editor links', () => {
	it('opens only web and e-mail links', () => {
		for (const s of ['http', 'https', 'mailto', 'HTTPS']) expect(opensExternally(s), s).toBe(true);
		for (const s of ['file', 'command', 'javascript', 'data', 'vscode', 'inmemory', 'tel', '']) {
			expect(opensExternally(s), s).toBe(false);
		}
	});

	it('hands web and e-mail links to the external opener and keeps the editor from opening any link', () => {
		type Uri = { scheme: string; toString(skipEncoding?: boolean): string };
		type Opener = { open(uri: Uri): boolean };
		// As the editor's URI writes itself out unencoded (toString(true)).
		const uri = (text: string): Uri => ({ scheme: text.slice(0, text.indexOf(':')), toString: () => text });
		const openers: Opener[] = [];
		const host = { editor: { registerLinkOpener: (o: Opener) => { openers.push(o); return { dispose() {} }; } } };
		const open = vi.fn();
		routeEditorLinks(host, open);
		routeEditorLinks(host, open); // global to every editor: once
		expect(openers).toHaveLength(1);
		const [opener] = openers;

		expect(opener.open(uri('http://127.0.0.1:23400/report?id=1 2'))).toBe(true);
		expect(opener.open(uri('mailto:info@techemv.it'))).toBe(true);
		expect(open.mock.calls).toEqual([['http://127.0.0.1:23400/report?id=1%202'], ['mailto:info@techemv.it']]);

		// Handled, so the editor neither runs a command nor navigates.
		expect(opener.open(uri('command:editor.action.selectAll'))).toBe(true);
		expect(opener.open(uri('file:///etc/passwd'))).toBe(true);
		expect(open).toHaveBeenCalledTimes(2);
	});
});

describe('editor settings → Monaco options', () => {
	it('maps the new settings onto Monaco option names', () => {
		expect(toMonacoOptions({ wordSuggestions: 'off' })).toEqual({ wordBasedSuggestions: 'off' });
		expect(toMonacoOptions({ wordSuggestions: 'allDocuments' })).toEqual({ wordBasedSuggestions: 'allDocuments' });
		expect(toMonacoOptions({ occurrencesHighlight: false })).toEqual({ occurrencesHighlight: 'off' });
		expect(toMonacoOptions({ occurrencesHighlight: true })).toEqual({ occurrencesHighlight: 'singleFile' });
		expect(toMonacoOptions({ links: false })).toEqual({ links: false });
		expect(toMonacoOptions({ stickyScroll: true })).toEqual({ stickyScroll: { enabled: true } });
	});

	it('leaves unset settings to the editor defaults', () => {
		expect(toMonacoOptions({})).toEqual({});
		expect(toMonacoOptions({ minimap: false, lineNumbers: false })).toEqual({ minimap: { enabled: false }, lineNumbers: 'off' });
	});
});
