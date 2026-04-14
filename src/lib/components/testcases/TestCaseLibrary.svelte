<script lang="ts">
	import { getTestCases, saveTestCase, deleteTestCase, type TestCase } from '$lib/ipc/testcases';

	interface Props {
		currentContent?: string;
		currentLabel?: string;
		onLoad: (testCase: TestCase) => void;
		onClose: () => void;
	}

	let { currentContent = '', currentLabel = '', onLoad, onClose }: Props = $props();

	let cases = $state<TestCase[]>([]);
	let selectedId = $state<string | null>(null);
	let search = $state('');
	let mode = $state<'list' | 'edit' | 'new'>('list');

	// Edit/new form state
	let formName = $state('');
	let formDescription = $state('');
	let formCategory = $state('general');
	let formTags = $state('');
	let formContent = $state('');

	let loaded = false;
	$effect(() => {
		if (loaded || typeof window === 'undefined') return;
		loaded = true;
		load();
	});

	async function load() {
		try { cases = await getTestCases(); } catch { cases = []; }
	}

	let filtered = $derived.by(() => {
		if (!search.trim()) return cases;
		const q = search.toLowerCase();
		return cases.filter(c =>
			c.name.toLowerCase().includes(q) ||
			c.description.toLowerCase().includes(q) ||
			c.tags.toLowerCase().includes(q) ||
			c.category.toLowerCase().includes(q)
		);
	});

	let byCategory = $derived.by(() => {
		const map: Record<string, TestCase[]> = {};
		for (const c of filtered) {
			if (!map[c.category]) map[c.category] = [];
			map[c.category].push(c);
		}
		return Object.entries(map).sort(([a], [b]) => a.localeCompare(b));
	});

	let selected = $derived(cases.find(c => c.id === selectedId));

	function startNew() {
		formName = currentLabel || '';
		formDescription = '';
		formCategory = 'general';
		formTags = '';
		formContent = currentContent;
		mode = 'new';
	}

	function startEdit(tc: TestCase) {
		formName = tc.name;
		formDescription = tc.description;
		formCategory = tc.category;
		formTags = tc.tags;
		formContent = tc.content;
		selectedId = tc.id;
		mode = 'edit';
	}

	async function handleSave() {
		if (!formName.trim() || !formContent.trim()) return;
		try {
			await saveTestCase({
				id: mode === 'edit' ? (selectedId ?? undefined) : undefined,
				name: formName,
				description: formDescription,
				category: formCategory || 'general',
				tags: formTags,
				content: formContent,
			});
			await load();
			mode = 'list';
		} catch (e) {
			alert(`Save failed: ${e}`);
		}
	}

	async function handleDelete(tc: TestCase) {
		if (!confirm(`Delete test case "${tc.name}"?`)) return;
		try {
			await deleteTestCase(tc.id);
			await load();
			if (selectedId === tc.id) selectedId = null;
		} catch (e) {
			alert(`Delete failed: ${e}`);
		}
	}
</script>

