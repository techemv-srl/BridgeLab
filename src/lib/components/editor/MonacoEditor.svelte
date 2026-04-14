<script lang="ts">
	import { registerHL7Language } from './HL7MonarchLanguage';

	type MonacoModule = typeof import('monaco-editor');
	type IStandaloneCodeEditor = import('monaco-editor').editor.IStandaloneCodeEditor;

	interface Props {
		content?: string;
		language?: string;
		theme?: string;
		readonly?: boolean;
		onContentChange?: (value: string) => void;
		onCursorChange?: (line: number, column: number) => void;
		/** Called when user wants to expand a truncated field at a specific line */
		onExpandTruncated?: (lineNumber: number, fieldMarker: string) => void;
		/** Called when user right-clicks a segment line and wants to navigate the tree */
		onNavigateToSegment?: (lineNumber: number, segmentType: string) => void;
		/** Called when user wants to re-truncate all expanded fields */
		onCollapseAll?: () => void;
	}

	let {
		content = '',
		language = 'hl7v2',
		theme = 'bridgelab-dark',
		readonly = false,
		onContentChange,
		onCursorChange,
		onExpandTruncated,
		onNavigateToSegment,
		onCollapseAll,
	}: Props = $props();

	let containerEl = $state<HTMLDivElement | undefined>(undefined);
	let editor = $state<IStandaloneCodeEditor | undefined>(undefined);
	let monacoMod = $state<MonacoModule | undefined>(undefined);
	let isUpdatingFromProp = false;
	let initializing = false;

	// Initialize Monaco when container element becomes available
	$effect(() => {
		if (containerEl && !editor && !initializing && typeof window !== 'undefined') {
			initializing = true;
			initMonaco();
		}
	});

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
				tabSize: 4,
				smoothScrolling: true,
				cursorBlinking: 'smooth',
				padding: { top: 8 },
				contextmenu: true,
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

			// Make truncation markers clickable
			addTruncationClickHandler(ed);

			editor = ed;
			ed.focus();
		} catch (err) {
			console.error('[Monaco] Init failed:', err);
			initializing = false;
		}
	}

	function addContextMenuActions(ed: IStandaloneCodeEditor, mod: MonacoModule) {
		// "Show in Tree" action
		ed.addAction({
			id: 'bridgelab.showInTree',
			label: 'Show Segment in Tree',
			contextMenuGroupId: 'navigation',
			contextMenuOrder: 1,
			keybindings: [mod.KeyMod.Alt | mod.KeyCode.KeyT],
			run: (editor) => {
				const line = editor.getPosition()?.lineNumber;
				if (!line) return;
				const lineContent = editor.getModel()?.getLineContent(line) ?? '';
				const segType = lineContent.substring(0, 3);
				if (segType && /^[A-Z][A-Z0-9]{2}$/.test(segType)) {
					onNavigateToSegment?.(line, segType);
				}
			}
		});

		// "Expand Truncated Field" action
		ed.addAction({
			id: 'bridgelab.expandTruncated',
			label: 'Expand Truncated Field',
			contextMenuGroupId: 'navigation',
			contextMenuOrder: 2,
			precondition: undefined,
			run: (editor) => {
				const line = editor.getPosition()?.lineNumber;
				if (!line) return;
				const lineContent = editor.getModel()?.getLineContent(line) ?? '';
				const match = lineContent.match(/\{\.\.\.(\d+) bytes\}/);
				if (match) {
					onExpandTruncated?.(line, match[0]);
				}
			}
		});

		// "Collapse All" action - re-truncate expanded fields
		ed.addAction({
			id: 'bridgelab.collapseAll',
			label: 'Collapse All Expanded Fields',
			contextMenuGroupId: 'navigation',
			contextMenuOrder: 3,
			run: () => {
				onCollapseAll?.();
			}
		});

		// "Copy Line as Segment" action
		ed.addAction({
			id: 'bridgelab.copySegment',
			label: 'Copy Segment',
			contextMenuGroupId: '9_cutcopypaste',
			contextMenuOrder: 10,
			keybindings: [mod.KeyMod.Alt | mod.KeyCode.KeyC],
			run: (editor) => {
				const line = editor.getPosition()?.lineNumber;
				if (!line) return;
				const lineContent = editor.getModel()?.getLineContent(line) ?? '';
				navigator.clipboard.writeText(lineContent);
			}
		});
	}

	function addTruncationClickHandler(ed: IStandaloneCodeEditor) {
		// Detect clicks on truncation markers via mouse down
		ed.onMouseDown((e) => {
			if (e.target.type !== 6) return; // 6 = CONTENT_TEXT
			const pos = e.target.position;
			if (!pos) return;

			const model = ed.getModel();
			if (!model) return;

			const lineContent = model.getLineContent(pos.lineNumber);
			// Check if click is near a truncation marker
			const markerRegex = /\{\.\.\.(\d+) bytes\}/g;
			let match;
			while ((match = markerRegex.exec(lineContent)) !== null) {
				const markerStart = match.index + 1; // 1-based column
				const markerEnd = markerStart + match[0].length;
				if (pos.column >= markerStart && pos.column <= markerEnd) {
					onExpandTruncated?.(pos.lineNumber, match[0]);
					return;
				}
			}
		});
	}

	// Sync content prop -> editor
	$effect(() => {
		const val = content ?? '';
		if (editor && !isUpdatingFromProp) {
			if (val !== editor.getValue()) {
				isUpdatingFromProp = true;
				editor.setValue(val);
				isUpdatingFromProp = false;
			}
		}
	});

	// Theme sync
	$effect(() => {
		if (monacoMod && editor) {
			try { monacoMod.editor.setTheme(theme); } catch { /* ignore */ }
		}
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

<div class="editor-container" bind:this={containerEl}></div>

<style>
	.editor-container {
		width: 100%;
		height: 100%;
		min-height: 200px;
	}
</style>
