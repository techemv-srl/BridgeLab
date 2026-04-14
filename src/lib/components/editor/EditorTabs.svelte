<script lang="ts">
	import type { MessageTab } from '$lib/stores/messages.svelte';
	import { t } from '$lib/i18n';

	interface Props {
		tabs: MessageTab[];
		activeTabId: string | null;
		onSelectTab: (tabId: string) => void;
		onCloseTab: (tabId: string) => void;
		onNewTab: () => void;
	}

	let { tabs, activeTabId, onSelectTab, onCloseTab, onNewTab }: Props = $props();

	let contextMenuTabId = $state<string | null>(null);
	let contextMenuPos = $state({ x: 0, y: 0 });

	function handleContextMenu(e: MouseEvent, tabId: string) {
		e.preventDefault();
		contextMenuTabId = tabId;
		contextMenuPos = { x: e.clientX, y: e.clientY };
	}

	function closeContextMenu() {
		contextMenuTabId = null;
	}

	function handleCloseTab(e: MouseEvent, tabId: string) {
		e.stopPropagation();
		onCloseTab(tabId);
	}

	function handleMiddleClick(e: MouseEvent, tabId: string) {
		if (e.button === 1) {
			e.preventDefault();
			onCloseTab(tabId);
		}
	}
</script>

<svelte:window onclick={closeContextMenu} />

<div class="tabs-container">
	<div class="tabs-scroll">
		{#each tabs as tab (tab.id)}
			<div
				class="tab"
				class:active={activeTabId === tab.id}
				class:modified={tab.isModified}
				onclick={() => onSelectTab(tab.id)}
				onauxclick={(e) => handleMiddleClick(e, tab.id)}
				oncontextmenu={(e) => handleContextMenu(e, tab.id)}
				onkeydown={(e) => { if (e.key === 'Enter') onSelectTab(tab.id); }}
				title={tab.filePath ?? tab.label}
				role="tab"
				tabindex={0}
				aria-selected={activeTabId === tab.id}
			>
				<span class="tab-label">
					{tab.label}
				</span>
				{#if tab.isModified}
					<span class="tab-modified-dot" title={t('editor.modified')}></span>
				{/if}
				<button
					class="tab-close"
					onclick={(e) => handleCloseTab(e, tab.id)}
					title={t('tabs.closeTab')}
				>
					&times;
				</button>
			</div>
		{/each}
	</div>
	<button class="tab-new" onclick={onNewTab} title={t('tabs.newTab')}>
		+
	</button>
</div>

{#if contextMenuTabId !== null}
	<div
		class="context-menu"
		style="left: {contextMenuPos.x}px; top: {contextMenuPos.y}px"
	>
		<button onclick={() => { onCloseTab(contextMenuTabId!); closeContextMenu(); }}>
			{t('tabs.closeTab')}
		</button>
		<button onclick={() => { closeContextMenu(); }}>
			{t('tabs.closeOthers')}
		</button>
	</div>
{/if}

<style>
	.tabs-container {
		display: flex;
		align-items: stretch;
		height: 32px;
		background-color: var(--color-bg-tertiary);
		border-bottom: 1px solid var(--color-border);
		flex-shrink: 0;
		overflow: hidden;
	}

	.tabs-scroll {
		display: flex;
		overflow-x: auto;
		flex: 1;
		scrollbar-width: none;
	}

	.tabs-scroll::-webkit-scrollbar {
		display: none;
	}

	.tab {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 0 12px;
		min-width: 100px;
		max-width: 200px;
		height: 100%;
		background: none;
		border: none;
		border-right: 1px solid var(--color-border);
		color: var(--color-text-secondary);
		font-size: 12px;
		font-family: inherit;
		cursor: pointer;
		white-space: nowrap;
		transition: background-color 0.1s;
		outline: none;
	}

	.tab:hover {
		background-color: var(--color-bg-secondary);
	}

	.tab.active {
		background-color: var(--color-bg-primary);
		color: var(--color-text-primary);
		border-bottom: 2px solid var(--color-accent);
	}

	.tab-label {
		overflow: hidden;
		text-overflow: ellipsis;
		flex: 1;
		text-align: left;
	}

	.tab-modified-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background-color: var(--color-accent);
		flex-shrink: 0;
	}

	.tab-close {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 18px;
		height: 18px;
		padding: 0;
		background: none;
		border: none;
		border-radius: 3px;
		color: var(--color-text-secondary);
		font-size: 14px;
		line-height: 1;
		cursor: pointer;
		flex-shrink: 0;
		opacity: 0;
		transition: opacity 0.1s, background-color 0.1s;
	}

	.tab:hover .tab-close,
	.tab.active .tab-close {
		opacity: 1;
	}

	.tab-close:hover {
		background-color: var(--color-error);
		color: var(--color-bg-primary);
	}

	.tab-new {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 100%;
		background: none;
		border: none;
		border-left: 1px solid var(--color-border);
		color: var(--color-text-secondary);
		font-size: 16px;
		cursor: pointer;
		flex-shrink: 0;
	}

	.tab-new:hover {
		background-color: var(--color-bg-secondary);
		color: var(--color-text-primary);
	}

	.context-menu {
		position: fixed;
		background-color: var(--color-bg-secondary);
		border: 1px solid var(--color-border);
		border-radius: 4px;
		padding: 4px 0;
		z-index: 200;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
	}

	.context-menu button {
		display: block;
		width: 100%;
		padding: 6px 16px;
		background: none;
		border: none;
		color: var(--color-text-primary);
		font-size: 12px;
		font-family: inherit;
		text-align: left;
		cursor: pointer;
	}

	.context-menu button:hover {
		background-color: var(--color-accent);
		color: var(--color-bg-primary);
	}
</style>
