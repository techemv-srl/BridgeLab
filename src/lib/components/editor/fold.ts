/**
 * Compact display of long fields in the editor.
 *
 * The document (what is parsed, validated, saved and kept in the session) is
 * always the full text. The editor shows a *display* text in which every
 * long run — a base64 attachment in OBX-5, a JSON "data" string, an XML
 * value attribute — is replaced by a short fold token such as
 * `⟨Base64 · 5.1 KB #3⟩`. The token carries a numeric id; the run it stands
 * for is kept in a registry, so the full text is rebuilt exactly with
 * {@link expandText}, whatever the user did around the token.
 *
 * Tokens are self-identifying, which keeps undo/redo, copy/paste of a
 * token inside the editor, and deletion simple: a token that is present
 * expands to its payload, a token that was deleted is gone with it.
 * Ids are global so a token copied into another tab still expands.
 */

export type FoldMode = 'hl7v2' | 'json' | 'xml' | 'none';

const payloads = new Map<number, string>();
/** Identical runs share one id, so re-folding the same field (tab switch,
 *  fold/expand cycles) never stores its content twice. */
const idOf = new Map<string, number>();
/** The exact text of each id's token. Only that text is a token: a message
 *  can hold token-shaped text of its own (`⟨scan #1⟩` in a note), which
 *  has the shape and a registered id but not the label, and must stay as
 *  written rather than become another field's content on save. */
const tokenTexts = new Map<number, string>();
let nextId = 1;

/** A token: ⟨label #id⟩. The brackets are U+27E8/U+27E9, never used by HL7. */
export const TOKEN_RE = /⟨[^⟩\r\n]* #(\d+)⟩/g;

/** Register a payload and return its token. */
export function tokenFor(payload: string): string {
	let id = idOf.get(payload);
	if (id === undefined) {
		id = nextId++;
		payloads.set(id, payload);
		idOf.set(payload, id);
		tokenTexts.set(id, `⟨${describe(payload)} #${id}⟩`);
	}
	return tokenTexts.get(id)!;
}

/** The payload `match` (a TOKEN_RE match with id `id`) stands for: only
 *  when it is exactly that id's token. */
function payloadFor(match: string, id: number): string | undefined {
	return tokenTexts.get(id) === match ? payloads.get(id) : undefined;
}

function forget(id: number, payload: string): void {
	payloads.delete(id);
	idOf.delete(payload);
	tokenTexts.delete(id);
}

/**
 * Forget every payload whose token is not in `text`. Call it when the text
 * is replaced wholesale (a new document, another tab): the editor's undo
 * history is reset then, so nothing can bring an old token back.
 */
export function retainOnly(text: string): void {
	const live = new Set(tokensIn(text).map((t) => t.id));
	for (const [id, payload] of payloads) {
		if (!live.has(id)) forget(id, payload);
	}
}

/** Keep only the payloads whose ids are in `live`: the tokens some open
 *  document shows or may bring back through its undo history. */
export function retainIds(live: Set<number>): void {
	for (const [id, payload] of payloads) {
		if (!live.has(id)) forget(id, payload);
	}
}

/** How many payloads are registered (tests). */
export function registrySize(): number {
	return payloads.size;
}

/** The payload a token id stands for, if this session created it. */
export function payloadOf(id: number): string | undefined {
	return payloads.get(id);
}

/** "Base64 · 5.1 KB", "Text · 812 B" — the chip's label. */
export function describe(payload: string): string {
	return `${kindOf(payload)} · ${formatSize(payload.length)}`;
}

function kindOf(s: string): string {
	// Hex first: its alphabet is a subset of base64's.
	if (s.length >= 16 && /^[0-9A-Fa-f]+$/.test(s)) return 'Hex';
	if (s.length >= 16 && /^[A-Za-z0-9+/]+={0,2}$/.test(s)) return 'Base64';
	return 'Text';
}

