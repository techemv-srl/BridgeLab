<script lang="ts">
	import {
		listFhirPackages, installFhirPackage, removeFhirPackage, fhirPackagesDir,
		type FhirPackageInfo,
	} from '$lib/ipc/fhirPackages';
	import { parseUpgradeError } from '$lib/ipc/licensing';
	import { t } from '$lib/i18n';

	interface Props {
		onClose: () => void;
	}

	let { onClose }: Props = $props();

	let packages = $state<FhirPackageInfo[]>([]);
	let dir = $state('');
	let loading = $state(true);
	let installing = $state(false);
	let error = $state<string | null>(null);

	$effect(() => {
		load();
	});

	async function load() {
		loading = true;
		try {
			packages = await listFhirPackages();
			dir = await fhirPackagesDir();
		} catch (e) {
			error = String(e);
		}
		loading = false;
	}

	async function install() {
		error = null;
		const { open } = await import('@tauri-apps/plugin-dialog');
		const picked = await open({
			multiple: false,
			filters: [{ name: 'FHIR package', extensions: ['tgz', 'gz'] }],
		});
		if (typeof picked !== 'string') return;

		// A core package holds thousands of files; reading it takes a moment,
		// so the button has to say what is happening.
		installing = true;
		try {
			packages = await installFhirPackage(picked);
		} catch (e) {
			const up = parseUpgradeError(e);
			error = up ? t('upgrade.required', { tier: up.tier }) : String(e);
		}
		installing = false;
	}

	async function remove(pkg: FhirPackageInfo) {
		error = null;
		try {
			packages = await removeFhirPackage(pkg.name, pkg.version);
		} catch (e) {
			error = String(e);
		}
	}
</script>

<div class="modal-backdrop" role="presentation" onclick={onClose}>
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div class="modal" onclick={(e) => e.stopPropagation()}>
		<div class="modal-header">
			<h2>{t('fhirPackages.title')}</h2>
			<button class="close-btn" onclick={onClose} aria-label="Close">×</button>
		</div>

		<div class="modal-body">
			<p class="intro">{t('fhirPackages.intro')}</p>

			{#if error}
				<div class="banner error">{error}</div>
			{/if}

			{#if loading}
				<div class="banner">{t('fhirPackages.loading')}</div>
			{:else if packages.length === 0}
				<div class="empty">
					<p>{t('fhirPackages.none')}</p>
					<p class="hint">{t('fhirPackages.whereToGet')}</p>
				</div>
			{:else}
				<table class="packages">
					<thead>
						<tr>
							<th>{t('fhirPackages.package')}</th>
							<th>{t('fhirPackages.version')}</th>
							<th>{t('fhirPackages.fhirVersion')}</th>
							<th class="num">{t('fhirPackages.profiles')}</th>
							<th></th>
						</tr>
					</thead>
					<tbody>
						{#each packages as pkg (pkg.name + pkg.version)}
							<tr>
								<td>
									<span class="pkg-name">{pkg.name}</span>
									{#if pkg.title}<span class="pkg-title">{pkg.title}</span>{/if}
								</td>
								<td>{pkg.version}</td>
								<td>{pkg.fhir_version || '—'}</td>
								<td class="num">{pkg.profile_count}</td>
								<td>
									<button class="btn small danger" onclick={() => remove(pkg)}>
										{t('fhirPackages.remove')}
									</button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}

			<p class="dir" title={dir}>{t('fhirPackages.storedIn')} <code>{dir}</code></p>
		</div>

		<div class="modal-footer">
			<button class="btn" onclick={onClose}>{t('common.close')}</button>
			<button class="btn btn-primary" onclick={install} disabled={installing}>
				{installing ? t('fhirPackages.installing') : t('fhirPackages.install')}
			</button>
		</div>
	</div>
</div>

<style>
	.modal-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 1000; }
	.modal { background: var(--color-bg-secondary); border: 1px solid var(--color-border); border-radius: 8px; display: flex; flex-direction: column; max-height: 85vh; width: min(760px, 92vw); box-shadow: 0 8px 32px rgba(0,0,0,0.4); }
	.modal-header { display: flex; align-items: center; justify-content: space-between; padding: 12px 16px; border-bottom: 1px solid var(--color-border); }
	.modal-header h2 { margin: 0; font-size: 14px; }
	.close-btn { background: none; border: none; color: var(--color-text-secondary); font-size: 20px; cursor: pointer; line-height: 1; }
	.modal-body { padding: 12px 16px; overflow-y: auto; font-size: 12px; }
	.modal-footer { display: flex; justify-content: flex-end; gap: 8px; padding: 10px 16px; border-top: 1px solid var(--color-border); }

	.intro { margin: 0 0 10px; color: var(--color-text-secondary); }
	.banner { padding: 8px 10px; border-radius: 4px; background: var(--color-bg-tertiary); margin-bottom: 8px; }
	.banner.error { background: rgba(220, 60, 60, 0.15); color: var(--color-error, #e06c6c); }
	.empty { padding: 16px; text-align: center; color: var(--color-text-secondary); }
	.hint { font-size: 11px; }

	.packages { width: 100%; border-collapse: collapse; }
	.packages th { text-align: left; font-size: 10px; text-transform: uppercase; color: var(--color-text-secondary); padding: 4px 8px; border-bottom: 1px solid var(--color-border); }
	.packages td { padding: 6px 8px; border-bottom: 1px solid var(--color-border); vertical-align: top; }
	.packages .num { text-align: right; }
	.pkg-name { display: block; font-family: var(--font-mono, monospace); }
	.pkg-title { display: block; font-size: 10px; color: var(--color-text-secondary); }

	.btn { padding: 5px 12px; border: 1px solid var(--color-border); border-radius: 4px; background: none; color: var(--color-text-primary); font-family: inherit; font-size: 12px; cursor: pointer; }
	.btn.small { padding: 2px 8px; font-size: 11px; }
	.btn.danger { color: #e06c6c; }
	.btn-primary { background: var(--color-accent); border-color: var(--color-accent); color: #fff; }
	.btn:disabled { opacity: 0.6; cursor: default; }

	.dir { margin: 10px 0 0; font-size: 10px; color: var(--color-text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
