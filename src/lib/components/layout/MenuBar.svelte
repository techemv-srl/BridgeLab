<script lang="ts">
	import { t, subscribeLocale } from '$lib/i18n';
	import type { RecentFile } from '$lib/ipc/database';
	import { shortcutStore } from '$lib/stores/shortcuts.svelte';

	/** Get current key combination for a shortcut id. */
	function sc(id: string): string {
		return shortcutStore.get(id);
	}

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') {
		subscribeLocale(() => { localeVersion++; });
	}
	function tr(key: string, params?: Record<string, string | number>): string {
		void localeVersion;
		return t(key, params);
	}

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
		onNewFromTemplate: () => void;
		onShowTestCases: () => void;
		onParse: () => void;
		onValidate: () => void;
		onToggleTree: () => void;
		onToggleValidation: () => void;
		onToggleCommunication: () => void;
		onAnonymize: () => void;
		onShowBundleVisualizer: () => void;
		onToggleFhirPath: () => void;
		onCopyFull: () => void;
		onCopyTruncated: () => void;
		onExportJson: () => void;
		onExportCsv: () => void;
		onSetTheme: (theme: string) => void;
		onSetLanguage: (lang: string) => void;
		onShowSettings: () => void;
		onCheckUpdates: () => void;
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
		onNewFromTemplate,
		onShowTestCases,
		onParse,
		onValidate,
		onToggleTree,
		onToggleValidation,
		onToggleCommunication,
		onAnonymize,
		onShowBundleVisualizer,
		onToggleFhirPath,
		onCopyFull,
		onCopyTruncated,
		onExportJson,
		onExportCsv,
		onSetTheme,
		onSetLanguage,
		onShowSettings,
		onCheckUpdates,
		onShowAbout,
	}: Props = $props();

	let openMenu = $state<string | null>(null);

	function toggleMenu(name: string) {
		openMenu = openMenu === name ? null : name;
	}

	function hoverMenu(name: string) {
		// Only switch menu on hover if one is already open (standard menubar UX)
		if (openMenu !== null && openMenu !== name) {
			openMenu = name;
		}
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
			onmouseenter={() => hoverMenu('file')}
		>
			{tr('menu.file')}
		</button>
		{#if openMenu === 'file'}
			<div class="menu-dropdown" onclick={(e) => e.stopPropagation()}>
				<button class="menu-item" onclick={() => menuAction(onNewFromTemplate)}>
					<span>New from Template...</span>
					<span class="shortcut">{sc('file.newFromTemplate')}</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onShowTestCases)}>
					<span>Test Case Library...</span>
					<span class="shortcut">{sc('file.testCases')}</span>
				</button>
				<div class="menu-separator"></div>
				<button class="menu-item" onclick={() => menuAction(onOpenFile)}>
					<span>{tr('menu.file.open')}</span>
					<span class="shortcut">{sc('file.open')}</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onSave)}>
					<span>{tr('menu.file.save')}</span>
					<span class="shortcut">{sc('file.save')}</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onSaveAs)}>
					<span>{tr('menu.file.saveAs')}</span>
					<span class="shortcut">{sc('file.saveAs')}</span>
				</button>
				<div class="menu-separator"></div>
				<button class="menu-item" onclick={() => menuAction(onCloseTab)}>
					<span>{tr('menu.file.close')}</span>
					<span class="shortcut">{sc('file.closeTab')}</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onCloseAllTabs)}>
					<span>{tr('menu.file.closeAll')}</span>
				</button>
				<div class="menu-separator"></div>
				{#if recentFiles.length > 0}
					<div class="menu-label">{tr('menu.file.recent')}</div>
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
						<span>{tr('menu.file.clearRecent')}</span>
					</button>
				{:else}
					<div class="menu-label menu-empty">{tr('menu.file.recent')}: —</div>
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
			onmouseenter={() => hoverMenu('edit')}
		>
			{tr('menu.edit')}
		</button>
		{#if openMenu === 'edit'}
			<div class="menu-dropdown" onclick={(e) => e.stopPropagation()}>
				<button class="menu-item" onclick={() => { document.execCommand('undo'); closeMenu(); }}>
					<span>{tr('menu.edit.undo')}</span>
					<span class="shortcut">Ctrl+Z</span>
				</button>
				<button class="menu-item" onclick={() => { document.execCommand('redo'); closeMenu(); }}>
					<span>{tr('menu.edit.redo')}</span>
					<span class="shortcut">Ctrl+Y</span>
				</button>
				<div class="menu-separator"></div>
				<button class="menu-item" onclick={() => { document.execCommand('cut'); closeMenu(); }}>
					<span>{tr('menu.edit.cut')}</span>
					<span class="shortcut">Ctrl+X</span>
				</button>
				<button class="menu-item" onclick={() => { document.execCommand('copy'); closeMenu(); }}>
					<span>{tr('menu.edit.copy')}</span>
					<span class="shortcut">Ctrl+C</span>
				</button>
				<button class="menu-item" onclick={() => { document.execCommand('paste'); closeMenu(); }}>
					<span>{tr('menu.edit.paste')}</span>
					<span class="shortcut">Ctrl+V</span>
				</button>
				<div class="menu-separator"></div>
				<button class="menu-item" onclick={() => menuAction(onShowSettings)}>
					<span>Settings</span>
					<span class="shortcut">{sc('edit.settings')}</span>
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
			onmouseenter={() => hoverMenu('view')}
		>
			{tr('menu.view')}
		</button>
		{#if openMenu === 'view'}
			<div class="menu-dropdown" onclick={(e) => e.stopPropagation()}>
				<button class="menu-item" onclick={() => menuAction(onToggleTree)}>
					<span>{tr('menu.view.tree')}</span>
					<span class="shortcut">{sc('view.toggleTree')}</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onToggleValidation)}>
					<span>{tr('menu.tools.validate')}</span>
					<span class="shortcut">{sc('view.toggleValidation')}</span>
				</button>
				<div class="menu-separator"></div>
				<div class="menu-label">{tr('menu.view.theme')}</div>
				<button class="menu-item" class:checked={theme === 'dark'} onclick={() => menuAction(() => onSetTheme('dark'))}>
					<span>{tr('menu.view.theme.dark')}</span>
				</button>
				<button class="menu-item" class:checked={theme === 'light'} onclick={() => menuAction(() => onSetTheme('light'))}>
					<span>{tr('menu.view.theme.light')}</span>
				</button>
				<div class="menu-separator"></div>
				<div class="menu-label">{tr('menu.view.language')}</div>
				<button class="menu-item" onclick={() => menuAction(() => onSetLanguage('en'))}><span>English</span></button>
				<button class="menu-item" onclick={() => menuAction(() => onSetLanguage('it'))}><span>Italiano</span></button>
				<button class="menu-item" onclick={() => menuAction(() => onSetLanguage('fr'))}><span>Français</span></button>
				<button class="menu-item" onclick={() => menuAction(() => onSetLanguage('es'))}><span>Español</span></button>
				<button class="menu-item" onclick={() => menuAction(() => onSetLanguage('de'))}><span>Deutsch</span></button>
			</div>
		{/if}
	</div>

	<!-- Tools Menu -->
	<div class="menu-wrapper">
		<button
			class="menu-trigger"
			class:active={openMenu === 'tools'}
			onclick={(e) => { e.stopPropagation(); toggleMenu('tools'); }}
			onmouseenter={() => hoverMenu('tools')}
		>
			{tr('menu.tools')}
		</button>
		{#if openMenu === 'tools'}
			<div class="menu-dropdown" onclick={(e) => e.stopPropagation()}>
				<button class="menu-item" onclick={() => menuAction(onParse)}>
					<span>{tr('menu.tools.parse')}</span>
					<span class="shortcut">{sc('tools.parse')}</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onValidate)}>
					<span>{tr('menu.tools.validate')}</span>
					<span class="shortcut">{sc('tools.validate')}</span>
				</button>
				<div class="menu-separator"></div>
				<button class="menu-item" onclick={() => menuAction(onToggleCommunication)}>
					<span>Communication Panel</span>
					<span class="shortcut">{sc('view.toggleCommunication')}</span>
				</button>
				<div class="menu-separator"></div>
				<button class="menu-item" onclick={() => menuAction(onShowBundleVisualizer)}>
					<span>FHIR Bundle Visualizer</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onToggleFhirPath)}>
					<span>FHIRPath Evaluator</span>
					<span class="shortcut">{sc('view.toggleFhirPath')}</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onAnonymize)}>
					<span>{tr('menu.tools.anonymize')}</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onCopyFull)}>
					<span>{tr('menu.tools.copyFull')}</span>
				</button>
				<button class="menu-item" onclick={() => menuAction(onCopyTruncated)}>
					<span>{tr('menu.tools.copyTruncated')}</span>
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
			onmouseenter={() => hoverMenu('help')}
		>
			{tr('menu.help')}
		</button>
		{#if openMenu === 'help'}
			<div class="menu-dropdown" onclick={(e) => e.stopPropagation()}>
				<button class="menu-item" onclick={() => menuAction(onCheckUpdates)}>
					<span>Check for Updates</span>
				</button>
				<div class="menu-separator"></div>
				<button class="menu-item" onclick={() => menuAction(onShowAbout)}>
					<span>{tr('menu.help.about')}</span>
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
