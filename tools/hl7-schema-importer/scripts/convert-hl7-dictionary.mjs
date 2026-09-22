#!/usr/bin/env node
/**
 * Convert hl7-dictionary (npm, MIT) definitions into the BridgeLab schema
 * JSON consumed by src-tauri/src/parser/hl7/schema/mod.rs.
 *
 * The dictionary ships CommonJS modules (real JS, not JSON), so the
 * conversion lives here in Node; the Rust importer's `bridgelab-json`
 * round-trip is then used to validate the output before shipping it:
 *
 *   npm pack hl7-dictionary && tar xzf hl7-dictionary-*.tgz
 *   node scripts/convert-hl7-dictionary.mjs ./package 2.5 /tmp/v2_5.json
 *   cargo run -- --format bridgelab-json --source-dir /tmp --hl7-version 2.5 \
 *         --output ../../src-tauri/resources/hl7/v2_5.json
 *
 * Mapping notes (calibrated against the v2.5 standard):
 *   opt: 2 -> required, anything else (1 optional, 3 conditional…) -> optional
 *   rep: 0 -> unbounded repeats, 1 -> no repeat, n>1 -> bounded repeats
 *   message nodes with `children` -> Group; min>=1 -> required;
 *   max===0 (unbounded) or max>1 -> repeats
 *   fields.js entries with subfields -> composites, without -> primitives
 *   table: n -> "000n" (zero-padded HL7 table id) on fields and components
 *   len: n -> max_length, except the 99999 the dictionary uses for "unbounded"
 *
 * With a fourth argument the HL7 value tables (lib/tables.js, one set for
 * every version) are written there as well:
 *
 *   node scripts/convert-hl7-dictionary.mjs ./package 2.5 /tmp/v2_5.json /tmp/tables.json
 */

import { createRequire } from 'node:module';
import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const require = createRequire(import.meta.url);

const [pkgDir, version, outPath, tablesOutPath] = process.argv.slice(2);
if (!pkgDir || !version || !outPath) {
	console.error('usage: convert-hl7-dictionary.mjs <hl7-dictionary-pkg-dir> <version> <out.json> [tables.json]');
	process.exit(1);
}

const lib = (name) => {
	const m = require(resolve(pkgDir, 'lib', version, name));
	return m.default ?? m;
};

const SEGMENTS = lib('segments.js');
const MESSAGES = lib('messages.js');
const DATATYPES = lib('fields.js'); // despite the name: datatype definitions

// The dictionary's datatype records carry `table` on components only from
// v2.5 on (v2.3 has it on 4 composites, v2.1 on none), although the older
// standards bind the same components to the same tables: MSH-9's message
// code was table 0076 in v2.3 as much as in v2.5. A coded component that
// the requested version leaves without a table inherits it from the newest
// version that defines the same composite (CM_MSG was renamed MSG in
// 2.3.1) with a component of the same position, name and coded data type.
const INHERIT_FROM = ['2.7', '2.6', '2.5', '2.4', '2.3.1'].filter((v) => v !== version);
const COMPOSITE_ALIASES = { CM_MSG: 'MSG' };
// Components renamed between releases without changing meaning: MSG.1
// was "Message Type" through v2.3.1 and "Message Code" from v2.4.
const COMPONENT_NAME_ALIASES = { messagetype: 'messagecode' };
const CODED_TYPES = new Set(['ID', 'IS']);
const norm = (s) => {
	const n = String(s ?? '').toLowerCase().replace(/[^a-z0-9]/g, '');
	return COMPONENT_NAME_ALIASES[n] ?? n;
};
const laterDatatypes = INHERIT_FROM.map((v) => {
	try {
		const m = require(resolve(pkgDir, 'lib', v, 'fields.js'));
		return m.default ?? m;
	} catch {
		return null;
	}
}).filter(Boolean);
let inheritedTables = 0;
function inheritedTable(code, sub, position) {
	if (!CODED_TYPES.has(sub.datatype)) return undefined;
	const candidates = [code, COMPOSITE_ALIASES[code]].filter(Boolean);
	for (const dts of laterDatatypes) {
		for (const c of candidates) {
			const later = dts[c]?.subfields?.[position - 1];
			if (later?.table && later.datatype === sub.datatype && norm(later.desc) === norm(sub.desc)) {
				inheritedTables++;
				return later.table;
			}
		}
	}
	return undefined;
}

const isRequired = (opt) => opt === 2;
const isRepeating = (rep) => rep === 0 || rep > 1;

/** HL7 table ids are written zero-padded to four digits ("0001"). */
const tableId = (n) => String(n).padStart(4, '0');

/** The dictionary marks unbounded fields (OBX-5) with len 99999. */
const maxLength = (len) => (typeof len === 'number' && len > 0 && len < 99999 ? len : undefined);

/** Optional keys are left out entirely so the JSON stays compact. */
const withOptional = (base, extra) => {
	for (const [k, v] of Object.entries(extra)) if (v !== undefined) base[k] = v;
	return base;
};

