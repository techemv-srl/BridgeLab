import { describe, it, expect } from 'vitest';
import type * as MonacoTypes from 'monaco-editor';
import { versionFromModel } from './HL7AutoComplete';

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
