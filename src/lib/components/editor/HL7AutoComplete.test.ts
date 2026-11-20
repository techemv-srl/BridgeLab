import { describe, it, expect } from 'vitest';
import type * as MonacoTypes from 'monaco-editor';
import { catalogueVersion, codeSpan, messageTypeCompletion, versionFromModel } from './HL7AutoComplete';

describe('codeSpan (field values in the hover)', () => {
	/** What the hover's Markdown renderer (marked, inside Monaco) reads as
	 *  one code span at the start of `md`, and the text it shows for it:
	 *  its code-span rule, then one space dropped from each end when both
	 *  ends have one. Null when `md` does not open with a code span. */
	const shownAsCode = (md: string): { length: number; text: string } | null => {
		const m = /^(`+)([^`]|[^`][\s\S]*?[^`])\1(?!`)/.exec(md);
		if (!m) return null;
		const text = /[^ ]/.test(m[2]) && /^ /.test(m[2]) && / $/.test(m[2]) ? m[2].slice(1, -1) : m[2];
		return { length: m[0].length, text };
	};

	it('shows a value exactly, as code, whatever it holds', () => {
		for (const value of [
			'AL',
			'x`[CLICK-ME](http://127.0.0.1:23400/md-hover-link)',
			'y`![IMGALT](data:image/gif;base64,R0lGODlhAQABAAAAACw=)',
			'z`[MAILME](mailto:a@example.invalid)',
			'``double`` and `single`',
			'`edge`',
			' padded ',
			'<b>html</b> **bold** _em_',
		]) {
			const md = `${codeSpan(value)} is not in HL7 table 0003 (Event type)`;
			// One span, ending where the value does: nothing after it is
			// part of the value, nothing of the value is Markdown.
			expect(shownAsCode(md), value).toEqual({ length: codeSpan(value).length, text: value });
		}
	});

	it('closes too early with the old plain span', () => {
		const value = 'x`[CLICK-ME](http://127.0.0.1:23400/md-hover-link)';
		expect(shownAsCode(`\`${value}\``)?.text).toBe('x');
	});

	it('is a plain span for an ordinary code', () => {
		expect(codeSpan('AA')).toBe('`AA`');
		expect(codeSpan('a`b')).toBe('``a`b``');
	});
});

/**
 * Minimal stand-in for a Monaco text model: `versionFromModel` only reads
 * `getLineCount()` and `getLineContent()`, so there is no reason to pull in
 * the whole editor to exercise the MSH-12 arithmetic.
 */
function model(text: string): MonacoTypes.editor.ITextModel {
	const lines = text.split('\n');
	return {
		getLineCount: () => lines.length,
		getLineContent: (n: number) => lines[n - 1] ?? '',
	} as unknown as MonacoTypes.editor.ITextModel;
}

const msh = (version: string) =>
	`MSH|^~\\&|SEND|FAC|RECV|FAC|20240101120000||ADT^A01|MSG0001|P|${version}`;

describe('versionFromModel', () => {
	it('reads the version declared in MSH-12', () => {
		expect(versionFromModel(model(msh('2.3')))).toBe('2.3');
		expect(versionFromModel(model(msh('2.5.1')))).toBe('2.5.1');
		expect(versionFromModel(model(msh('2.7')))).toBe('2.7');
	});

	it('keeps only the first component of MSH-12', () => {
		expect(versionFromModel(model(msh('2.4^AUS^2.4')))).toBe('2.4');
	});

	it('falls back to 2.5 for a version with no shipped definitions', () => {
		expect(versionFromModel(model(msh('2.9')))).toBe('2.5');
		expect(versionFromModel(model(msh('')))).toBe('2.5');
	});

	it('falls back to 2.5 when MSH-12 is absent entirely', () => {
		expect(versionFromModel(model('MSH|^~\\&|SEND|FAC'))).toBe('2.5');
	});

	it('finds MSH when the file opens with a batch header', () => {
		expect(versionFromModel(model(`FHS|^~\\&|SEND\nBHS|^~\\&|SEND\n${msh('2.6')}`))).toBe('2.6');
	});

	it('falls back to 2.5 when there is no MSH at all', () => {
		expect(versionFromModel(model('{"resourceType":"Patient"}'))).toBe('2.5');
	});

	it('uses the first message of a multi-message file', () => {
		expect(versionFromModel(model(`${msh('2.3')}\nPID|1\n${msh('2.7')}`))).toBe('2.3');
	});
});

describe('catalogueVersion', () => {
	it('falls back to major.minor before the default, like the validator', () => {
		expect(catalogueVersion('2.7.2')).toBe('2.7');
		expect(catalogueVersion('2.5.9')).toBe('2.5');
		expect(catalogueVersion('2.8')).toBe('2.5');
		expect(catalogueVersion('2.3.1')).toBe('2.3.1');
		expect(catalogueVersion(undefined)).toBe('2.5');
	});
});

describe('messageTypeCompletion (MSH-9)', () => {
	const std = { field: '|', component: '^' };

	it('replaces the whole field typed so far, so ADT^ never becomes ADT^ADT^A04', () => {
		const before = 'MSH|^~\\&|A|B|C|D|20240101||ADT^';
		const { startColumn, values } = messageTypeCompletion(before, std);
		// The column of the "A" of ADT: what the suggestion replaces.
		expect(before.slice(startColumn - 1)).toBe('ADT^');
		expect(values).toContain('ADT^A04');
		const line = before.slice(0, startColumn - 1) + 'ADT^A04';
		expect(line.endsWith('||ADT^A04')).toBe(true);
	});

	it('starts at the cursor when the field is still empty', () => {
		const before = 'MSH|^~\\&|A|B|C|D|20240101||';
		expect(messageTypeCompletion(before, std).startColumn).toBe(before.length + 1);
	});

	it("writes the pairs with the message's own separators", () => {
		const before = 'MSH#@!\\$#A#B#C#D#20240101##ADT@';
		const { startColumn, values } = messageTypeCompletion(before, { field: '#', component: '@' });
		expect(before.slice(startColumn - 1)).toBe('ADT@');
		expect(values).toContain('ORU@R01');
		expect(values).toContain('ACK');
		expect(values.some((v) => v.includes('^'))).toBe(false);
	});
});
