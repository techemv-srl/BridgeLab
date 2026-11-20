/** Pure helpers of the Communication panel, kept apart to be tested. */

import type { ReplyProblem } from '$lib/ipc/communication';

/** The outcome family of an ACK code: the commit-mode codes (CA, CE, CR)
 *  count with the application ones (AA, AE, AR) they correspond to. */
export function ackFamily(code: string | null | undefined): 'AA' | 'AE' | 'AR' | null {
	switch ((code ?? '').trim().toUpperCase()) {
		case 'AA': case 'CA': return 'AA';
		case 'AE': case 'CE': return 'AE';
		case 'AR': case 'CR': return 'AR';
		default: return null;
	}
}

/** What an MLLP reply says, from its MSA-1 (read by the backend with the
 *  reply's own separators). */
export function ackLabel(code: string | null | undefined): string {
	switch ((code ?? '').trim().toUpperCase()) {
		case 'AA': return 'ACK (Accept)';
		case 'AE': return 'NACK (Application Error)';
		case 'AR': return 'NACK (Application Reject)';
		case 'CA': return 'Commit ACK (Accept)';
		case 'CE': return 'Commit NACK (Error)';
		case 'CR': return 'Commit NACK (Reject)';
		default: return 'Response';
	}
}

/** The label over an MLLP reply: its MSA-1 (see `ackLabel`), unless it is
 *  the ACK of another message, or an acceptance naming no message, whose
 *  code says nothing about this one. */
export function replyLabel(
	code: string | null | undefined,
	problem: ReplyProblem | null | undefined,
	translate: (key: string) => string,
): string {
	if (problem?.kind === 'other_message') return translate('comm.replyLabelOtherMessage');
	if (problem?.kind === 'unmatched') return translate('comm.replyLabelUnmatched');
	return ackLabel(code);
}

/** Why an MLLP reply accepts nothing sent, in the user's language. */
export function replyProblemText(
	problem: ReplyProblem,
	translate: (key: string, params?: Record<string, string | number>) => string,
): string {
	switch (problem.kind) {
		case 'other_message':
			return translate('comm.replyOtherMessage', { acknowledged: problem.acknowledged, sent: problem.sent });
		case 'unmatched':
			return translate('comm.replyUnmatched', { sent: problem.sent });
		default:
			return translate('comm.replyNotAck');
	}
}

/** Whether an HTTP method sends the active tab's message when the Body
 *  box is empty: GET and DELETE send a body only when one is typed. */
export function methodSendsMessage(method: string): boolean {
	return !['GET', 'DELETE', 'HEAD'].includes(method.toUpperCase());
}

/** The body of an HTTP request: the typed one, else the active message
 *  for methods that carry one, else none. */
export function httpBodyFor(method: string, typed: string, message: string): string | undefined {
	if (typed.trim()) return typed.trim();
	return methodSendsMessage(method) && message ? message : undefined;
}

/** The line under an HTTP result whose 3xx was not followed, saying why:
 *  'Follow Redirects' was off for the request, or the redirect points to
 *  another server or from https to http (the only ones not followed when
 *  it is on). Null when there is no redirect to speak of. */
export function unfollowedRedirectText(
	status: number,
	location: string | undefined,
	followed: boolean,
	translate: (key: string, params?: Record<string, string | number>) => string,
): string | null {
	if (status < 300 || status >= 400 || !location) return null;
	return translate(followed ? 'comm.redirectNotFollowed' : 'comm.redirectFollowOff', { url: location });
}

/** Why the listener cannot start with this port and read timeout, as the
 *  i18n key of the text that says so; null when it can. A cleared number
 *  box binds as null (or NaN), and a fraction or a port past 65535 is no
 *  number the backend takes: sent as they were, each failed with a raw IPC
 *  error. Port 0 is let through: the system picks a free port, and the
 *  status then shows the one it picked (see `listenHost`). */
