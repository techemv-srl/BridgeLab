<script lang="ts">
	import { getTemplatesGrouped, type MessageTemplate } from '$lib/ipc/templates';

	interface Props {
		onSelect: (template: MessageTemplate) => void;
		onClose: () => void;
	}

	let { onSelect, onClose }: Props = $props();

	let groups = $state<[string, MessageTemplate[]][]>([]);
	let selectedId = $state<string | null>(null);
	let search = $state('');

	// Builtin templates (fallback when no backend)
	const builtinTemplates: MessageTemplate[] = [
		{
			id: 'adt-a01', name: 'ADT^A01 - Patient Admission', message_type: 'ADT',
			category: 'Admission / Discharge / Transfer',
			description: 'Patient admission / visit notification',
			content: 'MSH|^~\\&|SENDING_APP|SENDING_FAC|RECEIVING_APP|RECEIVING_FAC|{NOW}||ADT^A01|{MSGID}|P|2.5\rEVN|A01|{NOW}\rPID|1||MRN001^^^HOSPITAL^MR||DOE^JOHN||19800101|M\rPV1|1|I|WARD01^101^A\r',
		},
	];

	let loaded = false;
	$effect(() => {
		if (loaded || typeof window === 'undefined') return;
		loaded = true;
		loadTemplates();
	});

	async function loadTemplates() {
		try {
			groups = await getTemplatesGrouped();
		} catch {
			// Web mode fallback
			groups = [['Built-in', builtinTemplates]];
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

	let selectedTemplate = $derived(
		groups.flatMap(([, items]) => items).find((t) => t.id === selectedId)
	);

	function handleSelect() {
		if (selectedTemplate) {
			onSelect(selectedTemplate);
		}
	}
</script>

<div class="tmpl-dialog">
	<div class="tmpl-header">
		<span>New Message from Template</span>
		<button class="close-btn" onclick={onClose}>&times;</button>
	</div>

	<div class="tmpl-search">
		<input
			type="text"
			bind:value={search}
			placeholder="Search templates..."
			class="search-input"
		/>
	</div>

	<div class="tmpl-body">
		<div class="tmpl-list">
			{#if filtered.length === 0}
				<div class="tmpl-empty">No templates match your search</div>
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
				<div class="preview-label">Preview</div>
				<pre class="preview-content">{selectedTemplate.content}</pre>
			{:else}
				<div class="preview-empty">Select a template to preview</div>
			{/if}
		</div>
	</div>

	<div class="tmpl-footer">
		<button class="btn" onclick={onClose}>Cancel</button>
		<button class="btn btn-primary" onclick={handleSelect} disabled={!selectedTemplate}>
			Create Message
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
