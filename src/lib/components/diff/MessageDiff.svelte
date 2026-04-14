<script lang="ts">
	import { onMount, onDestroy } from 'svelte';

	interface Props {
		originalText: string;
		modifiedText: string;
		originalLabel?: string;
		modifiedLabel?: string;
		theme?: string;
	}

	let {
		originalText,
		modifiedText,
		originalLabel = 'Original',
		modifiedLabel = 'Modified',
		theme = 'bridgelab-dark',
	}: Props = $props();

	let containerEl: HTMLDivElement;
	let diffEditor: any;

	onMount(async () => {
		const monaco = await import('monaco-editor');

		diffEditor = monaco.editor.createDiffEditor(containerEl, {
			theme,
			readOnly: true,
			renderSideBySide: true,
			fontSize: 12,
			fontFamily: "'JetBrains Mono', monospace",
			minimap: { enabled: false },
			scrollBeyondLastLine: false,
			automaticLayout: true,
		});

		const originalModel = monaco.editor.createModel(originalText, 'hl7v2');
		const modifiedModel = monaco.editor.createModel(modifiedText, 'hl7v2');

		diffEditor.setModel({
			original: originalModel,
			modified: modifiedModel,
		});
	});

	onDestroy(() => {
		diffEditor?.dispose();
	});

	$effect(() => {
		if (diffEditor) {
			const monaco = (window as any).monaco;
			if (monaco) {
				const orig = monaco.editor.createModel(originalText, 'hl7v2');
				const mod = monaco.editor.createModel(modifiedText, 'hl7v2');
				diffEditor.setModel({ original: orig, modified: mod });
			}
		}
	});
</script>

<div class="diff-container">
	<div class="diff-labels">
		<span class="diff-label">{originalLabel}</span>
		<span class="diff-label">{modifiedLabel}</span>
	</div>
	<div class="diff-editor" bind:this={containerEl}></div>
</div>

<style>
	.diff-container { display: flex; flex-direction: column; height: 100%; }
	.diff-labels { display: flex; justify-content: space-around; padding: 4px 8px; background: var(--color-bg-tertiary); font-size: 11px; font-weight: 600; color: var(--color-text-secondary); flex-shrink: 0; }
	.diff-label { flex: 1; text-align: center; }
	.diff-editor { flex: 1; min-height: 200px; }
</style>
