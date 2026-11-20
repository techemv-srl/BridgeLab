// The open HL7 v2 message read as sentences, for the interpretation panel:
// "Patient DOE, JOHN, Male, born 1980-01-01, identifiers 123456 (Medical
// record number)." Each value in a sentence links to the field it comes
// from. The common segments of ADT, ORU and ORM messages (MSH, EVN, PID,
// PV1, NK1, ORC, OBR, OBX, AL1, DG1, NTE) get their own wording; any other
// segment, a Z-segment included, is read field by field.
//
// Sentences are built from clauses, each an `interp.*` string with
// placeholders; a clause is left out when a value it needs is missing, so
// "born {date}" only appears when PID-7 has a date. Code meanings come from
// the HL7 tables and stay in English, as the tree shows them.

import { t } from '$lib/i18n';
import type { FieldTarget } from './segment-lines';

export interface ReadValue {
	text: string;
	truncated: boolean;
	components: string[];
	meaning: string | null;
	component_meanings: (string | null)[];
}

export interface ReadField {
	position: number;
	name: string;
	data_type: string;
	repetitions: ReadValue[];
}

export interface ReadSegment {
	segment_idx: number;
	segment_type: string;
	segment_name: string;
	fields: ReadField[];
}

export interface Reading {
	version: string;
	message_type: string;
	segments: ReadSegment[];
}

/** A piece of a sentence; `link` selects the field it comes from. */
export interface Part {
	text: string;
	link?: { segmentIdx: number } & FieldTarget;
}

export interface SegmentSentences {
	segmentIdx: number;
	segmentType: string;
	segmentName: string;
	/** True when the segment has no wording of its own and is read field by field. */
	fieldByField: boolean;
	sentences: Part[][];
}

type Value = Part[] | null;

/** A table meaning as a phrase: single spaces, no closing period ("Final
 *  results; Can only be changed with a corrected result."). */
function tidy(meaning: string | null | undefined): string | null {
	const s = meaning?.replace(/\s+/g, ' ').trim().replace(/\.$/, '');
	if (!s) return null;
	// The tables add notes after the meaning ("Final results; Can only be
	// changed with a corrected result", "Normal (applies to non-numeric
	// results)"): a sentence keeps the meaning, the editor's tooltip the rest.
	const short = s.split('; ')[0].replace(/\s*\([^()]*\)$/, '').trim();
	return short || s;
}

/** Reads one segment: lookups by field position, values as linked parts. */
class Seg {
	constructor(readonly s: ReadSegment) {}

	field(pos: number): ReadField | undefined {
		return this.s.fields.find((f) => f.position === pos);
	}

	first(pos: number): ReadValue | undefined {
		return this.field(pos)?.repetitions[0];
	}

	link(pos: number, repetition: number | null = null, component: number | null = null): Part['link'] {
		return { segmentIdx: this.s.segment_idx, field: pos, repetition, component };
	}

	/** The field's first value as written. */
	text(pos: number): Value {
		const v = this.first(pos);
		return v && v.text.trim() ? [{ text: v.text.trim() + (v.truncated ? '…' : ''), link: this.link(pos) }] : null;
	}

	/** A component of the field's first value. */
	component(pos: number, comp: number): Value {
		const c = this.first(pos)?.components[comp - 1]?.trim();
		return c ? [{ text: c, link: this.link(pos, null, comp) }] : null;
	}

	/** A coded value: its table meaning, or the code itself. */
	coded(pos: number): Value {
		const v = this.first(pos);
		const code = v?.components[0]?.trim();
		if (!v || !code) return null;
		return [{ text: tidy(v.meaning) ?? code, link: this.link(pos) }];
	}

	/** A coded element (CE/CWE): "text (code)", or the table meaning. */
	codedElement(pos: number): Value {
		const v = this.first(pos);
		if (!v) return null;
		const [code = '', text = ''] = v.components.map((c) => c.trim());
		const label = text || tidy(v.meaning) || '';
		if (!label && !code) return null;
		const shown = label && code && label !== code ? `${label} (${code})` : label || code;
		return [{ text: shown, link: this.link(pos) }];
	}

