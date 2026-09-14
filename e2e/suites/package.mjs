// Package-level checks. These need no display: they read the built artifacts
// and, when the package is installed, what it put on the system.
//
// Run as part of `node e2e/run.mjs`, or on its own with:
//   node e2e/suites/package.mjs <path-to-.deb>

import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { Report } from '../lib.mjs';

const run = (cmd, args) => execFileSync(cmd, args, { encoding: 'utf8' });

export async function packageSuite(debPath) {
	const r = new Report('package');
	console.log('\n=== package ===');

	if (!debPath || !existsSync(debPath)) {
		r.record('.deb present', false, `not found: ${debPath}`);
		return r;
	}

	let control = '';
	let contents = '';
	await r.check('.deb is readable', () => {
		control = run('dpkg', ['-I', debPath]);
		contents = run('dpkg', ['-c', debPath]);
		return `${(readFileSync(debPath).length / 1048576).toFixed(1)} MB`;
	});

	await r.check('version matches package.json', () => {
		const want = JSON.parse(readFileSync(new URL('../../package.json', import.meta.url))).version;
		const got = (control.match(/^\s*Version:\s*(\S+)/m) || [])[1];
		if (got !== want) throw new Error(`package.json says ${want}, .deb says ${got}`);
		return got;
	});

	await r.check('runtime dependencies declared exactly once', () => {
		const line = (control.match(/^\s*Depends:\s*(.+)$/m) || [])[1] || '';
		const deps = line.split(',').map((s) => s.trim()).filter(Boolean);
		const dupes = deps.filter((d, i) => deps.indexOf(d) !== i);
		if (dupes.length) throw new Error(`duplicated: ${[...new Set(dupes)].join(', ')}`);
		for (const need of ['libwebkit2gtk-4.1-0', 'libgtk-3-0']) {
			if (!deps.includes(need)) throw new Error(`missing ${need}; have: ${deps.join(', ')}`);
		}
		return deps.join(', ');
	});

	await r.check('section and priority set', () => {
		if (!/^\s*Section:\s*utils/m.test(control)) throw new Error('Section is not utils');
		if (!/^\s*Priority:\s*optional/m.test(control)) throw new Error('Priority is not optional');
		return 'utils / optional';
	});

	await r.check('binary, desktop entry and icons shipped', () => {
		for (const path of [
			'usr/bin/bridgelab',
			'usr/share/applications/BridgeLab.desktop',
			'usr/share/icons/hicolor/128x128/apps/bridgelab.png',
		]) {
			if (!contents.includes(path)) throw new Error(`missing ${path}`);
		}
		return 'present';
	});

	// The desktop entry claiming a MIME type is not enough on its own: without
	// a shared-mime-info definition nothing maps *.hl7 to that type, and
	// double-clicking a message never reaches the app.
	await r.check('MIME definition shipped for the .hl7 association', () => {
		if (!contents.includes('usr/share/mime/packages/')) {
			throw new Error('no /usr/share/mime/packages/ entry — .hl7 will not open BridgeLab');
		}
		return 'shared-mime-info definition present';
	});

	// ---- what installation actually put on this system ------------------
	const installed = existsSync('/usr/share/applications/BridgeLab.desktop');
	if (!installed) {
		console.log('  (skipping installed-system checks: package is not installed)');
		return r;
	}

	await r.check('desktop entry declares the MIME type', () => {
		const d = readFileSync('/usr/share/applications/BridgeLab.desktop', 'utf8');
		if (!/^MimeType=.*application\/hl7-v2/m.test(d)) throw new Error('MimeType missing');
		if (!/^Exec=bridgelab/m.test(d)) throw new Error('Exec is not bridgelab');
		return 'application/hl7-v2';
	});

	await r.check('*.hl7 resolves to application/hl7-v2', () => {
		if (!existsSync('/usr/share/mime/globs')) throw new Error('no MIME database on this system');
		const globs = readFileSync('/usr/share/mime/globs', 'utf8');
		if (!globs.includes('application/hl7-v2:*.hl7')) {
			throw new Error('glob not registered — did the shared-mime-info trigger run?');
		}
		return 'application/hl7-v2:*.hl7';
	});

	await r.check('content detection recognises a message without an extension', () => {
		if (!existsSync('/usr/share/mime/magic')) throw new Error('no magic database');
		const magic = readFileSync('/usr/share/mime/magic');
		if (!magic.includes('hl7-v2') || !magic.includes('MSH|')) {
			throw new Error('MSH| magic rule not registered');
		}
		return 'MSH| magic registered';
	});

	return r;
}

if (import.meta.url === `file://${process.argv[1]}`) {
	const r = await packageSuite(process.argv[2]);
	process.exit(r.finish() ? 0 : 1);
}
