import type { ParseResult } from '$lib/types/hl7';

/** Why a Tools command (Anonymize, Copy Truncated, Export) did not run. */
export type ToolRefusal =
	| { reason: 'empty' }
	| { reason: 'fhir' }
	| { reason: 'notHl7' }
	| { reason: 'parseFailed'; error: string }
	/** The tab was edited, switched or closed while its text was parsed:
	 *  the parse no longer describes what is on screen. */
	| { reason: 'changed' };

export type ToolTarget = { ok: true; tabId: string; result: ParseResult } | ({ ok: false } & ToolRefusal);

export interface ToolTargetDeps {
	parse: (content: string) => Promise<ParseResult>;
	/** The tab on screen now, and its text (null when there is none). */
	current: () => { id: string; content: string } | null;
	looksLikeHl7: (trimmed: string) => boolean;
	looksLikeFhir: (trimmed: string) => boolean;
}

/**
 * The HL7 v2 message a Tools command works on: the text in the editor of
 * `tab`, parsed now. Never the tab's previous parse: with auto-parse off,
 * or once the header was deleted, that describes an older message and the
 * command would act on its patient. The result is bound to the tab the
 * command started on; if the user switched tabs or edited meanwhile it is
 * refused rather than applied to whatever tab is active.
 */
export async function hl7ToolTarget(
	tab: { id: string; content: string } | null,
	deps: ToolTargetDeps,
): Promise<ToolTarget> {
	const content = tab?.content ?? '';
	const trimmed = content.trim();
	if (!tab || !trimmed) return { ok: false, reason: 'empty' };
	if (!deps.looksLikeHl7(trimmed)) {
		return { ok: false, reason: deps.looksLikeFhir(trimmed) ? 'fhir' : 'notHl7' };
	}
	let result: ParseResult;
	try {
		result = await deps.parse(content);
	} catch (e) {
		return { ok: false, reason: 'parseFailed', error: String(e) };
	}
	const now = deps.current();
	if (!now || now.id !== tab.id || now.content !== content) return { ok: false, reason: 'changed' };
	if (result.format?.startsWith('FHIR')) return { ok: false, reason: 'fhir' };
	return { ok: true, tabId: tab.id, result };
}