	/** A person's name (XPN): "FAMILY, Given Middle". */
	name(pos: number, first = 0): Value {
		const v = this.first(pos);
		if (!v) return null;
		const [family = '', given = '', middle = ''] = v.components.slice(first).map((c) => c.trim());
		const rest = [given, middle].filter(Boolean).join(' ');
		const shown = family && rest ? `${family}, ${rest}` : family || rest;
		return shown ? [{ text: shown, link: this.link(pos) }] : null;
	}

	/** A person with an id (XCN): "FAMILY, Given (id)". */
	person(pos: number): Value {
		const v = this.first(pos);
		if (!v) return null;
		const id = v.components[0]?.trim() ?? '';
		const name = this.name(pos, 1)?.[0]?.text ?? '';
		const shown = name && id ? `${name} (${id})` : name || id;
		return shown ? [{ text: shown, link: this.link(pos) }] : null;
	}

	/** Every identifier of a CX field: "123456 (Medical record number)". */
	identifiers(pos: number): Value {
		const f = this.field(pos);
		if (!f) return null;
		const parts: Part[] = [];
		f.repetitions.forEach((v, i) => {
			const id = v.components[0]?.trim();
			if (!id) return;
			const type = tidy(v.component_meanings[4]) ?? v.components[4]?.trim();
			if (parts.length) parts.push({ text: ', ' });
			parts.push({
				text: type ? `${id} (${type})` : id,
				link: this.link(pos, f.repetitions.length > 1 ? i + 1 : null),
			});
		});
		return parts.length ? parts : null;
	}

	/** An entity identifier (EI): its first component. */
	entity(pos: number): Value {
		return this.component(pos, 1);
	}

	/** An application or facility (HD): the namespace, else the universal id. */
	hierarchic(pos: number): Value {
		const v = this.first(pos);
		const shown = v?.components[0]?.trim() || v?.components[1]?.trim();
		return shown ? [{ text: shown, link: this.link(pos) }] : null;
	}

	/** A date or timestamp, written the readable way. */
	date(pos: number): Value {
		const raw = this.first(pos)?.components[0]?.trim();
		return raw ? [{ text: formatHl7Date(raw), link: this.link(pos) }] : null;
	}

	/** A person location (PL): "W3, room 12, bed 1". */
	location(pos: number): Value {
		const v = this.first(pos);
		if (!v) return null;
		const [unit = '', room = '', bed = ''] = v.components.map((c) => c.trim());
		const bits = [
			unit,
			room ? t('interp.location.room', { room }) : '',
			bed ? t('interp.location.bed', { bed }) : '',
		].filter(Boolean);
		return bits.length ? [{ text: bits.join(', '), link: this.link(pos) }] : null;
	}

	/** An address (XAD): "1 MAIN ST, SPRINGFIELD 12345, USA". */
	address(pos: number): Value {
		const v = this.first(pos);
		if (!v) return null;
		const [street = '', other = '', city = '', state = '', zip = '', country = ''] = v.components.map((c) => c.trim());
		const place = [city, state, zip].filter(Boolean).join(' ');
		const shown = [street, other, place, country].filter(Boolean).join(', ');
		return shown ? [{ text: shown, link: this.link(pos) }] : null;
	}

	/** A telephone number (XTN): the number as written, else its parts. */
	phone(pos: number): Value {
		const v = this.first(pos);
		if (!v) return null;
		const c = v.components.map((x) => x.trim());
		const shown = c[0] || c[11] || [c[5], c[6]].filter(Boolean).join(' ');
		return shown ? [{ text: shown, link: this.link(pos) }] : null;
	}

	/** A field read by its data type, for segments with no wording of their
	 *  own: every repetition, each linked to itself. */
	typed(f: ReadField): Value {
		if (f.data_type === 'CX') return this.identifiers(f.position);
		const parts: Part[] = [];
		f.repetitions.forEach((v, i) => {
			// The helpers read a field's first value: give them this one.
			const one = new Seg({ ...this.s, fields: [{ ...f, repetitions: [v] }] }).typedValue(f);
			if (!one) return;
			const repetition = f.repetitions.length > 1 ? i + 1 : null;
			if (parts.length) parts.push({ text: ', ' });
			parts.push(...one.map((p) => (p.link ? { ...p, link: { ...p.link, repetition } } : p)));
		});
		return parts.length ? parts : null;
	}

