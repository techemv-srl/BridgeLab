import { describe, it, expect } from 'vitest';
import { interpretMessage, formatHl7Date, type Reading, type ReadValue, type Part } from './interpretation';
import { setLocale } from '$lib/i18n';

/** A value as interpret_message returns it. */
function v(text: string, meaning: string | null = null, componentMeanings: (string | null)[] = []): ReadValue {
	const components = text.split('^');
	return {
		text,
		truncated: false,
		components,
		meaning,
		component_meanings: components.map((_, i) => componentMeanings[i] ?? null),
	};
}

const READING: Reading = {
	version: '2.5',
	message_type: 'ADT^A01',
	segments: [
		{
			segment_idx: 0, segment_type: 'MSH', segment_name: 'Message Header',
			fields: [
				{ position: 3, name: 'Sending Application', data_type: 'HD', repetitions: [v('LAB')] },
				{ position: 4, name: 'Sending Facility', data_type: 'HD', repetitions: [v('HOSP')] },
				{ position: 5, name: 'Receiving Application', data_type: 'HD', repetitions: [v('EHR')] },
				{ position: 7, name: 'Date/Time of Message', data_type: 'TS', repetitions: [v('20260105103000')] },
				{ position: 9, name: 'Message Type', data_type: 'MSG', repetitions: [v('ADT^A01^ADT_A01', 'ADT message', [null, 'ADT/ACK - Admit/visit notification'])] },
				{ position: 10, name: 'Message Control ID', data_type: 'ST', repetitions: [v('MSG001')] },
				{ position: 12, name: 'Version ID', data_type: 'VID', repetitions: [v('2.5')] },
			],
		},
		{
			segment_idx: 1, segment_type: 'PID', segment_name: 'Patient Identification',
			fields: [
				{ position: 3, name: 'Patient Identifier List', data_type: 'CX', repetitions: [
					v('123456^^^HOSP^MR', null, [null, null, null, null, 'Medical record number']),
					v('987^^^NAT^SS', null, [null, null, null, null, 'Social Security number']),
				] },
				{ position: 5, name: 'Patient Name', data_type: 'XPN', repetitions: [v('DOE^JOHN^Q')] },
				{ position: 8, name: 'Administrative Sex', data_type: 'IS', repetitions: [v('M', 'Male')] },
			],
		},
		{
			segment_idx: 2, segment_type: 'OBX', segment_name: 'Observation/Result',
			fields: [
				{ position: 2, name: 'Value Type', data_type: 'ID', repetitions: [v('NM', 'Numeric')] },
				{ position: 3, name: 'Observation Identifier', data_type: 'CWE', repetitions: [v('GLU^Glucose^L')] },
				{ position: 5, name: 'Observation Value', data_type: 'varies', repetitions: [v('98')] },
				{ position: 6, name: 'Units', data_type: 'CWE', repetitions: [v('mg/dL')] },
				{ position: 8, name: 'Abnormal Flags', data_type: 'IS', repetitions: [v('H', 'Above high normal')] },
				{ position: 11, name: 'Observation Result Status', data_type: 'ID', repetitions: [v('F', 'Final results')] },
			],
		},
		{
			segment_idx: 3, segment_type: 'ZXY', segment_name: '',
			fields: [{ position: 1, name: '', data_type: '', repetitions: [v('custom')] }],
		},
	],
};

const text = (parts: Part[]) => parts.map((p) => p.text).join('');

describe('interpretMessage', () => {
	setLocale('en');
	const blocks = interpretMessage(READING);

	it('reads the header as a sentence with the event meaning', () => {
		const msh = blocks[0];
		expect(msh.fieldByField).toBe(false);
		expect(text(msh.sentences[0])).toBe(
			'Message ADT^A01 (ADT/ACK - Admit/visit notification), sent 2026-01-05 10:30, from LAB (HOSP), to EHR.',
		);
		expect(text(msh.sentences[1])).toBe('Control ID MSG001, HL7 version 2.5.');
	});

	it('reads the patient with names, codes and every identifier', () => {
		const pid = blocks[1];
		expect(text(pid.sentences[0])).toBe(
			'Patient DOE, JOHN Q, Male, identifiers 123456 (Medical record number), 987 (Social Security number).',
		);
		// No address, phone or account: the second sentence is left out.
		expect(pid.sentences).toHaveLength(1);
	});

	it('links each value to its field, repetition included', () => {
		const parts = blocks[1].sentences[0];
		expect(parts.find((p) => p.text === 'DOE, JOHN Q')?.link).toEqual({ segmentIdx: 1, field: 5, repetition: null, component: null });
		expect(parts.find((p) => p.text.startsWith('987'))?.link).toEqual({ segmentIdx: 1, field: 3, repetition: 2, component: null });
		expect(parts.find((p) => p.text === ', ')?.link).toBeUndefined();
	});

	it('reads a numeric observation with its units and flags', () => {
		expect(text(blocks[2].sentences[0])).toBe('Glucose (GLU): 98 mg/dL, Above high normal, status Final results.');
	});

	it('reads any other segment field by field', () => {
		const z = blocks[3];
		expect(z.fieldByField).toBe(true);
		expect(text(z.sentences[0])).toBe('field 1: custom');
	});

	it('follows the interface language', () => {
		setLocale('it');
		try {
			const it = interpretMessage(READING);
			expect(text(it[1].sentences[0])).toMatch(/^Paziente DOE, JOHN Q, Male, identificativi /);
		} finally {
			setLocale('en');
		}
	});
});

