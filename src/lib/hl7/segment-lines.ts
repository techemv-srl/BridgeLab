/**
 * Where the parser's segments are in the editor's text.
 *
 * The parser numbers segments; the editor numbers lines. They differ when
 * the text has blank lines (between segments, or before MSH), an MLLP
 * frame, or a byte-order mark: the parser skips those (see lexer.rs), so
 * segment N is not line N + 1. Every jump between tree, grid, validation
 * panel and editor goes through these helpers, which skip the same things
 * the parser does.
 */

/** A line holding only these characters is no segment (lexer.rs is_blank_byte). */
const BLANK_LINE = /^[ \t\v\f\x1c]*$/;
/** MLLP frame bytes before a segment are not part of it. */
const FRAME_LEAD = /^[\v\x1c]*/;
/** Before the first segment the parser also skips a BOM and spaces. */
const MESSAGE_LEAD = /^[﻿ \t\v\f\x1c]*/;

export interface Separators {
	field: string;
	component: string;
	repetition: string;
	escape: string;
	subcomponent: string;
}

export const DEFAULT_SEPARATORS: Separators = { field: '|', component: '^', repetition: '~', escape: '\\', subcomponent: '&' };

export function splitLines(text: string): string[] {
	return text.split(/\r\n|\r|\n/);
}

/** 0-based line index of every segment, in segment order. */
export function segmentLineIndexes(lines: string[]): number[] {
	const out: number[] = [];
	for (let i = 0; i < lines.length; i++) {
		const line = out.length === 0 ? lines[i].replace(MESSAGE_LEAD, '') : lines[i];
		if (!BLANK_LINE.test(line)) out.push(i);
	}
	return out;
}

/** The line (0-based) of segment `segmentIdx`, or null past the last one. */
export function lineOfSegment(lines: string[], segmentIdx: number): number | null {
	return segmentLineIndexes(lines)[segmentIdx] ?? null;
}

/** The segment on line `lineIdx` (0-based), or null for a blank line. */
export function segmentOfLine(lines: string[], lineIdx: number): number | null {
	const idx = segmentLineIndexes(lines).indexOf(lineIdx);
	return idx < 0 ? null : idx;
}

/** Where the segment starts on its line: past what the parser skips. */
export function segmentStart(line: string, firstSegment: boolean): number {
	return (line.match(firstSegment ? MESSAGE_LEAD : FRAME_LEAD)?.[0] ?? '').length;
}

function isHeader(type: string): boolean {
	return type === 'MSH' || type === 'FHS' || type === 'BHS';
}

/** The separators the message declares in its first header (MSH-1/MSH-2,
 *  or FHS/BHS in a batch file); the standard ones otherwise. */
export function separatorsOf(lines: string[]): Separators {
	// Only the first segment declares them (the parser reads them there).
	const first = lines.findIndex((l) => !BLANK_LINE.test(l.replace(MESSAGE_LEAD, '')));
	if (first < 0) return DEFAULT_SEPARATORS;
	const body = lines[first].slice(segmentStart(lines[first], true));
	if (isHeader(body.slice(0, 3)) && body.length >= 8) {
		return { field: body[3], component: body[4], repetition: body[5], escape: body[6], subcomponent: body[7] };
	}
	return DEFAULT_SEPARATORS;
}

/** The segment type: everything before the first field separator. */
export function segmentType(body: string, seps: Separators): string {
	const end = body.indexOf(seps.field);
	return end < 0 ? body : body.slice(0, end);
}

/** A position inside a segment: field (0 = the segment itself), and the
 *  repetition and component when the field has more than one of them. */
export interface FieldTarget {
	field: number;
	repetition: number | null;
	component: number | null;
}

/**
 * The field under the caret at 1-based `column` of a segment line.
 * MSH (and FHS/BHS) count differently: the separator itself is field 1,
 * the encoding characters field 2.
 */
export function fieldAtColumn(line: string, column: number, seps: Separators, firstSegment = false): FieldTarget | null {
	const start = segmentStart(line, firstSegment);
	const body = line.slice(start);
	const type = segmentType(body, seps);
	if (!type) return null;
	const off = column - 1 - start;
	const none: FieldTarget = { field: 0, repetition: null, component: null };
	if (off < type.length) return none;
	const header = isHeader(type) && type.length === 3;
	if (header && off === 3) return { field: 1, repetition: null, component: null };
	let seen = 0;
	let fieldStart = 0;
	for (let i = 0; i < off && i < body.length; i++) {
		if (body[i] === seps.field) {
			seen++;
			fieldStart = i + 1;
		}
	}
	if (header && seen <= 1) return { field: seen === 0 ? 1 : 2, repetition: null, component: null };
	const field = header ? seen + 1 : seen;
	if (field === 0) return none;
	let fieldEnd = body.indexOf(seps.field, fieldStart);
	if (fieldEnd < 0) fieldEnd = body.length;
	const text = body.slice(fieldStart, fieldEnd);
	const inField = Math.max(0, off - fieldStart);
	const reps = text.split(seps.repetition);
	const repIdx = text.slice(0, inField).split(seps.repetition).length - 1;
	const repText = reps[repIdx] ?? '';
	const repStart = reps.slice(0, repIdx).reduce((n, r) => n + r.length + 1, 0);
	const comps = repText.split(seps.component);
	const compIdx = repText.slice(0, inField - repStart).split(seps.component).length - 1;
	return {
		field,
		repetition: reps.length > 1 ? repIdx + 1 : null,
		component: comps.length > 1 ? compIdx + 1 : null,
	};
}