export function listenSettingsProblem(port: unknown, readTimeoutSecs: unknown): string | null {
	const whole = (v: unknown, min: number, max: number) =>
		typeof v === 'number' && Number.isInteger(v) && v >= min && v <= max;
	if (!whole(port, 0, 65535)) return 'comm.listenPortInvalid';
	if (!whole(readTimeoutSecs, 1, 600)) return 'comm.listenReadTimeoutInvalid';
	return null;
}

/** Whether a "Listen on" value keeps the listener on this computer only:
 *  empty (the backend's default), `localhost`, or a loopback address in any
 *  spelling, `[::1]` and `127.0.0.2` included. Anything else lets other
 *  machines connect. */
export function isLocalBindAddress(address: string): boolean {
	let a = address.trim().toLowerCase();
	if (a.startsWith('[') && a.endsWith(']')) a = a.slice(1, -1);
	if (a === '' || a === 'localhost' || a.startsWith('127.')) return true;
	if (a.startsWith('::ffff:127.')) return true;
	if (!a.includes(':')) return false;
	// IPv6 loopback (::1), however many zeros are written out.
	const halves = a.split('::');
	if (halves.length > 2) return false;
	const head = halves[0] ? halves[0].split(':') : [];
	const tail = halves.length === 2 && halves[1] ? halves[1].split(':') : [];
	const groups = halves.length === 2 ? [...head, ...Array(Math.max(0, 8 - head.length - tail.length)).fill('0'), ...tail] : head;
	return groups.length === 8 && groups.slice(0, 7).every((g) => /^0{1,4}$/.test(g)) && /^0{0,3}1$/.test(groups[7]);
}

/** The listening address as the status shows it before `:port`: an IPv6
 *  address in brackets (`[::1]`, `[fe80::1%3]`), so the port that follows
 *  reads as the port, as a peer must be told it. */
export function listenHost(address: string | null | undefined): string {
	const a = (address ?? '').trim();
	return a.includes(':') && !a.startsWith('[') ? `[${a}]` : a;
}

/** The Authorization header of HTTP Basic authentication, with the user
 *  and password in UTF-8 (RFC 7617). `btoa` alone takes Latin-1 only: it
 *  threw on '€' or any non-Latin letter, and sent 'ä' as one Latin-1 byte
 *  that a UTF-8 server reads as a different password. */
export function basicAuthorization(user: string, password: string): string {
	let binary = '';
	for (const byte of new TextEncoder().encode(`${user}:${password}`)) binary += String.fromCharCode(byte);
	return 'Basic ' + btoa(binary);
}

/** Size of a message in bytes as UTF-8, with HL7 v2 segments ending in a
 *  single CR as they do on the wire, the last one too, and blank lines
 *  left out: the same rule as the backend's `to_wire_segments`
 *  (src-tauri/src/parser/hl7/mod.rs), whose header check needs a
 *  delimiter, not a letter, digit or space, after `MSH`. */
export function wireSize(message: string): number {
	const head = message.replace(/^[\uFEFF\r\n]+/, '');
	const fourth = head.codePointAt(3);
	const isV2 = /^(MSH|FHS|BHS)/.test(head)
		&& fourth !== undefined
		&& !/^[\p{Alphabetic}\p{N}\p{White_Space}]$/u.test(String.fromCodePoint(fourth));
	// Blank as Rust's str::trim sees it (JavaScript's trim also drops U+FEFF).
	const text = isV2
		? head.split(/[\r\n]/).filter((s) => !/^\p{White_Space}*$/u.test(s)).map((s) => s + '\r').join('')
		: message;
	return new TextEncoder().encode(text).length;
}

/** The text for a request that got no answer: why, in the user's language,
 *  when the backend could tell (a certificate this computer does not
 *  trust, a closed port, a name that does not resolve…), else the
 *  backend's own message. `translate` returns the key for a missing text. */
export function sendErrorText(
	error: string | null | undefined,
	failure: { kind: string; detail: string } | null | undefined,
	translate: (key: string, params?: Record<string, string | number>) => string,
): string {
	if (failure) {
		const key = `comm.sendError.${failure.kind}`;
		const text = translate(key, { detail: failure.detail });
		if (text !== key) return text;
	}
	return error ?? '';
}
