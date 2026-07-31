<script lang="ts">
	import { generateTestMessages, type GeneratedMessage } from '$lib/ipc/batch';
	import { t, subscribeLocale } from '$lib/i18n';
	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		onClose: () => void;
		/** Open a generated message in a new editor tab. */
		onOpenMessage: (content: string, label: string) => void;
	}

	let { onClose, onOpenMessage }: Props = $props();

	const KINDS = ['ADT^A01', 'ADT^A08', 'ORU^R01', 'ORM^O01', 'mixed'];
	/** Opening hundreds of tabs would grind the UI — above this, save to folder. */
	const MAX_OPEN_TABS = 20;

	let kind = $state('ADT^A01');
	let count = $state(10);
	let seedText = $state('');
	let messages = $state<GeneratedMessage[]>([]);
	let generating = $state(false);
	let saving = $state(false);
	let errorMsg = $state('');
	let savedTo = $state('');

	async function handleGenerate() {
		generating = true;
		errorMsg = '';
		savedTo = '';
		try {
			const seed = seedText.trim() === '' ? undefined : Number(seedText);
			if (seed !== undefined && !Number.isFinite(seed)) {
				errorMsg = tr('gen.badSeed');
				return;
			}
			const n = Number.isFinite(count) ? Math.max(1, Math.min(500, Math.trunc(count))) : 10;
			messages = await generateTestMessages(kind, n, seed);
		} catch (e) {
			errorMsg = String(e);
		} finally {
			generating = false;
		}
	}

	function openInTabs() {
		for (const m of messages.slice(0, MAX_OPEN_TABS)) {
			onOpenMessage(m.content, m.label);
		}
		onClose();
	}

	async function saveToFolder() {
		if (messages.length === 0) return;
		saving = true;
		errorMsg = '';
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const dir = await open({ directory: true });
			if (!dir || Array.isArray(dir)) { saving = false; return; }
			const { writeTextFile } = await import('@tauri-apps/plugin-fs');
			const prefix = kind === 'mixed' ? 'msg' : kind.replace('^', '_').toLowerCase();
			for (let i = 0; i < messages.length; i++) {
				const name = `${prefix}_${String(i + 1).padStart(3, '0')}.hl7`;
				await writeTextFile(`${dir}/${name}`, messages[i].content);
			}
			savedTo = String(dir);
		} catch (e) {
			errorMsg = String(e);
		} finally {
			saving = false;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') { e.preventDefault(); onClose(); }
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="modal-overlay" onclick={onClose} role="presentation">
	<!-- svelte-ignore a11y_interactive_supports_focus -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="modal" onclick={(e) => e.stopPropagation()} role="dialog">
		<div class="modal-header">
			<span>{tr('gen.title')}</span>
			<button class="modal-close" onclick={onClose}>&times;</button>
		</div>
		<div class="modal-body">
			<p class="gen-intro">{tr('gen.intro')}</p>
			<div class="gen-form">
				<label class="gen-field">
					<span>{tr('gen.kind')}</span>
					<select bind:value={kind}>
						{#each KINDS as k (k)}
							<option value={k}>{k === 'mixed' ? tr('gen.mixed') : k}</option>
						{/each}
					</select>
				</label>
				<label class="gen-field">
					<span>{tr('gen.count')}</span>
					<input type="number" min={1} max={500} bind:value={count} />
				</label>
				<label class="gen-field">
					<span>{tr('gen.seed')}</span>
					<input type="text" bind:value={seedText} placeholder={tr('gen.seedHint')} />
				</label>
				<button class="btn btn-primary" onclick={handleGenerate} disabled={generating}>
					{generating ? tr('gen.generating') : tr('gen.generate')}
				</button>
			</div>

			{#if errorMsg}
				<div class="gen-error">{errorMsg}</div>
			{/if}
			{#if savedTo}
				<div class="gen-ok">{tr('gen.savedTo', { dir: savedTo })}</div>
			{/if}

			{#if messages.length > 0}
				<div class="gen-preview-label">{tr('gen.preview', { count: messages.length })}</div>
				<pre class="gen-preview">{messages[0].content.split('\r').join('\n')}</pre>
				<div class="gen-actions">
					<button class="btn btn-primary" onclick={openInTabs}>
						{messages.length > MAX_OPEN_TABS
							? tr('gen.openFirst', { count: MAX_OPEN_TABS })
							: tr('gen.openTabs', { count: messages.length })}
					</button>
					<button class="btn" onclick={saveToFolder} disabled={saving}>
						{saving ? tr('gen.saving') : tr('gen.saveFolder')}
					</button>
				</div>
			{/if}
		</div>
		<div class="modal-footer">
			<button class="btn" onclick={onClose}>{tr('modal.close')}</button>
		</div>
	</div>
</div>

<style>
	.modal-overlay { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.5); display: flex; align-items: center; justify-content: center; z-index: 1000; }
	.modal { background: var(--color-bg, #1e1e2e); color: var(--color-text, #cdd6f4); border: 1px solid var(--color-border, #313244); border-radius: 6px; width: min(680px, 92vw); max-height: 86vh; display: flex; flex-direction: column; box-shadow: 0 10px 40px rgba(0, 0, 0, 0.4); }
	.modal-header { display: flex; justify-content: space-between; align-items: center; padding: 0.75rem 1rem; border-bottom: 1px solid var(--color-border, #313244); font-weight: 600; }
	.modal-close { background: transparent; border: none; color: var(--color-text, #cdd6f4); font-size: 1.25rem; cursor: pointer; padding: 0 0.25rem; }
	.modal-body { padding: 1rem; display: flex; flex-direction: column; gap: 0.7rem; overflow: auto; flex: 1; }
	.modal-footer { display: flex; justify-content: flex-end; gap: 0.5rem; padding: 0.75rem 1rem; border-top: 1px solid var(--color-border, #313244); }
	.btn { background: var(--color-input-bg, #313244); color: var(--color-text, #cdd6f4); border: 1px solid var(--color-border, #45475a); border-radius: 4px; padding: 0.4rem 0.9rem; font-size: 0.85rem; cursor: pointer; font-family: inherit; }
	.btn:hover:not(:disabled) { filter: brightness(1.15); }
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.btn-primary { background: var(--color-accent, #89b4fa); color: var(--color-bg, #1e1e2e); border-color: var(--color-accent, #89b4fa); }

	.gen-intro { margin: 0; font-size: 0.85rem; color: var(--color-text-secondary, #a6adc8); }
	.gen-form { display: flex; align-items: flex-end; gap: 10px; flex-wrap: wrap; }
	.gen-field { display: flex; flex-direction: column; gap: 3px; font-size: 0.75rem; color: var(--color-text-secondary, #a6adc8); }
	.gen-field select, .gen-field input { padding: 0.35rem 0.5rem; border: 1px solid var(--color-border, #45475a); border-radius: 4px; background: var(--color-input-bg, #313244); color: var(--color-text, #cdd6f4); font-family: inherit; font-size: 0.85rem; }
	.gen-field input[type="number"] { width: 80px; }
	.gen-field input[type="text"] { width: 130px; }

	.gen-error { padding: 6px 10px; border: 1px solid var(--color-error, #f38ba8); border-radius: 4px; color: var(--color-error, #f38ba8); font-size: 12px; }
	.gen-ok { padding: 6px 10px; border: 1px solid var(--color-success, #a6e3a1); border-radius: 4px; color: var(--color-success, #a6e3a1); font-size: 12px; word-break: break-all; }

	.gen-preview-label { font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; color: var(--color-text-secondary, #a6adc8); }
	.gen-preview { margin: 0; padding: 8px; background: var(--color-bg-secondary, #181825); border: 1px solid var(--color-border, #313244); border-radius: 4px; font-family: 'JetBrains Mono', monospace; font-size: 11px; white-space: pre-wrap; word-break: break-all; max-height: 200px; overflow-y: auto; }
	.gen-actions { display: flex; gap: 8px; }
</style>
