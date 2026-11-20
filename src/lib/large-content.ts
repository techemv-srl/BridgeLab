/**
 * Sizes at which BridgeLab asks before it loads or sends a message, and the
 * questions it asks. A message of tens of MB takes the window seconds to
 * load and several GB of memory, and many receivers refuse one: the user is
 * told before, as when opening a large file, rather than left with a window
 * that stops answering.
 */
import { dialogStore } from '$lib/stores/dialog.svelte';
import { getLocale, t } from '$lib/i18n';

const MiB = 1024 * 1024;

/** Text larger than this is loaded into a tab only after a question: the
 *  same size as for opening a file (`OPEN_CONFIRM_BYTES` in
 *  src-tauri/src/commands/fileio.rs). */
export const ASK_ABOVE_BYTES = 16 * MiB;

/** Text larger than this is never loaded into a tab, as a file is never
 *  opened (`OPEN_MAX_BYTES` in fileio.rs). */
export const REFUSE_ABOVE_BYTES = 64 * MiB;

/** A message larger than this is sent only after a question. BridgeLab's
 *  own listener refuses one (`MAX_MSG_BYTES` in
 *  src-tauri/src/communication/mllp_listener.rs), and so do many engines. */
export const SEND_ASK_ABOVE_BYTES = 10 * MiB;

/** Bytes `text` takes in UTF-8, counted without encoding it. */
export function utf8Length(text: string): number {
	let n = 0;
	for (let i = 0; i < text.length; i++) {
		const c = text.charCodeAt(i);
		if (c < 0x80) n += 1;
		else if (c < 0x800) n += 2;
		else if (c >= 0xd800 && c <= 0xdbff && isLowSurrogate(text.charCodeAt(i + 1))) {
			// A surrogate pair is one character of four bytes.
			n += 4;
			i++;
		} else n += 3; // a lone surrogate is written as U+FFFD, three bytes
	}
	return n;
}

function isLowSurrogate(c: number): boolean {
	return c >= 0xdc00 && c <= 0xdfff;
}

/** Whether `copies` of `text` may be over `limit` bytes in UTF-8: a UTF-16
 *  unit is at most three bytes, so most text is cleared without counting. */
export function mayExceed(text: string, limit: number, copies = 1): boolean {
	return text.length * 3 * copies > limit;
}

/** How many times a paste into the editor writes `text`: once per cursor,
 *  unless the text has exactly one line per cursor, which Monaco spreads
 *  one line to each (its default `multiCursorPaste: "spread"`, after
 *  dropping a final line feed). Counted without splitting the text. */
export function pasteCopies(text: string, cursors: number): number {
	if (cursors <= 1) return 1;
	// Monaco drops a final LF, then a final CR, before it counts the lines.
	let end = text.length;
	if (text.charCodeAt(end - 1) === 10) end--;
	if (text.charCodeAt(end - 1) === 13) end--;
	let lines = 1;
	for (let i = 0; i < end && lines <= cursors; i++) {
		const c = text.charCodeAt(i);
		if (c === 13) {
			lines++;
			if (i + 1 < end && text.charCodeAt(i + 1) === 10) i++;
		} else if (c === 10) lines++;
	}
	return lines === cursors ? 1 : cursors;
}

/** A number with one decimal in the interface language: "20,0" in Italian,
 *  "20.0" in English. */
export function oneDecimal(n: number): string {
	return new Intl.NumberFormat(getLocale(), { minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(n);
}

/** MB with one decimal, in the interface language. */
function mb(bytes: number): string {
	return oneDecimal(bytes / MiB);
}

/** A size for display, in B, KB or MB, decimals in the interface language. */
export function formatSize(bytes: number): string {
	if (bytes < 1024) return `${bytes} B`;
	if (bytes < MiB) return `${oneDecimal(bytes / 1024)} KB`;
	return `${oneDecimal(bytes / MiB)} MB`;
}

/**
 * Whether pasted `text`, written `copies` times (once per cursor), may go
 * into a tab: at once up to [`ASK_ABOVE_BYTES`], after a question up to
 * [`REFUSE_ABOVE_BYTES`], and never beyond, with a message that says so.
 */
export async function mayLoadPastedText(text: string, copies = 1): Promise<boolean> {
	if (!mayExceed(text, ASK_ABOVE_BYTES, copies)) return true;
	const bytes = utf8Length(text) * copies;
	if (bytes <= ASK_ABOVE_BYTES) return true;
	if (bytes > REFUSE_ABOVE_BYTES) {
		await dialogStore.info(
			t('paste.tooLarge', { size: mb(bytes), limit: REFUSE_ABOVE_BYTES / MiB }),
			t('paste.largeTitle'),
		);
		return false;
	}
	return dialogStore.confirm(
		t('paste.largeConfirm', { size: mb(bytes), limit: ASK_ABOVE_BYTES / MiB }),
		t('paste.largeTitle'),
	);
}

/** Whether a message of `bytes` may be sent: at once up to
 *  [`SEND_ASK_ABOVE_BYTES`], otherwise once the user agreed. */
export async function maySendMessage(bytes: number): Promise<boolean> {
	if (bytes <= SEND_ASK_ABOVE_BYTES) return true;
	return dialogStore.confirm(
		t('comm.largeSendConfirm', { size: mb(bytes), limit: SEND_ASK_ABOVE_BYTES / MiB }),
		t('comm.largeSendTitle'),
	);
}
