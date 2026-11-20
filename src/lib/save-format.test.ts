import { describe, it, expect } from 'vitest';
import { hasExtension, saveAsDialog, saveFormatOf } from './save-format';

const PATIENT = '{"resourceType":"Patient","id":"p1"}';
const PATIENT_XML = '<Patient xmlns="http://hl7.org/fhir"><id value="p1"/></Patient>';
const HL7 = 'MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1\r';

const tab = (over: Partial<Parameters<typeof saveAsDialog>[0]> = {}) => ({
	filePath: null,
	label: 'Untitled',
	content: '',
	parseResult: null,
	...over,
});

describe('saveFormatOf', () => {
	it('follows the format the tab parsed as', () => {
		expect(saveFormatOf('FHIR JSON', PATIENT).ext).toBe('json');
		expect(saveFormatOf('FHIR XML', PATIENT_XML).ext).toBe('xml');
		expect(saveFormatOf('HL7v2', HL7).ext).toBe('hl7');
	});

	it('reads what unparsed text starts with', () => {
		expect(saveFormatOf(null, PATIENT).ext).toBe('json');
		expect(saveFormatOf(undefined, '\uFEFF  \n[1, 2]').ext).toBe('json');
		expect(saveFormatOf('', '<?xml version="1.0"?><Patient/>').ext).toBe('xml');
		expect(saveFormatOf(null, HL7).ext).toBe('hl7');
		expect(saveFormatOf(null, 'SH|^~\\&|typo').ext).toBe('hl7');
		expect(saveFormatOf(null, '').ext).toBe('hl7');
	});
});

describe('hasExtension', () => {
	it('sees a file type at the end of the name', () => {
		for (const p of ['/home/u/patient.json', 'C:\\data\\msg.HL7', '/x/a.b.txt', 'report.v1', '/x/.hidden.xml']) {
			expect(hasExtension(p), p).toBe(true);
		}
	});

	it('does not take a version, a date or a dot folder for one', () => {
		for (const p of ['/home/u/patient', '/home/u/ADT^A01 v2.5', '/x/batch 2024.11', '/x/.profile', '/my.dir/name', 'C:\\my.dir\\name', '/x/name.', '/x/a.toolongextension']) {
			expect(hasExtension(p), p).toBe(false);
		}
	});
});

describe('saveAsDialog', () => {
	it('offers the FHIR JSON filter and a .json name for a FHIR JSON tab', () => {
		const d = saveAsDialog(tab({ content: PATIENT, parseResult: { format: 'FHIR JSON' } }));
		expect(d.format.ext).toBe('json');
		expect(d.defaultPath).toBe('Untitled.json');
		expect(d.filters).toEqual([
			{ name: 'FHIR JSON', extensions: ['json'] },
			{ name: 'All Files', extensions: ['*'] },
		]);
	});

	it('offers FHIR XML for a FHIR XML tab and HL7 for an HL7 tab', () => {
		const xml = saveAsDialog(tab({ content: PATIENT_XML, parseResult: { format: 'FHIR XML' } }));
		expect(xml.filters[0]).toEqual({ name: 'FHIR XML', extensions: ['xml'] });
		expect(xml.defaultPath).toBe('Untitled.xml');
		const hl7 = saveAsDialog(tab({ content: HL7, parseResult: { format: 'HL7v2' } }));
		expect(hl7.filters[0]).toEqual({ name: 'HL7 Messages', extensions: ['hl7'] });
		expect(hl7.defaultPath).toBe('Untitled.hl7');
		expect(hl7.filters[1]).toEqual({ name: 'All Files', extensions: ['*'] });
	});

	it('goes by the text when the tab has not parsed, or its result is for an earlier text', () => {
		expect(saveAsDialog(tab({ content: PATIENT })).format.ext).toBe('json');
		const stale = tab({ content: HL7, parseResult: { format: 'FHIR JSON' }, parseStale: 'not JSON' });
		expect(saveAsDialog(stale).format.ext).toBe('hl7');
	});

	it("keeps the tab's own file as the default", () => {
		const d = saveAsDialog(tab({ filePath: '/data/patient.txt', label: 'patient.txt', content: PATIENT, parseResult: { format: 'FHIR JSON' } }));
		expect(d.defaultPath).toBe('/data/patient.txt');
		expect(d.filters[0].extensions).toEqual(['json']);
	});

	it('makes a file name of a new tab\'s label', () => {
		const named = (label: string) => saveAsDialog(tab({ label, content: HL7 })).defaultPath;
		expect(named('ORU^R01 v2.5.1')).toBe('ORU^R01 v2.5.1.hl7');
		expect(named('Inbox 12:30:05')).toBe('Inbox 12_30_05.hl7');
		expect(named('a/b\\c')).toBe('a_b_c.hl7');
		expect(named('ward.hl7')).toBe('ward.hl7');
		expect(named(' .. ')).toBe('Untitled.hl7');
	});
});
