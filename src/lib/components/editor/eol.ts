/**
 * Line endings of a document in the editor.
 *
 * Monaco keeps one line ending per model, LF or CR LF, and reads a bare CR
 * (the HL7 segment terminator) as a line break it then writes its own way.
 * The editor gives the text back with the line endings the document had,
 * so saving never rewrites them: one ending for the whole document in the
 * usual case, one per line when the file mixes them (CR between segments
 * and CR LF after the last, LF and CR LF and CR from several tools).
 */

export type Eol = '\r' | '\n' | '\r\n';

const BREAK = /\r\n|\r|\n/g;

/** The first line ending in `text`, or null when it has none. */
export function firstEol(text: string): Eol | null {
	const i = text.search(/\r\n|\r|\n/);
	if (i < 0) return null;
	return text[i] === '\n' ? '\n' : text[i + 1] === '\n' ? '\r\n' : '\r';
}

/**
 * The line ending of a document that had none when it was opened, once
 * lines are added: CR for an HL7 v2 message (MSH, or FHS/BHS for a batch,
 * followed by its field separator), whose segments end with CR; LF for
 * anything else. Decided by the text alone: not by the operating system
 * (Monaco's default is CR LF on Windows) or by the document shown before.
 */
export function defaultEol(text: string): Eol {
	const head = text.replace(/^[﻿\s\v\f\x1c]+/, '');
	return /^(MSH|FHS|BHS)[^\p{L}\p{N}\s]/u.test(head) ? '\r' : '\n';
}

/** Every line ending of `text`, in order; null when they are all the same
 *  (or there are none), which one ending for the document describes. */
export function mixedEols(text: string): Eol[] | null {
	const ends = (text.match(BREAK) ?? []) as Eol[];
	return ends.some((e) => e !== ends[0]) ? ends : null;
}

/** One edit as Monaco reports it: lines `startLine`..`endLine` (1-based)
 *  partly or wholly replaced by `text`. */
export interface LineEdit {
	startLine: number;
	endLine: number;
	text: string;
}

/**
 * The line endings after `edits`, given in Monaco's order (from the end of
 * the document to the start, so each applies to the text as it was). An
 * edit removes the endings inside its range and adds one per line break it
 * inserts: the endings it keeps stay as they were, in order, and new lines
 * get `eol`. Replacing lines by as many lines (folding a field, a find and
 * replace within lines) keeps every ending.
 */
export function applyLineEdits(ends: readonly Eol[], edits: readonly LineEdit[], eol: Eol): readonly Eol[] {
	// Typing within lines changes no ending: the same array, not a copy, so
	// the versions kept for undo (see LineEndings) share it.
	const lineCount = (e: LineEdit) => (e.text.match(BREAK) ?? []).length;
	if (edits.every((e) => lineCount(e) === Math.max(0, e.endLine - e.startLine))) return ends;
	const out = [...ends];
	for (const edit of edits) {
		const removed = Math.max(0, edit.endLine - edit.startLine);
		const added = (edit.text.match(BREAK) ?? []).length;
		const kept = out.slice(edit.startLine - 1, edit.startLine - 1 + Math.min(removed, added));
		const fresh = Array<Eol>(Math.max(0, added - removed)).fill(eol);
		out.splice(edit.startLine - 1, removed, ...kept, ...fresh);
	}
	return out;
}

/**
 * The line endings of a document that mixes them, kept for each version
 * of its Monaco model. An undo is reported as the inverse edit: a deleted
 * line break comes back as a newly typed one, which `applyLineEdits` can
 * only give the document's first ending, so a CR LF deleted and restored
 * among LF lines came back as LF. Undo and redo return the model to a
 * version it had (its alternative version id), and the endings that
 * version had come back with it.
 */
export class LineEndings {
	private readonly byVersion = new Map<number, readonly Eol[]>();

	constructor(private current: readonly Eol[], private readonly limit = 1000) {}

	get ends(): readonly Eol[] {
		return this.current;
	}

	/** The model is at `version` with the current endings (just loaded). */
	at(version: number): void {
		this.remember(version);
	}

	/** `edits` took the model to `version`; `undoRedo` when an undo or a
	 *  redo did. */
	edit(version: number, edits: readonly LineEdit[], eol: Eol, undoRedo: boolean): void {
		const known = undoRedo ? this.byVersion.get(version) : undefined;
		this.current = known ?? applyLineEdits(this.current, edits, eol);
		this.remember(version);
	}

	private remember(version: number): void {
		this.byVersion.delete(version);
		this.byVersion.set(version, this.current);
		// A bounded history: past it, an undo falls back to applyLineEdits.
		if (this.byVersion.size > this.limit) {
			const oldest = this.byVersion.keys().next().value;
			if (oldest !== undefined) this.byVersion.delete(oldest);
		}
	}
}

/** `text` (lines as Monaco gives them, LF or CR LF) with each line ending
 *  put back from `ends`; null when the line count no longer matches. */
export function joinLines(text: string, ends: readonly Eol[]): string | null {
	const lines = text.split(/\r\n|\n/);
	if (lines.length - 1 !== ends.length) return null;
	let out = lines[0];
	for (let i = 0; i < ends.length; i++) out += ends[i] + lines[i + 1];
	return out;
}

/** `text` (lines as Monaco gives them) with every line ending `eol`. */
export function withEol(text: string, eol: Eol, modelEol: string): string {
	if (eol === modelEol) return text;
	return eol === '\n' ? text.replace(/\r\n/g, '\n') : text.replace(/\r\n|\n/g, eol);
}
