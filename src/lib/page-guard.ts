/**
 * What the webview must not do by itself in the app's pages. The main
 * window and the manual window share the root layout, which installs
 * these guards on both.
 *
 * - Links: a mailto: or web link followed inside the app's own window
 *   replaced the app, on Linux with "The URL can't be shown". Every such
 *   link, in the app and in the manual, opens with the system's mail
 *   client or browser instead, whether it is clicked or middle-clicked.
 *   (The backend refuses any navigation that still gets through.)
 * - Context menu: the webview's own menu is never shown. Outside a text
 *   field it offers Back, Forward and Reload, and a reload dropped every
 *   unsaved tab when session restore was off; in a text field WebView2's
 *   adds Emoji, password import and "More tools", which have no place in
 *   the app. Cut, Copy and Paste keep their keys (Ctrl+X, Ctrl+C,
 *   Ctrl+V). The app's own menus (editor, tabs, tree) are not affected.
 */

import { t } from '$lib/i18n';

/** Whether reloading the page now loses work: unsaved tabs the session
 *  does not keep (restore off), or edits its autosave has not written yet
 *  (it waits for a pause in typing). */
export function reloadLosesWork(unsavedTabs: number, restoreEnabled: boolean, autosavePending: boolean): boolean {
	return unsavedTabs > 0 && (!restoreEnabled || autosavePending);
}

/** The URL to hand to the system's browser or mail client for a link with
 *  `href` on `page`, or null when the link is the app's own (an anchor in
 *  the page, a route) and the webview may follow it. */
export function externalHref(href: string, page: { href: string; origin: string }): string | null {
	if (!/^(mailto:|tel:|https?:)/i.test(href)) return null;
	if (/^https?:/i.test(href)) {
		try {
			if (new URL(href, page.href).origin === page.origin) return null;
		} catch {
			return null;
		}
	}
	return href;
}

/** The parts of a click the link decision reads. */
export interface LinkClick {
	type: string;
	button: number;
	defaultPrevented: boolean;
	target: EventTarget | null;
}

/** The URL a click on a page should open outside the app, or null when
 *  the webview may handle it. A plain click (with Ctrl or Shift too)
 *  arrives as `click`, a middle click as `auxclick`: the webview follows
 *  the link on either. */
export function linkToOpen(e: LinkClick, page: { href: string; origin: string }): string | null {
	const button = e.type === 'auxclick' ? 1 : 0;
	if (e.defaultPrevented || e.button !== button) return null;
	const a = (e.target as Element | null)?.closest?.('a[href]');
	if (!a) return null;
	return externalHref(a.getAttribute('href') ?? '', page);
}

/** What becomes of a link the system could not open. By default a notice
 *  drawn in the page itself, since the manual window has no app dialogs;
 *  the main window shows its own dialog (setOpenFailedHandler). */
let openFailed: (url: string) => void = (url) => showLinkNotice(document, url);

export function setOpenFailedHandler(handler: (url: string) => void): void {
	openFailed = handler;
}

/** True in the app's own windows, false in a browser (development). */
function inApp(): boolean {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * Open `url` with the system's browser or mail client. When none can be
 * started (Linux with neither xdg-open nor gio), say so and give the
 * address to copy: window.open does nothing in the webview, so every link
 * did nothing and said nothing, and sending the window itself to the link
 * replaced the app. Only in a plain browser does a new tab open instead.
 */
export async function openExternal(url: string): Promise<void> {
	if (!inApp()) {
		window.open(url, '_blank');
		return;
	}
	try {
		const { openUrl } = await import('@tauri-apps/plugin-opener');
		await openUrl(url);
	} catch {
		openFailed(url);
	}
}

/** A notice at the bottom of the page: the link could not be opened, and
 *  its address, selected, to copy. */
export function showLinkNotice(doc: Document, url: string): void {
	doc.getElementById('bl-link-notice')?.remove();
	const box = doc.createElement('div');
	box.id = 'bl-link-notice';
	box.setAttribute('role', 'alert');
	box.style.cssText = 'position:fixed;left:16px;right:16px;bottom:16px;z-index:10000;display:flex;flex-wrap:wrap;gap:8px;align-items:center;'
		+ 'padding:10px 12px;border-radius:6px;background:#313244;color:#cdd6f4;font:13px system-ui,sans-serif;box-shadow:0 4px 16px rgba(0,0,0,.4)';
	const text = doc.createElement('span');
	text.style.flex = '1 1 100%';
	text.textContent = t('link.openFailed');
	const address = doc.createElement('input');
	address.readOnly = true;
	address.value = url;
	address.style.cssText = 'flex:1;min-width:0;padding:3px 6px;font:12px monospace';
	const close = doc.createElement('button');
	close.textContent = '×';
	close.setAttribute('aria-label', t('modal.close'));
	close.style.cssText = 'padding:2px 8px;font-size:14px;cursor:pointer';
	close.addEventListener('click', () => box.remove());
	box.append(text, address, close);
	doc.body.append(box);
	address.focus();
	address.select();
}

/** Install the guards on `win` (the page, or the manual's in-window
 *  fallback frame). Returns the function that removes them. */
export function installPageGuards(win: Window, open: (url: string) => void = openExternal): () => void {
	const doc = win.document;
	const onLink = (e: MouseEvent) => {
		const url = linkToOpen(e, win.location);
		if (!url) return;
		e.preventDefault();
		open(url);
	};
	// Capture phase, so a handler that stops the event cannot let the
	// webview's menu through. The app's own menus still open: they draw
	// themselves, and cancel the event anyway.
	const onMenu = (e: MouseEvent) => {
		e.preventDefault();
	};
	// Capture phase for links too, so this runs before the opener plugin's
	// own handler, which only takes links with target="_blank".
	const capture = { capture: true };
	doc.addEventListener('click', onLink, capture);
	doc.addEventListener('auxclick', onLink, capture);
	win.addEventListener('contextmenu', onMenu, capture);
	return () => {
		doc.removeEventListener('click', onLink, capture);
		doc.removeEventListener('auxclick', onLink, capture);
		win.removeEventListener('contextmenu', onMenu, capture);
	};
}
