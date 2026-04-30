<script lang="ts">
	import type { LicenseStatus } from '$lib/ipc/licensing';
	import { t, subscribeLocale } from '$lib/i18n';

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		status: LicenseStatus;
		onActivate: () => void;
	}

	let { status, onActivate }: Props = $props();

	// Persist dismissal across restarts, scoped to the current license_type
	// so the banner reappears if the user transitions trial→free→expired etc.
	let dismissedFor = $state<string | null>(
		typeof window !== 'undefined'
			? localStorage.getItem('trial_banner_dismissed_for')
			: null
	);

	// Urgent = banner cannot be dismissed:
	// - expired Pro/Enterprise license (must reactivate to regain features)
	// - trial with ≤3 days left (final warning)
	// Free (trial elapsed) is NOT urgent — community tier remains usable.
	let urgent = $derived(
		status.license_type === 'expired' ||
		(status.license_type === 'trial' && (status.days_remaining ?? 0) <= 3)
	);

	let visible = $derived.by(() => {
		if (status.license_type === 'professional' || status.license_type === 'enterprise') return false;
		if (urgent) return true;
		return dismissedFor !== status.license_type;
	});

	function dismiss() {
		dismissedFor = status.license_type;
		if (typeof window !== 'undefined') {
			localStorage.setItem('trial_banner_dismissed_for', status.license_type);
		}
	}
</script>

{#if visible}
	<div class="trial-banner" class:urgent>
		<span class="banner-text">
			{#if status.license_type === 'trial'}
				{tr('banner.trialPro', { days: status.days_remaining ?? 0 })}
			{:else if status.license_type === 'free'}
				{tr('banner.freeAfterTrial')}
			{:else if status.license_type === 'expired'}
				{tr('banner.expired')}
			{/if}
		</span>
		<button class="banner-btn" onclick={onActivate}>
			{tr('activate')}
		</button>
		{#if !urgent}
			<button class="banner-dismiss" onclick={dismiss} aria-label="Dismiss">&times;</button>
		{/if}
	</div>
{/if}

<style>
	.trial-banner {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 12px;
		height: 28px;
		background-color: var(--color-warning);
		color: #1e1e2e;
		font-size: 11px;
		font-weight: 600;
		flex-shrink: 0;
	}

	.trial-banner.urgent {
		background-color: var(--color-error);
		color: white;
	}

	.banner-btn {
		padding: 2px 10px;
		border: 1px solid currentColor;
		border-radius: 3px;
		background: transparent;
		color: inherit;
		font-size: 10px;
		font-family: inherit;
		font-weight: 600;
		cursor: pointer;
	}

	.banner-btn:hover {
		background: rgba(0, 0, 0, 0.15);
	}

	.banner-dismiss {
		background: none;
		border: none;
		color: inherit;
		font-size: 16px;
		cursor: pointer;
		line-height: 1;
		padding: 0 4px;
		opacity: 0.7;
	}

	.banner-dismiss:hover {
		opacity: 1;
	}
</style>
