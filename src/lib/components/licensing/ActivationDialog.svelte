<script lang="ts">
	import { activateLicense, deactivateLicense, getHardwareId, type LicenseStatus } from '$lib/ipc/licensing';
	import { t, subscribeLocale } from '$lib/i18n';

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		currentStatus: LicenseStatus;
		onClose: () => void;
		onStatusChange: (status: LicenseStatus) => void;
	}

	let { currentStatus, onClose, onStatusChange }: Props = $props();

	let licenseKey = $state('');
	let hardwareId = $state('');
	let error = $state('');
	let activating = $state(false);

	// Live-decode the pasted key to preview licensee + email
	let keyPreview = $derived.by<{ licensee: string; email: string; type: string; features: string[] } | null>(() => {
		try {
			const trimmed = licenseKey.trim();
			if (!trimmed || trimmed.length < 20) return null;
			const json = atob(trimmed);
			const parsed = JSON.parse(json);
			const p = parsed.payload;
			if (p && p.licensee) {
				return {
					licensee: p.licensee,
					email: p.email || '',
					type: (p.license_type || '').replace('_', ' '),
					features: p.features || [],
				};
			}
		} catch { /* not valid Base64/JSON yet */ }
		return null;
	});

	let loaded = false;
	$effect(() => {
		if (loaded || typeof window === 'undefined') return;
		loaded = true;
		getHardwareId().then(id => { hardwareId = id; }).catch(() => {});
	});

	async function handleActivate() {
		if (!licenseKey.trim()) {
			error = tr('act.key') + ' is required';
			return;
		}
		error = '';
		activating = true;
		try {
			// Licensee and email are embedded in the signed key payload;
			// we pass empty strings since the backend extracts them from the key.
			const status = await activateLicense(licenseKey.trim(), '', '');
			if (status.is_valid) {
				onStatusChange(status);
				onClose();
			} else {
				error = status.message;
			}
		} catch (e) {
			error = String(e);
		}
		activating = false;
	}

	async function handleDeactivate() {
		try {
			const status = await deactivateLicense();
			onStatusChange(status);
		} catch (e) {
			error = String(e);
		}
	}

	let isActive = $derived(
		currentStatus.license_type !== 'trial' &&
		currentStatus.license_type !== 'expired'
	);
</script>

