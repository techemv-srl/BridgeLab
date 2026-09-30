import { describe, it, expect } from 'vitest';
import { toMonacoOptions } from './monaco-options';

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