	/** The field's first value by its data type: names, people, codes,
	 *  dates, places as in the sentences, other values as their non-empty
	 *  components. */
	private typedValue(f: ReadField): Value {
		switch (f.data_type) {
			case 'CE': case 'CWE': case 'CNE': case 'CF': return this.codedElement(f.position);
			case 'XPN': return this.name(f.position);
			case 'XCN': return this.person(f.position);
			case 'XAD': return this.address(f.position);
			case 'XTN': return this.phone(f.position);
			case 'PL': return this.location(f.position);
			case 'HD': return this.hierarchic(f.position);
			case 'EI': return this.entity(f.position);
			case 'TS': case 'DTM': case 'DT': return this.date(f.position);
			case 'SN': return this.numeric(f.position);
		}
		const v = this.first(f.position);
		const shown = v?.components.map((c) => c.trim()).filter(Boolean).join(' ');
		if (!v || !shown) return null;
		// A code keeps its value next to the meaning, as coded elements do
		// ("New appointment (NEW)"): with no sentence around it, the code is
		// what one looks for.
		const meaning = tidy(v.meaning);
		const text = meaning ? `${meaning} (${shown})` : shown + (v.truncated ? '…' : '');
		return [{ text, link: this.link(f.position) }];
	}

	/** A structured numeric (SN) value as it reads: comparator, number,
	 *  separator or suffix, second number, with no separators between
	 *  them ("<^5" → "<5", "^1^:^128" → "1:128", "^10^-^20" → "10-20"). */
	numeric(pos: number): Value {
		const v = this.first(pos);
		const shown = v?.components.slice(0, 4).map((c) => c.trim()).join('');
		return v && shown ? [{ text: shown + (v.truncated ? '…' : ''), link: this.link(pos) }] : null;
	}

	/** Every repetition of a text field, joined. */
	allText(pos: number): Value {
		const f = this.field(pos);
		const shown = f?.repetitions.map((r) => r.text.trim()).filter(Boolean).join(' ');
		return shown ? [{ text: shown, link: this.link(pos) }] : null;
	}
}

/** "19800101" → "1980-01-01", "20260105103000+0100" → "2026-01-05 10:30 +01:00". */
export function formatHl7Date(raw: string): string {
	const m = /^(\d{4})(\d{2})?(\d{2})?(\d{2})?(\d{2})?(\d{2})?(\.\d+)?([+-]\d{4})?$/.exec(raw.trim());
	if (!m) return raw;
	const [, y, mo, d, h, mi, s, , zone] = m;
	let out = [y, mo, d].filter(Boolean).join('-');
	if (h) out += ` ${h}:${mi ?? '00'}${s && s !== '00' ? `:${s}` : ''}`;
	if (zone) out += ` ${zone.slice(0, 3)}:${zone.slice(3)}`;
	return out;
}

interface Clause {
	key: string;
	values: Record<string, Value>;
}

/** A clause from its `interp.*` string, or null when a value is missing. */
function renderClause(c: Clause): Part[] | null {
	if (Object.values(c.values).some((v) => !v || v.length === 0)) return null;
	const template = t(c.key);
	const out: Part[] = [];
	for (const piece of template.split(/(\{\w+\})/)) {
		const name = /^\{(\w+)\}$/.exec(piece)?.[1];
		if (name && c.values[name]) out.push(...(c.values[name] as Part[]));
		else if (piece) out.push({ text: piece });
	}
	return out;
}

