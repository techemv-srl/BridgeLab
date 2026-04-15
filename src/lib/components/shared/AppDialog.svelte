<script lang="ts">
	import { t, subscribeLocale } from '$lib/i18n';
	import { dialogStore } from '$lib/stores/dialog.svelte';

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	let d = $derived(dialogStore.active);

	function handleOk() { dialogStore.close(true); }
	function handleCancel() { dialogStore.close(false); }

	function handleKeydown(e: KeyboardEvent) {
		if (!d) return;
		if (e.key === 'Escape') { e.preventDefault(); handleCancel(); }
		else if (e.key === 'Enter') { e.preventDefault(); handleOk(); }
	}

	function iconFor(kind: string): string {
		switch (kind) {
			case 'error': return '\u2716';
			case 'warning': return '\u26A0';
			case 'success': return '\u2714';
			case 'confirm': return '?';
			default: return '\u2139';
		}
	}

	function defaultTitle(kind: string): string {
		switch (kind) {
			case 'error': return tr('dialog.errorTitle');
			case 'warning': return tr('dialog.warningTitle');
			case 'success': return tr('dialog.successTitle');
			case 'confirm': return tr('dialog.confirmTitle');
			default: return tr('dialog.infoTitle');
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if d}
	<div class="dialog-overlay" onclick={handleCancel} role="presentation">
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<div class="dialog {d.kind}" onclick={(e) => e.stopPropagation()} role="alertdialog" aria-modal="true">
			<div class="dialog-header">
				<span class="dialog-icon">{iconFor(d.kind ?? 'info')}</span>
				<span class="dialog-title">{d.title ?? defaultTitle(d.kind ?? 'info')}</span>
				<button class="close-x" onclick={handleCancel} aria-label="Close">&times;</button>
			</div>
			<div class="dialog-body">
				<p class="dialog-message">{d.message}</p>
				{#if d.details}
					<pre class="dialog-details">{d.details}</pre>
				{/if}
			</div>
			<div class="dialog-footer">
				{#if d.showCancel}
					<button class="btn" onclick={handleCancel}>
						{d.cancelLabel ?? tr('dialog.cancel')}
					</button>
				{/if}
				<button class="btn btn-primary" onclick={handleOk} autofocus>
					{d.okLabel ?? tr('dialog.ok')}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.dialog-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.55);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 9999;
	}
	.dialog {
		background: var(--color-bg-secondary);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		width: 440px;
		max-width: 90vw;
		max-height: 80vh;
		display: flex;
		flex-direction: column;
		box-shadow: 0 10px 40px rgba(0, 0, 0, 0.5);
		border-left: 4px solid var(--color-accent);
	}
	.dialog.error { border-left-color: var(--color-error); }
	.dialog.warning { border-left-color: var(--color-warning); }
	.dialog.success { border-left-color: var(--color-success); }
	.dialog.confirm { border-left-color: var(--color-accent); }

	.dialog-header {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 12px 16px 10px;
		border-bottom: 1px solid var(--color-border);
	}
	.dialog-icon { font-size: 16px; font-weight: 700; }
	.dialog.error .dialog-icon { color: var(--color-error); }
	.dialog.warning .dialog-icon { color: var(--color-warning); }
	.dialog.success .dialog-icon { color: var(--color-success); }
	.dialog.confirm .dialog-icon,
	.dialog.info .dialog-icon { color: var(--color-accent); }

	.dialog-title { font-weight: 700; font-size: 13px; flex: 1; }
	.close-x {
		background: none;
		border: none;
		color: var(--color-text-secondary);
		font-size: 22px;
		cursor: pointer;
		line-height: 1;
		padding: 0 4px;
	}
	.close-x:hover { color: var(--color-text-primary); }

	.dialog-body {
		padding: 14px 16px;
		overflow-y: auto;
	}
	.dialog-message {
		font-size: 13px;
		color: var(--color-text-primary);
		margin: 0 0 8px;
		white-space: pre-wrap;
	}
	.dialog-details {
		font-family: 'JetBrains Mono', monospace;
		font-size: 11px;
		background: var(--color-bg-primary);
		border: 1px solid var(--color-border);
		border-radius: 4px;
		padding: 8px;
		color: var(--color-text-secondary);
		max-height: 180px;
		overflow: auto;
		white-space: pre-wrap;
		margin: 0;
	}

	.dialog-footer {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		padding: 10px 16px 12px;
		border-top: 1px solid var(--color-border);
	}
	.btn {
		padding: 5px 14px;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		background: var(--color-bg-tertiary);
		color: var(--color-text-primary);
		font-size: 12px;
		font-family: inherit;
		cursor: pointer;
	}
	.btn:hover { background: var(--color-border); }
	.btn-primary {
		background: var(--color-accent);
		color: var(--color-bg-primary);
		border-color: var(--color-accent);
	}
	.btn-primary:hover { opacity: 0.9; }
	.dialog.error .btn-primary { background: var(--color-error); border-color: var(--color-error); color: white; }
	.dialog.warning .btn-primary { background: var(--color-warning); border-color: var(--color-warning); color: var(--color-bg-primary); }
</style>
