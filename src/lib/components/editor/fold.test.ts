import { describe, it, expect } from 'vitest';
import { collapseText, expandText, tokensIn, fullToDisplayCol, displayToFullCol, describe as label, retainOnly, registrySize, searchMatcher, tokensMatching } from './fold';

const b64 = 'QUJD'.repeat(300); // 1200 chars of base64
const oru = [
	'MSH|^~\\&|LAB|FAC|EHR|FAC|20240101||ORU^R01|1|P|2.5',
	'PID|1||123||Rossi^Mario',
	`OBX|1|ED|PDF^Report||^application^pdf^Base64^${b64}||||||F`,
].join('\r');

describe('fold', () => {
	it('collapses only the long run, keeping the field structure visible', () => {
		const display = collapseText(oru, 'hl7v2', 100);
		const obx = display.split('\r')[2];
		expect(obx).toMatch(/^OBX\|1\|ED\|PDF\^Report\|\|\^application\^pdf\^Base64\^⟨Base64 · 1\.2 KB #\d+⟩\|\|\|\|\|\|F$/);
		expect(display.split('\r')[0]).toBe(oru.split('\r')[0]);
	});

	it('round-trips exactly, CR/LF and all', () => {
		for (const eol of ['\r', '\n', '\r\n']) {
			const full = oru.replaceAll('\r', eol) + eol;
			expect(expandText(collapseText(full, 'hl7v2', 100))).toBe(full);
		}
	});

	it('never folds MSH-1/MSH-2 and honours custom delimiters', () => {
		const long = 'X'.repeat(200);
		const msg = `MSH#*~\\&#A#B#C#D#20240101##ADT*A01#1#P#2.5\rNTE#1##${long}*${long}`;
		const display = collapseText(msg, 'hl7v2', 100);
		expect(display.startsWith('MSH#*~\\&#')).toBe(true);
		expect(tokensIn(display)).toHaveLength(2);
		expect(expandText(display)).toBe(msg);
	});

	it('keeps edits around a token and drops a deleted one', () => {
		const display = collapseText(oru, 'hl7v2', 100);
		const edited = display.replace('Rossi', 'Bianchi');
		expect(expandText(edited)).toBe(oru.replace('Rossi', 'Bianchi'));
		const token = tokensIn(display)[0];
		const removed = display.slice(0, token.index) + display.slice(token.index + token.length);
		expect(expandText(removed)).toBe(oru.replace(b64, ''));
	});

	it('a copied token expands wherever it is pasted', () => {
		const display = collapseText(oru, 'hl7v2', 100);
		const token = display.slice(tokensIn(display)[0].index, tokensIn(display)[0].index + tokensIn(display)[0].length);
		expect(expandText(`NTE|1||${token}`)).toBe(`NTE|1||${b64}`);
	});

	it('leaves text that merely looks like a token alone', () => {
		expect(expandText('NTE|1||⟨Text · 1 B #999999⟩')).toBe('NTE|1||⟨Text · 1 B #999999⟩');
	});

	it('folds long JSON strings and XML values', () => {
		const json = `{"resourceType":"Binary","data":"${b64}"}`;
		const dj = collapseText(json, 'json', 100);
		expect(dj).toMatch(/"data":"⟨Base64 · 1\.2 KB #\d+⟩"/);
		expect(expandText(dj)).toBe(json);
		const xml = `<Binary xmlns="http://hl7.org/fhir"><data value="${b64}"/></Binary>`;
		const dx = collapseText(xml, 'xml', 100);
		expect(tokensIn(dx)).toHaveLength(1);
		expect(expandText(dx)).toBe(xml);
	});

	it('maps columns between the full and the display line', () => {
		const line = `NTE|1||${'X'.repeat(500)}|end`;
		const disp = collapseText(line, 'hl7v2', 100);
		const tok = tokensIn(disp)[0];
		const endFull = line.indexOf('end') + 1;
		const endDisp = disp.indexOf('end') + 1;
		expect(fullToDisplayCol(disp, endFull)).toBe(endDisp);
		expect(displayToFullCol(disp, endDisp)).toBe(endFull);
		expect(fullToDisplayCol(disp, 3)).toBe(3); // before the token
		expect(fullToDisplayCol(disp, 250)).toBe(tok.index + 1); // inside the folded run
	});

	it('labels payloads by kind and size', () => {
		expect(label(b64)).toBe('Base64 · 1.2 KB');
		expect(label('ab'.repeat(40))).toBe('Hex · 80 B');
		expect(label('free text '.repeat(20))).toBe('Text · 200 B');
	});

	it('reuses one id for the same run and frees what is no longer shown', () => {
		const a = collapseText(oru, 'hl7v2', 100);
		const b = collapseText(oru, 'hl7v2', 100);
		expect(a).toBe(b); // folding the same text twice stores nothing new
		retainOnly(a);
		expect(registrySize()).toBe(1);
		retainOnly('PID|1||no tokens');
		expect(registrySize()).toBe(0);
		expect(expandText(a)).toBe(a); // its token is now unknown, left as is
	});

	it('does nothing below the threshold or in plain mode', () => {
		expect(collapseText('PID|1||short', 'hl7v2', 100)).toBe('PID|1||short');
		expect(collapseText(oru, 'none', 100)).toBe(oru);
	});

	it('finds the folds a search must open', () => {
		const note = 'Patient Smith reports mild discomfort after the procedure; follow-up with Dr Smith scheduled in two weeks, no further action required.';
		const msg = `MSH|^~\\&|L|F|E|F|20240101||ORU^R01|1|P|2.5\rPID|1||1||Smith^John\rNTE|1||${note}\rOBX|1|ED|X||^application^pdf^Base64^${b64}`;
		const display = collapseText(msg, 'hl7v2', 100);
		expect(tokensIn(display)).toHaveLength(2);
		const smith = tokensMatching(display, searchMatcher('smith', { isRegex: false, matchCase: false })!);
		expect(smith.map((t) => t.payload)).toEqual([note]);
		expect(tokensMatching(display, searchMatcher('smith', { isRegex: false, matchCase: true })!)).toHaveLength(0);
		expect(tokensMatching(display, searchMatcher('Dr\\s+Sm', { isRegex: true, matchCase: true })!)).toHaveLength(1);
		// A query that matches a chip's own label opens it too: a replace
		// would otherwise rewrite the chip text.
		expect(tokensMatching(display, searchMatcher('KB', { isRegex: false, matchCase: true })!)).toHaveLength(1);
		expect(searchMatcher('', { isRegex: false, matchCase: false })).toBeNull();
		expect(searchMatcher('(', { isRegex: true, matchCase: false })).toBeNull();
		expect(searchMatcher('a.b', { isRegex: false, matchCase: false })!('axb')).toBe(false);
	});
});
