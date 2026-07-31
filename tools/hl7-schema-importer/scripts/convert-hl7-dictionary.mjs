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
 */

import { createRequire } from 'node:module';
import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const require = createRequire(import.meta.url);

const [pkgDir, version, outPath] = process.argv.slice(2);
if (!pkgDir || !version || !outPath) {
	console.error('usage: convert-hl7-dictionary.mjs <hl7-dictionary-pkg-dir> <version> <out.json>');
	process.exit(1);
}

const lib = (name) => {
	const m = require(resolve(pkgDir, 'lib', version, name));
	return m.default ?? m;
};

const SEGMENTS = lib('segments.js');
const MESSAGES = lib('messages.js');
const DATATYPES = lib('fields.js'); // despite the name: datatype definitions

const isRequired = (opt) => opt === 2;
const isRepeating = (rep) => rep === 0 || rep > 1;

// ---------- segments ----------
const segments = Object.entries(SEGMENTS).map(([code, def]) => ({
	code,
	name: def.desc ?? code,
	fields: (def.fields ?? []).map((f, i) => ({
		position: i + 1,
		name: f.desc ?? `${code}-${i + 1}`,
		data_type: f.datatype ?? 'ST',
		required: isRequired(f.opt),
		repeats: isRepeating(f.rep),
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
			components: subs.map((c, i) => ({
				position: i + 1,
				name: c.desc ?? `${code}.${i + 1}`,
				data_type: c.datatype ?? 'ST',
				required: isRequired(c.opt),
			})),
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
	`${composites.length} composites, ${primitives.length} primitives -> ${outPath}`);
if (skipped.length) {
	console.log(`skipped ${skipped.length}:`);
	for (const s of skipped) console.log('  - ' + s);
}
