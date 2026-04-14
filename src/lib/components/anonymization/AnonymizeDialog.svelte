<script lang="ts">
	import { detectPhi, anonymizeMessage, type PhiLocation, type AnonymizeResult } from '$lib/ipc/anonymization';
	import { t } from '$lib/i18n';

	interface Props {
		messageId: string;
		onAnonymized: (text: string) => void;
		onClose: () => void;
	}

	let { messageId, onAnonymized, onClose }: Props = $props();

	let phiLocations = $state<PhiLocation[]>([]);
	let loading = $state(true);
	let anonymizing = $state(false);
	let result = $state<AnonymizeResult | null>(null);

	$effect(() => {
		loadPhi();
	});

	async function loadPhi() {
		loading = true;
		try {
			phiLocations = await detectPhi(messageId);
		} catch (e) {
			console.error('PHI detection failed:', e);
		}
		loading = false;
	}

	async function handleAnonymize() {
		anonymizing = true;
		try {
			result = await anonymizeMessage(messageId);
		} catch (e) {
			console.error('Anonymization failed:', e);
		}
		anonymizing = false;
	}

	function handleApply() {
		if (result) {
			onAnonymized(result.anonymized_text);
		}
	}

	function handleCopyAnonymized() {
		if (result) {
			navigator.clipboard.writeText(result.anonymized_text);
		}
	}

	function sensitivityBadge(s: string): string {
		switch (s) {
			case 'high': return 'HIGH';
			case 'medium': return 'MED';
			case 'low': return 'LOW';
			default: return s;
		}
	}
</script>

<div class="anon-dialog">
	<div class="anon-header">
		<span>{t('menu.tools.anonymize')}</span>
		<button class="close-btn" onclick={onClose}>&times;</button>
	</div>

	<div class="anon-body">
		{#if loading}
			<div class="anon-loading">Detecting PHI fields...</div>
		{:else if phiLocations.length === 0}
			<div class="anon-empty">No PHI fields detected in this message.</div>
		{:else}
			<div class="phi-summary">
				Found <strong>{phiLocations.length}</strong> PHI fields:
			</div>
			<div class="phi-list">
				{#each phiLocations as phi}
					<div class="phi-row">
						<span class="phi-badge {phi.sensitivity}">{sensitivityBadge(phi.sensitivity)}</span>
						<span class="phi-field">{phi.segment_type}-{phi.field_position}</span>
						<span class="phi-name">{phi.field_name}</span>
						<span class="phi-value">{phi.current_value}</span>
					</div>
				{/each}
			</div>
		{/if}

		{#if result}
			<div class="anon-result">
				<div class="result-header">Anonymized ({result.phi_fields_masked} fields masked)</div>
				<pre class="result-preview">{result.anonymized_text.substring(0, 1000)}{result.anonymized_text.length > 1000 ? '...' : ''}</pre>
			</div>
		{/if}
	</div>

	<div class="anon-footer">
		{#if !result}
			<button class="btn btn-primary" onclick={handleAnonymize} disabled={anonymizing || phiLocations.length === 0}>
				{anonymizing ? 'Anonymizing...' : 'Anonymize'}
			</button>
		{:else}
			<button class="btn btn-primary" onclick={handleApply}>Open in New Tab</button>
			<button class="btn" onclick={handleCopyAnonymized}>Copy to Clipboard</button>
		{/if}
		<button class="btn" onclick={onClose}>{t('dialog.cancel')}</button>
	</div>
</div>

<style>
	.anon-dialog { display: flex; flex-direction: column; max-height: 70vh; }
	.anon-header { display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; border-bottom: 1px solid var(--color-border); font-weight: 600; }
	.close-btn { background: none; border: none; color: var(--color-text-secondary); cursor: pointer; font-size: 20px; }
	.anon-body { flex: 1; overflow-y: auto; padding: 12px 16px; }
	.anon-loading, .anon-empty { text-align: center; color: var(--color-text-secondary); padding: 20px; font-style: italic; }
	.phi-summary { margin-bottom: 8px; font-size: 13px; }
	.phi-list { display: flex; flex-direction: column; gap: 2px; max-height: 200px; overflow-y: auto; }
	.phi-row { display: flex; align-items: center; gap: 8px; padding: 4px 6px; border-bottom: 1px solid var(--color-border); font-size: 12px; }
	.phi-badge { padding: 1px 6px; border-radius: 3px; font-size: 10px; font-weight: 700; flex-shrink: 0; }
	.phi-badge.high { background: var(--color-error); color: var(--color-bg-primary); }
	.phi-badge.medium { background: var(--color-warning); color: var(--color-bg-primary); }
	.phi-badge.low { background: var(--color-accent); color: var(--color-bg-primary); }
	.phi-field { font-family: 'JetBrains Mono', monospace; font-weight: 600; color: var(--color-segment); width: 50px; flex-shrink: 0; }
	.phi-name { flex: 1; color: var(--color-text-primary); }
	.phi-value { max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--color-text-secondary); font-family: 'JetBrains Mono', monospace; font-size: 11px; }
	.anon-result { margin-top: 12px; border: 1px solid var(--color-success); border-radius: 4px; }
	.result-header { padding: 4px 8px; background: var(--color-bg-tertiary); font-size: 11px; font-weight: 600; color: var(--color-success); }
	.result-preview { padding: 8px; margin: 0; font-size: 11px; font-family: 'JetBrains Mono', monospace; white-space: pre-wrap; word-break: break-all; max-height: 150px; overflow-y: auto; color: var(--color-text-primary); }
	.anon-footer { display: flex; gap: 8px; padding: 12px 16px; border-top: 1px solid var(--color-border); }
	.btn { padding: 5px 12px; border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 12px; font-family: inherit; cursor: pointer; }
	.btn:hover { background: var(--color-border); }
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.btn-primary { background: var(--color-accent); color: var(--color-bg-primary); border-color: var(--color-accent); }
</style>
