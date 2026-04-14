<script lang="ts">
	import { analyzeFhirBundle, getFhirBundleEntry, type BundleAnalysis, type BundleEntry } from '$lib/ipc/bundle';

	interface Props {
		messageId: string;
		onClose: () => void;
	}

	let { messageId, onClose }: Props = $props();

	let analysis = $state<BundleAnalysis | null>(null);
	let selectedIndex = $state<number | null>(null);
	let entryContent = $state<string>('');
	let loading = $state(true);
	let error = $state<string>('');
	let typeFilter = $state<string>('');
	let textFilter = $state<string>('');

	let loaded = false;
	$effect(() => {
		if (loaded || typeof window === 'undefined') return;
		loaded = true;
		load();
	});

	async function load() {
		loading = true;
		error = '';
		try {
			analysis = await analyzeFhirBundle(messageId);
			if (analysis.entries.length > 0) {
				await selectEntry(0);
			}
		} catch (e) {
			error = String(e);
		}
		loading = false;
	}

	async function selectEntry(idx: number) {
		selectedIndex = idx;
		try {
			entryContent = await getFhirBundleEntry(messageId, idx);
		} catch (e) {
			entryContent = String(e);
		}
	}

	let filteredEntries = $derived.by(() => {
		if (!analysis) return [];
		let list = analysis.entries;
		if (typeFilter) list = list.filter(e => e.resource_type === typeFilter);
		if (textFilter) {
			const q = textFilter.toLowerCase();
			list = list.filter(e =>
				e.display_name.toLowerCase().includes(q) ||
				e.summary.toLowerCase().includes(q) ||
				(e.full_url?.toLowerCase().includes(q) ?? false)
			);
		}
		return list;
	});

	let selectedEntry = $derived(
		selectedIndex !== null && analysis
			? analysis.entries.find(e => e.index === selectedIndex)
			: null
	);

	let incomingRefs = $derived(
		analysis && selectedEntry
			? analysis.references.filter(r => r.to_index === selectedEntry.index)
			: []
	);

	let outgoingRefs = $derived(
		analysis && selectedEntry
			? analysis.references.filter(r => r.from_index === selectedEntry.index)
			: []
	);
</script>

