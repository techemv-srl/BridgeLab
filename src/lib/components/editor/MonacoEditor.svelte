<script lang="ts">
	import { registerHL7Language } from './HL7MonarchLanguage';
	import { registerHL7AutoComplete } from './HL7AutoComplete';
	import { t, subscribeLocale } from '$lib/i18n';

	type MonacoModule = typeof import('monaco-editor');
	type IStandaloneCodeEditor = import('monaco-editor').editor.IStandaloneCodeEditor;
	type IDisposable = import('monaco-editor').IDisposable;

	/** User-tunable editor options, loaded from preferences by the host.
	 *  Every field optional — absent fields keep the built-in default. */
	export interface EditorOptions {
		fontSize?: number;
		fontFamily?: string;
		wordWrap?: 'on' | 'off' | 'wordWrapColumn' | 'bounded';
		minimap?: boolean;
		lineNumbers?: boolean;
		tabSize?: number;
		renderWhitespace?: 'none' | 'boundary' | 'all';
		smoothScrolling?: boolean;
		cursorBlinking?: string;
		bracketPairColorization?: boolean;
	}

	interface Props {
		content?: string;
		language?: string;
		theme?: string;
		readonly?: boolean;
		/** Preference-driven options; changes apply live via updateOptions. */
		options?: EditorOptions;
		onContentChange?: (value: string) => void;
		onCursorChange?: (line: number, column: number) => void;
		onExpandTruncated?: (lineNumber: number, fieldPosition: string) => void;
		onExpandAll?: () => void;
		onNavigateToSegment?: (lineNumber: number, segmentType: string, fieldPosition?: number) => void;
		onCollapseAll?: () => void;
		onCopyFullMessage?: () => void;
		onCopyTruncatedMessage?: () => void;
		/** External navigation request (e.g., from tree). Stamp forces re-trigger on identical targets. */
		navigation?: { line: number; column: number; selectionLength: number; stamp: number } | null;
	}

	let {
		content = '',
		language = 'hl7v2',
		theme = 'bridgelab-dark',
		readonly = false,
		options = {},
		onContentChange,
		onCursorChange,
		onExpandTruncated,
		onExpandAll,
		onNavigateToSegment,
		onCollapseAll,
		onCopyFullMessage,
		onCopyTruncatedMessage,
		navigation = null,
	}: Props = $props();

	let containerEl = $state<HTMLDivElement | undefined>(undefined);
	let editor = $state<IStandaloneCodeEditor | undefined>(undefined);
	let monacoMod = $state<MonacoModule | undefined>(undefined);
	let initError = $state('');
	let isUpdatingFromProp = false;
	let initializing = false;
	let actionDisposables: IDisposable[] = [];

	/** Map preference options to Monaco's option shape. */
	function toMonacoOptions(o: EditorOptions) {
		return {
			...(o.fontSize !== undefined && { fontSize: o.fontSize }),
			...(o.fontFamily !== undefined && { fontFamily: o.fontFamily }),
			...(o.wordWrap !== undefined && { wordWrap: o.wordWrap }),
			...(o.minimap !== undefined && { minimap: { enabled: o.minimap } }),
			...(o.lineNumbers !== undefined && { lineNumbers: (o.lineNumbers ? 'on' : 'off') as 'on' | 'off' }),
			...(o.tabSize !== undefined && { tabSize: o.tabSize }),
			...(o.renderWhitespace !== undefined && { renderWhitespace: o.renderWhitespace }),
			...(o.smoothScrolling !== undefined && { smoothScrolling: o.smoothScrolling }),
			...(o.cursorBlinking !== undefined && { cursorBlinking: o.cursorBlinking as never }),
			...(o.bracketPairColorization !== undefined && { bracketPairColorization: { enabled: o.bracketPairColorization } }),
		};
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
				getWorker(_: string, _label: string) {
					return new Worker(
						new URL('monaco-editor/esm/vs/editor/editor.worker.js', import.meta.url),
						{ type: 'module' }
					);
				}
			};

			registerHL7Language(mod);
			registerHL7AutoComplete(mod);

			const ed = mod.editor.create(containerEl!, {
				value: content || '',
				language,
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
				// Preference overrides win over the defaults above
				...toMonacoOptions(options),
			});

			ed.onDidChangeModelContent(() => {
				if (isUpdatingFromProp) return;
				onContentChange?.(ed.getValue());
			});

			ed.onDidChangeCursorPosition((e) => {
				onCursorChange?.(e.position.lineNumber, e.position.column);
			});

			// Add custom context menu actions
			addContextMenuActions(ed, mod);

			editor = ed;
			initError = '';
			// Note: Monaco's default Ctrl+K (delete line), Ctrl+J (join), Ctrl+L are kept.
			// Users can reassign app shortcuts that conflict via Settings > Keyboard Shortcuts.
			// When Monaco has focus, its shortcuts take priority; click outside editor to use
			// app-level shortcuts like Ctrl+K for Communication panel.

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
				if (!pos) return;
				const lineContent = editor.getModel()?.getLineContent(pos.lineNumber) ?? '';
				const segType = lineContent.substring(0, 3);
				if (segType && /^[A-Z][A-Z0-9]{2}$/.test(segType)) {
					// Calculate field position based on pipe count before cursor column
					const textBefore = lineContent.substring(0, pos.column - 1);
					const pipeCount = (textBefore.match(/\|/g) || []).length;
					// MSH has offset: pipeCount == 1 means MSH-1 (the separator itself)
					const fieldPosition = pipeCount; // 0 = segment name, 1+ = field
					onNavigateToSegment?.(pos.lineNumber, segType, fieldPosition);
				}
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
				const pos = editor.getPosition();
				if (!pos) return;
				const lineContent = editor.getModel()?.getLineContent(pos.lineNumber) ?? '';

				// Find the truncation marker closest to cursor
				const markerRegex = /\{\.\.\.(\d+) bytes\}/g;
				let closestMarkerCol = -1;
				let closestDist = Infinity;
				let matchResult;
				while ((matchResult = markerRegex.exec(lineContent)) !== null) {
					const markerCenter = matchResult.index + matchResult[0].length / 2;
					const dist = Math.abs(pos.column - 1 - markerCenter); // pos.column is 1-based
					if (dist < closestDist) {
						closestDist = dist;
						closestMarkerCol = matchResult.index;
					}
				}

				if (closestMarkerCol >= 0) {
					// Count pipe separators before the marker to determine field position
					const textBeforeMarker = lineContent.substring(0, closestMarkerCol);
					const pipeCount = (textBeforeMarker.match(/\|/g) || []).length;
					// For PID: PID|f1|f2|f3|... -> pipeCount pipes = field position pipeCount
					// For MSH: MSH|^~\&|f3|f4|... -> field numbering is offset
					const segType = lineContent.substring(0, 3);
					const fieldPosition = segType === 'MSH' ? pipeCount + 1 : pipeCount;
					// Pass "lineNumber:fieldPosition"
					onExpandTruncated?.(pos.lineNumber, String(fieldPosition));
				}
			}
		}));

		// "Expand All Truncated Fields" action
		actionDisposables.push(ed.addAction({
			id: 'bridgelab.expandAll',
			label: t('ctx.expandAll'),
			contextMenuGroupId: 'navigation',
			contextMenuOrder: 3,
			run: () => {
				onExpandAll?.();
			}
		}));

		// "Collapse All" action - re-truncate expanded fields
		actionDisposables.push(ed.addAction({
			id: 'bridgelab.collapseAll',
			label: t('ctx.collapseAll'),
			contextMenuGroupId: 'navigation',
			contextMenuOrder: 4,
			run: () => {
				onCollapseAll?.();
			}
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
				const lineContent = editor.getModel()?.getLineContent(line) ?? '';
				navigator.clipboard.writeText(lineContent).catch(() => { /* non-secure context */ });
			}
		}));
	}

	// No click handler - expand only via context menu to avoid accidental triggers

	// Sync content prop -> editor
	$effect(() => {
		const val = content ?? '';
		if (editor && !isUpdatingFromProp) {
			if (val !== editor.getValue()) {
				isUpdatingFromProp = true;
				try {
					editor.setValue(val);
				} finally {
					// finally: if setValue throws and the flag stays latched,
					// every subsequent user edit stops propagating.
					isUpdatingFromProp = false;
				}
			}
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
			editor.revealLineInCenter(line);
			editor.setPosition({ lineNumber: line, column });
			if (selectionLength > 0) {
				editor.setSelection({
					startLineNumber: line,
					startColumn: column,
					endLineNumber: line,
					endColumn: column + selectionLength,
				});
			}
			editor.focus();
		} catch { /* ignore */ }
	});

	export function setValue(value: string) {
		if (editor) {
			isUpdatingFromProp = true;
			editor.setValue(value);
			isUpdatingFromProp = false;
		}
	}

	export function getValue(): string {
		return editor?.getValue() ?? '';
	}

	export function focus() {
		editor?.focus();
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
</style>
