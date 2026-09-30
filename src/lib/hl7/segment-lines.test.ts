import { describe, expect, it } from 'vitest';
import {
	DEFAULT_SEPARATORS as SEPS,
	fieldAtColumn,
	fieldRange,
	insertionLine,
	lineOfSegment,
	placeAbsentSegments,
	segmentLineIndexes,
	segmentOfLine,
	segmentSkeleton,
	separatorsOf,
	splitLines,
} from './segment-lines';

const MSH = 'MSH|^~\\&|SEND|FAC|RECV|FAC|20240101120000||ADT^A01|MSG1|P|2.5';

describe('segment lines', () => {
	it('skips blank lines between and before segments', () => {
		const lines = splitLines(`\r\n${MSH}\n\nEVN|A01\n  \nPID|1||1\n`);
		expect(segmentLineIndexes(lines)).toEqual([1, 3, 5]);
		expect(lineOfSegment(lines, 2)).toBe(5);
		expect(lineOfSegment(lines, 3)).toBeNull();
		expect(segmentOfLine(lines, 3)).toBe(1);
		expect(segmentOfLine(lines, 4)).toBeNull();
	});

	it('treats a BOM or MLLP start byte as part of the first line lead', () => {
		const lines = splitLines(`﻿\r\v${MSH}\rPID|1\r\x1c\r`);
		expect(segmentLineIndexes(lines)).toEqual([1, 2]);
		expect(separatorsOf(lines).field).toBe('|');
	});

	it('reads custom separators from MSH or a batch header', () => {
		expect(separatorsOf(splitLines('MSH#@!\\$#A')).component).toBe('@');
		expect(separatorsOf(splitLines('FHS#@!\\$#A\rMSH#@!\\$#A')).field).toBe('#');
		expect(separatorsOf(splitLines('PID|1'))).toEqual(SEPS);
	});
});

describe('fieldAtColumn', () => {
	it('counts MSH fields from the separator itself', () => {
		expect(fieldAtColumn(MSH, 2, SEPS)?.field).toBe(0);
		expect(fieldAtColumn(MSH, 4, SEPS)?.field).toBe(1);
		expect(fieldAtColumn(MSH, 6, SEPS)?.field).toBe(2);
		expect(fieldAtColumn(MSH, 11, SEPS)?.field).toBe(3); // in SEND
		const col = MSH.indexOf('ADT') + 1;
		expect(fieldAtColumn(MSH, col + 1, SEPS)).toEqual({ field: 9, repetition: null, component: 1 });
		expect(fieldAtColumn(MSH, col + 5, SEPS)).toEqual({ field: 9, repetition: null, component: 2 });
	});

	it('finds field, repetition and component in other segments', () => {
		const pid = 'PID|1||111^^^H^MR~222^^^H^SS||Doe^John';
		expect(fieldAtColumn(pid, 3, SEPS)?.field).toBe(0);
		expect(fieldAtColumn(pid, 6, SEPS)).toEqual({ field: 1, repetition: null, component: null });
		expect(fieldAtColumn(pid, pid.indexOf('222') + 2, SEPS)).toEqual({ field: 3, repetition: 2, component: 1 });
		expect(fieldAtColumn(pid, pid.indexOf('SS') + 1, SEPS)).toEqual({ field: 3, repetition: 2, component: 5 });
		expect(fieldAtColumn(pid, pid.indexOf('John') + 1, SEPS)).toEqual({ field: 5, repetition: null, component: 2 });
	});

	it('honours the message separators', () => {
		const seps = { ...SEPS, field: '#', component: '@' };
		expect(fieldAtColumn('PID#1##X@Y', 10, seps)).toEqual({ field: 3, repetition: null, component: 2 });
		expect(fieldAtColumn('MSH#@~\\&#SEND', 11, seps)?.field).toBe(3);
	});

	it('skips the lead of the first segment', () => {
		expect(fieldAtColumn(`  ${MSH}`, 13, SEPS, true)?.field).toBe(3);
	});
});

describe('fieldRange', () => {
	it('selects MSH-1, MSH-2 and later MSH fields', () => {
		expect(fieldRange(MSH, { field: 1, repetition: null, component: null }, SEPS)).toEqual({ column: 4, length: 1 });
		expect(fieldRange(MSH, { field: 2, repetition: null, component: null }, SEPS)).toEqual({ column: 5, length: 4 });
		expect(fieldRange(MSH, { field: 3, repetition: null, component: null }, SEPS)).toEqual({ column: 10, length: 4 });
		const r = fieldRange(MSH, { field: 9, repetition: null, component: 2 }, SEPS);
		expect(MSH.substr(r.column - 1, r.length)).toBe('A01');
	});

	it('narrows to a repetition and a component', () => {
		const pid = 'PID|1||111^^^H^MR~222^^^H^SS||Doe^John';
		const r = fieldRange(pid, { field: 3, repetition: 2, component: 5 }, SEPS);
		expect(pid.substr(r.column - 1, r.length)).toBe('SS');
		const f = fieldRange(pid, { field: 5, repetition: null, component: null }, SEPS);
		expect(pid.substr(f.column - 1, f.length)).toBe('Doe^John');
		expect(fieldRange(pid, { field: 0, repetition: null, component: null }, SEPS)).toEqual({ column: 1, length: pid.length });
	});
});

describe('inserting a segment', () => {
	it('builds the skeleton with the message separators', () => {
		expect(segmentSkeleton('NK1', 2, { ...SEPS, field: '#' })).toBe('NK1##');
		expect(segmentSkeleton('PV1', 0, SEPS)).toBe('PV1|');
	});

	it('goes after its predecessor, skipping blank lines', () => {
		const lines = splitLines(`${MSH}\n\nPID|1\nPV1|1\n`);
		expect(insertionLine(lines, 1, 'NK1')).toBe(3);
		expect(insertionLine(lines, null, 'EVN')).toBe(1);
		expect(insertionLine(lines, null, 'MSH')).toBe(0);
		expect(insertionLine(lines, 2, 'ROL')).toBe(4); // not after the final empty line
	});

	it('places absent segments by the structure, not by the last instance of a repeated code', () => {
		const adt = ['MSH', 'EVN', 'PID', 'PD1', 'ROL', 'NK1', 'PV1', 'PV2', 'ROL', 'DB1'].map((code) => ({ code }));
		const placed = placeAbsentSegments(['MSH', 'EVN', 'PID', 'PV1', 'ROL'], adt);
		expect(placed.find((p) => p.code === 'NK1')?.before).toBe(3); // before PV1
		expect(placed.find((p) => p.code === 'DB1')?.before).toBe(5); // the end

		const oru = ['MSH', 'SFT', 'PID', 'PD1', 'NTE', 'NK1', 'PV1', 'PV2', 'ORC', 'OBR', 'NTE', 'TQ1', 'OBX', 'NTE'].map((code) => ({ code }));
		const p2 = placeAbsentSegments(['MSH', 'PID', 'OBR', 'OBX', 'NTE'], oru);
		expect(p2.find((p) => p.code === 'PV1')?.before).toBe(2); // before OBR
		expect(p2.find((p) => p.code === 'ORC')?.before).toBe(2);
		expect(p2.find((p) => p.code === 'TQ1')?.before).toBe(3); // between OBR and OBX
		expect(p2.find((p) => p.code === 'SFT')?.before).toBe(1);
		expect(p2.filter((p) => p.code === 'NTE')).toEqual([]);
	});
});
