import { describe, it, expect } from 'vitest';
import { baseName, normalizePath, samePath, withExtension } from './paths';

describe('baseName', () => {
	it('splits unix and windows paths', () => {
		expect(baseName('/data/msgs/adt.hl7')).toBe('adt.hl7');
		expect(baseName('C:\\data\\oru.hl7')).toBe('oru.hl7');
		expect(baseName('\\\\server\\share\\a.hl7')).toBe('a.hl7');
	});

	it('keeps a backslash that is part of a unix file name', () => {
		expect(baseName('/tmp/back\\slash.hl7')).toBe('back\\slash.hl7');
	});
});

describe('normalizePath', () => {
	it('removes dot segments and doubled separators', () => {
		expect(normalizePath('/files/../files/./a.hl7')).toBe('/files/a.hl7');
		expect(normalizePath('/tmp//x/../x/a.hl7')).toBe('/tmp/x/a.hl7');
		expect(normalizePath('/../a.hl7')).toBe('/a.hl7');
		expect(normalizePath('C:\\a\\..\\b\\c.hl7')).toBe('C:\\b\\c.hl7');
		expect(normalizePath('C:/a/b.hl7')).toBe('C:\\a\\b.hl7');
		expect(normalizePath('\\\\srv\\share\\x\\..\\a.hl7')).toBe('\\\\srv\\share\\a.hl7');
	});
});

describe('samePath', () => {
	it('matches the same file written differently', () => {
		expect(samePath('/tmp/x/a.hl7', '/tmp/x/../x/a.hl7')).toBe(true);
		expect(samePath('C:\\X.HL7', 'c:\\x.hl7')).toBe(true);
	});

	it('keeps case on unix', () => {
		expect(samePath('/tmp/A.hl7', '/tmp/a.hl7')).toBe(false);
	});
});

describe('withExtension', () => {
	it('adds the extension to a name typed without one', () => {
		expect(withExtension('/home/u/report', 'json')).toBe('/home/u/report.json');
		expect(withExtension('/home/u/report.JSON', 'json')).toBe('/home/u/report.JSON');
		expect(withExtension('C:\\out\\ADT_A01', 'xsd')).toBe('C:\\out\\ADT_A01.xsd');
		expect(withExtension('/tmp/v1.2/report', 'csv')).toBe('/tmp/v1.2/report.csv');
		expect(withExtension('/tmp/.csv', 'csv')).toBe('/tmp/.csv.csv');
	});

	it('gives a pack its double extension but keeps a plain .json', () => {
		expect(withExtension('/p/cases', 'bltests.json', ['json'])).toBe('/p/cases.bltests.json');
		expect(withExtension('/p/cases.bltests.json', 'bltests.json', ['json'])).toBe('/p/cases.bltests.json');
		expect(withExtension('/p/cases.json', 'bltests.json', ['json'])).toBe('/p/cases.json');
	});
});
