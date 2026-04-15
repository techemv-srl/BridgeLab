<script lang="ts">
	import type { TreeNode } from '$lib/types/hl7';
	import { getTreeChildren, getFieldContent } from '$lib/ipc/parser';
	import { getSegmentInfo } from '$lib/ipc/tables';
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
		/** HL7 version used to look up schema field definitions */
		version?: string;
		/** When true, inject placeholder rows for schema-defined fields that are absent from the message */
		showSchemaFields?: boolean;
	}

	let {
		messageId,
		roots,
		onNodeSelect,
		onFieldExpand,
		navigateTo = null,
		onNavigateToEditor,
		version = '',
		showSchemaFields = false,
	}: Props = $props();

	type VNode = TreeNode & { _children?: TreeNode[]; _expanded?: boolean; _isPlaceholder?: boolean };

	// Flat list of visible nodes for virtual scrolling
	let visibleNodes = $state<VNode[]>([]);
	let selectedNodeId = $state<string | null>(null);

	// Initialize with root nodes (also re-init when showSchemaFields toggles so
	// previously-expanded segments pick up / drop placeholder rows).
	$effect(() => {
		void showSchemaFields;
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

	/** Extract segment-type code (e.g. "PID") from a segment node's label. */
	function segmentTypeFromNode(node: TreeNode): string | null {
		const m = node.label.match(/^([A-Z][A-Z0-9]{2})/);
		return m ? m[1] : null;
	}

	/**
	 * If showSchemaFields is on and the node being expanded is a segment, merge
	 * the real children with placeholder nodes for schema-defined fields that
	 * are absent from the actual message.
	 */
	async function mergeSchemaPlaceholders(segNode: VNode, realChildren: TreeNode[]): Promise<VNode[]> {
		if (!showSchemaFields || !version || segNode.node_type !== 'segment') {
			return realChildren.map((c) => ({ ...c, _expanded: false }));
		}
		const segType = segmentTypeFromNode(segNode);
		if (!segType) return realChildren.map((c) => ({ ...c, _expanded: false }));

		try {
			const info = await getSegmentInfo(segType, version);
			if (!info) return realChildren.map((c) => ({ ...c, _expanded: false }));

			const segId = segNode.id; // "seg{N}"
			const existing = new Set<number>();
			for (const c of realChildren) {
				const m = c.id.match(/\.f(\d+)$/);
				if (m) existing.add(parseInt(m[1]));
			}

			const placeholders: VNode[] = info.fields
				.filter((f) => !existing.has(f.position))
				.map((f) => ({
					id: `${segId}.f${f.position}`,
					label: `${segType}-${f.position} ${f.name}`,
					value_preview: '',
					node_type: 'field' as const,
					depth: segNode.depth + 1,
					has_children: false,
					is_truncated: false,
					child_count: 0,
					_expanded: false,
					_isPlaceholder: true,
				}));

			const merged: VNode[] = [
				...realChildren.map((c) => ({ ...c, _expanded: false })),
				...placeholders,
			];
			// Sort by field position
			merged.sort((a, b) => {
				const am = a.id.match(/\.f(\d+)$/);
				const bm = b.id.match(/\.f(\d+)$/);
				return (am ? parseInt(am[1]) : 0) - (bm ? parseInt(bm[1]) : 0);
			});
			return merged;
		} catch {
			return realChildren.map((c) => ({ ...c, _expanded: false }));
		}
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
			const childNodes = await mergeSchemaPlaceholders(node, node._children!);
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
					isPlaceholder={node._isPlaceholder ?? false}
					onToggle={() => toggleNode(node)}
					onSelect={() => selectNode(node)}
					onExpandTruncated={() => expandTruncated(node)}
					onShowInEditor={onNavigateToEditor && !node._isPlaceholder ? () => showInEditor(node) : undefined}
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
