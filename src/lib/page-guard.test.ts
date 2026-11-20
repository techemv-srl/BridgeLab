import { describe, it, expect, vi, afterEach } from 'vitest';
import { externalHref, installPageGuards, linkToOpen, openExternal, reloadLosesWork, setOpenFailedHandler } from './page-guard';

const page = { href: 'tauri://localhost/', origin: 'tauri://localhost' };
const windowsPage = { href: 'http://tauri.localhost/', origin: 'http://tauri.localhost' };

/** Something inside a link with `href`, as a click's target. */
const inLink = (href: string) => ({
	closest: (sel: string) => (sel === 'a[href]' ? { getAttribute: () => href } : null),
});
const outsideLinks = { closest: () => null };

describe('externalHref', () => {
	it('hands e-mail and web links to the system', () => {
		expect(externalHref('mailto:info@techemv.it', page)).toBe('mailto:info@techemv.it');
		expect(externalHref('MAILTO:info@techemv.it', page)).toBe('MAILTO:info@techemv.it');
		expect(externalHref('tel:+39000', page)).toBe('tel:+39000');
		expect(externalHref('https://www.techemv.it', page)).toBe('https://www.techemv.it');
		expect(externalHref('http://example.com/a', windowsPage)).toBe('http://example.com/a');
	});

	it("leaves the page's own links to the webview", () => {
		expect(externalHref('#licensing', page)).toBeNull();
		expect(externalHref('/manual?lang=en', page)).toBeNull();
		expect(externalHref('http://tauri.localhost/manual', windowsPage)).toBeNull();
		expect(externalHref('javascript:void(0)', page)).toBeNull();
		expect(externalHref('', page)).toBeNull();
	});
});

describe('linkToOpen', () => {
	const click = (over: Partial<Parameters<typeof linkToOpen>[0]>) => ({
		type: 'click', button: 0, defaultPrevented: false, target: inLink('mailto:info@techemv.it') as never, ...over,
	});

	it('takes a plain click and a middle click on an e-mail or web link', () => {
		expect(linkToOpen(click({}), page)).toBe('mailto:info@techemv.it');
		expect(linkToOpen(click({ type: 'auxclick', button: 1 }), page)).toBe('mailto:info@techemv.it');
		expect(linkToOpen(click({ type: 'auxclick', button: 1, target: inLink('https://www.techemv.it') as never }), page))
			.toBe('https://www.techemv.it');
	});

	it('leaves other buttons, handled clicks and non-links alone', () => {
		expect(linkToOpen(click({ type: 'auxclick', button: 2 }), page)).toBeNull();
		expect(linkToOpen(click({ button: 1 }), page)).toBeNull();
		expect(linkToOpen(click({ defaultPrevented: true }), page)).toBeNull();
		expect(linkToOpen(click({ target: outsideLinks as never }), page)).toBeNull();
		expect(linkToOpen(click({ target: null }), page)).toBeNull();
		expect(linkToOpen(click({ target: inLink('#top') as never }), page)).toBeNull();
	});
});

describe('reloadLosesWork', () => {
	it('is true for unsaved tabs the session does not hold', () => {
		expect(reloadLosesWork(1, false, false)).toBe(true);
		expect(reloadLosesWork(2, true, true)).toBe(true);
	});

	it('is false with nothing unsaved, or with every edit in the saved session', () => {
		expect(reloadLosesWork(0, false, true)).toBe(false);
		expect(reloadLosesWork(3, true, false)).toBe(false);
	});
});

describe('installPageGuards', () => {
	/** A window and document that record and dispatch events (no DOM in
	 *  this environment): enough to see what the guards cancel. */
	function fakeWindow() {
		const document = new EventTarget();
		const win = Object.assign(new EventTarget(), { document, location: page });
		return { win: win as unknown as Window, document };
	}
	const fire = (on: EventTarget, type: string, button: number, target: unknown) => {
		const e = new Event(type, { cancelable: true });
		Object.defineProperty(e, 'button', { value: button });
		Object.defineProperty(e, 'target', { value: target });
		on.dispatchEvent(e);
		return e.defaultPrevented;
	};

	it('opens a clicked or middle-clicked link outside the app and cancels the native menu', () => {
		const { win, document } = fakeWindow();
		const open = vi.fn();
		const remove = installPageGuards(win, open);

		expect(fire(document, 'auxclick', 1, inLink('mailto:info@techemv.it'))).toBe(true);
		expect(fire(document, 'click', 0, inLink('https://www.techemv.it'))).toBe(true);
		expect(open.mock.calls).toEqual([['mailto:info@techemv.it'], ['https://www.techemv.it']]);
		expect(fire(document, 'click', 0, inLink('#section'))).toBe(false);

		expect(fire(win, 'contextmenu', 2, { tagName: 'DIV' })).toBe(true);
		// Text fields too: WebView2's menu there has Emoji, password import
		// and "More tools"; Cut, Copy and Paste keep their keys.
		expect(fire(win, 'contextmenu', 2, { tagName: 'INPUT', type: 'text' })).toBe(true);
		expect(fire(win, 'contextmenu', 2, { tagName: 'TEXTAREA' })).toBe(true);

		remove();
		expect(fire(win, 'contextmenu', 2, { tagName: 'DIV' })).toBe(false);
		expect(fire(document, 'auxclick', 1, inLink('mailto:info@techemv.it'))).toBe(false);
		expect(open).toHaveBeenCalledTimes(2);
	});
});

const openUrl = vi.fn();
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: (url: string) => openUrl(url) }));

describe('openExternal', () => {
	/** A window of the app, or of a plain browser when not `inApp`. */
	function stubWindow(inApp: boolean) {
		const win = { open: vi.fn(), location: { href: 'tauri://localhost/' }, ...(inApp ? { __TAURI_INTERNALS__: {} } : {}) };
		vi.stubGlobal('window', win);
		return win;
	}

	afterEach(() => {
		vi.unstubAllGlobals();
	});

	it('hands the link to the system browser or mail client', async () => {
		const win = stubWindow(true);
		const failed = vi.fn();
		setOpenFailedHandler(failed);
		openUrl.mockReset().mockResolvedValue(undefined);
		await openExternal('https://www.techemv.it');
		expect(openUrl).toHaveBeenCalledWith('https://www.techemv.it');
		expect(failed).not.toHaveBeenCalled();
		expect(win.open).not.toHaveBeenCalled();
	});

	it('without a program to open it, says so with the address and never navigates the window', async () => {
		// Linux with neither xdg-open nor gio: window.open did nothing, and
		// the quote button sent the window itself to the mailto: link.
		const win = stubWindow(true);
		const failed = vi.fn();
		setOpenFailedHandler(failed);
		openUrl.mockReset().mockRejectedValue(new Error('No such file or directory (os error 2)'));
		await openExternal('mailto:info@techemv.it?subject=Quote');
		expect(failed).toHaveBeenCalledWith('mailto:info@techemv.it?subject=Quote');
		expect(win.open).not.toHaveBeenCalled();
		expect(win.location.href).toBe('tauri://localhost/');
	});

	it('opens a new tab in a plain browser (development)', async () => {
		const win = stubWindow(false);
		openUrl.mockReset();
		await openExternal('https://www.techemv.it');
		expect(win.open).toHaveBeenCalledWith('https://www.techemv.it', '_blank');
		expect(openUrl).not.toHaveBeenCalled();
	});
});
