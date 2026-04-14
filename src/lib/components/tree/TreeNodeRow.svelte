<script lang="ts">
	import type { TreeNode } from '$lib/types/hl7';

	interface Props {
		node: TreeNode;
		isSelected: boolean;
		isExpanded: boolean;
		onToggle: () => void;
		onSelect: () => void;
		onExpandTruncated: () => void;
	}

	let { node, isSelected, isExpanded, onToggle, onSelect, onExpandTruncated }: Props = $props();

	const indent = $derived((node.depth - 1) * 16);

	function handleClick() {
		onSelect();
		if (node.has_children) {
			onToggle();
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			handleClick();
		}
	}

	function handleTruncatedClick(e: MouseEvent) {
		e.stopPropagation();
		onExpandTruncated();
	}
</script>

<div
	class="tree-node"
	class:selected={isSelected}
	class:segment={node.node_type === 'segment'}
	class:field={node.node_type === 'field'}
	class:component={node.node_type === 'component'}
	data-node-id={node.id}
	role="treeitem"
	tabindex={0}
	aria-expanded={node.has_children ? isExpanded : undefined}
	aria-selected={isSelected}
	onclick={handleClick}
	onkeydown={handleKeydown}
	style="padding-left: {indent + 8}px"
>
	<!-- Expand/Collapse arrow -->
	{#if node.has_children}
		<span class="arrow" class:expanded={isExpanded}>&#9656;</span>
	{:else}
		<span class="arrow-placeholder"></span>
	{/if}

	<!-- Label -->
	<span class="label">{node.label}</span>

	<!-- Value preview -->
	{#if node.value_preview}
		<span class="value">
			{#if node.is_truncated}
				<span class="value-text">{node.value_preview}</span>
				<button class="truncated-btn" onclick={handleTruncatedClick} title="Click to view full content">
					{'{...}'}
				</button>
			{:else}
				<span class="value-text">{node.value_preview}</span>
			{/if}
		</span>
	{/if}

	<!-- Child count badge -->
	{#if node.child_count > 0 && !isExpanded}
		<span class="badge">{node.child_count}</span>
	{/if}
</div>

<style>
	.tree-node {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 2px 8px;
		cursor: pointer;
		white-space: nowrap;
		min-height: 22px;
		border-left: 2px solid transparent;
		transition: background-color 0.1s;
	}

	.tree-node:hover {
		background-color: var(--color-bg-tertiary);
	}

	.tree-node.selected {
		background-color: var(--color-bg-tertiary);
		border-left-color: var(--color-accent);
	}

	.arrow {
		display: inline-block;
		width: 12px;
		flex-shrink: 0;
		color: var(--color-text-secondary);
		transition: transform 0.15s;
		font-size: 10px;
	}

	.arrow.expanded {
		transform: rotate(90deg);
	}

	.arrow-placeholder {
		display: inline-block;
		width: 12px;
		flex-shrink: 0;
	}

	.label {
		flex-shrink: 0;
		font-weight: 500;
	}

	.segment .label {
		color: var(--color-segment);
		font-weight: 700;
	}

	.field .label {
		color: var(--color-field);
	}

	.component .label {
		color: var(--color-component);
	}

	.value {
		color: var(--color-text-secondary);
		overflow: hidden;
		text-overflow: ellipsis;
		margin-left: 8px;
		flex: 1;
		min-width: 0;
	}

	.value-text {
		opacity: 0.8;
	}

	.truncated-btn {
		background: none;
		border: 1px solid var(--color-error);
		color: var(--color-error);
		padding: 0 4px;
		border-radius: 3px;
		cursor: pointer;
		font-family: inherit;
		font-size: 11px;
		margin-left: 4px;
	}

	.truncated-btn:hover {
		background-color: var(--color-error);
		color: var(--color-bg-primary);
	}

	.badge {
		flex-shrink: 0;
		background-color: var(--color-bg-tertiary);
		color: var(--color-text-secondary);
		font-size: 10px;
		padding: 0 5px;
		border-radius: 8px;
		margin-left: 4px;
	}
</style>