// ---------- segments ----------
const segments = Object.entries(SEGMENTS).map(([code, def]) => ({
	code,
	name: def.desc ?? code,
	fields: (def.fields ?? []).map((f, i) => withOptional({
		position: i + 1,
		name: f.desc ?? `${code}-${i + 1}`,
		data_type: f.datatype ?? 'ST',
		required: isRequired(f.opt),
		repeats: isRepeating(f.rep),
	}, {
		max_length: maxLength(f.len),
		table: f.table ? tableId(f.table) : undefined,
	})),
}));
const segmentCodes = new Set(segments.map((s) => s.code));

// ---------- messages ----------
function convertElement(node) {
	const required = (node.min ?? 0) >= 1;
	const repeats = node.max === 0 || (node.max ?? 1) > 1;
	if (Array.isArray(node.children) && node.children.length > 0) {
		return {
			Group: {
				name: node.name,
				required,
				repeats,
				elements: node.children.map(convertElement),
			},
		};
	}
	// hl7-dictionary encodes an xsd:choice as one childless node whose name
	// is the comma-separated list of alternative segments
	// ("OBR,RQD,RQ1,RXO,ODS,ODT" in ORM_O01's ORDER_DETAIL).
	if (node.name.includes(',')) {
		return {
			Choice: {
				required,
				repeats,
				segments: node.name.split(',').map((s) => s.trim()),
			},
		};
	}
	return { Segment: { code: node.name, required, repeats } };
}

/** Collect every segment code referenced by a message tree. */
function referencedSegments(elements, out = new Set()) {
	for (const el of elements) {
		if (el.Segment) out.add(el.Segment.code);
		else if (el.Group) referencedSegments(el.Group.elements, out);
		else if (el.Choice) for (const c of el.Choice.segments) out.add(c);
	}
	return out;
}

const messages = [];
const skipped = [];
for (const [code, def] of Object.entries(MESSAGES)) {
	const roots = def.segments?.segments;
	if (!Array.isArray(roots) || roots.length === 0) {
		skipped.push(`${code}: no structure`);
		continue;
	}
	const elements = roots.map(convertElement);
	// A message referencing a segment the dictionary doesn't define would
	// fail the Rust importer's validation — skip it loudly instead.
	const missing = [...referencedSegments(elements)].filter((c) => !segmentCodes.has(c));
	if (missing.length > 0) {
		skipped.push(`${code}: undefined segments ${missing.join(',')}`);
		continue;
	}
	messages.push({
		code,
		event: code.includes('_') ? code.replace('_', '^') : code,
		description: def.desc ?? code,
		elements,
	});
}

// ---------- datatypes ----------
const composites = [];
const primitives = [];
for (const [code, def] of Object.entries(DATATYPES)) {
	const subs = def.subfields ?? [];
	if (subs.length > 0) {
		composites.push({
			code,
			components: subs.map((c, i) => {
				const table = c.table ?? inheritedTable(code, c, i + 1);
				return withOptional({
					position: i + 1,
					name: c.desc ?? `${code}.${i + 1}`,
					data_type: c.datatype ?? 'ST',
					required: isRequired(c.opt),
				}, {
					max_length: maxLength(c.len),
					table: table ? tableId(table) : undefined,
				});
			}),
		});
	} else {
		primitives.push({ code });
	}
}
const knownTypes = new Set([...composites.map((c) => c.code), ...primitives.map((p) => p.code)]);

// Any data type referenced by segments/composites but not defined in
// fields.js becomes a primitive fallback ("varies" and friends).
const referencedTypes = new Set();
for (const s of segments) for (const f of s.fields) referencedTypes.add(f.data_type);
for (const c of composites) for (const comp of c.components) referencedTypes.add(comp.data_type);
for (const t of referencedTypes) {
	if (!knownTypes.has(t)) {
		primitives.push({ code: t });
		knownTypes.add(t);
	}
}

const out = { messages, segments, composites, primitives };
writeFileSync(outPath, JSON.stringify(out, null, 1) + '\n');

console.log(`v${version}: ${messages.length} messages, ${segments.length} segments, ` +
	`${composites.length} composites, ${primitives.length} primitives -> ${outPath}` +
	(inheritedTables ? ` (${inheritedTables} component tables inherited from later versions)` : ''));
if (skipped.length) {
	console.log(`skipped ${skipped.length}:`);
	for (const s of skipped) console.log('  - ' + s);
}

// ---------- value tables ----------
// lib/tables.js is one set for every version (the dictionary does not
// version its tables), keyed by the unpadded number. Written sorted by id
// so regenerating the file produces a stable diff.
if (tablesOutPath) {
	const TABLES = (() => {
		const m = require(resolve(pkgDir, 'lib', 'tables.js'));
		return m.default ?? m;
	})();
	const tables = Object.entries(TABLES)
		.map(([id, def]) => ({
			id: tableId(id),
			name: def.desc ?? `Table ${tableId(id)}`,
			values: Object.entries(def.values ?? {}).map(([code, description]) => ({ code, description })),
		}))
		.filter((t) => t.values.length > 0)
		.sort((a, b) => a.id.localeCompare(b.id));
	writeFileSync(tablesOutPath, JSON.stringify({ tables }, null, 1) + '\n');
	const n = tables.reduce((acc, t) => acc + t.values.length, 0);
	console.log(`tables: ${tables.length} tables, ${n} values -> ${tablesOutPath}`);
}
