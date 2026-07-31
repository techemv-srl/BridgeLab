<script lang="ts">
	import { batchValidate, type BatchFileResult } from '$lib/ipc/batch';
	import { parseUpgradeError } from '$lib/ipc/licensing';
	import { t, subscribeLocale } from '$lib/i18n';
	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		onClose: () => void;
		/** Open one of the validated files in a new editor tab. */
		onOpenFile: (path: string) => void;
	}

	let { onClose, onOpenFile }: Props = $props();

	let results = $state<BatchFileResult[]>([]);
	let skipped = $state(0);
	let running = $state(false);
	let errorMsg = $state('');
	let onlyFailures = $state(false);
	let ranOnce = $state(false);

	function isFailure(r: BatchFileResult): boolean {
		return r.parse_error !== null || r.error_count > 0;
	}

	let visible = $derived(onlyFailures ? results.filter(isFailure) : results);
	let failCount = $derived(results.filter(isFailure).length);

	async function run(paths: string[]) {
		if (paths.length === 0) return;
		running = true;
		errorMsg = '';
		try {
			const report = await batchValidate(paths);
			results = report.results;
			skipped = report.skipped;
			ranOnce = true;
		} catch (e) {
			const up = parseUpgradeError(e);
			errorMsg = up ? tr('upgrade.required', { tier: up.tier }) : String(e);
		} finally {
			running = false;
		}
	}

	async function pickFiles() {
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const sel = await open({
				multiple: true,
				filters: [{ name: 'HL7', extensions: ['hl7', 'txt', 'dat'] }],
			});
			if (!sel) return;
			await run(Array.isArray(sel) ? sel : [sel]);
		} catch (e) { errorMsg = String(e); }
	}

	async function pickFolder() {
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const sel = await open({ directory: true });
			if (!sel) return;
			await run(Array.isArray(sel) ? sel : [sel]);
		} catch (e) { errorMsg = String(e); }
	}

	async function exportCsv() {
		if (results.length === 0) return;
		// Quote AND neutralize formula-leading characters (=, +, -, @, tab, CR):
		// filenames or MSH-9 values are untrusted input for the spreadsheet
		// that will open this CSV.
		const esc = (s: string) => {
			const neutralized = /^[=+\-@\t\r]/.test(s) ? "'" + s : s;
			return '"' + neutralized.replaceAll('"', '""') + '"';
		};
		const rows = [
			['file', 'path', 'message_type', 'version', 'segments', 'errors', 'warnings', 'status'].join(','),
			...results.map((r) => [
				esc(r.file_name), esc(r.path), esc(r.message_type), esc(r.version),
				String(r.segment_count), String(r.error_count), String(r.warning_count),
				esc(r.parse_error ?? (r.error_count > 0 ? 'INVALID' : 'OK')),
			].join(',')),
		].join('\n');
		try {
			const { save } = await import('@tauri-apps/plugin-dialog');
			const { writeTextFile } = await import('@tauri-apps/plugin-fs');
			const path = await save({
				defaultPath: 'batch-validation.csv',
				filters: [{ name: 'CSV', extensions: ['csv'] }],
			});
			if (path) await writeTextFile(path, rows);
		} catch (e) { errorMsg = String(e); }
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') { e.preventDefault(); onClose(); }
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="modal-overlay" onclick={onClose} role="presentation">
	<!-- svelte-ignore a11y_interactive_supports_focus -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="modal modal-large" onclick={(e) => e.stopPropagation()} role="dialog">
		<div class="modal-header">
			<span>{tr('batch.title')}</span>
			<button class="modal-close" onclick={onClose}>&times;</button>
		</div>
		<div class="modal-body">
			<div class="batch-toolbar">
				<button class="btn btn-primary" onclick={pickFiles} disabled={running}>{tr('batch.pickFiles')}</button>
				<button class="btn btn-primary" onclick={pickFolder} disabled={running}>{tr('batch.pickFolder')}</button>
				{#if results.length > 0}
					<label class="only-failures">
						<input type="checkbox" bind:checked={onlyFailures} />
						{tr('batch.onlyFailures')} ({failCount})
					</label>
					<span class="batch-summary" class:all-ok={failCount === 0}>
						{tr('batch.summary', { total: results.length, failed: failCount })}
					</span>
					<button class="btn" onclick={exportCsv}>{tr('batch.exportCsv')}</button>
				{/if}
			</div>

			{#if errorMsg}
				<div class="batch-error">{errorMsg}</div>
			{/if}
			{#if skipped > 0}
				<div class="batch-warn">{tr('batch.skipped', { count: skipped })}</div>
			{/if}

			<div class="batch-table-wrap">
				{#if running}
					<div class="batch-empty">{tr('batch.running')}</div>
				{:else if results.length === 0}
					<div class="batch-empty">{ranOnce ? tr('batch.noFiles') : tr('batch.intro')}</div>
				{:else}
					<table class="batch-table">
						<thead>
							<tr>
								<th></th>
								<th>{tr('batch.colFile')}</th>
								<th>{tr('batch.colType')}</th>
								<th>{tr('xsd.version')}</th>
								<th>{tr('batch.colSegments')}</th>
								<th>✖</th>
								<th>⚠</th>
								<th>{tr('batch.colDetail')}</th>
							</tr>
						</thead>
						<tbody>
							{#each visible as r (r.path)}
								<tr
									class:row-fail={isFailure(r)}
									onclick={() => { if (!r.parse_error) onOpenFile(r.path); }}
									title={r.parse_error ? r.path : tr('batch.openHint')}
								>
									<td class="st">{isFailure(r) ? '✖' : '✓'}</td>
									<td class="fn" title={r.path}>{r.file_name}</td>
									<td>{r.message_type || '—'}</td>
									<td>{r.version || '—'}</td>
									<td class="num">{r.segment_count}</td>
									<td class="num err">{r.error_count}</td>
									<td class="num warn">{r.warning_count}</td>
									<td class="detail">{r.parse_error ?? ''}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				{/if}
			</div>
		</div>
		<div class="modal-footer">
			<button class="btn" onclick={onClose}>{tr('modal.close')}</button>
		</div>
	</div>
</div>

<style>
	.modal-overlay { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.5); display: flex; align-items: center; justify-content: center; z-index: 1000; }
	.modal.modal-large { background: var(--color-bg, #1e1e2e); color: var(--color-text, #cdd6f4); border: 1px solid var(--color-border, #313244); border-radius: 6px; width: min(1100px, 94vw); height: min(720px, 88vh); display: flex; flex-direction: column; box-shadow: 0 10px 40px rgba(0, 0, 0, 0.4); }
	.modal-header { display: flex; justify-content: space-between; align-items: center; padding: 0.75rem 1rem; border-bottom: 1px solid var(--color-border, #313244); font-weight: 600; }
	.modal-close { background: transparent; border: none; color: var(--color-text, #cdd6f4); font-size: 1.25rem; cursor: pointer; padding: 0 0.25rem; }
	.modal-body { padding: 1rem; display: flex; flex-direction: column; gap: 0.6rem; overflow: hidden; flex: 1; }
	.modal-footer { display: flex; justify-content: flex-end; gap: 0.5rem; padding: 0.75rem 1rem; border-top: 1px solid var(--color-border, #313244); }
	.btn { background: var(--color-input-bg, #313244); color: var(--color-text, #cdd6f4); border: 1px solid var(--color-border, #45475a); border-radius: 4px; padding: 0.4rem 0.9rem; font-size: 0.85rem; cursor: pointer; font-family: inherit; }
	.btn:hover:not(:disabled) { filter: brightness(1.15); }
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.btn-primary { background: var(--color-accent, #89b4fa); color: var(--color-bg, #1e1e2e); border-color: var(--color-accent, #89b4fa); }

	.batch-toolbar { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
	.only-failures { display: flex; align-items: center; gap: 5px; font-size: 12px; cursor: pointer; }
	.batch-summary { font-size: 12px; font-weight: 700; color: var(--color-error, #f38ba8); }
	.batch-summary.all-ok { color: var(--color-success, #a6e3a1); }
	.batch-error { padding: 6px 10px; border: 1px solid var(--color-error, #f38ba8); border-radius: 4px; color: var(--color-error, #f38ba8); font-size: 12px; }
	.batch-warn { padding: 4px 10px; border-left: 3px solid var(--color-warning, #f9e2af); font-size: 12px; color: var(--color-text-secondary, #a6adc8); }
	.batch-empty { padding: 40px; text-align: center; font-style: italic; color: var(--color-text-secondary, #a6adc8); }

	.batch-table-wrap { flex: 1; min-height: 0; overflow: auto; border: 1px solid var(--color-border, #313244); border-radius: 4px; }
	.batch-table { width: 100%; border-collapse: collapse; font-size: 12px; }
	.batch-table th { position: sticky; top: 0; background: var(--color-bg-tertiary, #313244); text-align: left; padding: 6px 8px; font-size: 11px; color: var(--color-text-secondary, #a6adc8); }
	.batch-table td { padding: 4px 8px; border-bottom: 1px solid var(--color-border, #313244); }
	.batch-table tbody tr { cursor: pointer; }
	.batch-table tbody tr:hover { background: var(--color-bg-tertiary, #313244); }
	.row-fail .st { color: var(--color-error, #f38ba8); font-weight: 700; }
	.st { color: var(--color-success, #a6e3a1); text-align: center; width: 24px; }
	.fn { font-family: 'JetBrains Mono', monospace; }
	.num { text-align: right; font-family: 'JetBrains Mono', monospace; }
	.num.err { color: var(--color-error, #f38ba8); }
	.num.warn { color: var(--color-warning, #f9e2af); }
	.detail { color: var(--color-text-secondary, #a6adc8); max-width: 280px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
