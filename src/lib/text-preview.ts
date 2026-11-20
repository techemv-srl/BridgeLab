/**
 * How much of a value the Full Field Content window draws. The window put
 * the whole value in one <pre>: a 5 MB base64 PDF in OBX-5 blocked the
 * window for 10 s while WebKitGTK laid it out (17 s at 10 MB). 64 K
 * characters is a few hundred wrapped lines, drawn at once, and more than
 * any value meant to be read; Copy still copies the whole value.
 */
export const FIELD_PREVIEW_CHARS = 64 * 1024;

/**
 * The start of `text`: all of it up to `max` characters (UTF-16 units, as
 * `length` counts them), otherwise the first `max`, one fewer when the cut
 * would split an emoji or other surrogate pair. `cut` says whether
 * anything was left out.
 */
export function previewText(text: string, max = FIELD_PREVIEW_CHARS): { text: string; cut: boolean } {
	if (text.length <= max) return { text, cut: false };
	let end = max;
	const last = text.charCodeAt(end - 1);
	if (last >= 0xd800 && last <= 0xdbff) end--;
	return { text: text.slice(0, end), cut: true };
}
