import { describe, it, expect } from 'vitest';
import { delimiterHint, firstLineStart, headerTypoHint, unparsedMessage } from './unparsed';

describe('firstLineStart', () => {
	it('skips what the lexer skips and cuts long lines', () => {
		expect(firstLineStart('﻿\r\n\u000BSH|^~\\&|APP\rPID|1')).toBe('SH|^~\\&|APP');
		expect(firstLineStart('X'.repeat(40))).toBe(`${'X'.repeat(24)}…`);
	});

	it('shows control characters instead of passing them on', () => {
		expect(firstLineStart('AB\u0007CD')).toBe('AB·CD');
	});
});

describe('unparsedMessage', () => {
	it('explains a missing M with the start of the file', () => {
		const m = unparsedMessage('a.hl7', 'SH|^~\\&|APP|FAC\rPID|1', 'Message does not start with MSH', 'missing_m');
		expect(m).toContain('a.hl7 is open as text');
		expect(m).toContain('“SH|^~\\&|APP|FAC”');
		expect(m).toContain('Is the M of MSH missing?');
		expect(m).not.toContain('does not start with MSH');
		expect(m).toContain('press F6');
	});

	it('keeps the parser detail where it says where the problem is', () => {
		const m = unparsedMessage('b.json', '{"a":', 'Invalid JSON: EOF while parsing an object at line 1 column 5', 'invalid_json');
		expect(m).toContain('the file is not valid JSON:');
		expect(m).toContain('line 1 column 5');
	});

	it('names the header the file has, a batch header included', () => {
		const m = unparsedMessage('d.hl7', 'FHS|^', 'MSH segment too short', 'short_header');
		expect(m).toContain('the FHS segment is too short');
		expect(m).toContain('as in FHS|^~\\&');
	});

	it('explains delimiters that cannot be used, with the parser detail naming the one', () => {
		const reason = 'MSH-1 declares a tab as the field separator: a delimiter must be a printable ASCII character other than a letter, a digit or a space';
		const m = unparsedMessage('t.hl7', 'MSH\t^~\\&\tAPP', reason, 'bad_delimiters');
		expect(m).toContain('the MSH segment declares delimiters that cannot be used');
		expect(m).toContain('a tab as the field separator');
	});

	it('quotes what the lexer does not skip, such as a non-breaking space', () => {
		expect(firstLineStart('\u00A0MSH|^~\\&')).toBe('\u00A0MSH|^~\\&');
	});

	it('falls back to the reason as it came for an unknown or missing kind', () => {
		expect(unparsedMessage('c.hl7', 'x', 'Something new', 'brand_new')).toContain('Something new');
		expect(unparsedMessage('c.hl7', 'x', 'Something new')).toContain('Something new');
	});
});

describe('delimiterHint', () => {
	it('explains an unusable declaration as a sentence, and only that', () => {
		expect(delimiterHint('MSH|^^\\&|APP', "MSH declares '^' as both the component separator and the repetition separator")).toMatch(
			/^The MSH segment declares delimiters that cannot be used\./,
		);
		expect(delimiterHint('MSH|^~', 'MSH segment too short: MSH-2 holds 2 of the four encoding characters')).toBeNull();
		expect(delimiterHint('PID|1', 'Message does not start with MSH')).toBeNull();
	});
});

describe('headerTypoHint', () => {
	it('names a missing M or a lower-case header as a sentence', () => {
		expect(headerTypoHint('SH|^~\\&|APP')).toMatch(/^The first line starts with “SH\|\^~\\&\|APP”\. Is the M of MSH missing\?/);
		expect(headerTypoHint('msh|^~\\&|APP')).toContain('Segment names are upper case: MSH, not msh.');
		expect(headerTypoHint('bhs|^~\\&|APP')).toContain('upper case: BHS, not bhs.');
	});

	it('says nothing for other first lines', () => {
		expect(headerTypoHint('PID|1||123')).toBeNull();
		expect(headerTypoHint('SHOW|1')).toBeNull();
		expect(headerTypoHint('MSH|^~\\&|APP')).toBeNull();
	});
});

describe('t() parameters', () => {
	it('fills every occurrence and keeps `$` sequences from the value', () => {
		const m = unparsedMessage('e.hl7', 'SH|$&$1|x', 'Message does not start with MSH', 'missing_m');
		expect(m).toContain('“SH|$&$1|x”');
	});
});
