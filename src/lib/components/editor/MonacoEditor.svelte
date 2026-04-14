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
	}

	let {
		content = '',
		language = 'hl7v2',
		theme = 'bridgelab-dark',
		readonly = false,
		onContentChange,
		onCursorChange,
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
			console.log('[Monaco] Loading module...');
			const mod = await import('monaco-editor');
			monacoMod = mod;

			console.log('[Monaco] Container size:', containerEl?.offsetWidth, 'x', containerEl?.offsetHeight);

			self.MonacoEnvironment = {
				getWorker(_: string, _label: string) {
					return new Worker(
						new URL('monaco-editor/esm/vs/editor/editor.worker.js', import.meta.url),
						{ type: 'module' }
					);
				}
			};

			registerHL7Language(mod);

			console.log('[Monaco] Creating editor with content length:', (content || '').length);

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
			});

			ed.onDidChangeModelContent(() => {
				if (isUpdatingFromProp) return;
				onContentChange?.(ed.getValue());
			});

			ed.onDidChangeCursorPosition((e) => {
				onCursorChange?.(e.position.lineNumber, e.position.column);
			});

			editor = ed;
			ed.focus();
			console.log('[Monaco] Editor ready, value length:', ed.getValue().length);
		} catch (err) {
			console.error('[Monaco] Init failed:', err);
			initializing = false;
		}
	}

	// Sync content prop -> editor
	$effect(() => {
		const val = content ?? '';
		if (editor && !isUpdatingFromProp) {
			if (val !== editor.getValue()) {
				console.log('[Monaco] Syncing content, length:', val.length);
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
