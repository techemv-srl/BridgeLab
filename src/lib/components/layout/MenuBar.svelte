<script lang="ts">
	import { t } from '$lib/i18n';
	import type { RecentFile } from '$lib/ipc/database';

	interface Props {
		recentFiles: RecentFile[];
		theme: string;
		onOpenFile: () => void;
		onSave: () => void;
		onSaveAs: () => void;
		onCloseTab: () => void;
		onCloseAllTabs: () => void;
		onClearRecent: () => void;
		onOpenRecentFile: (path: string) => void;
		onParse: () => void;
		onValidate: () => void;
		onToggleTree: () => void;
		onToggleValidation: () => void;
		onToggleCommunication: () => void;
		onAnonymize: () => void;
		onCopyFull: () => void;
		onCopyTruncated: () => void;
		onExportJson: () => void;
		onExportCsv: () => void;
		onSetTheme: (theme: string) => void;
		onSetLanguage: (lang: string) => void;
		onShowAbout: () => void;
	}

	let {
		recentFiles,
		theme,
		onOpenFile,
		onSave,
		onSaveAs,
		onCloseTab,
		onCloseAllTabs,
		onClearRecent,
		onOpenRecentFile,
		onParse,
		onValidate,
		onToggleTree,
		onToggleValidation,
		onToggleCommunication,
		onAnonymize,
		onCopyFull,
		onCopyTruncated,
		onExportJson,
		onExportCsv,
		onSetTheme,
		onSetLanguage,
		onShowAbout,
	}: Props = $props();

	let openMenu = $state<string | null>(null);

	function toggleMenu(name: string) {
		openMenu = openMenu === name ? null : name;
	}

	function closeMenu() {
		openMenu = null;
	}

	function menuAction(fn: () => void) {
		fn();
		closeMenu();
	}
</script>

<svelte:window onclick={closeMenu} />

