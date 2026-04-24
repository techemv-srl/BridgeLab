<script lang="ts">
	import { t, subscribeLocale } from '$lib/i18n';
	import { listVersions, listMessages, exportXsd, type VersionOption, type MessageOption } from '$lib/ipc/schemaExport';

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props { onClose: () => void; }
	let { onClose }: Props = $props();

	let versions = $state<VersionOption[]>([]);
	let messages = $state<MessageOption[]>([]);
	let selectedVersion = $state('');
	let selectedMessage = $state('');
	let preview = $state('');
	let error = $state('');
	let loading = $state(false);

	// On mount, fetch versions, pick the first, load its messages.
	$effect(() => {
		if (typeof window === 'undefined') return;
		(async () => {
			try {
				versions = await listVersions();
				if (versions.length > 0) {
					selectedVersion = versions[0].key;
					await reloadMessages();
				}
			} catch (e) {
				error = `Failed to load versions: ${e}`;
			}
		})();
	});

	async function reloadMessages() {
		try {
			messages = await listMessages(selectedVersion);
			if (messages.length > 0) {
				selectedMessage = messages[0].code;
				await regeneratePreview();
			}
		} catch (e) {
			error = `Failed to load messages: ${e}`;
		}
	}

	async function onVersionChange() {
		preview = '';
		error = '';
		await reloadMessages();
	}

	async function regeneratePreview() {
		if (!selectedVersion || !selectedMessage) return;
		error = '';
		loading = true;
		try {
			preview = await exportXsd(selectedVersion, selectedMessage);
		} catch (e) {
			error = `${e}`;
			preview = '';
		} finally {
			loading = false;
		}
	}

	async function saveToFile() {
		if (!preview) return;
		try {
			const { save } = await import('@tauri-apps/plugin-dialog');
			const { writeTextFile } = await import('@tauri-apps/plugin-fs');
			const suggested = `${selectedMessage}.xsd`;
			const path = await save({
				title: tr('xsd.saveTitle'),
				defaultPath: suggested,
				filters: [{ name: 'XSD', extensions: ['xsd'] }],
			});
			if (path) {
				await writeTextFile(path, preview);
			}
		} catch (e) {
			error = `Save failed: ${e}`;
		}
	}

	async function copyToClipboard() {
		if (!preview) return;
		try {
			await navigator.clipboard.writeText(preview);
		} catch (e) {
			error = `Copy failed: ${e}`;
		}
	}
</script>

<div class="modal-overlay" onclick={onClose} role="presentation">
	<!-- svelte-ignore a11y_interactive_supports_focus -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="modal modal-large" onclick={(e) => e.stopPropagation()} role="dialog">
		<div class="modal-header">
			<span>{tr('xsd.title')}</span>
			<button class="modal-close" onclick={onClose}>&times;</button>
		</div>
		<div class="modal-body">
			<p class="xsd-intro">{tr('xsd.intro')}</p>

			<div class="xsd-controls">
				<label class="xsd-field">
					<span>{tr('xsd.version')}</span>
					<select bind:value={selectedVersion} onchange={onVersionChange}>
						{#each versions as v (v.key)}
							<option value={v.key}>HL7 v{v.label}</option>
						{/each}
					</select>
				</label>

				<label class="xsd-field">
					<span>{tr('xsd.message')}</span>
					<select bind:value={selectedMessage} onchange={regeneratePreview}>
						{#each messages as m (m.code)}
							<option value={m.code}>{m.event} — {m.description}</option>
						{/each}
					</select>
				</label>
			</div>

			{#if error}
				<div class="xsd-error">{error}</div>
			{/if}

			<div class="xsd-preview-wrap">
				{#if loading}
					<div class="xsd-loading">{tr('xsd.loading')}</div>
				{:else}
					<textarea class="xsd-preview" readonly value={preview} spellcheck="false"></textarea>
				{/if}
			</div>
		</div>
		<div class="modal-footer">
			<button class="btn" onclick={copyToClipboard} disabled={!preview || loading}>
				{tr('xsd.copy')}
			</button>
			<button class="btn btn-primary" onclick={saveToFile} disabled={!preview || loading}>
				{tr('xsd.save')}
			</button>
			<button class="btn" onclick={onClose}>{tr('modal.close')}</button>
		</div>
	</div>
</div>

<style>
	.xsd-intro {
		margin: 0 0 1rem;
		font-size: 0.85rem;
		color: var(--color-text-muted, #888);
	}

	.xsd-controls {
		display: grid;
		grid-template-columns: 1fr 2fr;
		gap: 0.75rem;
		margin-bottom: 0.75rem;
	}

	.xsd-field {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.85rem;
	}

	.xsd-field span {
		color: var(--color-text-muted, #888);
	}

	.xsd-field select {
		padding: 0.35rem 0.5rem;
		background: var(--color-input-bg, #1e1e2e);
		color: var(--color-text, #cdd6f4);
		border: 1px solid var(--color-border, #313244);
		border-radius: 4px;
	}

	.xsd-preview-wrap {
		flex: 1;
		min-height: 24rem;
		display: flex;
	}

	.xsd-preview {
		flex: 1;
		width: 100%;
		font-family: var(--font-mono, 'JetBrains Mono', Consolas, monospace);
		font-size: 0.75rem;
		padding: 0.5rem;
		background: var(--color-bg-alt, #181825);
		color: var(--color-text, #cdd6f4);
		border: 1px solid var(--color-border, #313244);
		border-radius: 4px;
		resize: none;
		white-space: pre;
		overflow: auto;
	}

	.xsd-loading {
		margin: auto;
		color: var(--color-text-muted, #888);
	}

	.xsd-error {
		margin: 0.5rem 0;
		padding: 0.5rem;
		background: var(--color-error-bg, #3a1f1f);
		color: var(--color-error, #f38ba8);
		border: 1px solid var(--color-error, #f38ba8);
		border-radius: 4px;
		font-size: 0.8rem;
	}
</style>
