// Everything that needs the running application: the shell, the HL7 version
// catalogue, the FHIRPath engine, the FHIR rules builder and profile
// validation.

import { readFileSync, rmSync, writeFileSync } from 'node:fs';
import { Key } from 'selenium-webdriver';
import { tmpdir } from 'node:os';
import {
	Report, newSession, ready, waitFor, js, sleep, paste, tools, closeModal,
	validate, fhirpath, PATIENT, BUNDLE, HL7_V23,
} from '../lib.mjs';

export async function appSuite() {
	const r = new Report('app');
	const d = await newSession();

	/**
	 * Answer the IPC requests for `commands` (`plugin:dialog|save`...) in
	 * the current window's page, recording their arguments, instead of
	 * letting them reach the backend: a native dialog or the system's
	 * browser is out of WebDriver's reach. The app's IPC goes through
	 * window.fetch. `ipcCalls` reads and clears what was recorded;
	 * `unstubIpc` puts the real fetch back.
	 */
	const stubIpc = (commands) => js(d, `
		const wanted = ${JSON.stringify(commands)};
		const stub = window.__blStub ?? (window.__blStub = { real: window.fetch });
		stub.calls = [];
		window.fetch = function (input, init) {
			const url = decodeURIComponent(typeof input === 'string' ? input : input.url);
			const m = /^(?:ipc:\\/\\/localhost|https?:\\/\\/ipc\\.localhost)\\/(.+)$/.exec(url);
			if (m && wanted.includes(m[1])) {
				let body = init && init.body;
				if (body && typeof body !== 'string') body = new TextDecoder().decode(body);
				stub.calls.push({ cmd: m[1], args: body ? JSON.parse(body) : null });
				return Promise.resolve(new Response('null', { status: 200, headers: { 'Content-Type': 'application/json', 'Tauri-Response': 'ok' } }));
			}
			return stub.real.apply(this, arguments);
		};`);
	const ipcCalls = () => js(d, `const c = window.__blStub?.calls ?? []; if (window.__blStub) window.__blStub.calls = []; return c;`);
	const unstubIpc = () => js(d, `if (window.__blStub) { window.fetch = window.__blStub.real; delete window.__blStub; }`);
	/** In the page: fire a cancelable `contextmenu` at `el`; true when
	 *  something cancelled it (the webview's own menu then stays shut). */
	const MENU_CANCELLED = `const menuCancelled = (el) => !el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2 }));`;

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
		await r.check('a sample message opens in a new tab, parsed', async () => {
			// A restored session (BL_KEEP_PROFILE=1) has no welcome screen;
			// the File menu offers the same dialog.
			const via = await js(d, `
				const b = [...document.querySelectorAll('.welcome-action')].find((x) => x.querySelector('.wa-label')?.textContent === 'Sample messages');
				if (b) { b.click(); return 'welcome screen'; }
				return null;`) ?? await (async () => {
				await js(d, `[...document.querySelectorAll('.menu-trigger')].find((x) => /file/i.test(x.textContent))?.click()`);
				await sleep(350);
				const hit = await js(d, `
					const i = [...document.querySelectorAll('.menu-item')].find((x) => x.textContent.includes('Sample Messages'));
					if (!i) return false; i.click(); return true;`);
				if (!hit) throw new Error('no "Sample messages" on the welcome screen or in the File menu');
				return 'File menu';
			})();
			await waitFor(d, 'return document.querySelectorAll(".smp-item").length >= 12', 10000, 'sample list');
			await js(d, `
				const it = [...document.querySelectorAll('.smp-item')].find((x) => x.textContent.includes('complete blood count'));
				it.click();`);
			await js(d, `[...document.querySelectorAll('.smp-footer .btn-primary')][0].click()`);
			const label = await waitFor(d, `
				const t = [...document.querySelectorAll('.tab')].map((x) => x.textContent).find((x) => x.includes('ORU^R01 v2.5.1'));
				return t && document.querySelectorAll('.tree-node, [data-node-id]').length > 0 ? t.trim() : null;`, 15000, 'sample tab parsed');
			return `${via}: ${label}`;
		});
		// Close the manual the way its title-bar button does: a close request
		// to the window, so its "destroyed" event reaches the main window.
		// WebDriver's own close only drops the page; the window stays known to
		// the app, a state no user can reach, and F1 then sees it still open.
		const closeManual = async (main) => {
			await d.switchTo().window(main);
			await js(d, `return window.__TAURI_INTERNALS__.invoke('plugin:window|close', { label: 'bridgelab-help' }).catch(() => null)`);
			for (const end = Date.now() + 10000; Date.now() < end;) {
				if ((await d.getAllWindowHandles()).length === 1) return;
				await sleep(300);
			}
			throw new Error('the manual window did not close');
		};
		await r.check('F1 opens the manual in its own window, with content', async () => {
			// The window used to load a blob: URL made by the main webview,
			// which WebKitGTK cannot load from a second webview: it opened
			// blank. It now loads the app's /manual page.
			const main = await d.getWindowHandle();
			await js(d, `window.dispatchEvent(new KeyboardEvent('keydown', { key: 'F1', bubbles: true }))`);
			const end = Date.now() + 15000;
			let found = null;
			try {
				while (!found && Date.now() < end) {
					await sleep(500);
					for (const h of await d.getAllWindowHandles()) {
						if (h === main) continue;
						await d.switchTo().window(h);
						const info = await js(d, `return {
							sections: document.querySelectorAll('main.content section').length,
							toc: document.querySelectorAll('aside.toc a[href^="#"]').length,
							title: document.title,
						}`);
						if (info.sections > 5) {
							found = info;
							await closeManual(main);
							break;
						}
					}
					await d.switchTo().window(main);
				}
			} finally {
				// Back to the main window whatever happened, so one failure
				// here does not fail every later check with NoSuchWindowError.
				await d.switchTo().window(main).catch(() => {});
			}
			if (!found) throw new Error('no window with manual content within 15 s');
			return `${found.title}: ${found.sections} sections, ${found.toc} contents links`;
		});
		await r.check("the webview's own context menu never opens, text fields included", async () => {
			// WebKitGTK's menu (Back, Forward, Stop, Reload) opened on the
			// status bar, the panels, the menu bar, the banners and Settings;
			// its Reload dropped every unsaved tab with session restore off.
			const got = await js(d, `${MENU_CANCELLED}
				const out = {};
				for (const [name, sel] of [['status bar', '.status-bar'], ['menu bar', '.menu-trigger'], ['tab strip', '.tab'], ['page', 'body']]) {
					const el = document.querySelector(sel);
					out[name] = el ? menuCancelled(el) : 'missing';
				}
				// A text field too: WebView2's menu there has Emoji, password
				// import and "More tools". Cut, Copy and Paste keep their keys.
				const field = document.createElement('input');
				field.type = 'text';
				document.body.appendChild(field);
				out['text field'] = menuCancelled(field);
				field.remove();
				return out;`);
			const wrong = Object.entries(got).filter(([, v]) => v !== true);
			if (wrong.length) throw new Error(`cancelled: ${JSON.stringify(got)}`);
			return Object.entries(got).map(([k, v]) => `${k} ${v ? 'cancelled' : 'native'}`).join(', ');
		});
		await r.check('in the manual window, the e-mail link goes to the mail client and the native menu stays shut', async () => {
			// A middle click there loaded mailto: in the manual window itself:
			// "The URL can't be shown", and the manual was gone.
			const main = await d.getWindowHandle();
			await js(d, `window.dispatchEvent(new KeyboardEvent('keydown', { key: 'F1', bubbles: true }))`);
			let manual = null;
			for (const end = Date.now() + 15000; !manual && Date.now() < end;) {
				await sleep(500);
				for (const h of await d.getAllWindowHandles()) {
					if (h === main) continue;
					await d.switchTo().window(h);
					if (await js(d, `return document.querySelectorAll('main.content section').length > 5`)) { manual = h; break; }
				}
				if (!manual) await d.switchTo().window(main);
			}
			if (!manual) throw new Error('no manual window within 15 s');
			try {
				await stubIpc(['plugin:opener|open_url']);
				const got = await js(d, `${MENU_CANCELLED}
					const a = document.querySelector('a[href^="mailto:"]');
					if (!a) return null;
					const middle = !a.dispatchEvent(new MouseEvent('auxclick', { bubbles: true, cancelable: true, button: 1 }));
					return { middle, menu: menuCancelled(document.body), href: location.href };`);
				if (!got) throw new Error('no e-mail link in the manual');
				const calls = await waitFor(d, `return window.__blStub.calls.length ? window.__blStub.calls : null`, 5000, 'the opener');
				if (!got.middle || calls[0].args?.url !== 'mailto:info@techemv.it') throw new Error(`middle click cancelled ${got.middle}, opener ${JSON.stringify(calls)}`);
				if (!got.menu) throw new Error('the native menu was not cancelled');
				if (!/\/manual/.test(got.href)) throw new Error(`the manual window is at ${got.href}`);
				return `opener got ${calls[0].args.url}; menu cancelled; still at the manual`;
			} finally {
				await closeManual(main);
			}
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
		await r.check('coded values are explained inline in the tree', async () => {
			// Expanding a segment loads its fields from the backend, which
			// attaches each coded value's meaning from its HL7 table. One
			// segment at a time, as a user would: the expansion is async and
			// the row list is rebuilt when it lands.
			const expand = (code) => js(d, `
				for (const row of document.querySelectorAll('.tree-node.segment')) {
					const label = row.querySelector('.label')?.textContent.trim() ?? '';
					if (label.startsWith('${code} ')) { row.click(); return true; }
				}
				return false;
			`);
			const descs = () => js(d, `
				return [...document.querySelectorAll('.tree-node .code-desc')].map((e) => e.textContent.trim());
			`);
			for (const [code, want] of [['PID', '— Female'], ['PV1', '— Inpatient']]) {
				if (!await expand(code)) throw new Error(`no ${code} row to expand`);
				await waitFor(d, `
					return [...document.querySelectorAll('.tree-node .code-desc')].some((e) => e.textContent.trim() === '${want}') || null;
				`, 15000, `${code} code descriptions`);
			}
			return (await descs()).join(', ');
		});
		await r.check('HL7 validation produces a report', async () => {
			const txt = await validate(d);
			if (!/severity|error|warning|info/i.test(txt)) throw new Error(txt.slice(0, 120));
			return txt.slice(0, 70);
		});
		await r.check('a Latin-1 file opens with its accents intact', async () => {
			// ISO-8859-1 with MSH-18 8859/1: refused before 1.9.0 ("stream did
			// not contain valid UTF-8"); now decoded in the declared charset.
			const path = `${process.cwd()}/tests/fixtures/hl7/adt_a01_latin1.hl7`;
			const res = await d.executeAsyncScript(`
				const done = arguments[arguments.length - 1];
				window.__TAURI_INTERNALS__.invoke('open_file', { path: arguments[0] })
					.then((r) => done({ ok: true, text: r.truncated_text, type: r.message_type }))
					.catch((e) => done({ ok: false, error: String(e) }));
			`, path);
			if (!res.ok) throw new Error(res.error);
			if (!res.text.includes('Müller^Jörg') || !res.text.includes('Lettò 2')) throw new Error(res.text.slice(0, 160));
			// Save writes it back in the charset MSH-18 declares.
			const out = `${tmpdir()}/bl-e2e-latin1-${process.pid}.hl7`;
			const saved = await d.executeAsyncScript(`
				const done = arguments[arguments.length - 1];
				window.__TAURI_INTERNALS__.invoke('save_file', { messageId: null, path: arguments[0], content: arguments[1] })
					.then(() => done({ ok: true })).catch((e) => done({ ok: false, error: String(e) }));
			`, out, res.text);
			if (!saved.ok) throw new Error(saved.error);
			const bytes = readFileSync(out);
			rmSync(out, { force: true });
			if (!bytes.includes(Buffer.from('M\xfcller', 'latin1'))) throw new Error('saved file is not ISO-8859-1');
			return `${res.type}: Müller^Jörg, Lettò 2; saved back as ISO-8859-1`;
		});
		await r.check('a greyed standard segment is inserted at its standard position', async () => {
			// Right-click a ghost row > Insert: the skeleton used to land on
			// line 1, above MSH, breaking the message.
			await paste(d, [
				'MSH|^~\\&|LAB|FAC|EHR|FAC|20240101120000||ORU^R01|MSG3|P|2.5',
				'PID|1||12345^^^FAC^MR||Smith^Jane||19900515|F',
				'OBR|1||ORD1|CBC^Blood count',
				'OBX|1|NM|WBC^Leukocytes||6.1|10*9/L|4.0-10.0|N|||F',
			].join('\r'));
			const viewMenu = (label) => js(d, `
				const t = [...document.querySelectorAll('.menu-trigger')].find((x) => /view/i.test(x.textContent));
				t.click();
				return new Promise((res) => setTimeout(() => {
					const i = [...document.querySelectorAll('.menu-item')].find((x) => x.textContent.includes(${JSON.stringify('Show Schema Fields')}));
					if (i) i.click();
					res(!!i);
				}, 300));`);
			if (!await viewMenu()) throw new Error('no "Show Schema Fields" in the View menu');
			try {
				await waitFor(d, `return [...document.querySelectorAll('.tree-node.placeholder .label')].some((l) => l.textContent.trim().startsWith('PV1')) || null`, 10000, 'ghost PV1 row');
				await js(d, `
					const row = [...document.querySelectorAll('.tree-node.placeholder')].find((r) => r.querySelector('.label')?.textContent.trim().startsWith('PV1'));
					row.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 50, clientY: 50 }));`);
				await sleep(300);
				const clicked = await js(d, `
					const b = [...document.querySelectorAll('.context-menu-item')].find((x) => x.textContent.includes('Insert PV1'));
					if (!b) return false; b.click(); return true;`);
				if (!clicked) throw new Error('no "Insert PV1 segment" in the context menu');
				const lines = await waitFor(d, `
					const t = [...document.querySelectorAll('.monaco-editor .view-line')].map((l) => l.textContent.replace(/\u00a0/g, ' ').trim()).filter(Boolean);
					return t.some((l) => l.startsWith('PV1')) ? t : null;`, 10000, 'PV1 line in the editor');
				const order = lines.map((l) => l.slice(0, 3));
				if (order[0] !== 'MSH') throw new Error(`first line is ${order[0]}: ${order.join(' ')}`);
				if (!(order.indexOf('PV1') > order.indexOf('PID') && order.indexOf('PV1') < order.indexOf('OBR'))) throw new Error(order.join(' '));
				return order.join(' ');
			} finally {
				await viewMenu();
			}
		});

		await r.check('a paste into a text field leaves the open message alone', async () => {
			// The window-level paste fallback used to replace the whole message
			// with whatever was pasted into a search box or the licence field.
			await paste(d, HL7_V23);
			const before = await js(d, `return document.querySelectorAll('.tab').length`);
			await js(d, `
				const i = document.createElement('input');
				i.id = 'bl-e2e-input';
				document.body.appendChild(i);
				i.focus();
				const dt = new DataTransfer();
				dt.setData('text/plain', 'Smith');
				i.dispatchEvent(new ClipboardEvent('paste', { clipboardData: dt, bubbles: true, cancelable: true }));`);
			await sleep(800);
			const text = await js(d, `return [...document.querySelectorAll('.monaco-editor .view-line')].map((l) => l.textContent).join('\\n')`);
			const after = await js(d, `document.getElementById('bl-e2e-input')?.remove(); return document.querySelectorAll('.tab').length`);
			if (!text.includes('MSH|')) throw new Error(`message replaced: ${text.slice(0, 80)}`);
			if (after !== before) throw new Error(`tabs ${before} -> ${after}`);
			return 'message intact';
		});
		await r.check('each tab keeps its own undo history across tab switches', async () => {
			// Switching tabs used to replace the editor text, which wiped the
			// undo history: Ctrl+Z after coming back did nothing.
			const text = () => js(d, `return [...document.querySelectorAll('.monaco-editor .view-line')].map((l) => l.textContent).join('\\n')`);
			// Key.chord inside sendKeys leaves Ctrl held on WebKitWebDriver, so
			// the text after it arrived as shortcuts: press and release it.
			const focus = () => js(d, `document.querySelector('.monaco-editor textarea')?.focus()`);
			const ctrl = async (key) => {
				await focus();
				await d.actions().keyDown(Key.CONTROL).sendKeys(key).keyUp(Key.CONTROL).perform();
				await sleep(600);
			};
			const type = async (s) => {
				await focus();
				await d.actions().sendKeys(s).perform();
				await sleep(600);
			};
			await paste(d, 'MSH|^~\\&|A|B|C|D|20240101||ADT^A01|UNDOA|P|2.5\rPID|1||U1');
			const tabA = (await js(d, 'return document.querySelectorAll(".tab").length')) - 1;
			await ctrl(Key.END);
			await type('ZZTYPED');
			if (!(await text()).includes('ZZTYPED')) throw new Error('typing did not reach the editor');
			await paste(d, 'MSH|^~\\&|A|B|C|D|20240101||ADT^A01|UNDOB|P|2.5\rPID|1||U2');
			await ctrl(Key.END);
			await type('QQOTHER');
			await js(d, `document.querySelectorAll('.tab')[${tabA}].click()`);
			await waitFor(d, `return [...document.querySelectorAll('.monaco-editor .view-line')].some((l) => l.textContent.includes('UNDOA')) || null`, 10000, 'tab A shown');
			await ctrl('z');
			const undone = await text();
			if (undone.includes('ZZTYPED')) throw new Error('Ctrl+Z after switching back did not undo the typing');
			if (!undone.includes('UNDOA')) throw new Error('undo went past the pasted message');
			await ctrl('y');
			if (!(await text()).includes('ZZTYPED')) throw new Error('Ctrl+Y did not redo');
			return 'undo and redo kept per tab';
		});
		await r.check('closing a tab with unsaved changes asks first', async () => {
			await paste(d, HL7_V23);
			const before = await js(d, `return document.querySelectorAll('.tab').length`);
			await js(d, `window.dispatchEvent(new KeyboardEvent('keydown', { key: 'w', ctrlKey: true, bubbles: true }))`);
			await waitFor(d, `return document.querySelector('.dialog[role="alertdialog"]') ? true : null`, 5000, 'confirmation dialog');
			await js(d, `[...document.querySelectorAll('.dialog-footer .btn')].find((b) => !b.classList.contains('btn-primary'))?.click()`);
			await sleep(400);
			const after = await js(d, `return document.querySelectorAll('.tab').length`);
			if (after !== before) throw new Error(`tab closed without confirmation: ${before} -> ${after}`);
			return 'kept after Cancel';
		});

		await r.check('app shortcuts work with the editor focused (Ctrl+L)', async () => {
			// Monaco binds Ctrl+L itself and used to swallow it.
			await paste(d, HL7_V23);
			await js(d, `
				const ta = document.querySelector('.monaco-editor textarea');
				ta.focus();
				ta.dispatchEvent(new KeyboardEvent('keydown', { key: 'l', code: 'KeyL', keyCode: 76, ctrlKey: true, bubbles: true, cancelable: true }));`);
			await waitFor(d, `return document.querySelector('.modal') ? true : null`, 5000, 'Test Case Library');
			await js(d, `window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }))`);
			await waitFor(d, `return document.querySelector('.modal') ? null : true`, 5000, 'library closed');
			return 'library opened from the editor';
		});
		await r.check('Escape closes About and Settings', async () => {
			for (const open of ['about', 'settings']) {
				await js(d, open === 'about'
					? `[...document.querySelectorAll('.menu-trigger')].find((x) => /help/i.test(x.textContent))?.click()`
					: `window.dispatchEvent(new KeyboardEvent('keydown', { key: ',', ctrlKey: true, bubbles: true }))`);
				await sleep(350);
				if (open === 'about') await js(d, `[...document.querySelectorAll('.menu-item')].find((x) => /about/i.test(x.textContent))?.click()`);
				await waitFor(d, `return document.querySelector('.modal') ? true : null`, 5000, `${open} dialog`);
				await js(d, `window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }))`);
				await waitFor(d, `return document.querySelector('.modal') ? null : true`, 5000, `${open} closed`);
			}
			return 'both closed';
		});
		await r.check('a middle click on the e-mail link in About opens the mail client and leaves the window alone', async () => {
			// Only a left click was taken: a middle click loaded mailto: in
			// the app's own window, which then showed "The URL can't be shown".
			await stubIpc(['plugin:opener|open_url']);
			try {
				await js(d, `[...document.querySelectorAll('.menu-trigger')].find((x) => /help/i.test(x.textContent))?.click()`);
				await sleep(350);
				await js(d, `[...document.querySelectorAll('.menu-item')].find((x) => /about/i.test(x.textContent))?.click()`);
				await waitFor(d, `return document.querySelector('.modal a[href^="mailto:"]') ? true : null`, 5000, 'About with its e-mail link');
				const cancelled = await js(d, `
					const a = document.querySelector('.modal a[href^="mailto:"]');
					return !a.dispatchEvent(new MouseEvent('auxclick', { bubbles: true, cancelable: true, button: 1 }));`);
				const calls = await waitFor(d, `return window.__blStub.calls.length ? window.__blStub.calls : null`, 5000, 'the opener');
				await sleep(500);
				const still = await js(d, `return { href: location.href, shell: !!document.querySelector('.menu-trigger') }`);
				if (!cancelled) throw new Error('the middle click was left to the webview');
				if (calls[0].args?.url !== 'mailto:info@techemv.it') throw new Error(`opener: ${JSON.stringify(calls)}`);
				if (!still.shell || /^mailto:|^about:/.test(still.href)) throw new Error(`window: ${JSON.stringify(still)}`);
				return `opener got ${calls[0].args.url}; window still at ${still.href}`;
			} finally {
				await unstubIpc();
				await js(d, `window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }))`);
				await waitFor(d, `return document.querySelector('.modal') ? null : true`, 5000, 'About closed').catch(() => {});
			}
		});
		await r.check('editor settings for suggestions, occurrences, links and sticky scroll are saved', async () => {
			const openSettings = async () => {
				await js(d, `window.dispatchEvent(new KeyboardEvent('keydown', { key: ',', ctrlKey: true, bubbles: true }))`);
				await waitFor(d, `return document.getElementById('s-wordsugg') ? true : null`, 5000, 'Settings → Editor');
				await sleep(800); // let it load the saved values before changing them
			};
			const read = () => js(d, `
				const box = (t) => [...document.querySelectorAll('.setting-check label')].find((l) => l.textContent.includes(t))?.querySelector('input')?.checked;
				return { sugg: document.getElementById('s-wordsugg').value, occ: box('occurrences'), links: box('Clickable links'), sticky: box('Sticky scroll') };`);
			const set = (want) => js(d, `
				const want = ${JSON.stringify(want)};
				const box = (t) => [...document.querySelectorAll('.setting-check label')].find((l) => l.textContent.includes(t))?.querySelector('input');
				for (const [t, v] of [['occurrences', want.occ], ['Clickable links', want.links], ['Sticky scroll', want.sticky]]) {
					const b = box(t); if (b && b.checked !== v) b.click();
				}
				const sel = document.getElementById('s-wordsugg'); sel.value = want.sugg; sel.dispatchEvent(new Event('change', { bubbles: true }));
				document.querySelector('.modal .btn-primary').click();`);
			await openSettings();
			const before = await read();
			if (before.sugg !== 'currentDocument' || before.occ !== true || before.links !== true || before.sticky !== false) {
				throw new Error(`unexpected defaults: ${JSON.stringify(before)}`);
			}
			const changed = { sugg: 'off', occ: false, links: false, sticky: true };
			await set(changed);
			await waitFor(d, `return document.getElementById('s-wordsugg') ? null : true`, 5000, 'Settings closed');
			const stored = await js(d, `return Promise.all(['editor_word_suggestions', 'editor_occurrences', 'editor_links', 'editor_sticky_scroll'].map((key) => window.__TAURI_INTERNALS__.invoke('get_preference', { key })))`);
			if (JSON.stringify(stored) !== JSON.stringify(['off', 'false', 'false', 'true'])) throw new Error(`stored: ${JSON.stringify(stored)}`);
			await openSettings();
			// The dialog loads the saved values after it opens.
			const want = JSON.stringify(changed);
			const after = await waitFor(d, `
				const box = (t) => [...document.querySelectorAll('.setting-check label')].find((l) => l.textContent.includes(t))?.querySelector('input')?.checked;
				const v = JSON.stringify({ sugg: document.getElementById('s-wordsugg').value, occ: box('occurrences'), links: box('Clickable links'), sticky: box('Sticky scroll') });
				return v === ${JSON.stringify(want)} ? v : null;`, 5000, 'saved values shown').catch(async () => JSON.stringify(await read()));
			if (after !== want) throw new Error(`reopened: ${after}`);
			// Back to the defaults, for the checks that follow.
			await set(before);
			await waitFor(d, `return document.getElementById('s-wordsugg') ? null : true`, 5000, 'Settings closed');
			return 'saved, reloaded and restored';
		});
		await r.check('native controls follow the theme (color-scheme)', async () => {
			// Without it, WebKitGTK drew every <select> as a light widget
			// under the dark theme's light text: the values were unreadable.
			const state = () => js(d, `return {
				theme: document.documentElement.getAttribute('data-theme') || 'dark',
				scheme: getComputedStyle(document.documentElement).colorScheme,
			}`);
			const pick = async (label) => {
				await js(d, `[...document.querySelectorAll('.menu-trigger')].find((x) => /view/i.test(x.textContent))?.click()`);
				await sleep(350);
				await js(d, `[...document.querySelectorAll('.menu-item')].find((x) => x.textContent.trim() === ${JSON.stringify(label)})?.click()`);
				await sleep(300);
				return state();
			};
			const start = await state();
			const light = await pick('Light');
			const dark = await pick('Dark');
			if (start.theme === 'light') await pick('Light');
			const bad = [start, light, dark].filter((s) => s.scheme !== s.theme);
			if (light.theme !== 'light' || dark.theme !== 'dark') throw new Error(`theme switch: ${JSON.stringify([light, dark])}`);
			if (bad.length) throw new Error(`color-scheme does not follow: ${JSON.stringify(bad)}`);
			return `${start.theme} start; light → ${light.scheme}, dark → ${dark.scheme}`;
		});
		await r.check('resetting a shortcut whose key moved elsewhere asks, then moves it back', async () => {
			// The reset next to an entry used to write its default key even
			// when another action had it: both showed Ctrl+O, and Ctrl+O only
			// ever opened the file dialog.
			await js(d, `window.dispatchEvent(new KeyboardEvent('keydown', { key: ',', ctrlKey: true, bubbles: true }))`);
			await waitFor(d, `return document.querySelector('.settings-nav') ? true : null`, 5000, 'Settings');
			try {
				await js(d, `[...document.querySelectorAll('.settings-nav .nav-item')].find((b) => /shortcut/i.test(b.textContent))?.click()`);
				await waitFor(d, `return document.querySelectorAll('.shortcut-row').length > 10 ? true : null`, 5000, 'Settings → Keyboard Shortcuts');
				await sleep(500); // the editor loads the saved bindings when it opens
				const row = (label) => `[...document.querySelectorAll('.shortcut-row')].find((r) => r.querySelector('.shortcut-label')?.textContent.trim() === ${JSON.stringify(label)})`;
				const keysOf = (label) => js(d, `return ${row(label)}?.querySelector('.binding, .capture-keys')?.textContent.trim() ?? null`);
				const before = { open: await keysOf('Open File'), validate: await keysOf('Validate') };
				if (before.open !== 'Ctrl+O' || before.validate !== 'F6') throw new Error(`not the defaults: ${JSON.stringify(before)}`);
				// Give Ctrl+O to Validate, with Reassign.
				await js(d, `${row('Validate')}.querySelector('.binding').click()`);
				await js(d, `(document.activeElement || document.body).dispatchEvent(new KeyboardEvent('keydown', { key: 'o', ctrlKey: true, bubbles: true, cancelable: true }))`);
				await waitFor(d, `return ${row('Validate')}?.querySelector('.conflict')?.textContent.includes('Open File') ? true : null`, 5000, 'conflict warning');
				await js(d, `${row('Validate')}.querySelector('.capture-area .btn-primary').click()`);
				await sleep(300);
				const moved = { open: await keysOf('Open File'), validate: await keysOf('Validate') };
				if (moved.validate !== 'Ctrl+O' || moved.open !== '(none)') throw new Error(`after Reassign: ${JSON.stringify(moved)}`);
				// The reset next to Open File asks first: its key is Validate's now.
				await js(d, `${row('Open File')}.querySelector('.reset-btn').click()`);
				const warning = await waitFor(d, `return ${row('Open File')}?.querySelector('.capture-area .conflict')?.textContent.trim() ?? null`, 5000, 'reset asks');
				if (!warning.includes('Validate')) throw new Error(`warning: ${warning}`);
				if (await keysOf('Validate') !== 'Ctrl+O') throw new Error('the reset changed Validate before it was confirmed');
				await js(d, `${row('Open File')}.querySelector('.capture-area .btn-primary').click()`);
				await sleep(300);
				const back = { open: await keysOf('Open File'), validate: await keysOf('Validate') };
				if (back.open !== 'Ctrl+O' || back.validate !== '(none)') throw new Error(`after reset: ${JSON.stringify(back)}`);
				const stored = JSON.parse(await js(d, `return window.__TAURI_INTERNALS__.invoke('get_preference', { key: 'shortcuts_json' })`));
				if (stored['file.open'] !== 'Ctrl+O' || stored['tools.validate'] !== '') throw new Error(`stored: ${JSON.stringify(stored)}`);
				// Validate's F6 is free: its reset applies at once. Defaults again.
				await js(d, `${row('Validate')}.querySelector('.reset-btn').click()`);
				await sleep(300);
				const restored = await keysOf('Validate');
				if (restored !== 'F6') throw new Error(`Validate reset to ${restored}`);
				return `reset asked (“${warning}”), moved Ctrl+O back; stored ${stored['file.open']} / ${JSON.stringify(stored['tools.validate'])}`;
			} finally {
				// Escape leaves a capture first, then closes Settings.
				for (let i = 0; i < 3; i++) {
					if (!(await js(d, `return !!document.querySelector('.settings-nav')`))) break;
					await js(d, `window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }))`);
					await sleep(300);
				}
			}
		});
		await r.check('a reload asks first while the newest edits are not in the saved session', async () => {
			// Nothing in the app reloads the page, but a reload with session
			// restore off dropped every unsaved tab without a question, and
			// with it on the edits the autosave had not written yet.
			const unload = `
				const e = document.createEvent('BeforeUnloadEvent');
				e.initEvent('beforeunload', false, true);
				window.dispatchEvent(e);
				return e.defaultPrevented || !!e.returnValue;`;
			await js(d, `
				if (document.activeElement && document.activeElement.blur) document.activeElement.blur();
				const dt = new DataTransfer();
				dt.setData('text/plain', 'MSH|^~\\\\&|A|B|C|D|20240101||ADT^A01|RELOAD|P|2.5\\rPID|1||1||Unsaved^Edit\\r');
				document.dispatchEvent(new ClipboardEvent('paste', { clipboardData: dt, bubbles: true, cancelable: true }));`);
			// Within the autosave's 0.8 s pause: the new tab is not saved yet.
			const pending = await js(d, unload);
			// Once it is, restore (on in a fresh profile) brings it back.
			await waitFor(d, `return (() => { ${unload} })() ? null : true`, 10000, 'the autosave to write the session');
			if (!pending) throw new Error('a reload right after an edit did not ask');
			return 'asks while the autosave is pending; not once the session holds the tab';
		});
		await r.check('Save As asks for the file type of the document', async () => {
			// FHIR tabs were offered "HL7 Messages (*.hl7)" and "Untitled": on
			// Windows a Patient saved as "patient" became patient.hl7.
			await stubIpc(['plugin:dialog|save']);
			try {
				const asked = {};
				for (const [kind, text] of [
					['FHIR JSON', PATIENT],
					['FHIR XML', '<Patient xmlns="http://hl7.org/fhir"><id value="x1"/><gender value="male"/></Patient>'],
					['HL7', HL7_V23],
				]) {
					await paste(d, text);
					await js(d, `window.dispatchEvent(new KeyboardEvent('keydown', { key: 'S', ctrlKey: true, shiftKey: true, bubbles: true, cancelable: true }))`);
					await waitFor(d, `return window.__blStub.calls.length ? true : null`, 5000, `the ${kind} save dialog`);
					const [call] = await ipcCalls(); // and cleared for the next document
					asked[kind] = call.args?.options;
				}
				const want = { 'FHIR JSON': 'json', 'FHIR XML': 'xml', HL7: 'hl7' };
				for (const [kind, ext] of Object.entries(want)) {
					const o = asked[kind];
					const first = o?.filters?.[0]?.extensions?.join(',');
					const last = o?.filters?.at(-1)?.extensions?.join(',');
					if (first !== ext || last !== '*' || !o.defaultPath?.endsWith(`.${ext}`)) {
						throw new Error(`${kind}: ${JSON.stringify(o)}`);
					}
				}
				return Object.entries(asked).map(([k, o]) => `${k} → ${o.filters[0].name} (*.${o.filters[0].extensions[0]}), ${o.defaultPath}`).join('; ');
			} finally {
				await unstubIpc();
			}
		});
		await r.check('a link in the editor opens in the browser from its hover', async () => {
			// The editor opened its links with window.open, which the webview
			// refuses: "Clickable links (Ctrl+click)" and the hover's "Follow
			// link" did nothing. (WebDriver cannot hold Ctrl through a click,
			// so the hover link stands in for Ctrl+click: both end in the
			// same opener.)
			await paste(d, 'MSH|^~\\&|S|F|R|F|20240101||ADT^A01|LINK|P|2.5\rOBX|1|TX|||See http://bl-e2e.invalid/report for the report\r');
			await stubIpc(['plugin:opener|open_url']);
			try {
				await waitFor(d, `return document.querySelector('.monaco-editor .detected-link') ? true : null`, 15000, 'the underlined link');
				await d.actions({ async: true }).move({ origin: await d.findElement({ css: '.monaco-editor .detected-link' }) }).perform();
				await waitFor(d, `return [...document.querySelectorAll('.monaco-hover a')].some((a) => /follow link/i.test(a.textContent)) || null`, 10000, '"Follow link" in the hover');
				await d.findElement({ xpath: "//*[contains(@class,'monaco-hover')]//a[contains(., 'Follow link')]" }).click();
				const calls = await waitFor(d, `return window.__blStub.calls.length ? window.__blStub.calls : null`, 5000, 'the opener');
				if (!String(calls[0].args?.url).startsWith('http://bl-e2e.invalid/report')) throw new Error(`opener: ${JSON.stringify(calls)}`);
				return `opener got ${calls[0].args.url}`;
			} finally {
				await unstubIpc();
				await d.actions({ async: true }).move({ x: 5, y: 5 }).perform().catch(() => {});
			}
		});
		await r.check('a launch file that does not exist is reported', async () => {
			const path = `${tmpdir()}/bl-e2e-missing-${process.pid}.hl7`;
			await js(d, `return window.__TAURI_INTERNALS__.invoke('plugin:event|emit', { event: 'app://open-files', payload: [${JSON.stringify(path)}] })`);
			const text = await waitFor(d, `return document.querySelector('.dialog[role="alertdialog"]')?.textContent ?? null`, 10000, 'error dialog');
			await js(d, `document.querySelector('.dialog-footer .btn-primary')?.click()`);
			await sleep(300);
			if (!text.includes(path)) throw new Error(`dialog: ${text.slice(0, 120)}`);
			return 'reported';
		});
		await r.check('Save asks before overwriting a file changed by another program', async () => {
			const path = `${tmpdir()}/bl-e2e-ext-${process.pid}.hl7`;
			const mine = 'MSH|^~\\&|A|B|C|D|20240101||ADT^A01|MINE|P|2.5\rPID|1||1||Old^One\r';
			const theirs = 'MSH|^~\\&|A|B|C|D|20240101||ADT^A08|THEIRS|P|2.5\rPID|1||1||New^Name^Longer\r';
			writeFileSync(path, mine);
			try {
				await js(d, `return window.__TAURI_INTERNALS__.invoke('plugin:event|emit', { event: 'app://open-files', payload: [${JSON.stringify(path)}] })`);
				await waitFor(d, `return [...document.querySelectorAll('.monaco-editor .view-line')].some((l) => l.textContent.includes('MINE')) ? true : null`, 10000, 'file open');
				writeFileSync(path, theirs);
				await js(d, `window.dispatchEvent(new KeyboardEvent('keydown', { key: 's', ctrlKey: true, bubbles: true }))`);
				await waitFor(d, `return document.querySelector('.dialog[role="alertdialog"]') ? true : null`, 5000, 'overwrite question');
				await js(d, `[...document.querySelectorAll('.dialog-footer .btn')].find((b) => !b.classList.contains('btn-primary'))?.click()`);
				await sleep(500);
				const disk = readFileSync(path, 'utf8');
				if (!disk.includes('THEIRS')) throw new Error('the newer file was overwritten');
				return 'asked; Cancel kept the newer file';
			} finally {
				rmSync(path, { force: true });
			}
		});

		await r.check('a long field is folded in the editor and Save keeps the file intact', async () => {
			// Save used to write the "{...N bytes}" placeholder over the file.
			const b64 = Buffer.from(Array.from({ length: 3840 }, (_, i) => (i * 37) % 256)).toString('base64');
			const msg = [
				'MSH|^~\\&|LAB|FAC|EHR|FAC|20240101120000||ORU^R01|MSGF|P|2.5',
				'PID|1||12345^^^FAC^MR||Smith^Jane||19900515|F',
				`OBX|1|ED|PDF^Report||^application^pdf^Base64^${b64}||||||F`,
			].join('\r') + '\r';
			const path = `${tmpdir()}/bl-e2e-fold-${process.pid}.hl7`;
			writeFileSync(path, msg);
			try {
				await js(d, `return window.__TAURI_INTERNALS__.invoke('plugin:event|emit', { event: 'app://open-files', payload: [${JSON.stringify(path)}] })`);
				const label = await waitFor(d, `
					const c = document.querySelector('.monaco-editor .bl-fold');
					return c ? c.textContent : null;`, 15000, 'folded chip');
				if (!/Base64\s·\s5\.0\sKB/.test(label)) throw new Error(`chip: ${label}`);
				await js(d, `document.querySelector('.monaco-editor textarea')?.focus()`);
				await js(d, `window.dispatchEvent(new KeyboardEvent('keydown', { key: 's', ctrlKey: true, bubbles: true }))`);
				await sleep(1500);
				const after = readFileSync(path, 'utf8');
				if (after !== msg) throw new Error(`file changed on save: ${msg.length} -> ${after.length} bytes`);
				// Clicking the chip expands it in place.
				await d.findElement({ css: '.monaco-editor .bl-fold' }).click();
				await waitFor(d, `return document.querySelector('.monaco-editor .bl-fold') ? null : true`, 5000, 'chip expanded');
				return `${label}; saved ${after.length} bytes unchanged; chip expands on click`;
			} finally {
				rmSync(path, { force: true });
			}
		});

		/** Expand segment `seg` in the tree (if it is not) and return the
		 *  position of field `f`'s {...} button and of its row, once drawn. */
		const cutFieldButton = async (seg, f) => {
			await js(d, `
				const row = document.querySelector('[data-node-id="seg${seg}"]');
				if (row && row.getAttribute('aria-expanded') !== 'true') row.click();`);
			return waitFor(d, `
				const b = document.querySelector('[data-node-id="seg${seg}.f${f}"] .truncated-btn');
				if (!b) return null;
				const r = b.getBoundingClientRect(), p = b.closest('.tree-node').getBoundingClientRect();
				return { btn: [Math.round(r.left), Math.round(r.right)], row: [Math.round(p.left), Math.round(p.right)] };`,
			10000, `{...} button on seg${seg}.f${f}`);
		};

		await r.check('the {...} button of a cut field is on screen at the default tree width', async () => {
			// It sat inside the value text, which is clipped: a cut value's
			// 55-character preview pushed it past the edge of the 350 px tree.
			const b64 = Buffer.from(Array.from({ length: 225 }, (_, i) => (i * 53) % 256)).toString('base64');
			await paste(d, [
				'MSH|^~\\&|LAB|FAC|EHR|FAC|20240101120000||ORU^R01|MSGB|P|2.5',
				'PID|1||12345^^^FAC^MR||Smith^Jane',
				`OBX|1|ED|PDF^Report||^AP^PDF^Base64^${b64}||||||F`,
			].join('\r'));
			const box = await cutFieldButton(2, 5);
			if (box.btn[0] < box.row[0] || box.btn[1] > box.row[1]) throw new Error(`button at ${box.btn} outside its row ${box.row}`);
			// A real click: WebDriver refuses an element that cannot be seen.
			await d.findElement({ css: '[data-node-id="seg2.f5"] .truncated-btn' }).click();
			const len = await waitFor(d, `return document.querySelector('.modal-body pre')?.textContent.length ?? null`, 5000, 'Full Field Content');
			await closeModal(d);
			if (len < b64.length) throw new Error(`the window shows ${len} characters`);
			return `button at x ${box.btn.join('-')}, row ${box.row.join('-')}; opens ${len} characters`;
		});

		await r.check('a 5 MB field opens in the full-value window at once, and Copy copies all of it', async () => {
			// The whole value went into one <pre>: 10 s with the window frozen
			// in WebKitGTK. Only its start is drawn now.
			const value = '^application^pdf^Base64^' + Buffer.alloc(3932160, 7).toString('base64'); // 5 MB of base64
			// OBX is segment 3 here, so the previous check's tree (OBX at 2)
			// cannot stand in for this one.
			const msg = [
				'MSH|^~\\&|LAB|FAC|EHR|FAC|20240101120000||ORU^R01|MSG5M|P|2.5',
				'PID|1||12345^^^FAC^MR||Smith^Jane||19900515|F',
				'OBR|1||ORD5M|PDF^Report',
				`OBX|1|ED|PDF^Report||${value}||||||F`,
			].join('\r') + '\r';
			// "65,536" / "65.536" / "65 536": the note follows the system locale.
			const grouped = (n) => new RegExp(String(n).replace(/\B(?=(\d{3})+(?!\d))/g, '\\D?'));
			const path = `${tmpdir()}/bl-e2e-5mb-${process.pid}.hl7`;
			writeFileSync(path, msg);
			try {
				await js(d, `return window.__TAURI_INTERNALS__.invoke('plugin:event|emit', { event: 'app://open-files', payload: [${JSON.stringify(path)}] })`);
				await waitFor(d, `return document.querySelector('.tab.active')?.textContent.includes('bl-e2e-5mb') && document.querySelector('[data-node-id="seg3"]') ? true : null`, 20000, '5 MB file open');
				await cutFieldButton(3, 5);
				// Copy goes to the system clipboard: count what it is given.
				await js(d, `window.__blCopied = null; navigator.clipboard.writeText = (t) => { window.__blCopied = t.length; return Promise.resolve(); };`);
				const t0 = Date.now();
				await js(d, `document.querySelector('[data-node-id="seg3.f5"] .truncated-btn').click()`);
				const shown = await waitFor(d, `
					const pre = document.querySelector('.modal-body pre');
					return pre ? { chars: pre.textContent.length, note: document.querySelector('.modal-info.cut')?.textContent.trim() ?? '' } : null;`,
				30000, 'Full Field Content');
				const ms = Date.now() - t0;
				await js(d, `[...document.querySelectorAll('.modal-footer .btn')].find((b) => /copy/i.test(b.textContent))?.click()`);
				const copied = await waitFor(d, `return window.__blCopied`, 5000, 'copy');
				await closeModal(d);
				// 65,536 characters and the "…" after them.
				if (shown.chars > 65537) throw new Error(`${shown.chars} characters drawn`);
				if (!grouped(65536).test(shown.note) || !grouped(value.length).test(shown.note)) throw new Error(`note: ${shown.note}`);
				if (copied !== value.length) throw new Error(`copied ${copied} of ${value.length} characters`);
				return `${shown.chars} characters drawn in ${ms} ms; “${shown.note}”; copied ${copied}`;
			} finally {
				rmSync(path, { force: true });
			}
		});

		await r.check('the segment grid lists every OBX as a row', async () => {
			await paste(d, [
				'MSH|^~\\&|LAB|FAC|EHR|FAC|20240101120000||ORU^R01|MSG2|P|2.5',
				'PID|1||12345^^^FAC^MR||Smith^Jane',
				'OBR|1||ORD1|CBC^Blood count',
				'OBX|1|NM|WBC^Leukocytes||6.1|10*9/L|4.0-10.0|N|||F',
				'OBX|2|NM|HGB^Hemoglobin||10.2|g/dL|12.0-16.0|L|||F',
				'OBX|3|NM|PLT^Platelets||250|10*9/L|150-400|N|||F',
			].join('\r'));
			await js(d, `window.dispatchEvent(new KeyboardEvent('keydown', { key: 'G', ctrlKey: true, shiftKey: true, bubbles: true }))`);
			const n = await waitFor(d, `
				const rows = document.querySelectorAll('.grid-table tbody tr').length;
				return rows >= 3 ? rows : null;
			`, 15000, 'grid rows');
			const info = await js(d, `return {
				seg: document.querySelector('.grid-table .col-pos')?.textContent.trim(),
				desc: [...document.querySelectorAll('.grid-table .desc')].map((e) => e.textContent.trim()),
			}`);
			if (!info.seg?.startsWith('OBX-')) throw new Error(`first column ${info.seg}`);
			if (n !== 3) throw new Error(`${n} rows`);
			// Close the panel again so later checks see the usual layout.
			await js(d, `window.dispatchEvent(new KeyboardEvent('keydown', { key: 'G', ctrlKey: true, shiftKey: true, bubbles: true }))`);
			return `${n} OBX rows; codes: ${info.desc.slice(0, 3).join(', ')}`;
		});

		await r.check('the interpretation panel reads the message and links each value', async () => {
			await paste(d, [
				'MSH|^~\\&|LAB|FAC|EHR|FAC|20240101120000||ADT^A01|MSG3|P|2.5',
				'EVN|A01|20240101120000',
				'PID|1||12345^^^FAC^MR||Rossi^Mario||19800515|M',
			].join('\r'));
			const toggle = `window.dispatchEvent(new KeyboardEvent('keydown', { key: 'E', ctrlKey: true, shiftKey: true, bubbles: true }))`;
			await js(d, toggle);
			const patient = await waitFor(d, `
				const p = [...document.querySelectorAll('.interp-sentence')].find((e) => e.textContent.startsWith('Patient'));
				return p ? p.textContent : null;`, 15000, 'patient sentence');
			if (!/Rossi, Mario, Male, born 1980-05-15, identifiers 12345 \(Medical record number\)\./.test(patient)) {
				throw new Error(`patient: ${patient}`);
			}
			// A value selects its field: PID-5 in the tree.
			await js(d, `[...document.querySelectorAll('.interp-value')].find((e) => e.textContent === 'Rossi, Mario')?.click()`);
			const tree = await waitFor(d, `return document.querySelector('.tree-node.selected')?.dataset.nodeId ?? null`, 5000, 'tree selection');
			if (tree !== 'seg2.f5') throw new Error(`tree selected ${tree}`);
			await js(d, toggle);
			return `${patient}; click selects ${tree}`;
		});

		await r.check('the interpretation panel draws a long result a page at a time', async () => {
			const lines = ['MSH|^~\\&|LAB|FAC|EHR|FAC|20240101120000||ORU^R01|BIG2|P|2.5', 'PID|1||1^^^H^MR||Big^Log', 'OBR|1||O1|CBC^Count'];
			for (let i = 1; i <= 1000; i++) lines.push(`OBX|${i}|NM|WBC^Leukocytes^LN||1.1|10*9/L|4.0-10.0|N|||F`);
			await paste(d, lines.join('\r'));
			const toggle = `window.dispatchEvent(new KeyboardEvent('keydown', { key: 'E', ctrlKey: true, shiftKey: true, bubbles: true }))`;
			await js(d, toggle);
			const page = `const more = document.querySelector('.interp-more');
				return more ? { blocks: document.querySelectorAll('.interp-block').length, more: more.textContent.trim() } : null;`;
			const first = await waitFor(d, page, 20000, 'first page');
			if (first.blocks !== 200 || !/\b200 of 1003\b/.test(first.more)) throw new Error(`${first.blocks} blocks; ${first.more}`);
			await js(d, `document.querySelector('.interp-more').click()`);
			const second = await waitFor(d, `const n = document.querySelectorAll('.interp-block').length; return n > 200 ? n : null;`, 5000, 'second page');
			await js(d, toggle);
			return `${first.blocks} of 1003 blocks drawn, then ${second}`;
		});

		await r.check('a message with thousands of segments draws only the rows in view', async () => {
			// Every segment used to get a DOM row: 20,000 OBX froze the window.
			const lines = ['MSH|^~\\&|LAB|FAC|EHR|FAC|20240101120000||ORU^R01|BIG|P|2.5', 'PID|1||1^^^H^MR||Big^Log', 'OBR|1||O1|CBC^Count'];
			for (let i = 1; i <= 5000; i++) lines.push(`OBX|${i}|NM|WBC^Leukocytes^LN||1.1|10*9/L|4.0-10.0|N|||F`);
			const t0 = Date.now();
			await paste(d, lines.join('\r'));
			const shown = await waitFor(d, `
				const badge = document.querySelector('.panel-badge')?.textContent.trim();
				return badge === '5003' ? document.querySelectorAll('.tree-node').length : null;`, 30000, 'tree of 5003 segments');
			if (shown > 300) throw new Error(`${shown} rows in the DOM`);
			// Keyboard: End goes to the last segment, which is then drawn.
			await js(d, `document.querySelector('.tree-node')?.focus()`);
			await js(d, `document.activeElement.dispatchEvent(new KeyboardEvent('keydown', { key: 'End', bubbles: true }))`);
			await waitFor(d, `return document.querySelector('[data-node-id="seg5002"].selected') ? true : null`, 5000, 'last row selected');
			return `${shown} rows drawn for 5003 segments; parsed and shown in ${Date.now() - t0} ms`;
		});

		await r.check('clicking a validation issue selects its field, blank lines and all', async () => {
			await paste(d, [
				'MSH|^~\\&|A|B|C|D|20240101120000||ADT^A01|1|P|2.5',
				'',
				'EVN|A01|20240101',
				'',
				'PID|1||1^^^H^MR||Rossi^Mario||19801399|F',
			].join('\n'));
			const txt = await validate(d);
			if (!/PID-7/.test(txt)) throw new Error(`no PID-7 issue: ${txt.slice(0, 120)}`);
			await js(d, `[...document.querySelectorAll('.issue-row')].find((x) => x.textContent.includes('PID-7'))?.click()`);
			const status = await waitFor(d, `
				const t = document.querySelector('.status-bar')?.textContent ?? '';
				return /Ln 5\\b/.test(t) ? t.replace(/\\s+/g, ' ') : null;`, 5000, 'caret on the PID line');
			const tree = await waitFor(d, `return document.querySelector('.tree-node.selected')?.dataset.nodeId ?? null`, 5000, 'tree selection');
			if (tree !== 'seg2.f7') throw new Error(`tree selected ${tree}`);
			return `${status.match(/Ln \d+, Col \d+/)?.[0]}; tree ${tree}`;
		});

		await r.check('the validation panel draws a long report a page at a time', async () => {
			// Every issue used to get a row: 10,000 issues froze the window for
			// seconds, 30,000 for a minute, and again at each re-validation.
			const lines = ['MSH|^~\\&|LAB|FAC|EHR|FAC|20240101120000||ORU^R01|BIG3|P|2.5', 'PID|1||1^^^H^MR||Big^Log', 'OBR|1||O1|CBC^Count'];
			// No OBX-11 (result status, required): one error per OBX.
			for (let i = 1; i <= 1000; i++) lines.push(`OBX|${i}|NM|WBC^Leukocytes^LN||1.1|10*9/L|4.0-10.0|N`);
			await paste(d, lines.join('\r'));
			await validate(d);
			const first = await waitFor(d, `
				const more = document.querySelector('.validation-more');
				const all = document.querySelector('.summary-badge')?.textContent.match(/\\((\\d+)\\)/)?.[1];
				return more ? { rows: document.querySelectorAll('.issue-row').length, more: more.textContent.trim(), all: Number(all) } : null;`, 20000, 'first page');
			// The badges count the whole report, the rows are its first page.
			if (first.all < 1000) throw new Error(`the badge counts ${first.all} issues`);
			if (first.rows !== 200 || !first.more.includes(`200 of ${first.all}`)) throw new Error(`${first.rows} rows; ${first.more}`);
			await js(d, `document.querySelector('.validation-more').click()`);
			const second = await waitFor(d, `const n = document.querySelectorAll('.issue-row').length; return n > 200 ? n : null;`, 5000, 'second page');
			if (second !== 400) throw new Error(`${second} rows after Show more`);
			// Another filter is another list: its own first page, counted whole.
			await js(d, `document.querySelector('.summary-badge.error')?.click()`);
			const errors = await waitFor(d, `
				const badge = document.querySelector('.summary-badge.error');
				const more = document.querySelector('.validation-more')?.textContent.trim();
				const n = document.querySelectorAll('.issue-row').length;
				return badge?.classList.contains('active') && more && n === 200
					? { count: Number(badge.textContent.replace(/\\D+/g, '')), more } : null;`, 5000, 'errors only, first page');
			await js(d, `document.querySelector('.summary-badge.error')?.click()`);
			if (errors.count < 1000 || !errors.more.includes(`200 of ${errors.count}`)) throw new Error(`errors: ${JSON.stringify(errors)}`);
			// Click-to-navigate works on a row past the first page too.
			await js(d, `document.querySelector('.validation-more').click()`);
			await waitFor(d, `return document.querySelectorAll('.issue-row').length > 200 ? true : null`, 5000, 'second page again');
			await js(d, `[...document.querySelectorAll('.issue-row')].find((x) => /OBX-11\\s*\\(300\\)/.test(x.textContent))?.click()`);
			const tree = await waitFor(d, `return document.querySelector('.tree-node.selected')?.dataset.nodeId ?? null`, 5000, 'tree selection');
			// OBX-11 is absent from these segments, so there is no field node
			// to select: the segment it belongs to is selected instead.
			if (!/^seg300(\.f11)?$/.test(tree)) throw new Error(`tree selected ${tree}`);
			return `${first.rows} of ${first.all} issues drawn, then ${second}; errors only: ${errors.more}; row 300 selects ${tree}`;
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

		await r.check('FHIR XML has typed primitives, as in JSON', async () => {
			// Every XML primitive used to arrive as a string: Patient.active
			// was never "= true" and a Quantity value could not be compared.
			await paste(d, '<Observation xmlns="http://hl7.org/fhir"><status value="final"/>'
				+ '<code><text value="Glucose"/></code>'
				+ '<valueQuantity><value value="6.30"/><unit value="mmol/L"/></valueQuantity></Observation>');
			const x = await fhirpath(d, 'Observation.value.value > 6.2');
			if (x.error) throw new Error(x.error);
			if (x.values.join('') !== 'true') throw new Error(JSON.stringify(x).slice(0, 120));
			return 'Observation.value.value > 6.2 = true';
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
			// The R4 core ships inside the binary, so an empty package manager
			// means the embedded index failed to load — a build defect.
			r.record('built-in FHIR R4 core is listed', false, 'package manager lists nothing');
		} else {
			await r.check('built-in FHIR R4 core is listed', () => {
				if (!/hl7\.fhir\.r4\.core/.test(installed)) throw new Error(installed.slice(0, 120));
				return installed.slice(0, 90);
			});

			// A type no package defines is the one honest "not checked" left.
			await r.check('conformance is reported as NOT checked for an unknown type', async () => {
				await paste(d, JSON.stringify({ resourceType: 'Patiend', id: 'p1' }, null, 1));
				const txt = await validate(d);
				if (!/not checked/i.test(txt)) throw new Error(`no such note: ${txt.slice(0, 150)}`);
				return 'note present';
			});

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

		// ---- content security policy ------------------------------------
		// The CSP in tauri.conf.json is what makes "the app only talks to
		// api.github.com on its own" true of the webview, not just of the
		// code. Monaco, the tree and the dialogs above already ran under it.
		console.log('\n=== content security policy ===');
		// Resolves with the violated directive, or 'none' once the request
		// settled without one (a network failure or the 3 s abort offline is
		// not a violation).
		const probe = (url) => d.executeAsyncScript(`
			const done = arguments[arguments.length - 1];
			let seen = null;
			const on = (e) => { seen = e.violatedDirective; };
			document.addEventListener('securitypolicyviolation', on);
			const end = () => setTimeout(() => {
				document.removeEventListener('securitypolicyviolation', on);
				done(seen || 'none');
			}, 300);
			// A CSP refusal is immediate; cap the network attempt so an
			// air-gapped or black-holed network cannot stall the probe.
			const ctl = new AbortController();
			setTimeout(() => ctl.abort(), 3000);
			fetch(${JSON.stringify(url)}, { method: 'HEAD', signal: ctl.signal }).then(end, end);
		`);
		await r.check('a connection to any other host is blocked', async () => {
			const v = await probe('https://example.com/');
			if (!String(v).startsWith('connect-src')) throw new Error(`violation: ${v}`);
			return v;
		});
		await r.check('the update-check host is allowed', async () => {
			const v = await probe('https://api.github.com/');
			if (v !== 'none') throw new Error(`violation: ${v}`);
			return 'no violation';
		});
		await r.check('inline styles still apply', async () => {
			const color = await js(d, `
				const s = document.createElement('style');
				s.textContent = '#bl-csp-probe { color: rgb(1, 2, 3); }';
				document.head.appendChild(s);
				const el = document.createElement('div');
				el.id = 'bl-csp-probe';
				document.body.appendChild(el);
				const c = getComputedStyle(el).color;
				el.remove(); s.remove();
				return c;`);
			if (color !== 'rgb(1, 2, 3)') throw new Error(`computed ${color}`);
			return color;
		});

		// ---- windows and navigation --------------------------------------
		// Last: the first can leave the page elsewhere if it fails, and the
		// second ends the app.
		console.log('\n=== windows and navigation ===');
		await r.check('the window refuses to load a page that is not the app', async () => {
			// A link the webview followed by itself (a middle click, its own
			// "Open Link") replaced the app. The backend now refuses every
			// navigation away from the app's pages; web and e-mail links go
			// to the browser or mail client, an unknown scheme nowhere.
			const before = await js(d, `return location.href`);
			await js(d, `setTimeout(() => { location.href = 'bl-e2e-nowhere:test'; }, 0)`);
			await sleep(2000);
			const after = await js(d, `return { href: location.href, shell: !!document.querySelector('.menu-trigger') }`).catch((e) => ({ error: String(e) }));
			if (after.href !== before || !after.shell) throw new Error(`before ${before}; after ${JSON.stringify(after)}`);
			return `still at ${after.href}`;
		});
		await r.check('closing the main window ends the app, the manual window included', async () => {
			// Left running with only the manual, the app kept the MLLP
			// listener bound, and every later launch, a double-clicked file
			// too, went to a main window that was gone: nothing appeared.
			const main = await d.getWindowHandle();
			await js(d, `window.dispatchEvent(new KeyboardEvent('keydown', { key: 'F1', bubbles: true }))`);
			let handles = [];
			for (const end = Date.now() + 15000; handles.length < 2 && Date.now() < end;) {
				await sleep(500);
				handles = await d.getAllWindowHandles();
			}
			if (handles.length < 2) throw new Error('the manual window did not open');
			await d.switchTo().window(main);
			// The close request the title-bar button sends. Session restore
			// is on in a fresh profile, so the tabs are saved, not asked about.
			await js(d, `window.__TAURI_INTERNALS__.invoke('plugin:window|close', { label: 'main' }).catch(() => {})`).catch(() => {});
			// Ended: the driver has no windows left, or no session.
			const answer = () => Promise.race([
				d.getAllWindowHandles().then((h) => h.length, () => 0),
				sleep(3000).then(() => null),
			]);
			let left = null;
			for (const end = Date.now() + 20000; left !== 0 && Date.now() < end;) {
				await sleep(500);
				left = await answer();
			}
			if (left === null) throw new Error('WebDriver stopped answering: check by hand that no BridgeLab process is left');
			if (left !== 0) throw new Error(`${left} window(s) still open 20 s after the main window was closed`);
			return 'no window left: the app has ended';
		});
	} finally {
		await d.quit().catch(() => {});
	}
	return r;
}

if (import.meta.url === `file://${process.argv[1]}`) {
	const r = await appSuite();
	process.exit(r.finish() ? 0 : 1);
}
