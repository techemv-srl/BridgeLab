<script lang="ts">
	import { onMount, onDestroy, tick } from 'svelte';
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
	let alive = true;

	onMount(async () => {
		monaco = await import('monaco-editor');

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

		editor.onDidChangeModelContent(() => {
			if (!alive || isUpdatingFromProp || !editor) return;
			const value = editor.getValue();
			onContentChange?.(value);
		});

		editor.onDidChangeCursorPosition((e) => {
			if (!alive) return;
			onCursorChange?.(e.position.lineNumber, e.position.column);
		});

		// Focus the editor so paste works immediately
		editor.focus();
	});

	onDestroy(() => {
		alive = false;
		if (editor) {
			editor.dispose();
			editor = undefined;
		}
	});

	// Sync content prop -> editor using a polling approach instead of $effect
	// This avoids Svelte 5 signal-null crashes during component teardown
	let syncInterval: ReturnType<typeof setInterval> | undefined;
	let lastSyncedContent = '';

	onMount(() => {
		lastSyncedContent = content || '';
		syncInterval = setInterval(() => {
			if (!alive || !editor) return;
			try {
				const propContent = content || '';
				if (propContent !== lastSyncedContent && propContent !== editor.getValue()) {
					isUpdatingFromProp = true;
					editor.setValue(propContent);
					isUpdatingFromProp = false;
					lastSyncedContent = propContent;
				}
			} catch {
				// Ignore errors during teardown
			}
		}, 100);
	});

	onDestroy(() => {
		if (syncInterval) clearInterval(syncInterval);
	});

	// Theme sync - safe because theme is a simple string, not bound to component lifecycle
	$effect(() => {
		if (alive && monaco) {
			try { monaco.editor.setTheme(theme); } catch { /* ignore */ }
		}
	});

	export function setValue(value: string) {
		if (editor && alive) {
			isUpdatingFromProp = true;
			editor.setValue(value);
			isUpdatingFromProp = false;
			lastSyncedContent = value;
		}
	}

	export function getValue(): string {
		return editor?.getValue() ?? '';
	}

	export function focus() {
		if (editor && alive) editor.focus();
	}

	export function revealLine(line: number) {
		if (editor && alive) editor.revealLineInCenter(line);
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
