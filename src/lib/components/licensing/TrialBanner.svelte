<script lang="ts">
	import type { LicenseStatus } from '$lib/ipc/licensing';
	import { getHardwareId } from '$lib/ipc/licensing';
	import { getPreference, setPreference } from '$lib/ipc/database';
	import { t, subscribeLocale } from '$lib/i18n';

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		status: LicenseStatus;
		onActivate: () => void;
	}

	let { status, onActivate }: Props = $props();

	const PLANS_URL = 'https://techemv-srl.github.io/BridgeLab/';
	const CONTACT_EMAIL = 'info@techemv.it';
	const DISMISS_KEY = 'trial_banner_dismissed_for';

	// Prefetched for the quote mailto; best-effort (both are Tauri-only IPC,
	// web mode simply omits them from the email body).
	let hwid = $state('');
	let appVersion = $state('');
	if (typeof window !== 'undefined') {
		getHardwareId().then((id) => { hwid = id; }).catch(() => {});
		import('@tauri-apps/api/app')
			.then(({ getVersion }) => getVersion())
			.then((v) => { appVersion = v; })
			.catch(() => {});
	}

	// Persist dismissal across restarts, scoped to the current license_type
	// so the banner reappears if the user transitions trial→free→expired etc.
	// Stored in the preferences DB (portable with the profile); reads migrate
	// any pre-0.7 localStorage value, and web mode falls back to localStorage.
	let dismissedFor = $state<string | null>(null);
	let dismissLoaded = $state(false);
	if (typeof window !== 'undefined') {
		(async () => {
			try {
				dismissedFor = await getPreference(DISMISS_KEY);
				if (dismissedFor === null) {
					const legacy = localStorage.getItem(DISMISS_KEY);
					if (legacy) {
						dismissedFor = legacy;
						void setPreference(DISMISS_KEY, legacy);
					}
				}
			} catch {
				dismissedFor = localStorage.getItem(DISMISS_KEY);
			}
			dismissLoaded = true;
		})();
	}

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
		// Don't flash the banner before the persisted dismissal is known
		if (!dismissLoaded) return false;
		return dismissedFor !== status.license_type;
	});

	function dismiss() {
		dismissedFor = status.license_type;
		if (typeof window !== 'undefined') {
			setPreference(DISMISS_KEY, status.license_type).catch(() => {
				localStorage.setItem(DISMISS_KEY, status.license_type);
			});
		}
	}

	function openPlans() {
		// window.open is a no-op inside the Tauri webview — use the opener
		// plugin, with the browser fallback for web mode.
		import('@tauri-apps/plugin-opener')
			.then(({ openUrl }) => openUrl(PLANS_URL))
			.catch(() => { window.open(PLANS_URL, '_blank'); });
	}

	function requestQuote() {
		const subject = tr('banner.quoteSubject');
		const body = tr('banner.quoteBody', { hwid: hwid || '-', version: appVersion || '-' });
		const url = `mailto:${CONTACT_EMAIL}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
		import('@tauri-apps/plugin-opener')
			.then(({ openUrl }) => openUrl(url))
			.catch(() => { window.location.href = url; });
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
		{#if status.license_type === 'free' || status.license_type === 'expired'}
			<button class="banner-btn" onclick={requestQuote}>
				{tr('banner.requestQuote')}
			</button>
		{/if}
		<button class="banner-btn banner-btn-ghost" onclick={openPlans}>
			{tr('banner.comparePlans')}
		</button>
		{#if !urgent}
			<button class="banner-dismiss" onclick={dismiss} aria-label={tr('modal.close')}>&times;</button>
		{/if}
	</div>
{/if}

<style>
	.trial-banner {
		display: flex;
		align-items: center;
		justify-content: center;
		/* Wrap on narrow (web) viewports: the post-trial text is long and the
		   browser build has no minimum window width — grow instead of clipping. */
		flex-wrap: wrap;
		gap: 2px 12px;
		min-height: 28px;
		padding: 3px 8px;
		background-color: var(--color-warning);
		color: #1e1e2e;
		font-size: 11px;
		font-weight: 600;
		flex-shrink: 0;
	}

	.banner-text {
		text-align: center;
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

	.banner-btn-ghost {
		border-color: transparent;
		opacity: 0.85;
		font-weight: 500;
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
