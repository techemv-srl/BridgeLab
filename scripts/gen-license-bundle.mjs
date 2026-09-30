#!/usr/bin/env node
// Writes the licence files the packages carry:
//
//   src-tauri/LICENSE-bundle.txt     the root LICENSE (MIT, with the note on
//                                    the pro/ directories) followed by the
//                                    Business Source License 1.1 of
//                                    src-tauri/src/pro/ and of src/lib/pro/,
//                                    as they are. Every bundle installs it as
//                                    LICENSE.txt (bundle.resources).
//   src-tauri/LICENSE-installer.txt  the same words laid out for the
//                                    installers' licence page (NSIS, MSI;
//                                    bundle.licenseFile). That page wraps
//                                    lines itself in a proportional font, so
//                                    the 77-column hard wraps of the sources
//                                    broke every line twice: paragraphs are
//                                    one line each, the aligned "Key: value"
//                                    blocks become "Key: value", and rules of
//                                    = or - are short.
//
//   node scripts/gen-license-bundle.mjs          regenerate both files
//   node scripts/gen-license-bundle.mjs --check  exit 1 if either is out of date
//
// src-tauri/tests/license_bundle.rs checks the bundle against its sources
// and the installer text against the bundle, word for word.
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const SOURCES = ['LICENSE', 'src-tauri/src/pro/LICENSE', 'src/lib/pro/LICENSE'];
const SEPARATOR = '\n\n' + '='.repeat(77) + '\n\n';
const BUNDLE = 'src-tauri/LICENSE-bundle.txt';
const INSTALLER = 'src-tauri/LICENSE-installer.txt';

const bundle =
	SOURCES.map((p) => readFileSync(join(root, p), 'utf8').replace(/\r\n/g, '\n').trimEnd()).join(SEPARATOR) + '\n';

const RULE = /^\s*([=-])\1{4,}\s*$/;
const KEY = /^([A-Z][A-Za-z ]*):\s{2,}(\S.*)$/;

/** The bundle's words, laid out for a text box that wraps by itself. */
export function forInstaller(text) {
	const out = [];
	let para = [];
	const flush = () => {
		if (!para.length) return;
		if (para.every((l) => /^\s{2,}\S/.test(l))) {
			// An indented list (the pro/ directories): one entry per line.
			for (const l of para) out.push('  ' + l.trim());
		} else if (KEY.test(para[0])) {
			// "Key:      value" with indented continuation lines.
			let cur = null;
			for (const l of para) {
				const m = KEY.exec(l);
				if (m) {
					if (cur) out.push(cur);
					cur = `${m[1]}: ${m[2].trim()}`;
				} else {
					cur = cur ? `${cur} ${l.trim()}` : l.trim();
				}
			}
			if (cur) out.push(cur);
		} else {
			out.push(para.map((l) => l.trim()).join(' '));
		}
		out.push('');
		para = [];
	};
	for (const line of text.split('\n')) {
		const rule = RULE.exec(line);
		if (rule) {
			// A rule right under a line underlines it (a heading); otherwise
			// it separates two parts.
			const heading = para.length > 0;
			flush();
			if (heading) out.pop();
			out.push(rule[1].repeat(20), '');
		} else if (line.trim() === '') {
			flush();
		} else {
			para.push(line);
		}
	}
	flush();
	// Collapse the blank lines rules and paragraphs leave next to each other.
	return out.join('\n').replace(/\n{3,}/g, '\n\n').trim() + '\n';
}

const files = [
	[BUNDLE, bundle],
	[INSTALLER, forInstaller(bundle)],
];

if (process.argv.includes('--check')) {
	let stale = false;
	for (const [path, text] of files) {
		let current = '';
		try {
			// A Windows checkout with core.autocrlf writes CRLF; compare the text.
			current = readFileSync(join(root, path), 'utf8').replace(/\r\n/g, '\n');
		} catch {
			/* missing counts as out of date */
		}
		if (current !== text) {
			console.error(`${path} is out of date: run node scripts/gen-license-bundle.mjs`);
			stale = true;
		} else {
			console.log(`${path} is up to date`);
		}
	}
	if (stale) process.exit(1);
} else if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
	for (const [path, text] of files) {
		writeFileSync(join(root, path), text);
		console.log(`wrote ${path}`);
	}
}
