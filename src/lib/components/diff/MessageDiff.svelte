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
	let models: { original: any; modified: any } | null = null;

	function disposeModels() {
		models?.original?.dispose();
		models?.modified?.dispose();
		models = null;
	}

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

		models = {
			original: monaco.editor.createModel(originalText, 'hl7v2'),
			modified: monaco.editor.createModel(modifiedText, 'hl7v2'),
		};
		diffEditor.setModel(models);
	});

	onDestroy(() => {
		diffEditor?.dispose();
		disposeModels();
	});

	// Update in place when texts change (no model churn, keeps scroll position).
	$effect(() => {
		const orig = originalText;
		const mod = modifiedText;
		if (models) {
			if (models.original.getValue() !== orig) models.original.setValue(orig);
			if (models.modified.getValue() !== mod) models.modified.setValue(mod);
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
