<script lang="ts">
	import { getTemplatesGrouped, type MessageTemplate } from '$lib/ipc/templates';
	import { t, subscribeLocale } from '$lib/i18n';
	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		onSelect: (template: MessageTemplate) => void;
		onClose: () => void;
	}

	let { onSelect, onClose }: Props = $props();

	let groups = $state<[string, MessageTemplate[]][]>([]);
	let selectedId = $state<string | null>(null);
	let search = $state('');
	let loading = $state(true);
	let searchInputEl: HTMLInputElement | undefined = $state();

	/** Builtin fallback for web mode / backend failure. The Rust backend
	 *  substitutes {now}/{msg_id}; here we must do it ourselves or the user
	 *  gets literal placeholders that fail parse + validation. */
	function builtinTemplates(): MessageTemplate[] {
		const now = new Date().toISOString().replace(/\D/g, '').slice(0, 14);
		const msgId = 'MSG' + now;
		return [
			{
				id: 'adt-a01', name: 'ADT^A01 - Patient Admission', message_type: 'ADT',
				category: 'Admission / Discharge / Transfer',
				description: 'Patient admission / visit notification',
				content: `MSH|^~\\&|SENDING_APP|SENDING_FAC|RECEIVING_APP|RECEIVING_FAC|${now}||ADT^A01|${msgId}|P|2.5\rEVN|A01|${now}\rPID|1||MRN001^^^HOSPITAL^MR||DOE^JOHN||19800101|M\rPV1|1|I|WARD01^101^A\r`,
			},
		];
	}

	let loaded = false;
	$effect(() => {
		if (loaded || typeof window === 'undefined') return;
		loaded = true;
		loadTemplates();
	});

	// Focus the search box as soon as it renders
	$effect(() => {
		searchInputEl?.focus();
	});

	async function loadTemplates() {
		loading = true;
		try {
			groups = await getTemplatesGrouped();
		} catch {
			// Web mode fallback
			groups = [['Built-in', builtinTemplates()]];
		} finally {
			loading = false;
		}
	}

	let filtered = $derived.by(() => {
		if (!search.trim()) return groups;
		const q = search.toLowerCase();
		return groups
			.map(([cat, items]) => [
				cat,
				items.filter(
					(t) =>
						t.name.toLowerCase().includes(q) ||
						t.description.toLowerCase().includes(q) ||
						t.message_type.toLowerCase().includes(q)
				),
			] as [string, MessageTemplate[]])
			.filter(([, items]) => items.length > 0);
	});

	// Resolve against the FILTERED list: a search that hides the selected
	// template must not leave "Create" enabled for an invisible item.
	let selectedTemplate = $derived(
		filtered.flatMap(([, items]) => items).find((t) => t.id === selectedId)
	);

	function handleSelect() {
		if (selectedTemplate) {
			onSelect(selectedTemplate);
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') { e.preventDefault(); onClose(); }
		else if (e.key === 'Enter' && selectedTemplate) { e.preventDefault(); handleSelect(); }
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="tmpl-dialog">
	<div class="tmpl-header">
		<span>{tr('tmpl.title')}</span>
		<button class="close-btn" onclick={onClose}>&times;</button>
	</div>

	<div class="tmpl-search">
		<input
			type="text"
			bind:this={searchInputEl}
			bind:value={search}
			placeholder={tr('tmpl.search')}
			class="search-input"
		/>
	</div>

	<div class="tmpl-body">
		<div class="tmpl-list">
			{#if loading}
				<div class="tmpl-empty">{tr('xsd.loading')}</div>
			{:else if groups.length === 0}
				<div class="tmpl-empty">{tr('tmpl.none')}</div>
			{:else if filtered.length === 0}
				<div class="tmpl-empty">{tr('tmpl.empty')}</div>
			{:else}
				{#each filtered as [category, items]}
					<div class="tmpl-category">{category}</div>
					{#each items as t (t.id)}
						<button
							class="tmpl-item"
							class:selected={selectedId === t.id}
							onclick={() => { selectedId = t.id; }}
							ondblclick={handleSelect}
						>
							<div class="tmpl-name">{t.name}</div>
							<div class="tmpl-desc">{t.description}</div>
						</button>
					{/each}
				{/each}
			{/if}
		</div>

		<div class="tmpl-preview">
			{#if selectedTemplate}
				<div class="preview-label">{tr('tmpl.preview')}</div>
				<pre class="preview-content">{selectedTemplate.content}</pre>
			{:else}
				<div class="preview-empty">{tr('tmpl.previewPrompt')}</div>
			{/if}
		</div>
	</div>

	<div class="tmpl-footer">
		<button class="btn" onclick={onClose}>{tr('dialog.cancel')}</button>
		<button class="btn btn-primary" onclick={handleSelect} disabled={!selectedTemplate}>
			{tr('tmpl.create')}
		</button>
	</div>
</div>

<style>
	.tmpl-dialog { display: flex; flex-direction: column; max-height: 80vh; }
	.tmpl-header { display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; border-bottom: 1px solid var(--color-border); font-weight: 700; font-size: 14px; }
	.close-btn { background: none; border: none; color: var(--color-text-secondary); cursor: pointer; font-size: 20px; }

	.tmpl-search { padding: 8px 12px; border-bottom: 1px solid var(--color-border); }
	.search-input { width: 100%; padding: 6px 10px; border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 12px; font-family: inherit; }

	.tmpl-body { display: flex; flex: 1; min-height: 300px; overflow: hidden; }
	.tmpl-list { width: 45%; overflow-y: auto; border-right: 1px solid var(--color-border); padding: 4px 0; }
	.tmpl-category { font-size: 10px; font-weight: 700; text-transform: uppercase; color: var(--color-text-secondary); padding: 8px 12px 4px; letter-spacing: 0.5px; }
	.tmpl-item { display: block; width: 100%; text-align: left; padding: 6px 12px; background: none; border: none; color: var(--color-text-primary); font-family: inherit; cursor: pointer; border-left: 2px solid transparent; }
	.tmpl-item:hover { background: var(--color-bg-tertiary); }
	.tmpl-item.selected { background: var(--color-bg-tertiary); border-left-color: var(--color-accent); }
	.tmpl-name { font-size: 12px; font-weight: 600; }
	.tmpl-desc { font-size: 11px; color: var(--color-text-secondary); margin-top: 2px; }
	.tmpl-empty { padding: 16px; text-align: center; color: var(--color-text-secondary); font-style: italic; }

	.tmpl-preview { flex: 1; padding: 12px; overflow: hidden; display: flex; flex-direction: column; }
	.preview-label { font-size: 10px; font-weight: 700; text-transform: uppercase; color: var(--color-text-secondary); margin-bottom: 4px; }
	.preview-content { flex: 1; margin: 0; padding: 8px; background: var(--color-bg-primary); border: 1px solid var(--color-border); border-radius: 4px; font-family: 'JetBrains Mono', monospace; font-size: 11px; white-space: pre-wrap; overflow: auto; color: var(--color-text-primary); }
	.preview-empty { flex: 1; display: flex; align-items: center; justify-content: center; color: var(--color-text-secondary); font-style: italic; font-size: 12px; }

	.tmpl-footer { display: flex; justify-content: flex-end; gap: 8px; padding: 12px 16px; border-top: 1px solid var(--color-border); }
	.btn { padding: 6px 16px; border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 12px; font-family: inherit; cursor: pointer; }
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.btn-primary { background: var(--color-accent); color: var(--color-bg-primary); border-color: var(--color-accent); }
	.btn-primary:hover:not(:disabled) { opacity: 0.9; }
</style>
