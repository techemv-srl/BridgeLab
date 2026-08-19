<script lang="ts">
	import { getPreference, setPreference } from '$lib/ipc/database';
	import { t, subscribeLocale } from '$lib/i18n';

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	// Non-blocking notice pushed by the license server through a telemetry
	// response (e.g. a revoked activation code). The local license is never
	// touched automatically — this banner is the only consequence, and
	// dismissing it clears the stored notice.
	const NOTICE_KEY = 'license_server_notice';

	let notice = $state('');
	if (typeof window !== 'undefined') {
		getPreference(NOTICE_KEY)
			.then((v) => { notice = v ?? ''; })
			.catch(() => {});
	}

	function dismiss() {
		notice = '';
		setPreference(NOTICE_KEY, '').catch(() => {});
	}
</script>

{#if notice}
	<div class="notice-banner">
		<span class="notice-text"><strong>{tr('banner.licenseNotice')}</strong> {notice}</span>
		<button class="notice-dismiss" onclick={dismiss} aria-label={tr('modal.close')}>&times;</button>
	</div>
{/if}

<style>
	.notice-banner {
		display: flex;
		align-items: center;
		justify-content: center;
		flex-wrap: wrap;
		gap: 2px 12px;
		min-height: 28px;
		padding: 3px 8px;
		background-color: var(--color-error);
		color: white;
		font-size: 11px;
		font-weight: 600;
		flex-shrink: 0;
	}

	.notice-text { text-align: center; }

	.notice-dismiss {
		background: none;
		border: none;
		color: inherit;
		font-size: 16px;
		cursor: pointer;
		line-height: 1;
		padding: 0 4px;
		opacity: 0.7;
	}

	.notice-dismiss:hover { opacity: 1; }
</style>
