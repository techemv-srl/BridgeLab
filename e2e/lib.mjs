// Shared helpers for the release E2E suites.
//
// Everything here talks to the real application through tauri-driver: the
// binary under test is the one the packages install, not a dev server.

import { Builder } from 'selenium-webdriver';

export const DRIVER_URL = process.env.BL_DRIVER_URL || 'http://127.0.0.1:4444';
export const APP_BINARY = process.env.BL_APP || '/usr/bin/bridgelab';

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Collects results so a failing check does not abort the whole suite. */
export class Report {
	constructor(suite) {
		this.suite = suite;
		this.results = [];
	}

	record(name, ok, detail = '') {
		this.results.push({ name, ok, detail });
		console.log(`  [${ok ? 'PASS' : 'FAIL'}] ${name}${detail ? ` — ${detail}` : ''}`);
	}

	async check(name, fn) {
		try {
			this.record(name, true, (await fn()) ?? '');
		} catch (e) {
			this.record(name, false, String(e.message || e).split('\n')[0].slice(0, 200));
		}
	}

	get failures() {
		return this.results.filter((r) => !r.ok);
	}

	finish() {
		const pass = this.results.length - this.failures.length;
		console.log(`\n  ${this.suite}: ${pass}/${this.results.length} passed`);
		for (const f of this.failures) console.log(`    - ${f.name}\n        ${f.detail}`);
		return this.failures.length === 0;
	}
}

export async function newSession() {
	return new Builder()
		.withCapabilities({
			'tauri:options': { application: APP_BINARY },
			browserName: 'wry',
		})
		.usingServer(DRIVER_URL)
		.build();
}

/** Run `script` in the webview until it returns something truthy. */
export async function waitFor(d, script, timeout = 20000, label = 'condition') {
	const end = Date.now() + timeout;
	while (Date.now() < end) {
		const v = await d.executeScript(script);
		if (v) return v;
		await sleep(250);
	}
	throw new Error(`timed out waiting for ${label}`);
}

export const js = (d, script) => d.executeScript(script);

/** Wait for the shell to be interactive. */
export const ready = (d) =>
	waitFor(d, 'return !!document.querySelector(".menu-trigger")', 30000, 'app shell');

/**
 * Load a document the way a user pastes one.
 *
 * AppShell hands the paste to Monaco when Monaco holds focus, so the focus is
 * dropped first — otherwise a synthetic event on `document` is (correctly)
 * ignored and the document silently stays as it was.
 */
export async function paste(d, text) {
	await js(d, `
		if (document.activeElement && document.activeElement.blur) document.activeElement.blur();
		document.body.focus();
	`);
	await js(d, `
		const dt = new DataTransfer();
		dt.setData('text/plain', ${JSON.stringify(text)});
		document.dispatchEvent(new ClipboardEvent('paste', {
			clipboardData: dt, bubbles: true, cancelable: true,
		}));
	`);
	await sleep(2200); // auto-parse debounce plus the IPC round trip
}

/** Click an item in the Tools menu by a fragment of its label. */
export async function tools(d, fragment) {
	await js(d, `
		const t = [...document.querySelectorAll('.menu-trigger')].find(x => /tool/i.test(x.textContent));
		if (!t) throw new Error('Tools menu not found');
		t.click();
	`);
	await sleep(350);
	await js(d, `
		const items = [...document.querySelectorAll('.menu-item')];
		const hit = items.find(i => i.textContent.toLowerCase().includes(${JSON.stringify(fragment.toLowerCase())}));
		if (!hit) throw new Error('no Tools item matching ' + ${JSON.stringify(fragment)} +
			'; available: ' + items.map(i => i.textContent.trim()).join(' / ').slice(0, 300));
		hit.click();
	`);
	await sleep(700);
}

export const closeModal = (d) =>
	js(d, `document.querySelector('.modal .close-btn, .modal .modal-close, .modal-header button')?.click();`)
		.catch(() => {})
		.then(() => sleep(400));

/** Validate the open document and return the panel's text. */
export async function validate(d) {
	await tools(d, 'validate');
	await sleep(2500);
	return waitFor(d, `
		const p = document.querySelector('[class*="validation"]');
		return p ? p.textContent.replace(/\\s+/g, ' ').trim() : 0;
	`, 15000, 'validation panel');
}

/** Evaluate one FHIRPath expression in the panel and read the result back. */
export async function fhirpath(d, expr) {
	await js(d, `
		const input = document.querySelector('#fp-expr');
		if (!input) throw new Error('FHIRPath panel is not open');
		const set = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
		set.call(input, ${JSON.stringify(expr)});
		input.dispatchEvent(new Event('input', { bubbles: true }));
	`);
	await sleep(150);
	await js(d, `
		const b = [...document.querySelectorAll('.fp-input-area button')].find(x => /eval/i.test(x.textContent));
		if (!b) throw new Error('Evaluate button not found');
		b.click();
	`);
	await sleep(900);
	return js(d, `
		const err = document.querySelector('.result-error');
		if (err) return { error: err.textContent.trim() };
		const count = document.querySelector('.result-count');
		return {
			count: count ? count.textContent.trim() : null,
			values: [...document.querySelectorAll('.result-value')].map(v => v.textContent.trim()),
			empty: !!document.querySelector('.result-empty'),
		};
	`);
}

// ---- fixtures ------------------------------------------------------------

export const PATIENT = JSON.stringify({
	resourceType: 'Patient', id: 'p1', active: true, gender: 'female',
	birthDate: '1990-05-15',
	name: [
		{ use: 'official', family: 'Smith', given: ['Jane', 'A'] },
		{ use: 'nickname', family: 'Doe', given: ['Jay'] },
	],
	telecom: [
		{ system: 'phone', value: '555-1234' },
		{ system: 'email', value: 'jane@example.com' },
	],
}, null, 1);

export const BUNDLE = JSON.stringify({
	resourceType: 'Bundle', type: 'collection',
	entry: [
		{ resource: { resourceType: 'Patient', id: 'p2', gender: 'male' } },
		{
			resource: {
				resourceType: 'Observation', id: 'o1', status: 'final',
				subject: { reference: 'Patient/p2' },
				valueQuantity: { value: 6.3, unit: 'mg', code: 'mg' },
			},
		},
	],
}, null, 1);

export const HL7_V23 =
	'MSH|^~\\&|SEND|FAC|RECV|FAC|20240101120000||ADT^A01|MSG1|P|2.3\r' +
	'EVN|A01|20240101120000\r' +
	'PID|1||12345^^^FAC^MR||Smith^Jane^A||19900515|F|||1 Main St^^Town^ST^12345\r' +
	'PV1|1|I|WARD^101^1||||1234^Who^Doctor';