<div class="bundle-viz">
	<div class="bv-header">
		<div class="bv-title">
			<span class="title-main">FHIR Bundle Visualizer</span>
			{#if analysis}
				<span class="title-stats">
					<span class="stat-badge">{analysis.bundle_type}</span>
					<span class="stat-badge">{analysis.entry_count} entries</span>
					{#if analysis.dangling_references > 0}
						<span class="stat-badge warn">{analysis.dangling_references} dangling refs</span>
					{/if}
				</span>
			{/if}
		</div>
		<button class="close-btn" onclick={onClose}>&times;</button>
	</div>

	{#if loading}
		<div class="bv-loading">Analyzing bundle...</div>
	{:else if error}
		<div class="bv-error">Error: {error}</div>
	{:else if analysis}
		<div class="bv-filters">
			<input
				type="text"
				bind:value={textFilter}
				placeholder="Search entries..."
				class="search-input"
			/>
			<select bind:value={typeFilter} class="type-filter">
				<option value="">All types ({analysis.entry_count})</option>
				{#each analysis.resource_type_counts as [rt, count]}
					<option value={rt}>{rt} ({count})</option>
				{/each}
			</select>
		</div>

		<div class="bv-body">
			<!-- Left: entry list -->
			<div class="bv-list">
				{#each filteredEntries as entry (entry.index)}
					<button
						class="bv-entry"
						class:selected={selectedIndex === entry.index}
						onclick={() => selectEntry(entry.index)}
					>
						<div class="entry-header">
							<span class="entry-type type-{entry.resource_type.toLowerCase()}">{entry.resource_type}</span>
							<span class="entry-index">#{entry.index}</span>
							{#if entry.request_method}
								<span class="entry-method">{entry.request_method}</span>
							{/if}
						</div>
						<div class="entry-name">{entry.display_name}</div>
						{#if entry.summary}
							<div class="entry-summary">{entry.summary}</div>
						{/if}
						{#if entry.references.length > 0}
							<div class="entry-refs">\u2192 {entry.references.length} ref{entry.references.length > 1 ? 's' : ''}</div>
						{/if}
					</button>
				{/each}
				{#if filteredEntries.length === 0}
					<div class="bv-empty">No entries match the filter</div>
				{/if}
			</div>

			<!-- Right: detail pane -->
			<div class="bv-detail">
				{#if selectedEntry}
					<div class="detail-section">
						<div class="detail-label">Resource Type</div>
						<div class="detail-value">{selectedEntry.resource_type}</div>

						<div class="detail-label">Display Name</div>
						<div class="detail-value">{selectedEntry.display_name}</div>

						{#if selectedEntry.full_url}
							<div class="detail-label">Full URL</div>
							<div class="detail-value mono">{selectedEntry.full_url}</div>
						{/if}

						{#if selectedEntry.resource_id}
							<div class="detail-label">Resource ID</div>
							<div class="detail-value mono">{selectedEntry.resource_id}</div>
						{/if}

						{#if selectedEntry.request_method}
							<div class="detail-label">Request</div>
							<div class="detail-value mono">{selectedEntry.request_method} {selectedEntry.request_url ?? ''}</div>
						{/if}

						{#if selectedEntry.response_status}
							<div class="detail-label">Response</div>
							<div class="detail-value mono">{selectedEntry.response_status}</div>
						{/if}
					</div>

					{#if outgoingRefs.length > 0}
						<div class="detail-section">
							<div class="detail-label">References out ({outgoingRefs.length})</div>
							{#each outgoingRefs as ref}
								<button
									class="ref-link"
									class:dangling={ref.to_index === null}
									onclick={() => ref.to_index !== null && selectEntry(ref.to_index)}
									disabled={ref.to_index === null}
									title={ref.to_index === null ? 'Dangling reference (target not in bundle)' : 'Click to navigate'}
								>
									\u2192 {ref.reference}
									{#if ref.to_index === null}
										<span class="dangling-badge">dangling</span>
									{:else}
										<span class="target-badge">#{ref.to_index}</span>
									{/if}
								</button>
							{/each}
						</div>
					{/if}

					{#if incomingRefs.length > 0}
						<div class="detail-section">
							<div class="detail-label">Referenced by ({incomingRefs.length})</div>
							{#each incomingRefs as ref}
								<button
									class="ref-link incoming"
									onclick={() => selectEntry(ref.from_index)}
								>
									\u2190 #{ref.from_index} ({analysis.entries[ref.from_index]?.resource_type})
								</button>
							{/each}
						</div>
					{/if}

					<div class="detail-section">
						<div class="detail-label">Resource JSON</div>
						<pre class="detail-json">{entryContent}</pre>
					</div>
				{:else}
					<div class="bv-empty">Select an entry to inspect</div>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.bundle-viz { display: flex; flex-direction: column; height: 100%; max-height: 85vh; background: var(--color-bg-secondary); }
	.bv-header { display: flex; justify-content: space-between; align-items: center; padding: 10px 14px; border-bottom: 1px solid var(--color-border); flex-shrink: 0; }
	.bv-title { display: flex; align-items: center; gap: 12px; }
	.title-main { font-weight: 700; font-size: 14px; }
	.title-stats { display: flex; gap: 6px; }
	.stat-badge { padding: 2px 8px; background: var(--color-bg-tertiary); border-radius: 10px; font-size: 10px; color: var(--color-text-secondary); }
	.stat-badge.warn { background: var(--color-warning); color: var(--color-bg-primary); font-weight: 600; }
	.close-btn { background: none; border: none; color: var(--color-text-secondary); cursor: pointer; font-size: 20px; }

	.bv-loading, .bv-error, .bv-empty { padding: 24px; text-align: center; color: var(--color-text-secondary); font-style: italic; }
	.bv-error { color: var(--color-error); }

	.bv-filters { display: flex; gap: 8px; padding: 8px 12px; border-bottom: 1px solid var(--color-border); flex-shrink: 0; }
	.search-input { flex: 1; padding: 4px 8px; border: 1px solid var(--color-border); border-radius: 3px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 11px; font-family: inherit; }
	.type-filter { padding: 4px 8px; border: 1px solid var(--color-border); border-radius: 3px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 11px; font-family: inherit; }

	.bv-body { display: flex; flex: 1; min-height: 0; overflow: hidden; }
	.bv-list { width: 40%; overflow-y: auto; border-right: 1px solid var(--color-border); }
	.bv-entry { display: block; width: 100%; padding: 8px 12px; background: none; border: none; border-bottom: 1px solid var(--color-border); color: var(--color-text-primary); font-family: inherit; cursor: pointer; text-align: left; }
	.bv-entry:hover { background: var(--color-bg-tertiary); }
	.bv-entry.selected { background: var(--color-bg-tertiary); border-left: 3px solid var(--color-accent); padding-left: 9px; }

	.entry-header { display: flex; align-items: center; gap: 6px; margin-bottom: 2px; }
	.entry-type { padding: 1px 6px; border-radius: 3px; font-size: 10px; font-weight: 700; background: var(--color-accent); color: var(--color-bg-primary); }
	.entry-index { font-size: 10px; color: var(--color-text-secondary); font-family: 'JetBrains Mono', monospace; }
	.entry-method { padding: 1px 5px; background: var(--color-bg-primary); border-radius: 3px; font-size: 10px; font-weight: 600; color: var(--color-success); }
	.entry-name { font-size: 12px; font-weight: 600; }
	.entry-summary { font-size: 11px; color: var(--color-text-secondary); margin-top: 2px; }
	.entry-refs { font-size: 10px; color: var(--color-accent); margin-top: 3px; }

	.bv-detail { flex: 1; overflow-y: auto; padding: 12px; display: flex; flex-direction: column; gap: 10px; }
	.detail-section { display: flex; flex-direction: column; gap: 3px; }
	.detail-label { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; color: var(--color-text-secondary); margin-bottom: 2px; }
	.detail-value { font-size: 12px; color: var(--color-text-primary); }
	.detail-value.mono { font-family: 'JetBrains Mono', monospace; font-size: 11px; word-break: break-all; }
	.detail-json { flex: 1; margin: 0; padding: 8px; background: var(--color-bg-primary); border: 1px solid var(--color-border); border-radius: 4px; font-family: 'JetBrains Mono', monospace; font-size: 11px; white-space: pre-wrap; overflow: auto; max-height: 300px; color: var(--color-text-primary); }

	.ref-link { display: flex; align-items: center; gap: 6px; width: 100%; padding: 4px 8px; margin-top: 2px; background: var(--color-bg-tertiary); border: 1px solid var(--color-border); border-radius: 3px; color: var(--color-accent); font-family: 'JetBrains Mono', monospace; font-size: 11px; text-align: left; cursor: pointer; }
	.ref-link:hover:not(:disabled) { background: var(--color-border); }
	.ref-link:disabled { cursor: not-allowed; color: var(--color-text-secondary); }
	.ref-link.dangling { border-color: var(--color-warning); }
	.ref-link.incoming { color: var(--color-success); }
	.dangling-badge { padding: 1px 5px; background: var(--color-warning); color: var(--color-bg-primary); border-radius: 2px; font-size: 9px; font-weight: 700; margin-left: auto; }
	.target-badge { padding: 1px 5px; background: var(--color-bg-primary); color: var(--color-text-secondary); border-radius: 2px; font-size: 9px; margin-left: auto; }
</style>
