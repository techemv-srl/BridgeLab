<script lang="ts">
	import { listenerStore, AUTO_OPEN_BURST, AUTO_OPEN_BYTES } from '$lib/stores/listener.svelte';
	import { t, subscribeLocale } from '$lib/i18n';

	let { onShowConsole }: { onShowConsole: () => void } = $props();

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	// One notice for a burst of received messages, however many: after
	// AUTO_OPEN_BURST tabs the listener stops opening them while nobody is
	// at the window, and only logs them in the console and the History.
</script>

{#if listenerStore.heldBack > 0}
	<div class="held-notice" role="status">
		<span>{tr('comm.inboxHeldBack', { count: listenerStore.heldBack, cap: AUTO_OPEN_BURST, mb: AUTO_OPEN_BYTES / (1024 * 1024) })}</span>
		<button class="held-btn" onclick={onShowConsole}>{tr('comm.inboxShowConsole')}</button>
		<button class="held-dismiss" onclick={() => listenerStore.resumeAutoOpen()} aria-label={tr('modal.close')}>&times;</button>
	</div>
{/if}

<style>
	.held-notice {
		display: flex;
		align-items: center;
		justify-content: center;
		flex-wrap: wrap;
		gap: 4px 12px;
		min-height: 28px;
		padding: 3px 8px;
		background-color: var(--color-bg-tertiary);
		color: var(--color-text-primary);
		border-bottom: 1px solid var(--color-border);
		font-size: 11px;
		flex-shrink: 0;
	}

	.held-btn {
		padding: 2px 10px;
		font-size: 11px;
		font-weight: 600;
		font-family: inherit;
		color: var(--color-bg-primary);
		background: var(--color-accent);
		border: none;
		border-radius: 3px;
		cursor: pointer;
	}

	.held-dismiss {
		background: none;
		border: none;
		color: inherit;
		font-size: 16px;
		cursor: pointer;
		line-height: 1;
		padding: 0 4px;
		opacity: 0.7;
	}
</style>