export function formatSize(n: number): string {
	if (n < 1024) return `${n} B`;
	if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
	return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

/** Rebuild the full text: every known token becomes its payload. */
export function expandText(display: string): string {
	if (!display.includes('⟨')) return display;
	return display.replace(TOKEN_RE, (tok, id) => payloadFor(tok, Number(id)) ?? tok);
}

/** The known tokens in a text, with their offsets. */
export function tokensIn(text: string): Array<{ index: number; length: number; id: number; payload: string }> {
	const out: Array<{ index: number; length: number; id: number; payload: string }> = [];
	if (!text.includes('⟨')) return out;
	for (const m of text.matchAll(TOKEN_RE)) {
		const payload = payloadFor(m[0], Number(m[1]));
		if (payload !== undefined) out.push({ index: m.index!, length: m[0].length, id: Number(m[1]), payload });
	}
	return out;
}

/**
 * Which of `tokens` a context-menu action on the caret means: the one the
 * caret is on or against (a caret is snapped to a token's edge), else the
 * nearest. Measured to each token's nearer edge, not its start: with two
 * folded fields side by side, a caret at the end of the first is closer to
 * the second one's start than to its own. -1 when there is none.
 */
export function tokenNearCaret(tokens: ReadonlyArray<{ index: number; length: number }>, caret: number): number {
	let best = -1;
	let bestDist = Infinity;
	tokens.forEach((f, i) => {
		const d = caret < f.index ? f.index - caret : caret > f.index + f.length ? caret - (f.index + f.length) : 0;
		if (d < bestDist) { best = i; bestDist = d; }
	});
	return best;
}

/** Replace every run longer than `threshold` with a token. */
export function collapseText(full: string, mode: FoldMode, threshold: number): string {
	if (mode === 'none' || threshold <= 0 || full.length <= threshold) return full;
	if (mode === 'json') return collapseJson(full, threshold);
	if (mode === 'xml') return collapseXml(full, threshold);
	return collapseHl7(full, threshold);
}

function collapseHl7(full: string, threshold: number): string {
	// Delimiters from the first MSH (or FHS/BHS) header, standard otherwise.
	const header = /(?:^|[\r\n])(MSH|FHS|BHS)(.)(.)(.)(.)(.)/.exec(full);
	const fs = header?.[2] ?? '|';
	const seps = new Set([fs, header?.[3] ?? '^', header?.[4] ?? '~', header?.[6] ?? '&']);
	return full.replace(/[^\r\n]+/g, (line) => {
		if (line.length <= threshold) return line;
		// MSH-1/MSH-2 are the delimiters themselves: never fold them.
		const keep = /^(MSH|FHS|BHS)/.test(line) ? 8 : 0;
		let out = line.slice(0, keep);
		let run = '';
		const flush = () => {
			out += run.length > threshold ? tokenFor(run) : run;
			run = '';
		};
		for (let i = keep; i < line.length; i++) {
			const c = line[i];
			if (seps.has(c)) {
				flush();
				out += c;
			} else {
				run += c;
			}
		}
		flush();
		return out;
	});
}

function collapseJson(full: string, threshold: number): string {
	return full.replace(/"((?:[^"\\]|\\.)*)"/g, (whole, inner: string) =>
		inner.length > threshold ? `"${tokenFor(inner)}"` : whole);
}

function collapseXml(full: string, threshold: number): string {
	return full
		.replace(/(=\s*")([^"]*)(")/g, (whole, a, inner: string, b) =>
			inner.length > threshold ? `${a}${tokenFor(inner)}${b}` : whole)
		.replace(/(>)([^<]+)(<)/g, (whole, a, inner: string, b) =>
			inner.trim().length > threshold ? `${a}${tokenFor(inner)}${b}` : whole);
}

/**
 * Map a 1-based column in the full line to the display line. A column
 * inside a folded run maps to the token's start.
 */
export function fullToDisplayCol(displayLine: string, fullCol: number): number {
	let delta = 0; // full length - display length so far
	for (const t of tokensIn(displayLine)) {
		const fullStart = t.index + delta; // 0-based
		if (fullCol - 1 < fullStart) break;
		if (fullCol - 1 < fullStart + t.payload.length) return t.index + 1;
		delta += t.payload.length - t.length;
	}
	return fullCol - delta;
}

/** Map a 1-based display column to the full line (a column inside a token maps to its payload start). */
export function displayToFullCol(displayLine: string, displayCol: number): number {
	let delta = 0;
	for (const t of tokensIn(displayLine)) {
		if (displayCol - 1 < t.index) break;
		if (displayCol - 1 < t.index + t.length) return t.index + delta + 1;
		delta += t.payload.length - t.length;
	}
	return displayCol + delta;
}

/**
 * A test for the find widget's query, `null` when there is nothing to look
 * for (or the regular expression does not compile). "Whole word" is left
 * out on purpose: this decides which folds to open, and opening one too
 * many only changes the view.
 */
export function searchMatcher(search: string, opts: { isRegex: boolean; matchCase: boolean }): ((s: string) => boolean) | null {
	if (!search) return null;
	const source = opts.isRegex ? search : search.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
	let re: RegExp;
	try {
		re = new RegExp(source, opts.matchCase ? 'm' : 'im');
	} catch {
		return null;
	}
	return (s) => re.test(s);
}

/** The tokens in `text` whose content matches, or whose own chip text
 *  does (a replace there would rewrite the chip instead of the field). */
export function tokensMatching(text: string, matches: (s: string) => boolean) {
	return tokensIn(text).filter((t) => matches(t.payload) || matches(text.slice(t.index, t.index + t.length)));
}
