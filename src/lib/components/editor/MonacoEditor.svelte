<script lang="ts">
	import { onMount } from 'svelte';
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

	let containerEl: HTMLDivElement;
	let editor = $state<IStandaloneCodeEditor | undefined>(undefined);
	let monaco = $state<MonacoModule | undefined>(undefined);
	let isUpdatingFromProp = false;

	onMount(() => {
		let alive = true;
		console.log('[Monaco] onMount, container:', containerEl, 'size:', containerEl?.offsetWidth, 'x', containerEl?.offsetHeight);

		async function init() {
			try {
				console.log('[Monaco] Loading monaco-editor module...');
				monaco = await import('monaco-editor');
				console.log('[Monaco] Module loaded, alive:', alive);
				if (!alive) return;

				self.MonacoEnvironment = {
					getWorker(_: string, _label: string) {
						return new Worker(
							new URL('monaco-editor/esm/vs/editor/editor.worker.js', import.meta.url),
							{ type: 'module' }
						);
					}
				};

				registerHL7Language(monaco);

				console.log('[Monaco] Creating editor, container size:', containerEl?.offsetWidth, 'x', containerEl?.offsetHeight);
				console.log('[Monaco] Initial content length:', (content || '').length, 'first 50 chars:', (content || '').substring(0, 50));

				editor = monaco.editor.create(containerEl, {
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

				console.log('[Monaco] Editor created:', !!editor, 'value length:', editor?.getValue().length);

				editor.onDidChangeModelContent(() => {
					if (!alive || isUpdatingFromProp || !editor) return;
					onContentChange?.(editor.getValue());
				});

				editor.onDidChangeCursorPosition((e) => {
					if (!alive) return;
					onCursorChange?.(e.position.lineNumber, e.position.column);
				});

				editor.focus();
				console.log('[Monaco] Editor focused, ready');
			} catch (err) {
				console.error('[Monaco] Init failed:', err);
			}
		}

		init();

		return () => {
			console.log('[Monaco] Cleanup/destroy');
			alive = false;
			if (editor) {
				editor.dispose();
				editor = undefined;
			}
		};
	});

	// Sync content prop -> editor reactively
	$effect(() => {
		const val = content ?? '';
		const hasEditor = !!editor;
		console.log('[Monaco] $effect content sync: editor=', hasEditor, 'content length=', val.length, 'first 30:', val.substring(0, 30));
		if (editor && !isUpdatingFromProp) {
			const current = editor.getValue();
			if (val !== current) {
				console.log('[Monaco] Updating editor value, old length:', current.length, 'new length:', val.length);
				isUpdatingFromProp = true;
				editor.setValue(val);
				isUpdatingFromProp = false;
			}
		}
	});

	// Theme sync
	$effect(() => {
		if (monaco && editor) {
			try { monaco.editor.setTheme(theme); } catch { /* ignore */ }
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
