<script lang="ts">
	// Segment grid: every occurrence of one segment type in the open HL7 v2
	// message as a table — one row per occurrence, the populated fields as
	// columns. Clicking a cell selects that field in the editor.
	import {
		getSegmentCounts, getSegmentGrid, defaultGridSegment,
		type SegmentCount, type SegmentGrid,
	} from '$lib/ipc/parser';
	import { untrack } from 'svelte';
	import { t, subscribeLocale } from '$lib/i18n';
	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		messageId: string;
		/** A request to show a segment (the tree's context menu); a new stamp
		 *  re-applies it even when it names the same segment again. */
		request?: { segment: string; stamp: number } | null;
		onNavigate: (segmentIdx: number, fieldPosition: number) => void;
	}

	let { messageId, request = null, onNavigate }: Props = $props();

	let counts = $state<SegmentCount[]>([]);
	let segment = $state<string | null>(null);
	let grid = $state<SegmentGrid | null>(null);
	let error = $state('');
	let filter = $state('');

	// Reload the picker when the message changes (a re-parse gives a new id),
	// keeping the chosen segment when the new message still has it.
	$effect(() => {
		const id = messageId;
		error = '';
		void (async () => {
			try {
				const c = await getSegmentCounts(id);
				if (id !== messageId) return;
				counts = c;
				segment = defaultGridSegment(c, untrack(() => segment ?? request?.segment));
			} catch (e) {
				error = String(e);
			}
		})();
	});

	// An explicit request selects its segment once; the picker stays free.
	$effect(() => {
		const r = request;
		if (r && untrack(() => counts).some((c) => c.segment_type === r.segment)) segment = r.segment;
	});

	$effect(() => {
		const id = messageId;
		const seg = segment;
		if (!seg) { grid = null; return; }
		void (async () => {
			try {
				const g = await getSegmentGrid(id, seg);
				if (id === messageId && seg === segment) grid = g;
			} catch (e) {
				error = String(e);
			}
		})();
	});

	let rows = $derived.by(() => {
		if (!grid) return [];
		const q = filter.trim().toLowerCase();
		if (!q) return grid.rows;
		return grid.rows.filter((r) =>
			r.cells.some((c) => c.value.toLowerCase().includes(q) || (c.code_desc ?? '').toLowerCase().includes(q)));
	});
</script>

<div class="grid-panel">
	<div class="grid-toolbar">
		<label for="grid-seg">{tr('grid.segment')}</label>
		<select id="grid-seg" bind:value={segment}>
			{#each counts as c (c.segment_type)}
				<option value={c.segment_type}>{c.segment_type} ({c.count})</option>
			{/each}
		</select>
		{#if grid}
			<span class="grid-meta">{grid.segment_name}{grid.segment_name ? ' · ' : ''}v{grid.version}</span>
		{/if}
		<input class="grid-filter" bind:value={filter} placeholder={tr('grid.filter')} />
		{#if grid}
			<span class="grid-meta">{tr('grid.rows', { shown: rows.length, total: grid.rows.length })}</span>
		{/if}
	</div>
	{#if error}
		<div class="grid-empty error">{error}</div>
	{:else if !grid}
		<div class="grid-empty">{tr('xsd.loading')}</div>
	{:else if grid.rows.length === 0}
		<div class="grid-empty">{tr('grid.none')}</div>
	{:else}
		<div class="grid-scroll">
			<table class="grid-table">
				<thead>
					<tr>
						<th class="idx">#</th>
						{#each grid.columns as col (col.position)}
							<th title={col.data_type ? `${col.name} (${col.data_type})` : col.name}>
								<div class="col-pos">{grid.segment_type}-{col.position}</div>
								{#if col.name}<div class="col-name">{col.name}</div>{/if}
							</th>
						{/each}
					</tr>
				</thead>
				<tbody>
					{#each rows as row (row.segment_idx)}
						<tr>
							<td class="idx">{grid.rows.indexOf(row) + 1}</td>
							{#each row.cells as cell, i (i)}
								<td
									title={cell.code_desc ? `${cell.value} — ${cell.code_desc}` : cell.value}
									onclick={() => grid && onNavigate(row.segment_idx, grid.columns[i].position)}
								>
									<span class="val">{cell.value}{#if cell.truncated}<span class="more">…</span>{/if}</span>
									{#if cell.code_desc}<span class="desc">{cell.code_desc}</span>{/if}
								</td>
							{/each}
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>

<style>
	.grid-panel { display: flex; flex-direction: column; height: 100%; min-height: 0; }
	.grid-toolbar { display: flex; align-items: center; gap: 8px; padding: 6px 10px; border-bottom: 1px solid var(--color-border); font-size: 12px; flex-wrap: wrap; }
	.grid-toolbar label { color: var(--color-text-secondary); }
	.grid-toolbar select, .grid-filter { padding: 3px 6px; border: 1px solid var(--color-border); border-radius: 3px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 12px; font-family: inherit; }
	.grid-filter { width: 180px; }
	.grid-meta { color: var(--color-text-secondary); font-size: 11px; }
	.grid-empty { padding: 16px; text-align: center; color: var(--color-text-secondary); font-style: italic; font-size: 12px; }
	.grid-empty.error { color: var(--color-error); font-style: normal; }
	.grid-scroll { flex: 1; overflow: auto; min-height: 0; }
	.grid-table { border-collapse: collapse; font-size: 11px; font-family: 'JetBrains Mono', monospace; }
	.grid-table th { position: sticky; top: 0; z-index: 1; background: var(--color-bg-tertiary); text-align: left; padding: 4px 8px; border-bottom: 1px solid var(--color-border); border-right: 1px solid var(--color-border); font-weight: 600; white-space: nowrap; }
	.col-pos { color: var(--color-accent); }
	.col-name { font-family: sans-serif; font-size: 10px; font-weight: 500; color: var(--color-text-secondary); }
	.grid-table td { padding: 3px 8px; border-bottom: 1px solid var(--color-border); border-right: 1px solid var(--color-border); white-space: nowrap; max-width: 320px; overflow: hidden; text-overflow: ellipsis; cursor: pointer; vertical-align: top; }
	.grid-table tbody tr:hover td { background: var(--color-bg-tertiary); }
	.grid-table .idx { color: var(--color-text-secondary); text-align: right; cursor: default; position: sticky; left: 0; background: var(--color-bg-secondary); }
	.grid-table th.idx { z-index: 2; background: var(--color-bg-tertiary); }
	.val { color: var(--color-text-primary); }
	.more { color: var(--color-text-secondary); }
	.desc { display: block; font-family: sans-serif; font-size: 10px; color: var(--color-text-secondary); }
</style>
