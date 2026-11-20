import { t } from '$lib/i18n';

/** Kinds open_file gives a file that does not parse (see
 *  `fileio::unparsed_kind`); each has an `file.unparsed.<kind>` string. */
const KINDS = new Set([
	'empty', 'missing_m', 'lowercase_header', 'short_header', 'bad_delimiters', 'no_header',
	'xml_not_fhir', 'invalid_json', 'json_not_fhir', 'fhir_invalid',
]);

/** Kinds whose English detail (a line and column, a parser message) is
 *  worth showing under the translated reason: which delimiter is wrong is
 *  said only there. */
const WITH_DETAIL = new Set(['invalid_json', 'fhir_invalid', 'bad_delimiters']);

/** The start of the first line the HL7 lexer looks at: after blanks, a
 *  byte-order mark and MLLP framing; control characters shown as `·`. */
export function firstLineStart(text: string, max = 24): string {
	// The same set the lexer skips (Hl7Lexer::message_start).
	const start = text.replace(/^\uFEFF/, '').replace(/^[ \t\u000B\u000C\u001C\r\n]+/, '');
	const line = start.split(/\r\n|\r|\n/, 1)[0] ?? '';
	const shown = line.length > max ? `${line.slice(0, max)}…` : line;
	return shown.replace(/[\u0000-\u001F\u007F]/g, '·');
}

/** Parameters of the `file.unparsed.*` strings: the start of the first
 *  line, and the header it names as written and upper-cased (`msh` and
 *  `MSH`, or `FHS`/`BHS` for a batch file). */
function headerParams(text: string): Record<string, string> {
	const start = firstLineStart(text);
	const found = start.slice(0, 3);
	return { start, found, header: found.toUpperCase() };
}

/** The "Opened as text" dialog text for a file that did not parse: the
 *  reason in the user's language with a hint when open_file classified it,
 *  else the English reason as it came. */
export function unparsedMessage(name: string, text: string, reason: string, kind?: string): string {
	if (!kind || !KINDS.has(kind)) {
		return t('file.openedUnparsed', { name, error: reason });
	}
	const explained = t(`file.unparsed.${kind}`, headerParams(text));
	return WITH_DETAIL.has(kind)
		? t('file.openedUnparsedDetail', { name, reason: explained, error: reason })
		: t('file.openedUnparsedKind', { name, reason: explained });
}

/** For the F6 report of a message whose header declares delimiters that
 *  cannot be used (a tab, a non-ASCII character, one character twice):
 *  the dialog's explanation as a sentence; null for any other error. */
export function delimiterHint(firstLine: string, parseError: string): string | null {
	if (!/^(MSH|FHS|BHS)/.test(firstLine) || !parseError.includes(' declares ')) return null;
	const s = t('file.unparsed.bad_delimiters', headerParams(firstLine));
	return s.charAt(0).toLocaleUpperCase() + s.slice(1);
}

/** For the F6 report of a message that does not parse: the dialog's hint
 *  when the first line is a mistyped header (`SH|…`, `msh|…`), as a
 *  sentence; null otherwise. */
export function headerTypoHint(firstLine: string): string | null {
	let kind: string | null = null;
	if (/^(msh|fhs|bhs)/i.test(firstLine) && !/^(MSH|FHS|BHS)/.test(firstLine)) {
		kind = 'lowercase_header';
	} else if (/^SH[^A-Za-z0-9]/.test(firstLine)) {
		kind = 'missing_m';
	}
	if (!kind) return null;
	const s = t(`file.unparsed.${kind}`, headerParams(firstLine));
	return s.charAt(0).toLocaleUpperCase() + s.slice(1);
}