<div class="tc-library">
	<div class="tc-header">
		<span>Test Case Library</span>
		<div class="header-actions">
			{#if mode === 'list'}
				<button class="btn btn-primary" onclick={startNew}>
					{currentContent ? '+ Save Current Message' : '+ New Test Case'}
				</button>
			{/if}
			<button class="close-btn" onclick={onClose}>&times;</button>
		</div>
	</div>

	{#if mode === 'list'}
		<div class="tc-search">
			<input bind:value={search} placeholder="Search test cases..." class="search-input" />
		</div>

		<div class="tc-body">
			<div class="tc-list">
				{#if cases.length === 0}
					<div class="empty">
						<p>No test cases saved yet.</p>
						{#if currentContent}
							<p>Click "Save Current Message" to add this message as a test case.</p>
						{/if}
					</div>
				{:else if filtered.length === 0}
					<div class="empty">No test cases match your search</div>
				{:else}
					{#each byCategory as [category, items]}
						<div class="tc-category">{category}</div>
						{#each items as tc (tc.id)}
							<button
								class="tc-item"
								class:selected={selectedId === tc.id}
								onclick={() => { selectedId = tc.id; }}
							>
								<div class="tc-name">{tc.name}</div>
								{#if tc.description}
									<div class="tc-desc">{tc.description}</div>
								{/if}
								{#if tc.tags}
									<div class="tc-tags">
										{#each tc.tags.split(',').filter(t => t.trim()) as tag}
											<span class="tag">{tag.trim()}</span>
										{/each}
									</div>
								{/if}
							</button>
						{/each}
					{/each}
				{/if}
			</div>

			<div class="tc-detail">
				{#if selected}
					<div class="detail-header">
						<h3>{selected.name}</h3>
						<div class="detail-actions">
							<button class="btn btn-primary" onclick={() => onLoad(selected)}>Load in Editor</button>
							<button class="btn" onclick={() => startEdit(selected)}>Edit</button>
							<button class="btn btn-danger" onclick={() => handleDelete(selected)}>Delete</button>
						</div>
					</div>
					{#if selected.description}
						<div class="detail-desc">{selected.description}</div>
					{/if}
					<div class="detail-meta">
						<span class="meta-item">Category: <strong>{selected.category}</strong></span>
						<span class="meta-item">Updated: {new Date(selected.updated_at).toLocaleString()}</span>
					</div>
					<pre class="detail-content">{selected.content}</pre>
				{:else}
					<div class="empty">Select a test case to view details</div>
				{/if}
			</div>
		</div>
	{:else}
		<!-- Edit / New form -->
		<div class="tc-form">
			<div class="form-row">
				<label for="tc-name">Name *</label>
				<input id="tc-name" bind:value={formName} placeholder="e.g. ADT^A01 admission test" class="form-input" />
			</div>
			<div class="form-row">
				<label for="tc-desc">Description</label>
				<textarea id="tc-desc" bind:value={formDescription} rows={2} placeholder="When to use this test case..." class="form-input"></textarea>
			</div>
			<div class="form-grid">
				<div class="form-row">
					<label for="tc-cat">Category</label>
					<input id="tc-cat" bind:value={formCategory} placeholder="admission, orders, ..." class="form-input" />
				</div>
				<div class="form-row">
					<label for="tc-tags">Tags (comma separated)</label>
					<input id="tc-tags" bind:value={formTags} placeholder="adt, inpatient, regression" class="form-input" />
				</div>
			</div>
			<div class="form-row">
				<label for="tc-content">Message Content *</label>
				<textarea id="tc-content" bind:value={formContent} rows={10} class="form-input mono"></textarea>
			</div>
			<div class="form-actions">
				<button class="btn" onclick={() => { mode = 'list'; }}>Cancel</button>
				<button class="btn btn-primary" onclick={handleSave} disabled={!formName.trim() || !formContent.trim()}>
					{mode === 'edit' ? 'Save Changes' : 'Save Test Case'}
				</button>
			</div>
		</div>
	{/if}
</div>

<style>
	.tc-library { display: flex; flex-direction: column; height: 100%; max-height: 85vh; background: var(--color-bg-secondary); }
	.tc-header { display: flex; justify-content: space-between; align-items: center; padding: 10px 14px; border-bottom: 1px solid var(--color-border); flex-shrink: 0; font-weight: 600; }
	.header-actions { display: flex; align-items: center; gap: 8px; }
	.close-btn { background: none; border: none; color: var(--color-text-secondary); font-size: 20px; cursor: pointer; }

	.tc-search { padding: 8px 12px; border-bottom: 1px solid var(--color-border); flex-shrink: 0; }
	.search-input { width: 100%; padding: 5px 8px; border: 1px solid var(--color-border); border-radius: 3px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 12px; font-family: inherit; }

	.tc-body { display: flex; flex: 1; min-height: 0; overflow: hidden; }
	.tc-list { width: 40%; overflow-y: auto; border-right: 1px solid var(--color-border); padding: 4px 0; }
	.tc-category { padding: 8px 12px 4px; font-size: 10px; font-weight: 700; text-transform: uppercase; color: var(--color-text-secondary); letter-spacing: 0.5px; }
	.tc-item { display: block; width: 100%; padding: 6px 12px; background: none; border: none; color: var(--color-text-primary); text-align: left; cursor: pointer; border-left: 2px solid transparent; font-family: inherit; }
	.tc-item:hover { background: var(--color-bg-tertiary); }
	.tc-item.selected { background: var(--color-bg-tertiary); border-left-color: var(--color-accent); }
	.tc-name { font-size: 12px; font-weight: 600; }
	.tc-desc { font-size: 11px; color: var(--color-text-secondary); margin-top: 2px; }
	.tc-tags { display: flex; flex-wrap: wrap; gap: 3px; margin-top: 3px; }
	.tag { padding: 1px 6px; background: var(--color-bg-primary); border-radius: 8px; font-size: 10px; color: var(--color-accent); }

	.tc-detail { flex: 1; padding: 12px; overflow-y: auto; display: flex; flex-direction: column; gap: 8px; }
	.detail-header { display: flex; justify-content: space-between; align-items: center; gap: 8px; }
	.detail-header h3 { margin: 0; font-size: 14px; }
	.detail-actions { display: flex; gap: 4px; }
	.detail-desc { font-size: 12px; color: var(--color-text-secondary); padding: 6px 10px; background: var(--color-bg-tertiary); border-radius: 4px; }
	.detail-meta { display: flex; gap: 12px; font-size: 11px; color: var(--color-text-secondary); }
	.detail-content { flex: 1; margin: 0; padding: 8px; background: var(--color-bg-primary); border: 1px solid var(--color-border); border-radius: 4px; font-family: 'JetBrains Mono', monospace; font-size: 11px; white-space: pre-wrap; overflow: auto; color: var(--color-text-primary); }

	.tc-form { flex: 1; overflow-y: auto; padding: 16px; display: flex; flex-direction: column; gap: 10px; }
	.form-row { display: flex; flex-direction: column; gap: 3px; }
	.form-row label { font-size: 11px; color: var(--color-text-secondary); font-weight: 600; }
	.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
	.form-input { padding: 6px 8px; border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 12px; font-family: inherit; }
	.form-input.mono { font-family: 'JetBrains Mono', monospace; font-size: 11px; }
	.form-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 8px; }

	.empty { padding: 24px; text-align: center; color: var(--color-text-secondary); font-style: italic; font-size: 12px; }

	.btn { padding: 5px 14px; border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 12px; font-family: inherit; cursor: pointer; }
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.btn-primary { background: var(--color-accent); color: var(--color-bg-primary); border-color: var(--color-accent); }
	.btn-danger { background: var(--color-error); color: white; border-color: var(--color-error); }
</style>
