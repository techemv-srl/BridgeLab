<script lang="ts">
	import { shortcutStore, SHORTCUTS, eventToKeys, type ShortcutDef } from '$lib/stores/shortcuts.svelte';
	import { dialogStore } from '$lib/stores/dialog.svelte';
	import { t, subscribeLocale } from '$lib/i18n';

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	let capturingId = $state<string | null>(null);
	let capturedKeys = $state('');
	let conflictWarning = $state('');

	// Load from preferences on first mount
	let loaded = false;
	$effect(() => {
		if (loaded || typeof window === 'undefined') return;
		loaded = true;
		shortcutStore.loadFromPrefs();
	});

	function startCapture(id: string) {
		capturingId = id;
		capturedKeys = shortcutStore.get(id);
		conflictWarning = '';
	}

	function handleCaptureKeydown(e: KeyboardEvent) {
		if (!capturingId) return;
		e.preventDefault();
		e.stopPropagation();

		if (e.key === 'Escape') {
			capturingId = null;
			capturedKeys = '';
			conflictWarning = '';
			return;
		}
		if (e.key === 'Backspace' || e.key === 'Delete') {
			capturedKeys = '';
			return;
		}

		const keys = eventToKeys(e);
		if (!keys) return; // bare modifier
		capturedKeys = keys;

		// Check for conflicts
		const existing = shortcutStore.findByKeys(keys, capturingId);
		const monacoConflict = shortcutStore.findMonacoConflict(keys);
		if (existing) {
			conflictWarning = `Already assigned to "${existing.label}" - will be reassigned.`;
		} else if (monacoConflict) {
			conflictWarning = `Will override editor shortcut "${monacoConflict.label}" inside the editor.`;
		} else {
			conflictWarning = '';
		}
	}

	async function applyCapture() {
		if (!capturingId) return;
		const newKeys = capturedKeys;

		// If conflict with another app shortcut, clear that one first
		const existing = shortcutStore.findByKeys(newKeys, capturingId);
		if (existing) {
			shortcutStore.set(existing.id, '');
		}

		shortcutStore.set(capturingId, newKeys);
		await shortcutStore.save();
		capturingId = null;
		capturedKeys = '';
		conflictWarning = '';
	}

	function cancelCapture() {
		capturingId = null;
		capturedKeys = '';
		conflictWarning = '';
	}

	async function resetToDefault(id: string) {
		const def = SHORTCUTS.find(s => s.id === id);
		if (!def) return;
		shortcutStore.set(id, def.defaultKeys);
		await shortcutStore.save();
	}

	async function resetAll() {
		if (!(await dialogStore.confirm(tr('dialog.resetShortcuts')))) return;
		shortcutStore.resetDefaults();
		await shortcutStore.save();
	}

	// Group by category
	const groupedShortcuts = $derived.by(() => {
		const groups: Record<string, ShortcutDef[]> = {};
		for (const s of SHORTCUTS) {
			if (!groups[s.category]) groups[s.category] = [];
			groups[s.category].push(s);
		}
		return groups;
	});

	const categoryLabels = $derived.by(() => {
		void localeVersion;
		return {
		file: tr('menu.file'),
		edit: tr('menu.edit'),
		view: tr('menu.view'),
		tools: tr('menu.tools'),
		editor: 'Editor (Monaco)',
	};
	});
</script>

<svelte:window onkeydown={handleCaptureKeydown} />

