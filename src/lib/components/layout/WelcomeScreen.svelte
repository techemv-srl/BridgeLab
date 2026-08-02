<script lang="ts">
	import { t, subscribeLocale } from '$lib/i18n';
	import { shortcutStore } from '$lib/stores/shortcuts.svelte';
	import { fileOpsStore } from '$lib/stores/file-ops.svelte';

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		onOpenFile: () => void;
		onNewFromTemplate: () => void;
		onShowTestCases: () => void;
		onShowHelp: () => void;
		onNewTab: () => void;
		onOpenRecentFile: (path: string) => void;
	}

	let { onOpenFile, onNewFromTemplate, onShowTestCases, onShowHelp, onNewTab, onOpenRecentFile }: Props = $props();
</script>

<div class="welcome">
	<div class="welcome-card">
		<div class="welcome-title">{tr('welcome.title')}</div>
		<div class="welcome-subtitle">{tr('welcome.subtitle')}</div>
		<div class="welcome-actions">
			<button class="welcome-action" onclick={onOpenFile}>
				<span class="wa-label">{tr('welcome.open')}</span>
				<kbd>{shortcutStore.get('file.open') || 'Ctrl+O'}</kbd>
			</button>
			<button class="welcome-action" onclick={onNewFromTemplate}>
				<span class="wa-label">{tr('welcome.template')}</span>
				<kbd>{shortcutStore.get('file.newFromTemplate') || 'Ctrl+N'}</kbd>
			</button>
			<button class="welcome-action" onclick={onShowTestCases}>
				<span class="wa-label">{tr('welcome.testCases')}</span>
				<kbd>{shortcutStore.get('file.testCases') || 'Ctrl+L'}</kbd>
			</button>
			<button class="welcome-action" onclick={onShowHelp}>
				<span class="wa-label">{tr('welcome.manual')}</span>
				<kbd>F1</kbd>
			</button>
			<button class="welcome-action" onclick={onNewTab}>
				<span class="wa-label">{tr('welcome.blank')}</span>
			</button>
		</div>
		<div class="welcome-paste-hint">{tr('welcome.pasteHint')}</div>
		{#if fileOpsStore.recentFiles.length > 0}
			<div class="welcome-recent">
				<div class="welcome-recent-title">{tr('menu.file.recent')}</div>
				{#each fileOpsStore.recentFiles.slice(0, 6) as rf (rf.path)}
					<button class="welcome-recent-item" onclick={() => onOpenRecentFile(rf.path)} title={rf.path}>
						{rf.path.split(/[\\/]/).pop()}
						<span class="wr-path">{rf.path}</span>
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>

<style>
	.welcome {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		overflow-y: auto;
		background: var(--color-bg-primary);
	}

	.welcome-card {
		max-width: 460px;
		width: 100%;
		padding: 32px;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.welcome-title { font-size: 22px; font-weight: 700; color: var(--color-text-primary); }
	.welcome-subtitle { font-size: 13px; color: var(--color-text-secondary); margin-bottom: 6px; }

	.welcome-actions { display: flex; flex-direction: column; gap: 6px; }
	.welcome-action {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 9px 14px;
		border: 1px solid var(--color-border);
		border-radius: 6px;
		background: var(--color-bg-secondary);
		color: var(--color-text-primary);
		font-size: 13px;
		font-family: inherit;
		cursor: pointer;
		text-align: left;
	}
	.welcome-action:hover { background: var(--color-bg-tertiary); border-color: var(--color-accent); }
	.welcome-action kbd {
		padding: 1px 8px;
		border: 1px solid var(--color-border);
		border-bottom-width: 2px;
		border-radius: 4px;
		background: var(--color-bg-tertiary);
		font-family: 'JetBrains Mono', monospace;
		font-size: 10px;
		color: var(--color-text-secondary);
	}

	.welcome-paste-hint { font-size: 11px; color: var(--color-text-secondary); font-style: italic; text-align: center; }

	.welcome-recent { display: flex; flex-direction: column; gap: 2px; margin-top: 4px; }
	.welcome-recent-title { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; color: var(--color-text-secondary); margin-bottom: 4px; }
	.welcome-recent-item {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 1px;
		padding: 5px 10px;
		border: none;
		border-radius: 4px;
		background: none;
		color: var(--color-accent);
		font-size: 12px;
		font-family: inherit;
		cursor: pointer;
		text-align: left;
		overflow: hidden;
	}
	.welcome-recent-item:hover { background: var(--color-bg-tertiary); }
	.wr-path { font-size: 10px; color: var(--color-text-secondary); max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
