<script lang="ts">
	import type { LicenseStatus } from '$lib/ipc/licensing';

	interface Props {
		status: LicenseStatus;
		onActivate: () => void;
	}

	let { status, onActivate }: Props = $props();

	let typeLabel = $derived(() => {
		switch (status.license_type) {
			case 'trial': return 'Trial';
			case 'free': return 'Free';
			case 'professional': return 'Professional';
			case 'enterprise': return 'Enterprise';
			case 'expired': return 'Expired';
			default: return status.license_type;
		}
	});

	let urgent = $derived(
		status.license_type === 'expired' ||
		(status.license_type === 'trial' && (status.days_remaining ?? 0) <= 7)
	);
</script>

{#if status.license_type !== 'professional' && status.license_type !== 'enterprise'}
	<div class="trial-banner" class:urgent>
		<span class="banner-text">
			{#if status.license_type === 'trial'}
				Trial: {status.days_remaining} days remaining
			{:else if status.license_type === 'expired'}
				License expired
			{:else if status.license_type === 'free'}
				Free license - Non-commercial use only
			{/if}
		</span>
		<button class="banner-btn" onclick={onActivate}>
			{status.license_type === 'expired' ? 'Activate' : 'Upgrade'}
		</button>
	</div>
{/if}

<style>
	.trial-banner {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 12px;
		height: 24px;
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
		padding: 1px 10px;
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
</style>
