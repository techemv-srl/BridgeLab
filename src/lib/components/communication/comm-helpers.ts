/** Pure helpers of the Communication panel, kept apart to be tested. */

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

/** Size of a message in bytes as UTF-8, with HL7 v2 segments ending in a
 *  single CR as they do on the wire. */
export function wireSize(message: string): number {
	const text = /^﻿?[\r\n]*(MSH|FHS|BHS)/.test(message)
		? message.replace(/^﻿?[\r\n]+/, '').replace(/\r\n/g, '\r').replace(/\n/g, '\r')
		: message;
	return new TextEncoder().encode(text).length;
}