describe('sentence wording', () => {
	setLocale('en');
	const reading: Reading = {
		version: '2.5',
		message_type: 'ORU^R01',
		segments: [
			{
				segment_idx: 0, segment_type: 'OBX', segment_name: 'Observation/Result',
				fields: [
					{ position: 2, name: 'Value Type', data_type: 'ID', repetitions: [v('TX', 'Text Data')] },
					{ position: 3, name: 'Observation Identifier', data_type: 'CWE', repetitions: [v('DS^Discharge summary')] },
					{ position: 5, name: 'Observation Value', data_type: 'varies', repetitions: [v('Troponin negative x2.')] },
					{ position: 8, name: 'Abnormal Flags', data_type: 'IS', repetitions: [v('N', 'Normal (applies to non-numeric results)')] },
					{ position: 11, name: 'Observation Result Status', data_type: 'ID', repetitions: [
						v('F', 'Final results; Can only be changed with a corrected result.'),
					] },
				],
			},
			{
				segment_idx: 1, segment_type: 'NTE', segment_name: 'Notes and Comments',
				fields: [{ position: 3, name: 'Comment', data_type: 'FT', repetitions: [v('Seen by Dr.')] }],
			},
			{
				segment_idx: 2, segment_type: 'ZAP', segment_name: '',
				fields: [
					{ position: 1, name: 'Reason', data_type: 'CWE', repetitions: [v('^Chest pain on exertion')] },
					{ position: 2, name: 'Resource', data_type: 'XCN', repetitions: [v('DOC4410^Verdi^Marco^^^Dr.')] },
					{ position: 3, name: 'Start', data_type: 'DTM', repetitions: [v('20260920101000')] },
					{ position: 4, name: 'Code', data_type: 'ID', repetitions: [v('NEW', 'New appointment.')] },
					{ position: 5, name: 'Note', data_type: 'ST', repetitions: [v('a^^b')] },
					{ position: 6, name: 'Where', data_type: 'ST', repetitions: [v('home'), v('work')] },
				],
			},
		],
	};
	const blocks = interpretMessage(reading);

	it('keeps the meaning of a table entry and leaves out its notes', () => {
		expect(text(blocks[0].sentences[0])).toBe(
			'Discharge summary (DS): Troponin negative x2, Normal, status Final results.',
		);
	});

	it('does not close a sentence twice', () => {
		expect(text(blocks[1].sentences[0])).toBe('Note: Seen by Dr.');
	});

	it('reads generic fields by their data type', () => {
		expect(text(blocks[2].sentences[0])).toBe(
			'Reason: Chest pain on exertion; Resource: Verdi, Marco (DOC4410); Start: 2026-09-20 10:10; Code: New appointment (NEW); Note: a b; Where: home, work',
		);
	});

	it('reads a structured numeric result without its component separators', () => {
		const sn = (value: string, units = ''): Reading => ({
			version: '2.5',
			message_type: 'ORU^R01',
			segments: [{
				segment_idx: 0, segment_type: 'OBX', segment_name: 'Observation/Result',
				fields: [
					{ position: 2, name: 'Value Type', data_type: 'ID', repetitions: [v('SN', 'Structured Numeric')] },
					{ position: 3, name: 'Observation Identifier', data_type: 'CWE', repetitions: [v('TIT^Titer')] },
					{ position: 5, name: 'Observation Value', data_type: 'varies', repetitions: [v(value)] },
					...(units ? [{ position: 6, name: 'Units', data_type: 'CWE', repetitions: [v(units)] }] : []),
					{ position: 8, name: 'Abnormal Flags', data_type: 'IS', repetitions: [v('A', 'Abnormal')] },
					{ position: 11, name: 'Observation Result Status', data_type: 'ID', repetitions: [v('F', 'Final results')] },
				],
			}],
		});
		const read = (r: Reading) => text(interpretMessage(r)[0].sentences[0]);
		expect(read(sn('<^5'))).toBe('Titer (TIT): <5, Abnormal, status Final results.');
		expect(read(sn('^1^:^128'))).toBe('Titer (TIT): 1:128, Abnormal, status Final results.');
		expect(read(sn('^10^-^20', 'mg/dL'))).toBe('Titer (TIT): 10-20 mg/dL, Abnormal, status Final results.');
		expect(read(sn('>=^100'))).toBe('Titer (TIT): >=100, Abnormal, status Final results.');
	});

	it('keeps every repetition of a generic field, each linked to itself', () => {
		const parts = blocks[2].sentences[0];
		expect(parts.find((p) => p.text === 'home')?.link).toEqual({ segmentIdx: 2, field: 6, repetition: 1, component: null });
		expect(parts.find((p) => p.text === 'work')?.link).toEqual({ segmentIdx: 2, field: 6, repetition: 2, component: null });
		expect(parts.find((p) => p.text === 'a b')?.link).toEqual({ segmentIdx: 2, field: 5, repetition: null, component: null });
	});
});

describe('formatHl7Date', () => {
	it('writes dates and timestamps the readable way', () => {
		expect(formatHl7Date('19800101')).toBe('1980-01-01');
		expect(formatHl7Date('202601')).toBe('2026-01');
		expect(formatHl7Date('20260105103000')).toBe('2026-01-05 10:30');
		expect(formatHl7Date('20260105103015+0100')).toBe('2026-01-05 10:30:15 +01:00');
		expect(formatHl7Date('not a date')).toBe('not a date');
	});
});