<div class="menubar" role="menubar">
	<!-- File Menu -->
	<div class="menu-wrapper">
		<button
			class="menu-trigger"
			class:active={openMenu === 'file'}
			onclick={(e) => { e.stopPropagation(); toggleMenu('file'); }}
		>
			{t('menu.file')}
		</button>
		{#if openMenu === 'file'}
			<div class="menu-dropdown" onclick={(e) => e.stopPropagation()}>
				<button class="menu-item" onclick={() => menuAction(onOpenFile)}>
					<span>{t('menu.file.open')}</span>
					<span class="shortcut">Ctrl+O</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onSave)}>
					<span>{t('menu.file.save')}</span>
					<span class="shortcut">Ctrl+S</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onSaveAs)}>
					<span>{t('menu.file.saveAs')}</span>
					<span class="shortcut">Ctrl+Shift+S</span>
				</button>
				<div class="menu-separator"></div>
				<button class="menu-item" onclick={() => menuAction(onCloseTab)}>
					<span>{t('menu.file.close')}</span>
					<span class="shortcut">Ctrl+W</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onCloseAllTabs)}>
					<span>{t('menu.file.closeAll')}</span>
				</button>
				<div class="menu-separator"></div>
				{#if recentFiles.length > 0}
					<div class="menu-label">{t('menu.file.recent')}</div>
					{#each recentFiles.slice(0, 8) as file}
						<button
							class="menu-item recent-item"
							onclick={() => menuAction(() => onOpenRecentFile(file.path))}
							title={file.path}
						>
							<span>{file.filename}</span>
						</button>
					{/each}
					<div class="menu-separator"></div>
					<button class="menu-item" onclick={() => menuAction(onClearRecent)}>
						<span>{t('menu.file.clearRecent')}</span>
					</button>
				{:else}
					<div class="menu-label menu-empty">{t('menu.file.recent')}: —</div>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Edit Menu -->
	<div class="menu-wrapper">
		<button
			class="menu-trigger"
			class:active={openMenu === 'edit'}
			onclick={(e) => { e.stopPropagation(); toggleMenu('edit'); }}
		>
			{t('menu.edit')}
		</button>
		{#if openMenu === 'edit'}
			<div class="menu-dropdown" onclick={(e) => e.stopPropagation()}>
				<button class="menu-item" onclick={() => { document.execCommand('undo'); closeMenu(); }}>
					<span>{t('menu.edit.undo')}</span>
					<span class="shortcut">Ctrl+Z</span>
				</button>
				<button class="menu-item" onclick={() => { document.execCommand('redo'); closeMenu(); }}>
					<span>{t('menu.edit.redo')}</span>
					<span class="shortcut">Ctrl+Y</span>
				</button>
				<div class="menu-separator"></div>
				<button class="menu-item" onclick={() => { document.execCommand('cut'); closeMenu(); }}>
					<span>{t('menu.edit.cut')}</span>
					<span class="shortcut">Ctrl+X</span>
				</button>
				<button class="menu-item" onclick={() => { document.execCommand('copy'); closeMenu(); }}>
					<span>{t('menu.edit.copy')}</span>
					<span class="shortcut">Ctrl+C</span>
				</button>
				<button class="menu-item" onclick={() => { document.execCommand('paste'); closeMenu(); }}>
					<span>{t('menu.edit.paste')}</span>
					<span class="shortcut">Ctrl+V</span>
				</button>
			</div>
		{/if}
	</div>

	<!-- View Menu -->
	<div class="menu-wrapper">
		<button
			class="menu-trigger"
			class:active={openMenu === 'view'}
			onclick={(e) => { e.stopPropagation(); toggleMenu('view'); }}
		>
			{t('menu.view')}
		</button>
		{#if openMenu === 'view'}
			<div class="menu-dropdown" onclick={(e) => e.stopPropagation()}>
				<button class="menu-item" onclick={() => menuAction(onToggleTree)}>
					<span>{t('menu.view.tree')}</span>
					<span class="shortcut">Ctrl+B</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onToggleValidation)}>
					<span>{t('menu.tools.validate')}</span>
					<span class="shortcut">Ctrl+J</span>
				</button>
				<div class="menu-separator"></div>
				<div class="menu-label">{t('menu.view.theme')}</div>
				<button class="menu-item" class:checked={theme === 'dark'} onclick={() => menuAction(() => onSetTheme('dark'))}>
					<span>{t('menu.view.theme.dark')}</span>
				</button>
				<button class="menu-item" class:checked={theme === 'light'} onclick={() => menuAction(() => onSetTheme('light'))}>
					<span>{t('menu.view.theme.light')}</span>
				</button>
				<div class="menu-separator"></div>
				<div class="menu-label">{t('menu.view.language')}</div>
				<button class="menu-item" onclick={() => menuAction(() => onSetLanguage('en'))}>
					<span>English</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(() => onSetLanguage('it'))}>
					<span>Italiano</span>
				</button>
			</div>
		{/if}
	</div>

	<!-- Tools Menu -->
	<div class="menu-wrapper">
		<button
			class="menu-trigger"
			class:active={openMenu === 'tools'}
			onclick={(e) => { e.stopPropagation(); toggleMenu('tools'); }}
		>
			{t('menu.tools')}
		</button>
		{#if openMenu === 'tools'}
			<div class="menu-dropdown" onclick={(e) => e.stopPropagation()}>
				<button class="menu-item" onclick={() => menuAction(onParse)}>
					<span>{t('menu.tools.parse')}</span>
					<span class="shortcut">F5</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onValidate)}>
					<span>{t('menu.tools.validate')}</span>
					<span class="shortcut">F6</span>
				</button>
				<div class="menu-separator"></div>
				<button class="menu-item" onclick={() => menuAction(onToggleCommunication)}>
					<span>Communication Panel</span>
					<span class="shortcut">Ctrl+K</span>
				</button>
				<div class="menu-separator"></div>
				<button class="menu-item" onclick={() => menuAction(onAnonymize)}>
					<span>{t('menu.tools.anonymize')}</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onCopyFull)}>
					<span>{t('menu.tools.copyFull')}</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onCopyTruncated)}>
					<span>{t('menu.tools.copyTruncated')}</span>
				</button>
				<div class="menu-separator"></div>
				<button class="menu-item" onclick={() => menuAction(onExportJson)}>
					<span>Export JSON</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onExportCsv)}>
					<span>Export CSV</span>
				</button>
			</div>
		{/if}
	</div>

	<!-- Help Menu -->
	<div class="menu-wrapper">
		<button
			class="menu-trigger"
			class:active={openMenu === 'help'}
			onclick={(e) => { e.stopPropagation(); toggleMenu('help'); }}
		>
			{t('menu.help')}
		</button>
		{#if openMenu === 'help'}
			<div class="menu-dropdown" onclick={(e) => e.stopPropagation()}>
				<button class="menu-item" onclick={() => menuAction(onShowAbout)}>
					<span>{t('menu.help.about')}</span>
				</button>
			</div>
		{/if}
	</div>
</div>

<style>
	.menubar {
		display: flex;
		align-items: stretch;
		height: 28px;
		background-color: var(--color-bg-secondary);
		border-bottom: 1px solid var(--color-border);
		flex-shrink: 0;
		user-select: none;
	}

	.menu-wrapper {
		position: relative;
	}

	.menu-trigger {
		height: 100%;
		padding: 0 10px;
		background: none;
		border: none;
		color: var(--color-text-primary);
		font-size: 12px;
		font-family: inherit;
		cursor: pointer;
	}

	.menu-trigger:hover,
	.menu-trigger.active {
		background-color: var(--color-bg-tertiary);
	}

	.menu-dropdown {
		position: absolute;
		top: 100%;
		left: 0;
		min-width: 220px;
		background-color: var(--color-bg-secondary);
		border: 1px solid var(--color-border);
		border-radius: 4px;
		padding: 4px 0;
		z-index: 100;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
	}

	.menu-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		width: 100%;
		padding: 5px 16px;
		background: none;
		border: none;
		color: var(--color-text-primary);
		font-size: 12px;
		font-family: inherit;
		text-align: left;
		cursor: pointer;
		gap: 16px;
	}

	.menu-item:hover {
		background-color: var(--color-accent);
		color: var(--color-bg-primary);
	}

	.menu-item.checked::before {
		content: '\2713';
		margin-right: 4px;
	}

	.menu-item.recent-item span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.shortcut {
		color: var(--color-text-secondary);
		font-size: 11px;
		flex-shrink: 0;
	}

	.menu-item:hover .shortcut {
		color: inherit;
	}

	.menu-separator {
		height: 1px;
		background-color: var(--color-border);
		margin: 4px 0;
	}

	.menu-label {
		padding: 4px 16px 2px;
		font-size: 10px;
		color: var(--color-text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.menu-empty {
		font-style: italic;
	}
</style>