/** The clauses that have their values, joined into one sentence. */
function sentence(clauses: Clause[]): Part[] | null {
	const rendered = clauses.map(renderClause).filter((c): c is Part[] => c !== null);
	if (rendered.length === 0) return null;
	const out: Part[] = [];
	rendered.forEach((parts, i) => {
		if (i > 0) out.push({ text: ', ' });
		out.push(...parts);
	});
	// A value can end with its own period ("Dr."): no second one.
	if (!out[out.length - 1].text.endsWith('.')) out.push({ text: '.' });
	// A sentence starts with a capital, whichever clause came first.
	const head = out[0];
	out[0] = { ...head, text: head.text.charAt(0).toLocaleUpperCase() + head.text.slice(1) };
	return out;
}

function clause(key: string, values: Record<string, Value>): Clause {
	return { key, values };
}

/** An application and its facility as one value: "LAB (HOSP)", each linked. */
function withPlace(app: Value, facility: Value): Value {
	if (app && facility) return [...app, { text: ' (' }, ...facility, { text: ')' }];
	return app ?? facility;
}

/** MSH-9 with the meaning of its event: "ADT^A01 (Admit/visit notification)". */
function messageType(s: Seg): Value {
	const v = s.first(9);
	if (!v) return null;
	const code = v.components.filter(Boolean).slice(0, 2).join('^');
	const event = tidy(v.component_meanings[1]) ?? tidy(v.meaning);
	return code ? [{ text: event ? `${code} (${event})` : code, link: s.link(9) }] : null;
}

/** OBX-5 read by its OBX-2 value type, with OBX-6 units for numbers. */
function observationValue(s: Seg): Value {
	const type = s.first(2)?.components[0]?.trim().toUpperCase() ?? '';
	if (type === 'ED' || type === 'RP') {
		return [{ text: t('interp.obx.embedded', { type }), link: s.link(5) }];
	}
	if (type === 'CE' || type === 'CWE' || type === 'CNE') return s.codedElement(5);
	if (type === 'DT' || type === 'TS' || type === 'DTM') return s.date(5);
	const read = type === 'SN' ? s.numeric(5) : type === 'TX' || type === 'FT' ? s.allText(5) : s.text(5);
	if (!read) return null;
	// Free text ends its own sentences ("Troponin negative x2."); the clauses
	// that follow bring the punctuation.
	const last = read[read.length - 1];
	const value = [...read.slice(0, -1), { ...last, text: last.text.replace(/\.$/, '') }];
	const units = s.first(6);
	const unit = units ? units.components[1]?.trim() || units.components[0]?.trim() : '';
	return unit ? [...value, { text: ' ' }, { text: unit, link: s.link(6) }] : value;
}

