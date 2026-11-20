import { describe, it, expect } from 'vitest';
import { applyLineEdits, defaultEol, firstEol, joinLines, LineEndings, mixedEols, withEol, type Eol } from './eol';

/** The text Monaco holds for `text`: every line ending its own (LF here). */
const model = (text: string) => text.replace(/\r\n|\r/g, '\n');

describe('line endings', () => {
	it('finds the first ending, or none', () => {
		expect(firstEol('MSH|^~\\&\rPID|1')).toBe('\r');
		expect(firstEol('a\r\nb')).toBe('\r\n');
		expect(firstEol('a\nb\r')).toBe('\n');
		expect(firstEol('MSH|^~\\&|A')).toBeNull();
	});

	it('gives a document with no ending yet CR for HL7 v2 and LF otherwise, whatever came before', () => {
		expect(defaultEol('MSH|^~\\&|A|B\nPID|1')).toBe('\r');
		expect(defaultEol('﻿\n  FHS|^~\\&|A')).toBe('\r');
		expect(defaultEol('MSH#@!\\$#A')).toBe('\r');
		expect(defaultEol('{"resourceType":"Patient"}\n')).toBe('\n');
		expect(defaultEol('MSHALLOW\nx')).toBe('\n');
		expect(defaultEol('')).toBe('\n');
	});

	it('keeps one ending per line only when the document mixes them', () => {
		expect(mixedEols('a\rb\rc\r')).toBeNull();
		expect(mixedEols('a\r\nb\r\n')).toBeNull();
		expect(mixedEols('a')).toBeNull();
		expect(mixedEols('a\rb\rc\r\n')).toEqual(['\r', '\r', '\r\n']);
		expect(mixedEols('a\nb\r\nc\rd')).toEqual(['\n', '\r\n', '\r']);
	});

	it('gives mixed and trailing endings back exactly', () => {
		for (const text of ['A\rB\rC\r\n', 'A\nB\r\nC\rD', 'A\r\n\rB']) {
			expect(joinLines(model(text), mixedEols(text)!)).toBe(text);
		}
		expect(joinLines('a\nb', ['\r', '\n'])).toBeNull();
	});

	it('keeps every ending through an edit inside a line', () => {
		const text = 'MSH|1\rPID|1\rPV1|1\r\n';
		const ends = mixedEols(text)!;
		// One character typed at the end of PV1 (line 3).
		const after = applyLineEdits(ends, [{ startLine: 3, endLine: 3, text: 'X' }], '\r');
		expect(joinLines('MSH|1\nPID|1\nPV1|1X\n', after)).toBe('MSH|1\rPID|1\rPV1|1X\r\n');
	});

	it('gives a new line the document ending and keeps the others in place', () => {
		const ends: Eol[] = ['\n', '\r\n', '\r'];
		// Enter in the middle of line 2: a new ending between line 2 and its old one.
		expect(applyLineEdits(ends, [{ startLine: 2, endLine: 2, text: '\n' }], '\n')).toEqual(['\n', '\n', '\r\n', '\r']);
		// Lines 2 and 3 joined (the break between them deleted).
		expect(applyLineEdits(ends, [{ startLine: 2, endLine: 3, text: '' }], '\n')).toEqual(['\n', '\r']);
		// Lines replaced by as many lines (fold all): nothing changes.
		expect(applyLineEdits(ends, [{ startLine: 1, endLine: 4, text: 'a\nb\nc\nd' }], '\n')).toEqual(ends);
		// Several edits, from the end of the document to the start.
		expect(
			applyLineEdits(ends, [{ startLine: 4, endLine: 4, text: 'x\ny' }, { startLine: 1, endLine: 2, text: '' }], '\r'),
		).toEqual(['\r\n', '\r', '\r']);
	});

	it('brings back a deleted ending on undo, and takes it away again on redo', () => {
		// Monaco reports the undo of a deleted line break as a line break
		// typed anew, and the alternative version id returns to the earlier
		// version's: 1 when loaded, 2 after the delete.
		const ends = new LineEndings(['\n', '\r\n', '\r']);
		ends.at(1);
		const join = { startLine: 2, endLine: 3, text: '' };
		const retype = { startLine: 2, endLine: 2, text: '\n' };
		ends.edit(2, [join], '\n', false);
		expect(ends.ends).toEqual(['\n', '\r']);
		ends.edit(1, [retype], '\n', true);
		expect(ends.ends).toEqual(['\n', '\r\n', '\r']);
		expect(joinLines('a\nb\nc\nd', ends.ends)).toBe('a\nb\r\nc\rd');
		ends.edit(2, [join], '\n', true);
		expect(ends.ends).toEqual(['\n', '\r']);
		// A new line typed by hand (a fresh version) gets the document's ending.
		ends.edit(3, [retype], '\n', false);
		expect(ends.ends).toEqual(['\n', '\n', '\r']);
	});

	it('falls back to the edit itself for a version it no longer keeps', () => {
		const ends = new LineEndings(['\n', '\r\n', '\r'], 2);
		ends.at(1);
		ends.edit(2, [{ startLine: 1, endLine: 1, text: 'x' }], '\n', false);
		ends.edit(3, [{ startLine: 2, endLine: 3, text: '' }], '\n', false);
		// Version 1 was dropped (only two kept): the undo is applied as an edit.
		ends.edit(1, [{ startLine: 2, endLine: 2, text: '\n' }], '\n', true);
		expect(ends.ends).toEqual(['\n', '\n', '\r']);
	});

	it('shares the endings array across edits within lines', () => {
		const before: Eol[] = ['\n', '\r'];
		expect(applyLineEdits(before, [{ startLine: 1, endLine: 1, text: 'abc' }], '\n')).toBe(before);
	});

	it('writes a uniform document with its own ending, whatever Monaco uses', () => {
		expect(withEol('a\r\nb\r\n', '\r', '\r\n')).toBe('a\rb\r');
		expect(withEol('a\r\nb', '\n', '\r\n')).toBe('a\nb');
		expect(withEol('a\nb', '\r\n', '\n')).toBe('a\r\nb');
		expect(withEol('a\nb', '\n', '\n')).toBe('a\nb');
	});
});
