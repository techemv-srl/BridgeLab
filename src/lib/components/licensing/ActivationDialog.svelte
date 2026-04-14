<script lang="ts">
	import { activateLicense, deactivateLicense, getHardwareId, type LicenseStatus } from '$lib/ipc/licensing';

	interface Props {
		currentStatus: LicenseStatus;
		onClose: () => void;
		onStatusChange: (status: LicenseStatus) => void;
	}

	let { currentStatus, onClose, onStatusChange }: Props = $props();

	let licenseKey = $state('');
	let licensee = $state('');
	let email = $state('');
	let hardwareId = $state('');
	let error = $state('');
	let activating = $state(false);

	// Load hardware ID
	let loaded = false;
	$effect(() => {
		if (loaded || typeof window === 'undefined') return;
		loaded = true;
		getHardwareId().then(id => { hardwareId = id; }).catch(() => {});
	});

	async function handleActivate() {
		if (!licenseKey.trim()) {
			error = 'Please enter a license key';
			return;
		}
		error = '';
		activating = true;
		try {
			const status = await activateLicense(licenseKey.trim(), licensee.trim(), email.trim());
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
		<span>License Activation</span>
		<button class="close-btn" onclick={onClose}>&times;</button>
	</div>

	<div class="dialog-body">
		<!-- Current status -->
		<div class="status-section">
			<div class="status-label">Current Status</div>
			<div class="status-value" class:valid={currentStatus.is_valid} class:invalid={!currentStatus.is_valid}>
				{currentStatus.license_type.toUpperCase()}
				{#if currentStatus.days_remaining !== null}
					- {currentStatus.days_remaining} days remaining
				{/if}
			</div>
			{#if currentStatus.licensee}
				<div class="status-detail">Licensed to: {currentStatus.licensee}</div>
			{/if}
		</div>

		<!-- Hardware ID -->
		<div class="hw-section">
			<div class="status-label">Hardware ID</div>
			<div class="hw-id">{hardwareId || '...'}</div>
		</div>

		{#if isActive}
			<!-- Active license info -->
			<div class="active-section">
				<div class="feature-list">
					<div class="status-label">Features</div>
					{#each currentStatus.features as feature}
						<span class="feature-tag">{feature}</span>
					{/each}
				</div>
				<button class="btn btn-danger" onclick={handleDeactivate}>
					Deactivate License
				</button>
			</div>
		{:else}
			<!-- Activation form -->
			<div class="form-section">
				<div class="status-label">Activate License</div>
				<div class="form-row">
					<label for="act-key">License Key</label>
					<input id="act-key" bind:value={licenseKey}
						placeholder="BL-PRO-XXXXXXXX"
						class="input-full" />
				</div>
				<div class="form-row">
					<label for="act-name">Name / Company</label>
					<input id="act-name" bind:value={licensee}
						placeholder="Your name or company"
						class="input-full" />
				</div>
				<div class="form-row">
					<label for="act-email">Email</label>
					<input id="act-email" type="email" bind:value={email}
						placeholder="your@email.com"
						class="input-full" />
				</div>

				{#if error}
					<div class="error-msg">{error}</div>
				{/if}

				<button class="btn btn-primary" onclick={handleActivate} disabled={activating}>
					{activating ? 'Activating...' : 'Activate License'}
				</button>
			</div>

			<div class="license-types">
				<div class="status-label">License Types</div>
				<div class="type-grid">
					<div class="type-card">
						<div class="type-name">Free</div>
						<div class="type-desc">Non-commercial use. Core HL7v2 features.</div>
					</div>
					<div class="type-card highlight">
						<div class="type-name">Professional</div>
						<div class="type-desc">FHIR, MLLP, HTTP, anonymization, export.</div>
					</div>
					<div class="type-card">
						<div class="type-name">Enterprise</div>
						<div class="type-desc">All features + SOAP, plugins, priority support.</div>
					</div>
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

	.status-section, .hw-section, .active-section, .form-section, .license-types { }
	.status-label { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; color: var(--color-text-secondary); margin-bottom: 4px; }
	.status-value { font-size: 14px; font-weight: 700; }
	.status-value.valid { color: var(--color-success); }
	.status-value.invalid { color: var(--color-error); }
	.status-detail { font-size: 12px; color: var(--color-text-secondary); }

	.hw-id { font-family: 'JetBrains Mono', monospace; font-size: 11px; color: var(--color-text-secondary); background: var(--color-bg-tertiary); padding: 4px 8px; border-radius: 3px; user-select: all; }

	.feature-list { display: flex; flex-wrap: wrap; gap: 4px; margin-bottom: 12px; }
	.feature-tag { padding: 2px 8px; background: var(--color-accent); color: var(--color-bg-primary); border-radius: 10px; font-size: 10px; font-weight: 600; }

	.form-row { display: flex; flex-direction: column; gap: 3px; margin-bottom: 8px; }
	.form-row label { font-size: 11px; color: var(--color-text-secondary); }
	.input-full { width: 100%; padding: 6px 8px; border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-family: 'JetBrains Mono', monospace; font-size: 12px; }

	.error-msg { padding: 6px 10px; background: var(--color-error); color: white; border-radius: 4px; font-size: 11px; margin-bottom: 8px; }

	.btn { padding: 6px 16px; border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 12px; font-family: inherit; cursor: pointer; }
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.btn-primary { background: var(--color-accent); color: var(--color-bg-primary); border-color: var(--color-accent); }
	.btn-danger { background: var(--color-error); color: white; border-color: var(--color-error); }

	.type-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }
	.type-card { padding: 10px; border: 1px solid var(--color-border); border-radius: 6px; text-align: center; }
	.type-card.highlight { border-color: var(--color-accent); background: var(--color-bg-tertiary); }
	.type-name { font-weight: 700; font-size: 13px; margin-bottom: 4px; }
	.type-desc { font-size: 10px; color: var(--color-text-secondary); }
</style>
