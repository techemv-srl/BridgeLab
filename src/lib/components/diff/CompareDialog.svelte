<script lang="ts">
	import MessageDiff from './MessageDiff.svelte';
	import type { MessageTab } from '$lib/stores/messages.svelte';
	import { t, subscribeLocale } from '$lib/i18n';

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		tabs: MessageTab[];
		activeTabId: string | null;
		theme?: string;
		onClose: () => void;
	}

	let { tabs, activeTabId, theme = 'bridgelab-dark', onClose }: Props = $props();

	// Default: left = active tab, right = first other tab. Initial values
	// only — the dialog is recreated on every open ({#if showCompare}).
	// svelte-ignore state_referenced_locally
	const initialLeft = activeTabId ?? tabs[0]?.id ?? '';
	let leftId = $state(initialLeft);
	// svelte-ignore state_referenced_locally
	let rightId = $state(tabs.find((t) => t.id !== initialLeft)?.id ?? '');

	let leftTab = $derived(tabs.find((t) => t.id === leftId));
	let rightTab = $derived(tabs.find((t) => t.id === rightId));

	function swap() {
		const tmp = leftId;
		leftId = rightId;
		rightId = tmp;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="modal-overlay" onclick={onClose} role="presentation">
	<!-- svelte-ignore a11y_interactive_supports_focus -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="modal modal-large" onclick={(e) => e.stopPropagation()} role="dialog">
		<div class="modal-header">
			<span>{tr('diff.title')}</span>
			<button class="modal-close" onclick={onClose}>&times;</button>
		</div>
		<div class="modal-body">
			<div class="diff-controls">
				<label class="diff-field">
					<span>{tr('diff.left')}</span>
					<select bind:value={leftId}>
						{#each tabs as tab (tab.id)}
							<option value={tab.id}>{tab.label}{tab.id === activeTabId ? ' •' : ''}</option>
						{/each}
					</select>
				</label>
				<button class="btn btn-swap" onclick={swap} title={tr('diff.swap')}>&#8646;</button>
				<label class="diff-field">
					<span>{tr('diff.right')}</span>
					<select bind:value={rightId}>
						{#each tabs as tab (tab.id)}
							<option value={tab.id}>{tab.label}{tab.id === activeTabId ? ' •' : ''}</option>
						{/each}
					</select>
				</label>
			</div>

			{#if leftId === rightId}
				<div class="diff-hint">{tr('diff.sameTab')}</div>
			{/if}

			<div class="diff-wrap">
				{#if leftTab && rightTab}
					{#key `${leftId}|${rightId}`}
						<MessageDiff
							originalText={leftTab.content}
							modifiedText={rightTab.content}
							originalLabel={leftTab.label}
							modifiedLabel={rightTab.label}
							{theme}
						/>
					{/key}
				{/if}
			</div>
		</div>
		<div class="modal-footer">
			<button class="btn" onclick={onClose}>{tr('modal.close')}</button>
		</div>
	</div>
</div>

<style>
	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal.modal-large {
		background: var(--color-bg, #1e1e2e);
		color: var(--color-text, #cdd6f4);
		border: 1px solid var(--color-border, #313244);
		border-radius: 6px;
		width: min(1100px, 94vw);
		height: min(760px, 88vh);
		display: flex;
		flex-direction: column;
		box-shadow: 0 10px 40px rgba(0, 0, 0, 0.4);
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--color-border, #313244);
		font-weight: 600;
	}

	.modal-close {
		background: transparent;
		border: none;
		color: var(--color-text, #cdd6f4);
		font-size: 1.25rem;
		cursor: pointer;
		padding: 0 0.25rem;
	}

	.modal-body {
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		overflow: hidden;
		flex: 1;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		border-top: 1px solid var(--color-border, #313244);
	}

	.diff-controls {
		display: flex;
		align-items: flex-end;
		gap: 0.75rem;
		flex-shrink: 0;
	}

	.diff-field {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		flex: 1;
		font-size: 0.8rem;
		color: var(--color-text-secondary, #a6adc8);
	}

	.diff-field select {
		padding: 0.35rem 0.5rem;
		border: 1px solid var(--color-border, #45475a);
		border-radius: 4px;
		background: var(--color-input-bg, #313244);
		color: var(--color-text, #cdd6f4);
		font-family: inherit;
		font-size: 0.85rem;
	}

	.diff-hint {
		font-size: 0.8rem;
		color: var(--color-text-secondary, #a6adc8);
		font-style: italic;
		flex-shrink: 0;
	}

	.diff-wrap {
		flex: 1;
		min-height: 0;
		border: 1px solid var(--color-border, #313244);
		border-radius: 4px;
		overflow: hidden;
	}

	.btn {
		background: var(--color-input-bg, #313244);
		color: var(--color-text, #cdd6f4);
		border: 1px solid var(--color-border, #45475a);
		border-radius: 4px;
		padding: 0.4rem 0.9rem;
		font-size: 0.85rem;
		cursor: pointer;
		font-family: inherit;
	}

	.btn:hover {
		filter: brightness(1.15);
	}

	.btn-swap {
		flex-shrink: 0;
		font-size: 1rem;
		padding: 0.35rem 0.6rem;
	}
</style>
