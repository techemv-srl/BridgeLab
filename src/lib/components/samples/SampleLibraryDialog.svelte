<script lang="ts">
	// Sample message library: finished, realistic HL7 v2 messages in several
	// versions, opened in a new tab. Complements the templates (skeletons to
	// fill in) — see src-tauri/src/samples.rs.
	import { getSamples, sampleName, sampleDescription, sampleCategory, type Sample } from '$lib/ipc/samples';
	import { t, subscribeLocale } from '$lib/i18n';
	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }
	// Localised labels (backend English as fallback), re-read on locale change.
	const sName = (s: Sample) => { void localeVersion; return sampleName(s); };
	const sDesc = (s: Sample) => { void localeVersion; return sampleDescription(s); };
	const sCat = (c: string) => { void localeVersion; return sampleCategory(c); };

	interface Props {
		onOpen: (sample: Sample) => void;
		onClose: () => void;
	}

	let { onOpen, onClose }: Props = $props();

	let samples = $state<Sample[]>([]);
	let loading = $state(true);
	let loadError = $state('');
	let search = $state('');
	let version = $state<string>('');
	let selectedId = $state<string | null>(null);
	let searchInputEl: HTMLInputElement | undefined = $state();

	let loaded = false;
	$effect(() => {
		if (loaded || typeof window === 'undefined') return;
		loaded = true;
		void (async () => {
			try {
				samples = await getSamples();
				selectedId = samples[0]?.id ?? null;
			} catch (e) {
				loadError = String(e);
			} finally {
				loading = false;
			}
		})();
	});

	$effect(() => { searchInputEl?.focus(); });

	let versions = $derived([...new Set(samples.map((s) => s.version))].sort((a, b) =>
		a.localeCompare(b, undefined, { numeric: true })));

	let filtered = $derived.by(() => {
		const q = search.trim().toLowerCase();
		return samples.filter((s) =>
			(!version || s.version === version) &&
			(!q || s.name.toLowerCase().includes(q) || s.description.toLowerCase().includes(q) ||
				sName(s).toLowerCase().includes(q) || sDesc(s).toLowerCase().includes(q) ||
				s.message_type.toLowerCase().includes(q)));
	});

	let groups = $derived.by(() => {
		const map = new Map<string, Sample[]>();
		for (const s of filtered) {
			if (!map.has(s.category)) map.set(s.category, []);
			map.get(s.category)!.push(s);
		}
		return [...map.entries()];
	});

	// Resolve against the filtered list, so a hidden sample cannot be opened.
	let selected = $derived(filtered.find((s) => s.id === selectedId));

	function open() {
		if (selected) onOpen(selected);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') { e.preventDefault(); onClose(); }
		else if (e.key === 'Enter' && selected) { e.preventDefault(); open(); }
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="smp-dialog">
	<div class="smp-header">
		<span>{tr('samples.title')}</span>
		<button class="close-btn" onclick={onClose} aria-label="Close">&times;</button>
	</div>
	<div class="smp-intro">{tr('samples.intro')}</div>

	<div class="smp-toolbar">
		<input type="text" bind:this={searchInputEl} bind:value={search} placeholder={tr('samples.search')} class="search-input" />
		<div class="chips" role="group" aria-label={tr('samples.version')}>
			<button class="chip" class:active={version === ''} onclick={() => (version = '')}>{tr('samples.allVersions')}</button>
			{#each versions as v (v)}
				<button class="chip" class:active={version === v} onclick={() => (version = v)}>v{v}</button>
			{/each}
		</div>
	</div>

	<div class="smp-body">
		<div class="smp-list">
			{#if loading}
				<div class="smp-empty">{tr('xsd.loading')}</div>
			{:else if loadError}
				<div class="smp-empty error">{loadError}</div>
			{:else if filtered.length === 0}
				<div class="smp-empty">{tr('samples.none')}</div>
			{:else}
				{#each groups as [category, items] (category)}
					<div class="smp-category">{sCat(category)}</div>
					{#each items as s (s.id)}
						<button class="smp-item" class:selected={selectedId === s.id}
							onclick={() => (selectedId = s.id)} ondblclick={open}>
							<div class="smp-name">
								{sName(s)}
								<span class="smp-type">{s.message_type.split('^').slice(0, 2).join('^')}</span>
								<span class="smp-ver">v{s.version}</span>
							</div>
							<div class="smp-desc">{sDesc(s)}</div>
						</button>
					{/each}
				{/each}
			{/if}
		</div>
		<div class="smp-preview">
			{#if selected}
				<div class="preview-label">{tr('tmpl.preview')}</div>
				<pre class="preview-content">{selected.content.replace(/\r/g, '\n')}</pre>
			{:else}
				<div class="preview-empty">{tr('tmpl.previewPrompt')}</div>
			{/if}
		</div>
	</div>

	<div class="smp-footer">
		<span class="smp-note">{tr('samples.fictional')}</span>
		<button class="btn" onclick={onClose}>{tr('dialog.cancel')}</button>
		<button class="btn btn-primary" onclick={open} disabled={!selected}>{tr('samples.open')}</button>
	</div>
</div>

<style>
	.smp-dialog { display: flex; flex-direction: column; max-height: 80vh; min-height: 0; }
	.smp-header { display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; border-bottom: 1px solid var(--color-border); font-weight: 700; font-size: 14px; }
	.close-btn { background: none; border: none; color: var(--color-text-secondary); cursor: pointer; font-size: 20px; }
	.smp-intro { padding: 8px 16px 0; font-size: 12px; color: var(--color-text-secondary); }
	.smp-toolbar { display: flex; flex-direction: column; gap: 6px; padding: 8px 12px; border-bottom: 1px solid var(--color-border); }
	.search-input { width: 100%; padding: 6px 10px; border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 12px; font-family: inherit; }
	.chips { display: flex; flex-wrap: wrap; gap: 4px; }
	.chip { padding: 2px 10px; border: 1px solid var(--color-border); border-radius: 10px; background: var(--color-bg-tertiary); color: var(--color-text-secondary); font-size: 11px; font-family: inherit; cursor: pointer; }
	.chip.active { background: var(--color-accent); border-color: var(--color-accent); color: var(--color-bg-primary); }
	/* The body gives way first: a fixed 320px pushed the footer out of an
	   80vh modal in a 900x600 window. */
	.smp-dialog > :not(.smp-body) { flex-shrink: 0; }
	.smp-body { display: flex; flex: 1 1 320px; min-height: 140px; overflow: hidden; }
	.smp-list { width: 45%; overflow-y: auto; border-right: 1px solid var(--color-border); padding: 4px 0; }
	.smp-category { font-size: 10px; font-weight: 700; text-transform: uppercase; color: var(--color-text-secondary); padding: 8px 12px 4px; letter-spacing: 0.5px; }
	.smp-item { display: block; width: 100%; text-align: left; padding: 6px 12px; background: none; border: none; color: var(--color-text-primary); font-family: inherit; cursor: pointer; border-left: 2px solid transparent; }
	.smp-item:hover { background: var(--color-bg-tertiary); }
	.smp-item.selected { background: var(--color-bg-tertiary); border-left-color: var(--color-accent); }
	.smp-name { font-size: 12px; font-weight: 600; display: flex; gap: 6px; align-items: baseline; flex-wrap: wrap; }
	.smp-type { font-family: 'JetBrains Mono', monospace; font-size: 10px; font-weight: 500; color: var(--color-accent); }
	.smp-ver { font-size: 10px; font-weight: 500; color: var(--color-text-secondary); }
	.smp-desc { font-size: 11px; color: var(--color-text-secondary); margin-top: 2px; }
	.smp-empty { padding: 16px; text-align: center; color: var(--color-text-secondary); font-style: italic; }
	.smp-empty.error { color: var(--color-error); font-style: normal; }
	.smp-preview { flex: 1; padding: 12px; overflow: hidden; display: flex; flex-direction: column; }
	.preview-label { font-size: 10px; font-weight: 700; text-transform: uppercase; color: var(--color-text-secondary); margin-bottom: 4px; }
	.preview-content { flex: 1; margin: 0; padding: 8px; background: var(--color-bg-primary); border: 1px solid var(--color-border); border-radius: 4px; font-family: 'JetBrains Mono', monospace; font-size: 11px; white-space: pre; overflow: auto; color: var(--color-text-primary); }
	.preview-empty { flex: 1; display: flex; align-items: center; justify-content: center; color: var(--color-text-secondary); font-style: italic; font-size: 12px; }
	.smp-footer { display: flex; justify-content: flex-end; align-items: center; gap: 8px; padding: 12px 16px; border-top: 1px solid var(--color-border); }
	.smp-note { margin-right: auto; font-size: 11px; color: var(--color-text-secondary); }
	.btn { padding: 6px 16px; border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 12px; font-family: inherit; cursor: pointer; }
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.btn-primary { background: var(--color-accent); color: var(--color-bg-primary); border-color: var(--color-accent); }
</style>
