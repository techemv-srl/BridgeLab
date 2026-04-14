<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { registerHL7Language } from './HL7MonarchLanguage';

	type MonacoModule = typeof import('monaco-editor');
	type MonacoEditor = import('monaco-editor').editor.IStandaloneCodeEditor;

	interface Props {
		content?: string;
		language?: string;
		theme?: string;
		readonly?: boolean;
		onContentChange?: (value: string) => void;
		onCursorChange?: (line: number, column: number) => void;
	}

	let {
		content = $bindable(''),
		language = 'hl7v2',
		theme = 'bridgelab-dark',
		readonly = false,
		onContentChange,
		onCursorChange,
	}: Props = $props();

	let containerEl: HTMLDivElement;
	let editor: MonacoEditor | undefined;
	let monaco: MonacoModule | undefined;
	let isUpdatingFromProp = false;

	onMount(async () => {
		// Dynamic import to avoid SSR issues
		monaco = await import('monaco-editor');

		// Configure Monaco environment for web workers
		self.MonacoEnvironment = {
			getWorker(_: string, _label: string) {
				return new Worker(
					new URL('monaco-editor/esm/vs/editor/editor.worker.js', import.meta.url),
					{ type: 'module' }
				);
			}
		};

		// Register HL7 language
		registerHL7Language(monaco);

		// Create the editor
		editor = monaco.editor.create(containerEl, {
			value: content,
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

		// Listen for content changes
		editor.onDidChangeModelContent(() => {
			if (isUpdatingFromProp) return;
			const value = editor!.getValue();
			content = value;
			onContentChange?.(value);
		});

		// Listen for cursor changes
		editor.onDidChangeCursorPosition((e) => {
			onCursorChange?.(e.position.lineNumber, e.position.column);
		});
	});

	onDestroy(() => {
		editor?.dispose();
	});

	// Update editor when content prop changes externally
	$effect(() => {
		if (editor && content !== editor.getValue()) {
			isUpdatingFromProp = true;
			editor.setValue(content);
			isUpdatingFromProp = false;
		}
	});

	// Update theme when prop changes
	$effect(() => {
		if (monaco) {
			monaco.editor.setTheme(theme);
		}
	});

	/** Set content programmatically */
	export function setValue(value: string) {
		if (editor) {
			editor.setValue(value);
		}
	}

	/** Get current content */
	export function getValue(): string {
		return editor?.getValue() ?? content;
	}

	/** Focus the editor */
	export function focus() {
		editor?.focus();
	}

	/** Reveal a specific line */
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