/**
 * The text range (1-based column, length) of a field, repetition or
 * component on a segment line, for selecting it in the editor. The whole
 * segment when `target.field` is 0 or the field does not exist.
 */
export function fieldRange(line: string, target: FieldTarget, seps: Separators, firstSegment = false): { column: number; length: number } {
	const start = segmentStart(line, firstSegment);
	const body = line.slice(start);
	const whole = { column: start + 1, length: Math.max(1, body.length) };
	if (target.field <= 0) return whole;
	const type = segmentType(body, seps);
	const header = isHeader(type) && type.length === 3;
	if (header && target.field === 1) return { column: start + 4, length: 1 };
	if (header && target.field === 2) {
		let end = body.indexOf(seps.field, 4);
		if (end < 0) end = body.length;
		return { column: start + 5, length: Math.max(1, end - 4) };
	}
	// Field separators to pass: MSH-3 follows the second one.
	const passes = header ? target.field - 1 : target.field;
	let cursor = 0;
	let seen = 0;
	while (cursor < body.length && seen < passes) {
		if (body[cursor] === seps.field) seen++;
		cursor++;
	}
	if (seen < passes) return whole;
	let end = body.indexOf(seps.field, cursor);
	if (end < 0) end = body.length;
	let from = cursor;
	let text = body.slice(cursor, end);
	if (target.repetition !== null && target.repetition > 0) {
		const reps = text.split(seps.repetition);
		if (target.repetition <= reps.length) {
			from += reps.slice(0, target.repetition - 1).reduce((n, r) => n + r.length + 1, 0);
			text = reps[target.repetition - 1];
		}
	}
	if (target.component !== null && target.component > 0) {
		const comps = text.split(seps.component);
		if (target.component <= comps.length) {
			from += comps.slice(0, target.component - 1).reduce((n, c) => n + c.length + 1, 0);
			text = comps[target.component - 1];
		}
	}
	return { column: start + from + 1, length: Math.max(1, text.length) };
}

/** A segment skeleton: the code and enough field separators to show the
 *  slots up to `fields`, written with the message's own separators. */
export function segmentSkeleton(code: string, fields: number, seps: Separators): string {
	if (code === 'MSH') {
		const f = seps.field;
		return `MSH${f}${seps.component}${seps.repetition}${seps.escape}${seps.subcomponent}` + f.repeat(Math.max(0, fields - 2));
	}
	return code + seps.field.repeat(Math.max(1, fields));
}

/**
 * The line index (0-based) a new segment goes on so that it follows
 * segment `afterSegmentIdx` (null: the top, but never above the header
 * unless the new segment is a header itself).
 */
export function insertionLine(lines: string[], afterSegmentIdx: number | null, code: string): number {
	const segLines = segmentLineIndexes(lines);
	let at: number;
	if (afterSegmentIdx === null) {
		at = isHeader(code) || segLines.length === 0 ? (segLines[0] ?? 0) : segLines[0] + 1;
	} else {
		const line = segLines[afterSegmentIdx];
		at = line === undefined ? lines.length : line + 1;
	}
	// Not after the empty line a final terminator leaves.
	if (at >= lines.length && lines.length > 0 && lines[lines.length - 1] === '') at = lines.length - 1;
	return Math.min(at, lines.length);
}

/**
 * Where the segments the standard expects but the message lacks belong.
 * `present` is the message's segment codes in order, `expected` the
 * message structure (a code may appear more than once: ROL, NTE).
 *
 * Each real segment is matched to its place in the structure (forward
 * from the previous one; back to the start for a repeated group), and an
 * absent code goes before the first real segment that comes after it in
 * the structure. Anchoring on "the last real instance of the previous
 * code" put NK1 after the second ROL of an ADT (ROL occurs twice).
 *
 * Returns, for each absent code (first occurrence in the structure), the
 * index of the real segment it goes before; `present.length` = the end.
 */
export function placeAbsentSegments(present: string[], expected: Array<{ code: string }>): Array<{ code: string; defIdx: number; before: number }> {
	const pos: number[] = [];
	let p = 0;
	for (const code of present) {
		let idx = expected.findIndex((e, i) => i >= p && e.code === code);
		if (idx < 0) idx = expected.findIndex((e) => e.code === code);
		// Unknown here (a Z-segment): stays with what precedes it.
		if (idx >= 0) p = idx;
		pos.push(p);
	}
	const presentCodes = new Set(present);
	const seen = new Set<string>();
	const out: Array<{ code: string; defIdx: number; before: number }> = [];
	expected.forEach((e, defIdx) => {
		if (presentCodes.has(e.code) || seen.has(e.code)) return;
		seen.add(e.code);
		let before = pos.findIndex((q) => q > defIdx);
		if (before < 0) before = present.length;
		out.push({ code: e.code, defIdx, before });
	});
	return out;
}
