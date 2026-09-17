#!/usr/bin/env node
//
// Release acceptance run: drives the real, installed BridgeLab binary and
// checks the package it came from.
//
//   node e2e/run.mjs [path-to-.deb]
//
// It starts Xvfb and tauri-driver itself and shuts them down afterwards, so
// a release check is one command. Set BL_APP to test a binary somewhere
// other than /usr/bin/bridgelab, or BL_DISPLAY to use an existing display.
//
// Prerequisites: Xvfb, WebKitWebDriver, tauri-driver (cargo install tauri-driver).

import { spawn, execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

import { APP_BINARY, DRIVER_URL } from './lib.mjs';
import { packageSuite } from './suites/package.mjs';
import { appSuite } from './suites/app.mjs';

const DISPLAY = process.env.BL_DISPLAY || ':99';
// A release check is a fresh-install check: the app gets an empty profile
// (no session, no packages, a new 14-day trial — which is what lets the Pro
// features be exercised) unless the caller asks for their own.
const KEEP_PROFILE = process.env.BL_KEEP_PROFILE === '1';
const DEB = process.argv[2]
	|| 'src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/deb/BridgeLab_'
		+ JSON.parse(await import('node:fs/promises').then((fs) => fs.readFile('package.json', 'utf8'))).version
		+ '_amd64.deb';

const children = [];
let profileDir = null;

function have(cmd) {
	try { execFileSync('sh', ['-c', `command -v ${cmd}`], { stdio: 'ignore' }); return true; }
	catch { return false; }
}

function start(cmd, args, env = {}) {
	const p = spawn(cmd, args, { env: { ...process.env, ...env }, stdio: 'ignore', detached: false });
	children.push(p);
	return p;
}

function stopAll() {
	for (const p of children) { try { p.kill('SIGTERM'); } catch { /* already gone */ } }
}

async function waitForDriver(timeoutMs = 20000) {
	const end = Date.now() + timeoutMs;
	while (Date.now() < end) {
		try {
			const res = await fetch(`${DRIVER_URL}/status`);
			if (res.ok) return true;
		} catch { /* not up yet */ }
		await delay(300);
	}
	return false;
}

async function main() {
	for (const tool of ['Xvfb', 'WebKitWebDriver', 'tauri-driver']) {
		if (!have(tool)) {
			console.error(`missing prerequisite: ${tool}`);
			console.error('  Xvfb + WebKitWebDriver come from your distro; tauri-driver from `cargo install tauri-driver`.');
			process.exit(2);
		}
	}
	if (!existsSync(APP_BINARY)) {
		console.error(`no application at ${APP_BINARY} — install the package first, or set BL_APP.`);
		process.exit(2);
	}

	const profileEnv = {};
	if (!KEEP_PROFILE) {
		// dirs::config_dir / data_dir / cache_dir honour these on Linux, so
		// the app under test writes nowhere near the real profile. The cache
		// directory matters: it holds the trial's anti-reset marker, and
		// without redirecting it a machine whose trial has lapsed hands the
		// "fresh" profile an already-expired trial — by design.
		const home = mkdtempSync(join(tmpdir(), 'bridgelab-e2e-'));
		profileEnv.XDG_CONFIG_HOME = join(home, 'config');
		profileEnv.XDG_DATA_HOME = join(home, 'data');
		profileEnv.XDG_CACHE_HOME = join(home, 'cache');
		profileDir = home;
	}
	console.log(`BridgeLab release check\n  binary : ${APP_BINARY}\n  package: ${DEB}\n  display: ${DISPLAY}\n  profile: ${KEEP_PROFILE ? 'the current user\'s' : `fresh (${profileDir})`}`);

	// A display left behind by a killed run makes Xvfb refuse to start.
	const n = DISPLAY.replace(':', '');
	for (const stale of [`/tmp/.X${n}-lock`, `/tmp/.X11-unix/X${n}`]) {
		if (existsSync(stale)) rmSync(stale, { force: true });
	}

	start('Xvfb', [DISPLAY, '-screen', '0', '1600x1000x24', '-nolisten', 'tcp']);
	await delay(1500);
	start('tauri-driver', ['--port', '4444'], { DISPLAY, ...profileEnv });
	if (!await waitForDriver()) {
		console.error('tauri-driver did not come up on 4444');
		stopAll();
		process.exit(2);
	}

	const reports = [];
	reports.push(await packageSuite(DEB));
	reports.push(await appSuite());

	console.log('\n────────────────────────────────────────');
	let ok = true;
	for (const r of reports) ok = r.finish() && ok;
	const total = reports.reduce((a, r) => a + r.results.length, 0);
	const failed = reports.reduce((a, r) => a + r.failures.length, 0);
	console.log(`\n${total - failed}/${total} checks passed`);
	return ok;
}

let ok = false;
try {
	ok = await main();
} catch (e) {
	console.error('\nrun failed:', e.message || e);
} finally {
	stopAll();
	await delay(500);
	if (profileDir) rmSync(profileDir, { recursive: true, force: true });
}
process.exit(ok ? 0 : 1);
