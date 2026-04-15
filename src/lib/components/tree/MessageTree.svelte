<script lang="ts">
	import type { TreeNode } from '$lib/types/hl7';
	import { getTreeChildren, getFieldContent } from '$lib/ipc/parser';
	import TreeNodeRow from './TreeNodeRow.svelte';

	interface Props {
		messageId: string;
		roots: TreeNode[];
		onNodeSelect?: (node: TreeNode) => void;
		onFieldExpand?: (content: string) => void;
		/** Navigate to a specific segment and optionally a field within it. Stamp forces re-trigger. */
		navigateTo?: { segmentIdx: number; fieldPosition: number | null; stamp: number } | null;
		/** Callback to request the editor to navigate to the selected tree node */
		onNavigateToEditor?: (segmentIdx: number, fieldPosition: number | null, componentIdx: number | null) => void;
	}

	let {
		messageId,
		roots,
		onNodeSelect,
		onFieldExpand,
		navigateTo = null,
		onNavigateToEditor,
	}: Props = $props();

	type VNode = TreeNode & { _children?: TreeNode[]; _expanded?: boolean };

	// Flat list of visible nodes for virtual scrolling
	let visibleNodes = $state<VNode[]>([]);
	let selectedNodeId = $state<string | null>(null);

	// Initialize with root nodes
	$effect(() => {
		visibleNodes = roots.map((r) => ({ ...r, _expanded: false }));
	});

	// Track the last processed stamp to avoid duplicate navigation
	let lastNavStamp = $state(0);

	// Navigate to a specific segment + optional field when requested
	$effect(() => {
		if (!navigateTo || navigateTo.stamp === lastNavStamp) return;
		lastNavStamp = navigateTo.stamp;
		void navigateToTarget(navigateTo.segmentIdx, navigateTo.fieldPosition);
	});

	async function navigateToTarget(segmentIdx: number, fieldPosition: number | null) {
		const segId = `seg${segmentIdx}`;
		let segNode = visibleNodes.find((n) => n.id === segId);
		if (!segNode) return;

		// Expand segment if we need to reach a field
		if (fieldPosition !== null && !segNode._expanded) {
			await toggleNode(segNode);
			segNode = visibleNodes.find((n) => n.id === segId);
		}

		let targetId = segId;
		if (fieldPosition !== null && segNode) {
			const fieldId = `${segId}.f${fieldPosition}`;
			const fieldNode = visibleNodes.find((n) => n.id === fieldId);
			if (fieldNode) targetId = fieldId;
		}

		selectedNodeId = targetId;
		// Scroll the target into view
		requestAnimationFrame(() => {
			const el = document.querySelector(`[data-node-id="${CSS.escape(targetId)}"]`);
			el?.scrollIntoView({ behavior: 'smooth', block: 'center' });
		});
	}

	async function toggleNode(node: VNode) {
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

	/** Parse a node id like "seg3", "seg3.f5", "seg3.f5.c2" into navigation parts. */
	function parseNodeId(id: string): { segmentIdx: number | null; fieldPosition: number | null; componentIdx: number | null } {
		const parts = id.split('.');
		let segmentIdx: number | null = null;
		let fieldPosition: number | null = null;
		let componentIdx: number | null = null;
		for (const p of parts) {
			if (p.startsWith('seg')) segmentIdx = parseInt(p.slice(3));
			else if (p.startsWith('f')) fieldPosition = parseInt(p.slice(1));
			else if (p.startsWith('c')) componentIdx = parseInt(p.slice(1));
		}
		return { segmentIdx, fieldPosition, componentIdx };
	}

	function showInEditor(node: TreeNode) {
		const { segmentIdx, fieldPosition, componentIdx } = parseNodeId(node.id);
		if (segmentIdx === null) return;
		onNavigateToEditor?.(segmentIdx, fieldPosition, componentIdx);
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
					onShowInEditor={onNavigateToEditor ? () => showInEditor(node) : undefined}
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
