// Everything that needs the running application: the shell, the HL7 version
// catalogue, the FHIRPath engine, the FHIR rules builder and profile
// validation.

import {
	Report, newSession, ready, waitFor, js, sleep, paste, tools, closeModal,
	validate, fhirpath, PATIENT, BUNDLE, HL7_V23,
} from '../lib.mjs';

export async function appSuite() {
	const r = new Report('app');
	const d = await newSession();

	try {
		await ready(d);

		// ---- shell -------------------------------------------------------
		console.log('\n=== shell ===');
		await r.check('window loads', async () => `title=${await js(d, 'return document.title')}`);
		await r.check('menu bar renders', async () => {
			const n = await js(d, 'return document.querySelectorAll(".menu-trigger").length');
			if (n < 4) throw new Error(`only ${n} menus`);
			return `${n} menus`;
		});
		await r.check('first run shows the welcome screen with no tabs', async () => {
			// The welcome card renders only once startup finishes deciding
			// whether to restore a session, which is later than the menu bar
			// ready() waits for. Until then neither is on screen — correct
			// behaviour, but it means settling has to be waited for, not
			// sampled.
			const state = await waitFor(d, `
				const tabs = document.querySelectorAll('.tab').length;
				const welcome = !!document.querySelector('.welcome');
				return (tabs || welcome) ? { tabs, welcome } : null;
			`, 20000, 'startup to settle');
			// A restored session is legitimate too; only a tabless run must
			// show the welcome card.
			if (state.tabs === 0 && !state.welcome) throw new Error('no tabs and no welcome screen');
			return state.tabs === 0 ? 'welcome screen' : `session restored (${state.tabs} tabs)`;
		});

		// ---- HL7 v2 ------------------------------------------------------
		console.log('\n=== HL7 v2 ===');
		await r.check('paste creates a tab and Monaco mounts', async () => {
			await paste(d, HL7_V23);
			await waitFor(d, 'return !!document.querySelector(".monaco-editor")', 20000, 'Monaco');
			return 'mounted';
		});
		await r.check('HL7 v2 parsed into the tree', async () => {
			const n = await waitFor(d, `
				const rows = document.querySelectorAll('[class*="tree"] [class*="node"], .tree-node');
				return rows.length || 0;
			`, 15000, 'tree rows');
			return `${n} nodes`;
		});
		await r.check('HL7 validation produces a report', async () => {
			const txt = await validate(d);
			if (!/severity|error|warning|info/i.test(txt)) throw new Error(txt.slice(0, 120));
			return txt.slice(0, 70);
		});

		// ---- version catalogue -------------------------------------------
		console.log('\n=== HL7 version catalogue ===');
		await r.check('XSD export dialog opens', async () => {
			await tools(d, 'xsd');
			await waitFor(d, 'return !!document.querySelector(".xsd-controls select")', 10000, 'dialog');
			return 'open';
		});
		const versionLabels = () => js(d, `
			return [...document.querySelectorAll('.xsd-controls select')[0].options].map(o => o.textContent.trim());
		`);
		// "HL7 v2.7.1 (= v2.7) (PRO)" exports 2.7.1 — the alias note and the
		// tier badge are not versions on offer, so take the first token only.
		// Matching the label as a substring would let v2.7.1's alias note
		// stand in for a missing v2.7 entry.
		const versionOf = (label) => {
			const tok = label.split(/\s+/).find((t) => /^v\d/.test(t));
			return tok ? tok.slice(1) : '';
		};
		const WANT = ['2.1', '2.2', '2.3', '2.3.1', '2.4', '2.5', '2.5.1', '2.6', '2.7', '2.7.1'];
		await r.check('every shipped version is offered', async () => {
			const offered = (await versionLabels()).map(versionOf);
			const missing = WANT.filter((v) => !offered.includes(v));
			if (missing.length) throw new Error(`missing v${missing.join(', v')}; offered: ${offered.join(', ')}`);
			if (offered.length !== WANT.length) throw new Error(`expected ${WANT.length}, got ${offered.length}`);
			return `${offered.length} versions`;
		});
		await r.check('v2.7.1 is marked as an alias of v2.7', async () => {
			const hit = (await versionLabels()).find((o) => o.includes('2.7.1'));
			if (!hit || !hit.includes('= v2.7')) throw new Error(`label is "${hit}"`);
			return hit;
		});
		await r.check('the oldest catalogue exports', async () => {
			const i = (await versionLabels()).map(versionOf).indexOf('2.1');
			if (i < 0) throw new Error('no v2.1 option to select');
			await js(d, `
				const s = document.querySelectorAll('.xsd-controls select')[0];
				s.selectedIndex = ${i};
				s.dispatchEvent(new Event('change', { bubbles: true }));
			`);
			const n = await waitFor(d, `
				const m = document.querySelectorAll('.xsd-controls select')[1];
				return m && m.options.length > 1 ? m.options.length : 0;
			`, 15000, 'v2.1 messages');
			return `v2.1 lists ${n} messages`;
		});
		await closeModal(d);

		// ---- FHIRPath ----------------------------------------------------
		console.log('\n=== FHIRPath ===');
		await r.check('FHIR JSON detected on paste', async () => {
			await paste(d, PATIENT);
			return waitFor(d, `
				const e = [...document.querySelectorAll('*')].find(x =>
					x.children.length === 0 && /FHIR (JSON|XML)/.test(x.textContent));
				return e ? e.textContent.trim() : 0;
			`, 15000, 'format badge');
		});
		await r.check('FHIRPath panel opens', async () => {
			await tools(d, 'fhirpath');
			await waitFor(d, 'return !!document.querySelector("#fp-expr")', 10000, 'panel');
			return 'open';
		});

		const cases = [
			['navigation', 'Patient.name.family', (x) => x.values.length === 2],
			['three-valued logic', 'true and {}', (x) => x.empty],
			['operator precedence', '1 + 2 * 3', (x) => x.values[0] === '7'],
			['partial-precision dates', '@2015-02-04 = @2015-02', (x) => x.empty],
			['unit conversion', "4 'g' = 4000 'mg'", (x) => x.values[0] === 'true'],
			['duration arithmetic', 'Patient.birthDate + 18 years', (x) => /2008-05-15/.test(x.values[0] || '')],
			['descending sort', "('a' | 'c' | 'b').sort(-$this)", (x) => /"c"/.test(x.values[0] || '')],
			['string functions', "Patient.name.family.join(', ')", (x) => /Smith/.test(x.values[0] || '')],
			['where + count', "Patient.telecom.where(system = 'phone').count()", (x) => x.values[0] === '1'],
			['unknown function is named', 'Patient.nosuchfn()', (x) => /unknown function/i.test(x.error || '')],
		];
		for (const [label, expr, ok] of cases) {
			await r.check(`${label}: ${expr}`, async () => {
				const x = await fhirpath(d, expr);
				if (!ok(x)) throw new Error(`got ${JSON.stringify(x).slice(0, 140)}`);
				return x.error ? 'reported' : x.empty ? 'empty (correct)' : x.values.join(' | ').slice(0, 50);
			});
		}

		await r.check('trace() shows its captures', async () => {
			await fhirpath(d, "Patient.name.trace('names').count()");
			const t = await js(d, `
				const b = document.querySelector('.trace-block');
				return b ? b.textContent.replace(/\\s+/g,' ').trim().slice(0, 60) : '';
			`);
			if (!t) throw new Error('no trace block');
			return t;
		});

		await r.check('resolve() follows a Reference inside a Bundle', async () => {
			await paste(d, BUNDLE);
			const x = await fhirpath(d, 'Bundle.entry.resource.ofType(Observation).subject.resolve().id');
			if (x.error) throw new Error(x.error);
            if (!x.values.join('').includes('p2')) throw new Error(JSON.stringify(x).slice(0, 120));
			return x.values.join('');
		});

		await r.check('a choice element is reached by its base name', async () => {
			const x = await fhirpath(d, 'Bundle.entry.resource.ofType(Observation).value.unit');
			if (x.error) throw new Error(x.error);
			if (!x.values.join('').includes('mg')) throw new Error(JSON.stringify(x).slice(0, 120));
			return x.values.join('');
		});

		// ---- FHIR rules builder -------------------------------------------
		console.log('\n=== FHIR rules builder ===');
		await r.check('rules dialog opens', async () => {
			await tools(d, 'rules');
			await waitFor(d, 'return !!document.querySelector(".rules-layout")', 10000, 'rules dialog');
			return 'open';
		});
		await r.check('a preset creates a rule', async () => {
			await js(d, `
				const c = document.querySelector('.preset-chip');
				if (!c) throw new Error('no preset chips'); c.click();
			`);
			await sleep(500);
			const n = await js(d, 'return document.querySelectorAll(".rule-row").length');
			if (!n) throw new Error('no rule row appeared');
			return `${n} rule(s)`;
		});
		await r.check('the editor exposes both rule forms', async () => {
			const ok = await js(d, `
				return document.querySelectorAll('.mode-btn').length === 2 &&
					document.querySelectorAll('.rule-editor input').length > 0;
			`);
			if (!ok) throw new Error('rule editor incomplete');
			return 'expression / path+check';
		});
		await closeModal(d);

		// ---- profile validation --------------------------------------------
		console.log('\n=== FHIR profile validation ===');
		await r.check('package manager opens', async () => {
			await tools(d, 'profile');
			await waitFor(d, 'return !!document.querySelector(".modal")', 10000, 'dialog');
			return 'open';
		});

		const installed = await js(d, `
			const t = document.querySelector('.modal table.packages');
			return t ? t.textContent.replace(/\\s+/g,' ').trim() : '';
		`);
		await closeModal(d);

		if (!installed) {
			// No package installed: the honest behaviour is to say conformance
			// was not checked rather than report a clean result.
			await r.check('conformance is reported as NOT checked', async () => {
				await paste(d, JSON.stringify({
					resourceType: 'Patient', id: 'p1',
					meta: { profile: ['http://example.org/StructureDefinition/x'] },
				}, null, 1));
				const txt = await validate(d);
				if (!/not checked|no FHIR profile package/i.test(txt)) {
					throw new Error(`no such note: ${txt.slice(0, 150)}`);
				}
				return 'note present';
			});
			console.log('  (install a FHIR package to exercise conformance itself)');
		} else {
			await r.check('installed package is listed', () => installed.slice(0, 90));

			await r.check('a conforming resource produces no profile findings', async () => {
				await paste(d, JSON.stringify({
					resourceType: 'Patient', id: 'p1', active: true, gender: 'female',
					name: [{ family: 'Smith', given: ['Jane'] }],
				}, null, 1));
				const txt = await validate(d);
				if (/is not defined by|must be a|is required/.test(txt)) {
					throw new Error(`unexpected finding: ${txt.slice(0, 160)}`);
				}
				if (!/installed FHIR profile package/i.test(txt)) {
					throw new Error('report does not say profiles were applied');
				}
				return 'clean, and reported as checked';
			});

			const broken = [
				['misspelled element', { resourceType: 'Patient', id: 'p', genderr: 'female' }, /genderr.*not defined/],
				['wrong JSON type', { resourceType: 'Patient', id: 'p', active: 'yes' }, /must be a boolean/i],
				['choice element in plain form',
					{ resourceType: 'Observation', id: 'o', status: 'final', code: { text: 'x' }, value: { value: 1 } },
					/'value'.*not defined/],
				['required element missing', { resourceType: 'Observation', id: 'o' }, /is required/],
				['profile declared but not installed',
					{ resourceType: 'Patient', id: 'p', meta: { profile: ['http://example.org/nope'] } },
					/not installed/i],
			];
			for (const [label, doc, pattern] of broken) {
				await r.check(`caught: ${label}`, async () => {
					await paste(d, JSON.stringify(doc, null, 1));
					const txt = await validate(d);
					if (!pattern.test(txt)) throw new Error(txt.slice(0, 170));
					return 'reported';
				});
			}
		}
	} finally {
		await d.quit().catch(() => {});
	}
	return r;
}

if (import.meta.url === `file://${process.argv[1]}`) {
	const r = await appSuite();
	process.exit(r.finish() ? 0 : 1);
}