<div class="activation-dialog">
	<div class="dialog-header">
		<span>{tr('act.title')}</span>
		<button class="close-btn" onclick={onClose}>&times;</button>
	</div>

	<div class="dialog-body">
		<!-- Current status -->
		<div class="status-section">
			<div class="status-label">{tr('act.currentStatus')}</div>
			<div class="status-value" class:valid={currentStatus.is_valid} class:invalid={!currentStatus.is_valid}>
				{currentStatus.license_type.toUpperCase()}
				{#if currentStatus.days_remaining !== null}
					- {tr('act.daysRemaining', { days: currentStatus.days_remaining })}
				{/if}
			</div>
			{#if currentStatus.licensee}
				<div class="status-detail">{currentStatus.licensee}</div>
			{/if}
			{#if currentStatus.email}
				<div class="status-detail">{currentStatus.email}</div>
			{/if}
		</div>

		<!-- Hardware ID -->
		<div class="hw-section">
			<div class="status-label">{tr('act.hardwareId')}</div>
			<div class="hw-id">{hardwareId || '...'}</div>
			<div class="hw-hint">{tr('act.hwHint')}</div>
		</div>

		{#if isActive}
			<!-- Active license info -->
			<div class="active-section">
				<div class="feature-list">
					<div class="status-label">{tr('act.features')}</div>
					{#each currentStatus.features as feature}
						<span class="feature-tag">{feature}</span>
					{/each}
				</div>
				<button class="btn btn-danger" onclick={handleDeactivate}>
					{tr('act.deactivate')}
				</button>
			</div>
		{:else}
			<!-- Activation form: only the key field -->
			<div class="form-section">
				<div class="status-label">{tr('act.activate')}</div>
				<div class="form-row">
					<label for="act-key">{tr('act.key')}</label>
					<textarea id="act-key" bind:value={licenseKey}
						placeholder={tr('act.keyPlaceholder')}
						rows="4"
						class="input-full key-input"></textarea>
				</div>

				{#if keyPreview}
					<div class="key-preview">
						<div class="preview-row">
							<label>{tr('act.nameCompany')}</label>
							<input type="text" value={keyPreview.licensee} readonly class="input-full readonly" />
						</div>
						<div class="preview-row">
							<label>Email</label>
							<input type="text" value={keyPreview.email} readonly class="input-full readonly" />
						</div>
						<div class="preview-row">
							<label>{tr('act.features')}</label>
							<div class="preview-features">
								{#each keyPreview.features as feat}
									<span class="feature-tag">{feat}</span>
								{/each}
							</div>
						</div>
					</div>
				{:else if licenseKey.trim().length > 0}
					<div class="key-hint">{tr('plugins.loading')}</div>
				{/if}

				{#if error}
					<div class="error-msg">{error}</div>
				{/if}

				<button class="btn btn-primary" onclick={handleActivate} disabled={activating}>
					{activating ? tr('act.activating') : tr('activate')}
				</button>
			</div>

			<div class="license-types">
				<div class="status-label">{tr('act.licenseTypes')}</div>
				<div class="type-grid">
					<div class="type-card">
						<div class="type-name">Community</div>
						<div class="type-desc">{tr('act.communityDesc')}</div>
						<div class="type-price">Free</div>
					</div>
					<div class="type-card highlight">
						<div class="type-name">Professional</div>
						<div class="type-desc">{tr('act.proDesc')}</div>
						<div class="type-price">{tr('act.contactUs')}</div>
					</div>
					<div class="type-card">
						<div class="type-name">Enterprise</div>
						<div class="type-desc">{tr('act.entDesc')}</div>
						<div class="type-price">{tr('act.contactUs')}</div>
					</div>
				</div>
				<div class="contact-info">
					info@techemv.it &middot; www.techemv.it
				</div>
			</div>
		{/if}
	</div>
</div>

<style>
	.activation-dialog { display: flex; flex-direction: column; max-height: 80vh; }
	.dialog-header { display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; border-bottom: 1px solid var(--color-border); font-weight: 700; font-size: 14px; }
	.close-btn { background: none; border: none; color: var(--color-text-secondary); cursor: pointer; font-size: 20px; }
	.dialog-body { flex: 1; overflow-y: auto; padding: 16px; display: flex; flex-direction: column; gap: 16px; }

	.status-label { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; color: var(--color-text-secondary); margin-bottom: 4px; }
	.status-value { font-size: 14px; font-weight: 700; }
	.status-value.valid { color: var(--color-success); }
	.status-value.invalid { color: var(--color-error); }
	.status-detail { font-size: 12px; color: var(--color-text-secondary); }

	.hw-id { font-family: 'JetBrains Mono', monospace; font-size: 11px; color: var(--color-text-secondary); background: var(--color-bg-tertiary); padding: 6px 10px; border-radius: 3px; user-select: all; }
	.hw-hint { font-size: 10px; color: var(--color-text-secondary); margin-top: 4px; font-style: italic; }

	.feature-list { display: flex; flex-wrap: wrap; gap: 4px; margin-bottom: 12px; }
	.feature-tag { padding: 2px 8px; background: var(--color-accent); color: var(--color-bg-primary); border-radius: 10px; font-size: 10px; font-weight: 600; }

	.form-row { display: flex; flex-direction: column; gap: 3px; margin-bottom: 8px; }
	.form-row label { font-size: 11px; color: var(--color-text-secondary); }
	.input-full { width: 100%; padding: 6px 8px; border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-family: 'JetBrains Mono', monospace; font-size: 12px; }
	.key-input { resize: vertical; min-height: 80px; line-height: 1.4; }
	.key-hint { font-size: 10px; color: var(--color-text-secondary); font-style: italic; }

	.key-preview { display: flex; flex-direction: column; gap: 6px; padding: 10px; background: var(--color-bg-tertiary); border-radius: 6px; border: 1px solid var(--color-border); }
	.preview-row { display: flex; flex-direction: column; gap: 2px; }
	.preview-row label { font-size: 10px; color: var(--color-text-secondary); font-weight: 600; text-transform: uppercase; letter-spacing: 0.3px; }
	.readonly { opacity: 0.85; cursor: default; background: var(--color-bg-primary); border-color: transparent; }
	.preview-features { display: flex; flex-wrap: wrap; gap: 4px; }

	.error-msg { padding: 6px 10px; background: var(--color-error); color: white; border-radius: 4px; font-size: 11px; margin-bottom: 8px; }

	.btn { padding: 6px 16px; border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 12px; font-family: inherit; cursor: pointer; }
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.btn-primary { background: var(--color-accent); color: var(--color-bg-primary); border-color: var(--color-accent); }
	.btn-danger { background: var(--color-error); color: white; border-color: var(--color-error); }

	.type-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }
	.type-card { padding: 10px; border: 1px solid var(--color-border); border-radius: 6px; text-align: center; }
	.type-card.highlight { border-color: var(--color-accent); background: var(--color-bg-tertiary); }
	.type-name { font-weight: 700; font-size: 13px; margin-bottom: 4px; }
	.type-desc { font-size: 10px; color: var(--color-text-secondary); margin-bottom: 6px; }
	.type-price { font-size: 12px; font-weight: 700; color: var(--color-accent); }

	.contact-info { text-align: center; font-size: 11px; color: var(--color-text-secondary); margin-top: 12px; }
</style>