/** The sentences for a segment with its own wording, or null for the rest. */
function worded(s: Seg): (Part[] | null)[] | null {
	switch (s.s.segment_type) {
		case 'MSH':
			return [
				sentence([
					clause('interp.msh.message', { type: messageType(s) }),
					clause('interp.msh.sent', { date: s.date(7) }),
					clause('interp.msh.from', { sender: withPlace(s.hierarchic(3), s.hierarchic(4)) }),
					clause('interp.msh.to', { receiver: withPlace(s.hierarchic(5), s.hierarchic(6)) }),
				]),
				sentence([
					clause('interp.msh.control', { control: s.text(10) }),
					clause('interp.msh.processing', { processing: s.coded(11) }),
					clause('interp.msh.version', { version: s.component(12, 1) }),
				]),
			];
		case 'EVN':
			return [
				sentence([
					clause('interp.evn.event', { event: s.coded(1) }),
					clause('interp.evn.recorded', { date: s.date(2) }),
					clause('interp.evn.planned', { date: s.date(3) }),
					clause('interp.evn.reason', { reason: s.coded(4) }),
					clause('interp.evn.operator', { person: s.person(5) }),
				]),
			];
		case 'PID':
			return [
				sentence([
					clause('interp.pid.patient', { name: s.name(5) }),
					clause('interp.pid.sex', { sex: s.coded(8) }),
					clause('interp.pid.born', { date: s.date(7) }),
					clause('interp.pid.ids', { ids: s.identifiers(3) }),
				]),
				sentence([
					clause('interp.pid.address', { address: s.address(11) }),
					clause('interp.pid.phone', { phone: s.phone(13) }),
					clause('interp.pid.account', { account: s.component(18, 1) }),
				]),
			];
		case 'PV1':
			return [
				sentence([
					clause('interp.pv1.class', { class: s.coded(2) }),
					clause('interp.pv1.location', { location: s.location(3) }),
					clause('interp.pv1.attending', { person: s.person(7) }),
					clause('interp.pv1.referring', { person: s.person(8) }),
				]),
				sentence([
					clause('interp.pv1.visit', { visit: s.component(19, 1) }),
					clause('interp.pv1.admitted', { date: s.date(44) }),
					clause('interp.pv1.discharged', { date: s.date(45) }),
				]),
			];
		case 'NK1':
			return [
				sentence([
					clause('interp.nk1.kin', { name: s.name(2) }),
					clause('interp.nk1.relationship', { relationship: s.codedElement(3) }),
					clause('interp.nk1.phone', { phone: s.phone(5) }),
				]),
			];
		case 'ORC':
			return [
				sentence([
					clause('interp.orc.control', { control: s.coded(1) }),
					clause('interp.order.placer', { id: s.entity(2) }),
					clause('interp.order.filler', { id: s.entity(3) }),
					clause('interp.orc.status', { status: s.coded(5) }),
					clause('interp.orc.orderedBy', { person: s.person(12) }),
				]),
			];
		case 'OBR':
			return [
				sentence([
					clause('interp.obr.service', { service: s.codedElement(4) }),
					clause('interp.order.placer', { id: s.entity(2) }),
					clause('interp.order.filler', { id: s.entity(3) }),
					clause('interp.obr.observed', { date: s.date(7) }),
					clause('interp.obr.status', { status: s.coded(25) }),
				]),
			];
		case 'OBX': {
			const observation = s.codedElement(3);
			const value = observationValue(s);
			const lead =
				observation && value
					? clause('interp.obx.result', { observation, value })
					: observation
						? clause('interp.obx.observation', { observation })
						: clause('interp.obx.value', { value });
			return [
				sentence([
					lead,
					clause('interp.obx.range', { range: s.text(7) }),
					clause('interp.obx.flag', { flag: s.coded(8) }),
					clause('interp.obx.status', { status: s.coded(11) }),
				]),
			];
		}
		case 'AL1':
			return [
				sentence([
					clause('interp.al1.allergy', { allergen: s.codedElement(3) }),
					clause('interp.al1.type', { type: s.coded(2) }),
					clause('interp.al1.severity', { severity: s.coded(4) }),
					clause('interp.al1.reaction', { reaction: s.text(5) }),
				]),
			];
		case 'DG1':
			return [
				sentence([
					clause('interp.dg1.diagnosis', { diagnosis: s.codedElement(3) }),
					clause('interp.dg1.type', { type: s.coded(6) }),
					clause('interp.dg1.date', { date: s.date(5) }),
				]),
			];
		case 'NTE':
			return [sentence([clause('interp.nte.note', { comment: s.allText(3) })])];
		default:
			return null;
	}
}

/** Any segment field by field: "Name: value; …", each value read by type. */
function fieldByField(s: Seg): Part[] | null {
	const out: Part[] = [];
	for (const f of s.s.fields) {
		const value = s.typed(f);
		if (!value) continue;
		if (out.length) out.push({ text: '; ' });
		out.push({ text: `${f.name || t('interp.field', { position: f.position })}: ` });
		out.push(...value);
	}
	return out.length ? out : null;
}

/** The whole message, segment by segment, as linked sentences. */
export function interpretMessage(reading: Reading): SegmentSentences[] {
	return reading.segments.map((raw) => {
		const s = new Seg(raw);
		const own = worded(s)?.filter((x): x is Part[] => x !== null) ?? null;
		const useFields = !own || own.length === 0;
		const sentences = useFields ? [fieldByField(s)].filter((x): x is Part[] => x !== null) : own;
		return {
			segmentIdx: raw.segment_idx,
			segmentType: raw.segment_type,
			segmentName: raw.segment_name,
			fieldByField: useFields,
			sentences,
		};
	});
}
