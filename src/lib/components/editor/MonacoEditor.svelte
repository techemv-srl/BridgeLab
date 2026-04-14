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
		content = '',
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
	let mounted = false;

	onMount(async () => {
		mounted = true;

		// Dynamic import to avoid SSR issues
		monaco = await import('monaco-editor');

		if (!mounted) return; // Component destroyed during async import

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

		// Listen for content changes
		editor.onDidChangeModelContent(() => {
			if (!mounted || isUpdatingFromProp) return;
			const value = editor!.getValue();
			onContentChange?.(value);
		});

		// Listen for cursor changes
		editor.onDidChangeCursorPosition((e) => {
			if (!mounted) return;
			onCursorChange?.(e.position.lineNumber, e.position.column);
		});
	});

	onDestroy(() => {
		mounted = false;
		editor?.dispose();
		editor = undefined;
	});

	// Update editor when content prop changes externally
	$effect(() => {
		const currentContent = content;
		if (!mounted || !editor) return;
		try {
			if (currentContent !== editor.getValue()) {
				isUpdatingFromProp = true;
				editor.setValue(currentContent || '');
				isUpdatingFromProp = false;
			}
		} catch {
			// Editor may be disposed during transition
		}
	});

	// Update theme when prop changes
	$effect(() => {
		const currentTheme = theme;
		if (!mounted || !monaco) return;
		try {
			monaco.editor.setTheme(currentTheme);
		} catch {
			// Ignore if disposed
		}
	});

	/** Set content programmatically */
	export function setValue(value: string) {
		if (editor && mounted) {
			isUpdatingFromProp = true;
			editor.setValue(value);
			isUpdatingFromProp = false;
		}
	}

	/** Get current content */
	export function getValue(): string {
		return editor?.getValue() ?? content;
	}

	/** Focus the editor */
	export function focus() {
		if (editor && mounted) editor.focus();
	}

	/** Reveal a specific line */
	export function revealLine(line: number) {
		if (editor && mounted) editor.revealLineInCenter(line);
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
