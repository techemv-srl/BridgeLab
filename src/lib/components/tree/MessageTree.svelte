<script lang="ts">
	import type { TreeNode } from '$lib/types/hl7';
	import { getTreeChildren, getFieldContent } from '$lib/ipc/parser';
	import TreeNodeRow from './TreeNodeRow.svelte';

	interface Props {
		messageId: string;
		roots: TreeNode[];
		onNodeSelect?: (node: TreeNode) => void;
		onFieldExpand?: (content: string) => void;
		/** Navigate to and expand a specific segment by index */
		navigateToSegmentIdx?: number | null;
	}

	let { messageId, roots, onNodeSelect, onFieldExpand, navigateToSegmentIdx = null }: Props = $props();

	// Flat list of visible nodes for virtual scrolling
	let visibleNodes = $state<(TreeNode & { _children?: TreeNode[]; _expanded?: boolean })[]>([]);
	let selectedNodeId = $state<string | null>(null);

	// Initialize with root nodes
	$effect(() => {
		visibleNodes = roots.map((r) => ({ ...r, _expanded: false }));
	});

	// Navigate to a specific segment when requested
	$effect(() => {
		if (navigateToSegmentIdx !== null && navigateToSegmentIdx !== undefined) {
			const segId = `seg${navigateToSegmentIdx}`;
			const node = visibleNodes.find(n => n.id === segId);
			if (node) {
				selectedNodeId = segId;
				if (!node._expanded) {
					toggleNode(node);
				}
				// Scroll to the node
				const el = document.querySelector(`[data-node-id="${segId}"]`);
				el?.scrollIntoView({ behavior: 'smooth', block: 'center' });
			}
		}
	});

	async function toggleNode(node: TreeNode & { _children?: TreeNode[]; _expanded?: boolean }) {
		const idx = visibleNodes.findIndex((n) => n.id === node.id);
		if (idx === -1) return;

		if (node._expanded) {
			// Collapse: remove all children recursively
			const depth = node.depth;
			let removeCount = 0;
			for (let i = idx + 1; i < visibleNodes.length; i++) {
				if (visibleNodes[i].depth > depth) {
					removeCount++;
				} else {
					break;
				}
			}
			visibleNodes = [
				...visibleNodes.slice(0, idx),
				{ ...node, _expanded: false },
				...visibleNodes.slice(idx + 1 + removeCount),
			];
		} else {
			// Expand: fetch children and insert
			if (!node._children) {
				const children = await getTreeChildren(messageId, node.id);
				node._children = children;
			}
			const childNodes = node._children!.map((c) => ({ ...c, _expanded: false }));
			visibleNodes = [
				...visibleNodes.slice(0, idx),
				{ ...node, _expanded: true },
				...childNodes,
				...visibleNodes.slice(idx + 1),
			];
		}
	}

	function selectNode(node: TreeNode) {
		selectedNodeId = node.id;
		onNodeSelect?.(node);
	}

	async function expandTruncated(node: TreeNode) {
		// Parse segment and field indices from node ID: "seg0.f5"
		const parts = node.id.split('.');
		if (parts.length < 2) return;

		const segIdx = parseInt(parts[0].replace('seg', ''));
		const fieldIdx = parseInt(parts[1].replace('f', ''));

		const content = await getFieldContent(messageId, segIdx, fieldIdx);
		onFieldExpand?.(content.full_text);
	}
</script>

<div class="tree-container">
	{#if visibleNodes.length === 0}
		<div class="tree-empty">No message loaded</div>
	{:else}
		<div class="tree-list" role="tree">
			{#each visibleNodes as node (node.id)}
				<TreeNodeRow
					{node}
					isSelected={selectedNodeId === node.id}
					isExpanded={node._expanded ?? false}
					onToggle={() => toggleNode(node)}
					onSelect={() => selectNode(node)}
					onExpandTruncated={() => expandTruncated(node)}
				/>
			{/each}
		</div>
	{/if}
</div>

<style>
	.tree-container {
		height: 100%;
		overflow-y: auto;
		overflow-x: hidden;
		font-family: 'JetBrains Mono', 'Fira Code', monospace;
		font-size: 12px;
		background-color: var(--color-bg-secondary);
	}

	.tree-empty {
		padding: 16px;
		color: var(--color-text-secondary);
		text-align: center;
		font-style: italic;
	}

	.tree-list {
		padding: 4px 0;
	}
</style>
