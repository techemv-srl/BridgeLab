<script lang="ts">
	import { untrack, onDestroy } from 'svelte';
	import { registerHL7Language } from './HL7MonarchLanguage';
	import { registerHL7AutoComplete } from './HL7AutoComplete';
	import { t, subscribeLocale } from '$lib/i18n';
	import { collapseText, expandText, tokensIn, describe, fullToDisplayCol, displayToFullCol, tokenFor, retainIds, tokensMatching, searchMatcher, type FoldMode } from './fold';
	// Monaco web workers, bundled by Vite. The JSON language service needs
	// its own worker: handing it the generic editor worker makes that worker
	// try to AMD-load the JSON module ("undefined is not an object
	// (evaluating 'require.toUrl')") and FHIR JSON tabs lose validation.
	import EditorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
	import JsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker';

	type MonacoModule = typeof import('monaco-editor');
	type IStandaloneCodeEditor = import('monaco-editor').editor.IStandaloneCodeEditor;
	type IDisposable = import('monaco-editor').IDisposable;

	/** User-tunable editor options, loaded from preferences by the host.
	 *  Every field optional — absent fields keep the built-in default. */
	import { toMonacoOptions, type EditorOptions } from './monaco-options';

	interface Props {
		content?: string;
		language?: string;
		theme?: string;
		readonly?: boolean;
		/** Preference-driven options; changes apply live via updateOptions. */
		options?: EditorOptions;
		/** Always the FULL text: folded runs are expanded before it is reported. */
		onContentChange?: (value: string) => void;
		onCursorChange?: (line: number, column: number) => void;
		/** Show Segment in Tree: the caret's line and full-text column. */
		onNavigateToSegment?: (lineNumber: number, column: number) => void;
		/** Runs longer than this many characters are shown folded (0 = never). */
		foldThreshold?: number;
		/** How many folded runs the editor currently shows. */
		onFoldCountChange?: (count: number) => void;
		onCopyFullMessage?: () => void;
		onCopyTruncatedMessage?: () => void;
		/** External navigation request (e.g., from tree). Stamp forces re-trigger on identical targets. */
		navigation?: { line: number; column: number; selectionLength: number; stamp: number } | null;
		/** Identifies the document (the tab): a new one gets its own caret back. */
		docKey?: string;
		/** Where the caret goes when a document is shown (full-text column). */
		cursorLine?: number;
		cursorColumn?: number;
		/** The documents still open (tab ids): the editor keeps a model for
		 *  each, with its own undo history, and drops the model of one that
		 *  is no longer listed. */
		openDocs?: string[];
	}

	let {
		content = '',
		language = 'hl7v2',
		theme = 'bridgelab-dark',
		readonly = false,
		options = {},
		onContentChange,
		onCursorChange,
		onNavigateToSegment,
		foldThreshold = 100,
		onFoldCountChange,
		onCopyFullMessage,
		onCopyTruncatedMessage,
		navigation = null,
		docKey = '',
		cursorLine = 1,
		cursorColumn = 1,
		openDocs,
	}: Props = $props();

	let containerEl = $state<HTMLDivElement | undefined>(undefined);
	let editor = $state<IStandaloneCodeEditor | undefined>(undefined);
	let monacoMod = $state<MonacoModule | undefined>(undefined);
	let initError = $state('');
	let isUpdatingFromProp = false;
	let initializing = false;
	let actionDisposables: IDisposable[] = [];

	// ---- Folded long runs ---------------------------------------------------
	// The model holds a *display* text where long runs are fold tokens (see
	// fold.ts); everything the editor reports outward is the full text.
	let foldDecorations: import('monaco-editor').editor.IEditorDecorationsCollection | undefined;
	let foldRefreshTimer: ReturnType<typeof setTimeout> | null = null;

	const foldMode = $derived<FoldMode>(language === 'json' ? 'json' : language === 'xml' ? 'xml' : language === 'hl7v2' ? 'hl7v2' : 'none');

	// Monaco knows only LF and CRLF line ends: a bare CR (the HL7 segment
	// terminator) comes back as CRLF. Remember what the document used and
	// give the text back that way, so saving never rewrites terminators.
	let docEol: '\r' | '\n' | '\r\n' = '\n';

	function eolOf(text: string): '\r' | '\n' | '\r\n' {
		const i = text.search(/\r\n|\r|\n/);
		if (i < 0) return docEol;
		return text[i] === '\n' ? '\n' : text[i + 1] === '\n' ? '\r\n' : '\r';
	}

	function outward(modelText: string): string {
		const full = expandText(modelText);
		return docEol === '\r' ? full.replace(/\r\n|\n/g, '\r') : full;
	}

	function fullText(): string {
		return outward(editor?.getValue() ?? '');
	}

	// ---- One model per document ---------------------------------------------
	// Each tab keeps its own Monaco model: switching tabs swaps models instead
	// of replacing the text, so every document keeps its undo/redo history,
	// caret, scroll and highlighting, and a switch costs no re-tokenising.
	interface DocState {
		model: import('monaco-editor').editor.ITextModel;
		view: import('monaco-editor').editor.ICodeEditorViewState | null;
		eol: '\r' | '\n' | '\r\n';
		/** Fold tokens this document has shown: its undo history may bring
		 *  any of them back, so their payloads live as long as the document. */
		used: Set<number>;
	}
	const docs = new Map<string, DocState>();

	function currentDoc(): DocState | undefined {
		return appliedDocKey !== undefined ? docs.get(appliedDocKey) : undefined;
	}

	function noteTokens(ids: Iterable<number>) {
		const doc = currentDoc();
		if (doc) for (const id of ids) doc.used.add(id);
	}

	/** Show document `key`: its own model, or a new one holding `val`. */
	function showDoc(key: string, val: string, mode: FoldMode, threshold: number, line: number, col: number) {
		if (!editor || !monacoMod) return;
		const prev = currentDoc();
		if (prev) {
			prev.view = editor.saveViewState();
			prev.eol = docEol;
		}
		let doc = docs.get(key);
		const fresh = !doc;
		if (!doc) {
			doc = { model: monacoMod.editor.createModel('', language), view: null, eol: eolOf(val), used: new Set() };
			docs.set(key, doc);
		}
		appliedDocKey = key;
		isUpdatingFromProp = true;
		try {
			editor.setModel(doc.model);
		} finally {
			isUpdatingFromProp = false;
		}
		docEol = doc.eol;
		const refold = appliedThreshold !== undefined && (threshold !== appliedThreshold || mode !== appliedMode);
		if (fresh || refold || fullText() !== val) {
			docEol = eolOf(val);
			setDisplay(collapseText(val, mode, threshold));
			placeCaret(line, col);
		} else {
			if (doc.view) editor.restoreViewState(doc.view);
			else placeCaret(line, col);
			refreshFolds();
		}
	}

	/** Drop the models of documents no longer open, and the fold payloads
	 *  only they used. */
	function pruneDocs(open: string[]) {
		const keep = new Set(open);
		let dropped = false;
		for (const [key, doc] of docs) {
			if (keep.has(key) || key === appliedDocKey) continue;
			doc.model.dispose();
			docs.delete(key);
			dropped = true;
		}
		if (dropped) {
			const live = new Set<number>();
			for (const doc of docs.values()) for (const id of doc.used) live.add(id);
			retainIds(live);
		}
	}

	$effect(() => {
		const open = openDocs;
		if (open && editor) untrack(() => pruneDocs(open));
	});

	onDestroy(() => {
		for (const doc of docs.values()) doc.model.dispose();
		docs.clear();
	});

	/** Replace the model text without it counting as a user edit. Resets the
	 *  document's undo history (a new or reloaded document). */
	function setDisplay(display: string) {
		if (!editor) return;
		isUpdatingFromProp = true;
		try {
			editor.setValue(display);
		} finally {
			isUpdatingFromProp = false;
		}
		refreshFolds();
	}

	/** Put the caret at a full-text position, clamped to the document. */
	function placeCaret(line: number, column: number) {
		const model = editor?.getModel();
		if (!editor || !model) return;
		const ln = Math.min(Math.max(1, line), model.getLineCount());
		const col = fullToDisplayCol(model.getLineContent(ln), Math.max(1, column));
		editor.setPosition({ lineNumber: ln, column: col });
		editor.revealPositionInCenterIfOutsideViewport({ lineNumber: ln, column: col });
	}

	/** Replace the whole text as one undoable edit (expand/collapse all). */
	function replaceAllUndoable(display: string) {
		const model = editor?.getModel();
		if (!editor || !model || model.getValue() === display) return;
		const pos = editor.getPosition();
		const fullCol = pos ? displayToFullCol(model.getLineContent(pos.lineNumber), pos.column) : 1;
		model.pushEditOperations(editor.getSelections() ?? [], [{ range: model.getFullModelRange(), text: display }], () => null);
		model.pushStackElement();
		if (pos) {
			const line = Math.min(pos.lineNumber, model.getLineCount());
			editor.setPosition({ lineNumber: line, column: fullToDisplayCol(model.getLineContent(line), fullCol) });
		}
	}

	function refreshFolds() {
		const model = editor?.getModel();
		if (!editor || !model || !monacoMod) return;
		const found = tokensIn(model.getValue());
		noteTokens(found.map((f) => f.id));
		const decos = found.map((f) => {
			const start = model.getPositionAt(f.index);
			const end = model.getPositionAt(f.index + f.length);
			const preview = f.payload.slice(0, 120).replace(/`/g, "'");
			return {
				range: new monacoMod!.Range(start.lineNumber, start.column, end.lineNumber, end.column),
				options: {
					inlineClassName: 'bl-fold',
					hoverMessage: { value: `**${describe(f.payload)}** — ${t('fold.hint')}

\`${preview}${f.payload.length > 120 ? '…' : ''}\`` },
					stickiness: monacoMod!.editor.TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
				},
			};
		});
		if (!foldDecorations) foldDecorations = editor.createDecorationsCollection(decos);
		else foldDecorations.set(decos);
		onFoldCountChange?.(found.length);
	}

	function scheduleFoldRefresh() {
		if (foldRefreshTimer) clearTimeout(foldRefreshTimer);
		foldRefreshTimer = setTimeout(refreshFolds, 60);
	}

	/** The token containing model offset `off` (strictly inside, or anywhere when `inclusive`). */
	function tokenAt(off: number, inclusive = false) {
		const model = editor?.getModel();
		if (!model) return undefined;
		return tokensIn(model.getValue()).find((f) => inclusive
			? off >= f.index && off <= f.index + f.length
			: off > f.index && off < f.index + f.length);
	}

	function expandToken(tok: { index: number; length: number; payload: string }) {
		const model = editor?.getModel();
		if (!editor || !model || !monacoMod) return;
		const s = model.getPositionAt(tok.index);
		const e = model.getPositionAt(tok.index + tok.length);
		model.pushEditOperations(editor.getSelections() ?? [], [{ range: new monacoMod.Range(s.lineNumber, s.column, e.lineNumber, e.column), text: tok.payload }], () => null);
		model.pushStackElement();
		editor.setPosition(s);
	}

	/** Expand every folded run (one undo step). */
	export function expandAllFolds() {
		const model = editor?.getModel();
		if (model) replaceAllUndoable(expandText(model.getValue()));
	}

	/** Fold every run longer than the threshold (one undo step). */
	export function collapseAllFolds() {
		const model = editor?.getModel();
		if (model) replaceAllUndoable(collapseText(expandText(model.getValue()), foldMode, foldThreshold));
	}

	/** Keep selections from cutting a token in half: grow them to whole tokens. */
	function snapSelectionToTokens() {
		const model = editor?.getModel();
		const sel = editor?.getSelection();
		if (!editor || !model || !sel || sel.isEmpty() || !monacoMod) return;
		let a = model.getOffsetAt(sel.getStartPosition());
		let b = model.getOffsetAt(sel.getEndPosition());
		for (const f of tokensIn(model.getValue())) {
			if (a > f.index && a < f.index + f.length) a = f.index;
			if (b > f.index && b < f.index + f.length) b = f.index + f.length;
		}
		const s = model.getPositionAt(a);
		const e = model.getPositionAt(b);
		if (!sel.getStartPosition().equals(s) || !sel.getEndPosition().equals(e)) {
			editor.setSelection(new monacoMod.Selection(s.lineNumber, s.column, e.lineNumber, e.column));
		}
	}

	/** The find widget's state (Monaco's FindReplaceState, not in its public types). */
	interface FindState {
		searchString: string;
		isRegex: boolean;
		matchCase: boolean;
		isRevealed: boolean;
		onFindReplaceStateChange(cb: (e: { searchString?: boolean; isRegex?: boolean; matchCase?: boolean; isRevealed?: boolean }) => void): IDisposable;
	}

	/**
	 * Find and Replace work on the model, which holds fold chips: a match
	 * inside a folded field was not counted, Replace All left it (and could
	 * rewrite a chip's own text, which then no longer expanded). While the
	 * find widget searches, every chip whose content (or label) matches is
	 * expanded, so the widget counts, shows and replaces the real text.
	 */
	function hookFindWidget(ed: IStandaloneCodeEditor) {
		const ctrl = ed.getContribution('editor.contrib.findController') as unknown as { getState?: () => FindState } | null;
		const state = ctrl?.getState?.();
		if (!state) return;
		state.onFindReplaceStateChange((e) => {
			if (!(e.searchString || e.isRegex || e.matchCase || e.isRevealed)) return;
			if (!state.isRevealed) return;
			expandFoldsMatching(searchMatcher(state.searchString, state));
		});
	}

	function expandFoldsMatching(matches: ((s: string) => boolean) | null) {
		const model = editor?.getModel();
		if (!editor || !model || !monacoMod || !matches) return;
		const found = tokensMatching(model.getValue(), matches);
		if (!found.length) return;
		const mod = monacoMod;
		const edits = found.map((tok) => {
			const s = model.getPositionAt(tok.index);
			const e = model.getPositionAt(tok.index + tok.length);
			return { range: new mod.Range(s.lineNumber, s.column, e.lineNumber, e.column), text: tok.payload };
		});
		model.pushStackElement();
		model.pushEditOperations(editor.getSelections() ?? [], edits, () => null);
		model.pushStackElement();
	}

	// Initialize Monaco when container element becomes available
	$effect(() => {
		if (containerEl && !editor && !initializing && typeof window !== 'undefined') {
			initializing = true;
			initMonaco();
		}
	});

	function retryInit() {
		initError = '';
		initializing = true;
		initMonaco();
	}

	async function initMonaco() {
		try {
			const mod = await import('monaco-editor');
			monacoMod = mod;

			self.MonacoEnvironment = {
				getWorker(_: string, label: string) {
					return label === 'json' ? new JsonWorker() : new EditorWorker();
				}
			};

			registerHL7Language(mod);
			registerHL7AutoComplete(mod);
			// F1 is BridgeLab's manual everywhere, editor included. Monaco's own
			// command palette moves to Ctrl/Cmd+Shift+P (as in VS Code) and stays
			// in the editor's context menu.
			mod.editor.addKeybindingRules([
				{ keybinding: mod.KeyCode.F1, command: '-editor.action.quickCommand' },
				{ keybinding: mod.KeyMod.CtrlCmd | mod.KeyMod.Shift | mod.KeyCode.KeyP, command: 'editor.action.quickCommand' },
			]);

			// The first model is ours, like every later one: a model the editor
			// creates itself (the `value` option) is disposed by Monaco as soon
			// as the editor shows another, and switching back would fail.
			docEol = eolOf(content || '');
			const firstModel = mod.editor.createModel(collapseText(content || '', foldMode, foldThreshold), language);
			const ed = mod.editor.create(containerEl!, {
				// The first document is folded like every later one.
				model: firstModel,
				theme,
				readOnly: readonly,
				minimap: { enabled: true },
				fontSize: 13,
				fontFamily: "'JetBrains Mono', 'Fira Code', 'Consolas', monospace",
				lineNumbers: 'on',
				wordWrap: 'on',
				scrollBeyondLastLine: false,
				automaticLayout: true,
				renderLineHighlight: 'line',
				bracketPairColorization: { enabled: false },
				// Render hover/suggest widgets outside the editor bounds to avoid clipping
				fixedOverflowWidgets: true,
				hover: {
					enabled: true,
					above: false,  // Prefer showing below cursor to avoid top clipping
					delay: 300,
					sticky: true,
				},
				tabSize: 4,
				smoothScrolling: true,
				cursorBlinking: 'smooth',
				padding: { top: 8 },
				contextmenu: true,
				// Word suggestions from the document being edited only: with a
				// model per tab, the default scans every open document (each
				// copied into a worker), megabytes for large messages.
				wordBasedSuggestions: 'currentDocument',
				// Preference overrides win over the defaults above
				...toMonacoOptions(options),
			});

			ed.onDidChangeModelContent(() => {
				scheduleFoldRefresh();
				if (isUpdatingFromProp) return;
				onContentChange?.(outward(ed.getValue()));
			});

			// A folded run behaves as one unit: the caret never rests inside
			// a token (arrow keys jump over it) and a selection never cuts one.
			let lastCaret = 0;
			ed.onDidChangeCursorSelection((e) => {
				const model = ed.getModel();
				if (!model) return;
				if (!e.selection.isEmpty()) { snapSelectionToTokens(); return; }
				const off = model.getOffsetAt(e.selection.getPosition());
				const tok = tokenAt(off);
				if (tok) {
					const target = off >= lastCaret ? tok.index + tok.length : tok.index;
					lastCaret = target;
					ed.setPosition(model.getPositionAt(target));
					return;
				}
				lastCaret = off;
			});

			ed.onDidChangeCursorPosition((e) => {
				// setValue moves the caret to 1:1: that is not where the user
				// was, and reporting it would overwrite the tab's position.
				if (isUpdatingFromProp) return;
				const model = ed.getModel();
				const col = model ? displayToFullCol(model.getLineContent(e.position.lineNumber), e.position.column) : e.position.column;
				onCursorChange?.(e.position.lineNumber, col);
			});

			// Clicking a chip expands it; Enter on a chip too. Backspace/Delete
			// next to a chip select it first, so it is never half-deleted.
			ed.onMouseDown((e) => {
				const model = ed.getModel();
				const pos = e.target.position;
				if (!model || !pos || !e.event.leftButton || e.event.detail > 1) return;
				const off = model.getOffsetAt(pos);
				const tok = tokenAt(off) ?? tokensIn(model.getValue()).find((f) => off === f.index + 1);
				if (tok && (e.target.element?.classList.contains('bl-fold') || e.target.element?.closest?.('.bl-fold'))) {
					e.event.preventDefault();
					expandToken(tok);
				}
			});
			ed.onKeyDown((e) => {
				const model = ed.getModel();
				const sel = ed.getSelection();
				if (!model || !sel) return;
				if (!sel.isEmpty()) { snapSelectionToTokens(); return; }
				const off = model.getOffsetAt(sel.getPosition());
				const tokens = tokensIn(model.getValue());
				const after = tokens.find((f) => off === f.index);           // caret before a chip
				const before = tokens.find((f) => off === f.index + f.length); // caret after a chip
				const K = mod.KeyCode;
				const select = (f: { index: number; length: number }) => {
					const s = model.getPositionAt(f.index);
					const en = model.getPositionAt(f.index + f.length);
					ed.setSelection(new mod.Selection(s.lineNumber, s.column, en.lineNumber, en.column));
					e.preventDefault(); e.stopPropagation();
				};
				if (e.keyCode === K.Backspace && before) select(before);
				else if (e.keyCode === K.Delete && after) select(after);
				else if (e.keyCode === K.Enter && (after || before) && !e.shiftKey && e.altKey) {
					expandToken((after ?? before)!); e.preventDefault(); e.stopPropagation();
				}
			});

			// Pasted long runs fold like opened ones.
			ed.onDidPaste((e) => {
				const model = ed.getModel();
				if (!model || foldMode === 'none' || foldThreshold <= 0) return;
				const range = new mod.Range(e.range.startLineNumber, 1, e.range.endLineNumber, model.getLineMaxColumn(e.range.endLineNumber));
				const text = model.getValueInRange(range);
				const folded = collapseText(text, foldMode, foldThreshold);
				if (folded !== text) {
					model.pushEditOperations(ed.getSelections() ?? [], [{ range, text: folded }], () => null);
					model.pushStackElement();
				}
			});

			// Copy/cut put the FULL text on the clipboard, never a token.
			const onClipboard = (ev: ClipboardEvent) => {
				const model = ed.getModel();
				const sel = ed.getSelection();
				if (!model || !sel || !ed.hasTextFocus()) return;
				const text = sel.isEmpty()
					? model.getLineContent(sel.startLineNumber) + model.getEOL()
					: model.getValueInRange(sel);
				if (!tokensIn(text).length) return;
				ev.clipboardData?.setData('text/plain', expandText(text));
				ev.preventDefault();
			};
			containerEl!.addEventListener('copy', onClipboard);
			containerEl!.addEventListener('cut', onClipboard);

			// Add custom context menu actions
			addContextMenuActions(ed, mod);
			hookFindWidget(ed);

			editor = ed;
			appliedThreshold = foldThreshold;
			appliedMode = foldMode;
			appliedDocKey = docKey;
			docs.set(docKey, { model: firstModel, view: null, eol: docEol, used: new Set() });
			refreshFolds();
			placeCaret(cursorLine, cursorColumn);
			initError = '';
			// App shortcuts (Ctrl+L, Ctrl+K...) are taken in the capture phase
			// by the shell, before Monaco sees them; Monaco keeps every other key.

			ed.focus();
		} catch (err) {
			console.error('[Monaco] Init failed:', err);
			initError = String(err);
			initializing = false;
		}
	}

	// Re-register context-menu actions when the locale changes so the
	// right-click menu doesn't stay in the previous language until restart.
	if (typeof window !== 'undefined') {
		subscribeLocale(() => {
			if (editor && monacoMod) {
				actionDisposables.forEach((d) => d.dispose());
				actionDisposables = [];
				addContextMenuActions(editor, monacoMod);
			}
		});
	}

	function addContextMenuActions(ed: IStandaloneCodeEditor, mod: MonacoModule) {
		// "Show in Tree" action
		actionDisposables.push(ed.addAction({
			id: 'bridgelab.showInTree',
			label: t('ctx.showInTree'),
			contextMenuGroupId: 'navigation',
			contextMenuOrder: 1,
			keybindings: [mod.KeyMod.Alt | mod.KeyCode.KeyT],
			run: (editor) => {
				const pos = editor.getPosition();
				const model = editor.getModel();
				if (!pos || !model) return;
				// The host works out segment, field, repetition and component
				// from the full text (a folded run is one chip here).
				const col = displayToFullCol(model.getLineContent(pos.lineNumber), pos.column);
				onNavigateToSegment?.(pos.lineNumber, col);
			}
		}));

		// "Expand Truncated Field" action
		actionDisposables.push(ed.addAction({
			id: 'bridgelab.expandTruncated',
			label: t('ctx.expandField'),
			contextMenuGroupId: 'navigation',
			contextMenuOrder: 2,
			precondition: undefined,
			run: (editor) => {
				// Expand the folded run nearest the caret on its line.
				const pos = editor.getPosition();
				const model = editor.getModel();
				if (!pos || !model) return;
				const lineStart = model.getOffsetAt({ lineNumber: pos.lineNumber, column: 1 });
				const lineEnd = model.getOffsetAt({ lineNumber: pos.lineNumber, column: model.getLineMaxColumn(pos.lineNumber) });
				const caret = model.getOffsetAt(pos);
				const onLine = tokensIn(model.getValue()).filter((f) => f.index >= lineStart && f.index < lineEnd);
				if (!onLine.length) return;
				onLine.sort((a, b) => Math.abs(a.index - caret) - Math.abs(b.index - caret));
				expandToken(onLine[0]);
			}
		}));

		// "Expand All Truncated Fields" action
		actionDisposables.push(ed.addAction({
			id: 'bridgelab.expandAll',
			label: t('ctx.expandAll'),
			contextMenuGroupId: 'navigation',
			contextMenuOrder: 3,
			run: () => expandAllFolds(),
		}));

		// "Collapse All" action - re-truncate expanded fields
		actionDisposables.push(ed.addAction({
			id: 'bridgelab.collapseAll',
			label: t('ctx.collapseAll'),
			contextMenuGroupId: 'navigation',
			contextMenuOrder: 4,
			run: () => collapseAllFolds(),
		}));

		// "Fold this field": fold the run under the caret, whatever its length.
		actionDisposables.push(ed.addAction({
			id: 'bridgelab.foldField',
			label: t('ctx.foldField'),
			contextMenuGroupId: 'navigation',
			contextMenuOrder: 5,
			run: (editor) => {
				const model = editor.getModel();
				const pos = editor.getPosition();
				if (!model || !pos) return;
				const line = model.getLineContent(pos.lineNumber);
				const seps = foldMode === 'hl7v2' ? /[|^~&\r\n]/ : /["<>=\s]/;
				let a = pos.column - 1;
				let b = pos.column - 1;
				while (a > 0 && !seps.test(line[a - 1])) a--;
				while (b < line.length && !seps.test(line[b])) b++;
				const run = line.slice(a, b);
				if (run.length < 16 || run.includes('⟨')) return;
				model.pushEditOperations(editor.getSelections() ?? [], [{
					range: new mod.Range(pos.lineNumber, a + 1, pos.lineNumber, b + 1),
					text: tokenFor(run),
				}], () => null);
				model.pushStackElement();
			},
		}));

		// "Copy Full Message" action
		actionDisposables.push(ed.addAction({
			id: 'bridgelab.copyFullMessage',
			label: t('ctx.copyFull'),
			contextMenuGroupId: '9_cutcopypaste',
			contextMenuOrder: 8,
			run: () => {
				onCopyFullMessage?.();
			}
		}));

		// "Copy Truncated Message" action
		actionDisposables.push(ed.addAction({
			id: 'bridgelab.copyTruncatedMessage',
			label: t('ctx.copyTruncated'),
			contextMenuGroupId: '9_cutcopypaste',
			contextMenuOrder: 9,
			run: () => {
				onCopyTruncatedMessage?.();
			}
		}));

		// "Copy Line as Segment" action
		actionDisposables.push(ed.addAction({
			id: 'bridgelab.copySegment',
			label: t('ctx.copySegment'),
			contextMenuGroupId: '9_cutcopypaste',
			contextMenuOrder: 10,
			keybindings: [mod.KeyMod.Alt | mod.KeyCode.KeyC],
			run: (editor) => {
				const line = editor.getPosition()?.lineNumber;
				if (!line) return;
				// The full segment, never a fold chip.
				const lineContent = expandText(editor.getModel()?.getLineContent(line) ?? '');
				navigator.clipboard.writeText(lineContent).catch(() => { /* non-secure context */ });
			}
		}));
	}

	// No click handler - expand only via context menu to avoid accidental triggers

	// Sync content prop (FULL text) -> editor (display text with folds).
	// Only an outside change re-folds: the prop echoing our own edit is equal
	// to the expanded model and leaves the view alone.
	let appliedThreshold: number | undefined;
	let appliedMode: FoldMode | undefined;
	let appliedDocKey: string | undefined;
	$effect(() => {
		const val = content ?? '';
		const mode = foldMode;
		const threshold = foldThreshold;
		const key = docKey;
		if (editor && !isUpdatingFromProp) {
			// A new threshold or language re-folds the open document too.
			const refold = appliedThreshold !== undefined && (threshold !== appliedThreshold || mode !== appliedMode);
			const newDoc = key !== appliedDocKey;
			if (newDoc) {
				const [line, col] = untrack(() => [cursorLine, cursorColumn]);
				showDoc(key, val, mode, threshold, line, col);
			} else if (val !== fullText() || refold) {
				// setValue sends the caret to 1:1: put back the document's own
				// (tab switch, reload), read without making every caret move
				// re-run this.
				const [line, col] = untrack(() => [cursorLine, cursorColumn]);
				docEol = eolOf(val);
				setDisplay(collapseText(val, mode, threshold));
				placeCaret(line, col);
			}
			appliedThreshold = threshold;
			appliedMode = mode;
			appliedDocKey = key;
		}
	});

	// Theme sync
	$effect(() => {
		if (monacoMod && editor) {
			try { monacoMod.editor.setTheme(theme); } catch { /* ignore */ }
		}
	});

	// Preference-driven options sync (font, wrap, minimap, ...): apply live
	// so Settings changes take effect without reopening the tab.
	$effect(() => {
		const monacoOpts = toMonacoOptions(options);
		if (editor) {
			try { editor.updateOptions(monacoOpts); } catch { /* ignore */ }
		}
	});

	// Language sync: FHIR tabs need JSON/XML highlighting, not hl7v2.
	$effect(() => {
		const lang = language;
		if (editor && monacoMod) {
			const model = editor.getModel();
			if (model && model.getLanguageId() !== lang) {
				try { monacoMod.editor.setModelLanguage(model, lang); } catch { /* ignore */ }
			}
		}
	});

	// Readonly sync
	$effect(() => {
		const ro = readonly;
		if (editor) {
			try { editor.updateOptions({ readOnly: ro }); } catch { /* ignore */ }
		}
	});

	// External navigation sync: scroll + select the requested range
	let lastNavStamp = 0;
	$effect(() => {
		if (!navigation || !editor || !monacoMod) return;
		if (navigation.stamp === lastNavStamp) return;
		lastNavStamp = navigation.stamp;
		const { line, column, selectionLength } = navigation;
		try {
			// Navigation targets are full-text columns: map them onto the
			// display line, where a folded run is a single chip.
			const lineText = editor.getModel()?.getLineContent(line) ?? '';
			const startCol = fullToDisplayCol(lineText, column);
			const endCol = Math.max(startCol, fullToDisplayCol(lineText, column + selectionLength));
			editor.revealLineInCenter(line);
			editor.setPosition({ lineNumber: line, column: startCol });
			if (endCol > startCol) {
				editor.setSelection({
					startLineNumber: line,
					startColumn: startCol,
					endLineNumber: line,
					endColumn: endCol,
				});
			}
			editor.focus();
		} catch { /* ignore */ }
	});

	export function setValue(value: string) {
		docEol = eolOf(value);
		setDisplay(collapseText(value, foldMode, foldThreshold));
	}

	/** The full text (folded runs expanded). */
	export function getValue(): string {
		return fullText();
	}

	export function focus() {
		editor?.focus();
	}

	/**
	 * Insert `text` as a new line at line index `lineIdx` (0-based; the line
	 * count appends it), as one undoable edit that keeps the rest of the
	 * undo history. False when there is no editor to do it.
	 */
	export function insertLine(lineIdx: number, text: string): boolean {
		const model = editor?.getModel();
		if (!editor || !model || !monacoMod || readonly) return false;
		const eol = model.getEOL();
		const count = model.getLineCount();
		const append = lineIdx >= count;
		const line = append ? count : lineIdx + 1;
		const col = append ? model.getLineMaxColumn(count) : 1;
		model.pushStackElement();
		model.pushEditOperations(editor.getSelections() ?? [], [{
			range: new monacoMod.Range(line, col, line, col),
			text: append ? eol + text : text + eol,
		}], () => null);
		model.pushStackElement();
		const at = append ? count + 1 : line;
		editor.setPosition({ lineNumber: at, column: 1 });
		editor.revealLineInCenterIfOutsideViewport(at);
		return true;
	}

	/** True while the editor's text area has the keyboard focus. */
	export function hasTextFocus(): boolean {
		return editor?.hasTextFocus() ?? false;
	}

	/**
	 * Edit menu commands. document.execCommand cannot reach the editor from
	 * a menu click (and 'paste' is blocked for scripts), so they go through
	 * the editor and the async clipboard. Resolves false when the clipboard
	 * refused (the caller points the user to the keyboard shortcut).
	 */
	export async function runEditCommand(cmd: 'undo' | 'redo' | 'cut' | 'copy' | 'paste'): Promise<boolean> {
		const model = editor?.getModel();
		if (!editor || !model || !monacoMod) return true;
		editor.focus();
		if (cmd === 'undo' || cmd === 'redo') {
			editor.trigger('menu', cmd, null);
			return true;
		}
		const sel = editor.getSelection();
		if (!sel) return true;
		if (cmd === 'copy' || cmd === 'cut') {
			// Like Ctrl+C: no selection copies the whole line.
			const range = sel.isEmpty()
				? new monacoMod.Range(sel.startLineNumber, 1, sel.startLineNumber + 1, 1)
				: sel;
			const text = sel.isEmpty() && sel.startLineNumber === model.getLineCount()
				? model.getLineContent(sel.startLineNumber) + model.getEOL()
				: model.getValueInRange(range);
			try {
				await navigator.clipboard.writeText(expandText(text));
			} catch {
				return false;
			}
			if (cmd === 'cut' && !readonly) {
				editor.executeEdits('menu', [{ range, text: '' }]);
				editor.pushUndoStop();
			}
			return true;
		}
		let text: string;
		try {
			text = await navigator.clipboard.readText();
		} catch {
			return false;
		}
		if (readonly || !text) return true;
		const start = sel.getStartPosition();
		editor.executeEdits('menu', [{ range: sel, text }]);
		editor.pushUndoStop();
		// Long runs pasted from the menu fold like a Ctrl+V paste.
		const end = editor.getPosition() ?? start;
		if (foldMode !== 'none' && foldThreshold > 0) {
			const range = new monacoMod.Range(start.lineNumber, 1, end.lineNumber, model.getLineMaxColumn(end.lineNumber));
			const inserted = model.getValueInRange(range);
			const folded = collapseText(inserted, foldMode, foldThreshold);
			if (folded !== inserted) {
				model.pushEditOperations(editor.getSelections() ?? [], [{ range, text: folded }], () => null);
				model.pushStackElement();
			}
		}
		return true;
	}

	export function revealLine(line: number) {
		editor?.revealLineInCenter(line);
		editor?.setPosition({ lineNumber: line, column: 1 });
	}
</script>

<div class="editor-container" bind:this={containerEl}>
	{#if initError}
		<div class="editor-error">
			<div class="error-text">{t('editor.initFailed')}</div>
			<div class="error-detail">{initError}</div>
			<button class="retry-btn" onclick={retryInit}>{t('editor.retry')}</button>
		</div>
	{/if}
</div>

<style>
	.editor-container {
		width: 100%;
		height: 100%;
		min-height: 200px;
		position: relative;
	}

	.editor-error {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 8px;
		background: var(--color-bg-secondary);
		z-index: 5;
		padding: 20px;
		text-align: center;
	}

	.error-text { color: var(--color-error); font-weight: 600; }
	.error-detail { color: var(--color-text-secondary); font-size: 11px; max-width: 480px; word-break: break-word; }
	.retry-btn {
		padding: 5px 14px;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		background: var(--color-bg-tertiary);
		color: var(--color-text-primary);
		cursor: pointer;
		font-family: inherit;
	}
	.retry-btn:hover { background: var(--color-border); }

	/* A folded run: a chip in the text flow. Only colours and a box-shadow
	   outline, no padding or border width, so Monaco's column measurements
	   stay exact. */
	:global(.monaco-editor .bl-fold) {
		background: var(--color-fold-bg, rgba(86, 156, 214, 0.18));
		color: var(--color-fold-fg, #6cb6ff) !important;
		box-shadow: inset 0 0 0 1px var(--color-fold-border, rgba(86, 156, 214, 0.45));
		border-radius: 3px;
		cursor: pointer;
		font-style: normal;
	}
	:global(.monaco-editor .bl-fold:hover) {
		background: var(--color-fold-bg-hover, rgba(86, 156, 214, 0.32));
	}
</style>
