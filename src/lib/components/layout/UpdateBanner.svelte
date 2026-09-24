<script lang="ts">
	import { getPreference, setPreference } from '$lib/ipc/database';
	import { t, subscribeLocale } from '$lib/i18n';
	import {
		fetchLatestRelease, startupCheckDue, shouldNotify, getUpdatePolicy, effectivePreference,
		PREF_STARTUP_CHECK, PREF_LAST_CHECK, PREF_SKIPPED, type LatestRelease,
	} from '$lib/updates';

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	// Once a day, a few seconds after startup, ask GitHub for the latest
	// release (see $lib/updates). Any failure — offline machine, proxy,
	// rate limit — is silent: this banner is the only thing it can show.
	const STARTUP_DELAY_MS = 8_000;

	let release = $state<LatestRelease | null>(null);
	let current = $state('');

	async function check() {
		try {
			const [userPref, last, policy] = await Promise.all([
				getPreference(PREF_STARTUP_CHECK).catch(() => null),
				getPreference(PREF_LAST_CHECK).catch(() => null),
				getUpdatePolicy().catch(() => null),
			]);
			const { value: enabled, seedFromInstaller } = effectivePreference(policy, userPref);
			// The Windows setup asked; its answer becomes the user's preference
			// once, and Settings shows it from then on.
			if (seedFromInstaller && enabled) await setPreference(PREF_STARTUP_CHECK, enabled).catch(() => {});
			if (!startupCheckDue(enabled, last, Date.now())) return;
			const { getVersion } = await import('@tauri-apps/api/app');
			current = await getVersion();
			const latest = await fetchLatestRelease(5_000);
			await setPreference(PREF_LAST_CHECK, String(Date.now())).catch(() => {});
			const skipped = await getPreference(PREF_SKIPPED).catch(() => null);
			if (shouldNotify(latest.version, current, skipped)) release = latest;
		} catch {
			/* silent by design */
		}
	}

	$effect(() => {
		if (typeof window === 'undefined') return;
		const timer = setTimeout(check, STARTUP_DELAY_MS);
		return () => clearTimeout(timer);
	});

	async function download() {
		if (!release) return;
		const url = release.url;
		try {
			const { openUrl } = await import('@tauri-apps/plugin-opener');
			await openUrl(url);
		} catch {
			window.open(url, '_blank');
		}
	}

	function skip() {
		if (release) setPreference(PREF_SKIPPED, release.version).catch(() => {});
		release = null;
	}
</script>

{#if release}
	<div class="update-banner" role="status">
		<span>{tr('update.bannerAvailable', { version: release.version, current })}</span>
		<button class="update-btn" onclick={download}>{tr('update.download')}</button>
		<button class="update-btn update-btn-ghost" onclick={skip}>{tr('update.skip')}</button>
		<button class="update-dismiss" onclick={() => { release = null; }} aria-label={tr('modal.close')}>&times;</button>
	</div>
{/if}

<style>
	.update-banner {
		display: flex;
		align-items: center;
		justify-content: center;
		flex-wrap: wrap;
		gap: 4px 12px;
		min-height: 28px;
		padding: 3px 8px;
		background-color: var(--color-accent);
		color: var(--color-bg-primary);
		font-size: 11px;
		font-weight: 600;
		flex-shrink: 0;
	}

	.update-btn {
		padding: 2px 10px;
		font-size: 11px;
		font-weight: 700;
		font-family: inherit;
		color: var(--color-accent);
		background: var(--color-bg-primary);
		border: none;
		border-radius: 3px;
		cursor: pointer;
	}

	.update-btn-ghost {
		color: var(--color-bg-primary);
		background: transparent;
		border: 1px solid var(--color-bg-primary);
	}

	.update-dismiss {
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
