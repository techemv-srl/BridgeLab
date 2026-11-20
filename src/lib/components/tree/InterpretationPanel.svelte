<script lang="ts">
	// Interpretation panel: the open HL7 v2 message read as sentences
	// ("Patient DOE, JOHN, Male, born 1980-01-01…"), one block per segment.
	// Every value links to its field: a click selects it in the editor.
	import { getMessageReading } from '$lib/ipc/parser';
	import { interpretMessage, type Part, type SegmentSentences } from '$lib/hl7/interpretation';
	import type { FieldTarget } from '$lib/hl7/segment-lines';
	import { t, subscribeLocale } from '$lib/i18n';
	import type { Reading } from '$lib/hl7/interpretation';

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		messageId: string;
		onNavigate: (segmentIdx: number, target: FieldTarget) => void;
	}

	let { messageId, onNavigate }: Props = $props();

	// The reading of the message it was made from: while the next one loads
	// (a re-parse, another tab) the previous stays on screen, dimmed and
	// inert, so the panel neither flickers nor points into another message.
	let loaded = $state<{ id: string; reading: Reading } | null>(null);
	let error = $state('');
	let current = $derived(loaded?.id === messageId);

	$effect(() => {
		const id = messageId;
		error = '';
		void (async () => {
			try {
				const reading = await getMessageReading(id);
				if (id === messageId) loaded = { id, reading };
			} catch (e) {
				if (id === messageId) error = String(e);
			}
		})();
	});

	// The sentences follow the interface language.
	let blocks = $derived.by<SegmentSentences[]>(() => {
		void localeVersion;
		return loaded ? interpretMessage(loaded.reading) : [];
	});

	// A result with thousands of OBX would be thousands of blocks: draw a
	// page at a time, the next one when the end of the list comes into view.
	const PAGE = 200;
	let limit = $state(PAGE);
	let shown = $derived(blocks.slice(0, limit));
	let moreEl = $state<HTMLElement | null>(null);
	$effect(() => {
		const el = moreEl;
		if (!el || typeof IntersectionObserver === 'undefined') return;
		const io = new IntersectionObserver((entries) => {
			if (entries.some((e) => e.isIntersecting)) limit += PAGE;
		});
		io.observe(el);
		return () => io.disconnect();
	});

	function navigate(segmentIdx: number, target: FieldTarget) {
		if (current) onNavigate(segmentIdx, target);
	}

	function go(part: Part) {
		if (!part.link) return;
		const { segmentIdx, field, repetition, component } = part.link;
		navigate(segmentIdx, { field, repetition, component });
	}
</script>

<div class="interp-panel">
	{#if error}
		<div class="interp-error">{error}</div>
	{:else if !loaded}
		<!-- first reading on its way -->
	{:else if blocks.length === 0}
		<div class="interp-empty">{tr('interp.empty')}</div>
	{:else}
		<div class="interp-hint">{tr('interp.hint')}</div>
		<ol class="interp-list" class:stale={!current}>
			{#each shown as b (b.segmentIdx)}
				<li class="interp-block" class:generic={b.fieldByField}>
					<button
						class="interp-seg"
						title={b.segmentName || b.segmentType}
						onclick={() => navigate(b.segmentIdx, { field: 0, repetition: null, component: null })}
					>{b.segmentType}</button>
					<div class="interp-text">
						{#if b.fieldByField}
							<span class="interp-meta">{b.segmentName ? `${b.segmentName} · ` : ''}{tr('interp.fieldByField')}</span>
						{/if}
						{#each b.sentences as s, si (si)}
							<p class="interp-sentence">{#each s as part, pi (pi)}{#if part.link}<button class="interp-value" onclick={() => go(part)}>{part.text}</button>{:else}{part.text}{/if}{/each}</p>
						{/each}
					</div>
				</li>
			{/each}
		</ol>
		{#if blocks.length > shown.length}
			<button class="interp-more" bind:this={moreEl} onclick={() => (limit += PAGE)}>
				{tr('interp.more', { shown: shown.length, total: blocks.length })}
			</button>
		{/if}
	{/if}
</div>

<style>
	.interp-panel { height: 100%; overflow: auto; padding: 6px 10px; font-size: 13px; line-height: 1.5; }
	.interp-hint { color: var(--color-text-secondary); font-size: 12px; margin-bottom: 6px; }
	.interp-empty, .interp-error { color: var(--color-text-secondary); padding: 8px 0; }
	.interp-error { color: var(--color-error); }
	.interp-list { list-style: none; margin: 0; padding: 0; }
	.interp-list.stale { opacity: 0.55; pointer-events: none; }
	.interp-more {
		margin: 6px 0; background: none; border: 1px solid var(--color-border); border-radius: 3px;
		color: var(--color-text-secondary); font-size: 12px; padding: 2px 8px; cursor: pointer;
	}
	.interp-more:hover { color: var(--color-accent); }
	.interp-block { display: flex; gap: 10px; align-items: baseline; padding: 4px 0; border-bottom: 1px solid var(--color-border); }
	.interp-block:last-child { border-bottom: none; }
	.interp-seg {
		flex: 0 0 auto; min-width: 3.2em; font-family: var(--font-mono, monospace); font-size: 12px; font-weight: 600;
		background: var(--color-surface-2, transparent); color: var(--color-accent); border: 1px solid var(--color-border);
		border-radius: 3px; padding: 0 5px; cursor: pointer;
	}
	.interp-text { flex: 1 1 auto; min-width: 0; }
	.interp-meta { display: block; color: var(--color-text-secondary); font-size: 11px; }
	.interp-sentence { margin: 0; overflow-wrap: anywhere; }
	.generic .interp-sentence { color: var(--color-text-secondary); }
	.interp-value {
		all: unset; cursor: pointer; color: var(--color-text); border-bottom: 1px dotted var(--color-accent);
		font-family: inherit;
	}
	.interp-value:hover, .interp-value:focus-visible { color: var(--color-accent); border-bottom-style: solid; outline: none; }
</style>