<div class="shortcuts-editor">
	<div class="header-row">
		<p class="intro">
			{tr('shortcuts.intro')}
		</p>
		<button class="btn-sm" onclick={resetAll}>{tr('shortcuts.resetAll')}</button>
	</div>

	{#each Object.entries(groupedShortcuts) as [cat, items]}
		<div class="category">
			<div class="category-title">{(categoryLabels as Record<string, string>)[cat] ?? cat}</div>
			{#each items as s (s.id)}
				<div class="shortcut-row">
					<span class="shortcut-label">
						{s.label}
						{#if s.isMonaco}
							<span class="monaco-tag">Monaco</span>
						{/if}
					</span>
					{#if capturingId === s.id}
						<div class="capture-area">
							<span class="capture-keys">
								{capturedKeys || tr('shortcuts.pressKey')}
							</span>
							{#if conflictWarning}
								<span class="conflict">{conflictWarning}</span>
							{/if}
							<button class="btn-xs btn-primary" onclick={applyCapture} disabled={!capturedKeys}>OK</button>
							<button class="btn-xs" onclick={cancelCapture}>{tr('dialog.cancel')}</button>
						</div>
					{:else}
						<div class="binding-area">
							<button class="binding" onclick={() => startCapture(s.id)} title={tr('shortcuts.clickToRebind')}>
								{shortcutStore.get(s.id) || tr('shortcuts.none')}
							</button>
							{#if shortcutStore.get(s.id) !== s.defaultKeys}
								<button class="btn-xs reset-btn" onclick={() => resetToDefault(s.id)} title="Reset to default ({s.defaultKeys})">
									&#8634;
								</button>
							{/if}
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/each}
</div>

<style>
	.shortcuts-editor { font-size: 12px; }

	.header-row { display: flex; justify-content: space-between; align-items: flex-start; gap: 10px; margin-bottom: 10px; padding-bottom: 8px; border-bottom: 1px solid var(--color-border); }
	.intro { font-size: 11px; color: var(--color-text-secondary); margin: 0; flex: 1; }
	.intro kbd { background: var(--color-bg-tertiary); padding: 1px 5px; border-radius: 3px; border: 1px solid var(--color-border); font-family: 'JetBrains Mono', monospace; font-size: 10px; }

	.category { margin-bottom: 12px; }
	.category-title { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; color: var(--color-text-secondary); padding: 4px 0; margin-bottom: 2px; border-bottom: 1px solid var(--color-border); }

	.shortcut-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 4px 0; }
	.shortcut-label { font-size: 12px; flex: 1; }
	.monaco-tag { padding: 1px 6px; background: var(--color-bg-tertiary); border-radius: 3px; font-size: 9px; color: var(--color-text-secondary); margin-left: 6px; text-transform: uppercase; letter-spacing: 0.5px; }

	.binding-area { display: flex; align-items: center; gap: 4px; }
	.binding { padding: 2px 10px; border: 1px solid var(--color-border); border-radius: 3px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-family: 'JetBrains Mono', monospace; font-size: 11px; cursor: pointer; min-width: 100px; text-align: center; }
	.binding:hover { background: var(--color-border); }
	.reset-btn { padding: 0 6px; font-size: 12px; background: none; border: none; color: var(--color-text-secondary); cursor: pointer; }
	.reset-btn:hover { color: var(--color-accent); }

	.capture-area { display: flex; align-items: center; gap: 4px; flex-wrap: wrap; }
	.capture-keys { padding: 2px 10px; background: var(--color-accent); color: var(--color-bg-primary); border-radius: 3px; font-family: 'JetBrains Mono', monospace; font-size: 11px; min-width: 120px; text-align: center; font-weight: 600; }
	.conflict { font-size: 10px; color: var(--color-warning); }

	.btn-sm { padding: 3px 10px; border: 1px solid var(--color-border); border-radius: 3px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 11px; font-family: inherit; cursor: pointer; }
	.btn-sm:hover { background: var(--color-border); }
	.btn-xs { padding: 2px 8px; border: 1px solid var(--color-border); border-radius: 3px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 10px; font-family: inherit; cursor: pointer; }
	.btn-xs:hover { background: var(--color-border); }
	.btn-xs:disabled { opacity: 0.5; cursor: not-allowed; }
	.btn-xs.btn-primary { background: var(--color-accent); color: var(--color-bg-primary); border-color: var(--color-accent); }
</style>
