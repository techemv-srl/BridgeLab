<script lang="ts">
	import { tick, untrack } from 'svelte';
	import type { TreeNode } from '$lib/types/hl7';
	import { placeAbsentSegments, type FieldTarget } from '$lib/hl7/segment-lines';
	import { getTreeChildren, getFieldContent, searchMessage, type SearchHit } from '$lib/ipc/parser';
	import { getFhirTreeChildren } from '$lib/ipc/validation';
	import {
		getExpectedSegments, getSegmentSchema, getCompositeComponents,
		type ExpectedSegment,
	} from '$lib/ipc/tables';
	import TreeNodeRow from './TreeNodeRow.svelte';
	import { t, subscribeLocale } from '$lib/i18n';
	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		messageId: string;
		roots: TreeNode[];
		/** The selected node; null when the selection went away (a re-parse
		 *  after the selected field was deleted). */
		onNodeSelect?: (node: TreeNode | null) => void;
		onFieldExpand?: (content: string) => void;
		/** Navigate to a segment and optionally a field, repetition and
		 *  component within it. Stamp forces re-trigger. */
		navigateTo?: { segmentIdx: number; target: FieldTarget | null; stamp: number } | null;
		/** Callback to request the editor to navigate to the selected tree node */
		onNavigateToEditor?: (segmentIdx: number, target: FieldTarget) => void;
		/** Insert a ghost segment's skeleton into the editor. afterSegmentIdx is
		 *  the real segment (line) it should follow, null = insert at the top. */
		onInsertSegment?: (code: string, afterSegmentIdx: number | null) => void;
		/** Show every occurrence of a segment type in the segment grid. */
		onShowInGrid?: (segmentType: string) => void;
		/** HL7 version used to look up schema field definitions */
		version?: string;
		/** Message type (e.g. "ORU^R01") used to look up the expected segment structure */
		messageType?: string;
		/** When true, inject placeholder rows for schema-defined fields that are
		 *  absent from the message AND ghost rows for expected-but-absent segments
		 *  (full standard-structure view). */
		showSchemaFields?: boolean;
		/** Parse format ("HL7v2", "FHIR JSON", "FHIR XML"). Search is HL7-only:
		 *  search_message looks up the HL7 store, FHIR resources live in a
		 *  separate store, so the box would always report no matches. */
		format?: string;
	}

	let {
		messageId,
		roots,
		onNodeSelect,
		onFieldExpand,
		navigateTo = null,
		onNavigateToEditor,
		onInsertSegment,
		onShowInGrid,
		version = '',
		messageType = '',
		showSchemaFields = false,
		format = 'HL7v2',
	}: Props = $props();

	const searchEnabled = $derived(format === 'HL7v2');

	type VNode = TreeNode & {
		_children?: TreeNode[];
		_expanded?: boolean;
		_isPlaceholder?: boolean;
		/** Data type of a placeholder field (composite codes expand into components). */
		_dataType?: string;
	};

	// Flat list of the expanded tree, in display order. Only the rows in
	// view are rendered (see the virtual list below): a message or log with
	// tens of thousands of segments used to create a DOM row for each and
	// froze the window. Raw state: the array is always replaced, never
	// mutated in place, and deep proxies of 20k nodes cost time for nothing.
	let visibleNodes = $state.raw<VNode[]>([]);
	let selectedNodeId = $state<string | null>(null);

	// --- Virtual list ---
	/** Every row is this tall (TreeNodeRow fixes its height to it). */
	const ROW = 22;
	/** Rows rendered beyond the viewport on each side. */
	const OVERSCAN = 20;
	let containerEl: HTMLDivElement | undefined = $state();
	let listEl: HTMLDivElement | undefined = $state();
	let scrollTop = $state(0);
	let viewportHeight = $state(600);
	/** Offset of the list inside the scroller (the search bar above it). */
	let listTop = $state(0);

	$effect(() => {
		const el = containerEl;
		if (!el || typeof ResizeObserver === 'undefined') return;
		const ro = new ResizeObserver(() => {
			viewportHeight = el.clientHeight;
			listTop = listEl?.offsetTop ?? 0;
		});
		ro.observe(el);
		return () => ro.disconnect();
	});

	let range = $derived.by(() => {
		const first = Math.floor(Math.max(0, scrollTop - listTop) / ROW);
		const count = Math.ceil(viewportHeight / ROW);
		return {
			start: Math.max(0, first - OVERSCAN),
			end: Math.min(visibleNodes.length, first + count + OVERSCAN),
		};
	});
	let renderedNodes = $derived(visibleNodes.slice(range.start, range.end));

	function handleScroll() {
		if (!containerEl) return;
		scrollTop = containerEl.scrollTop;
		listTop = listEl?.offsetTop ?? listTop;
	}

	/** Scroll row `idx` into view (centred on request, else just enough),
	 *  once the list has its new height (an expansion just grew it). */
	async function scrollToRow(idx: number, center = false) {
		await tick();
		const el = containerEl;
		if (!el || idx < 0) return;
		listTop = listEl?.offsetTop ?? listTop;
		const rowTop = listTop + idx * ROW;
		// The sticky search bar covers the top of the viewport.
		const covered = listTop;
		if (center) {
			el.scrollTop = Math.max(0, rowTop - (el.clientHeight - covered) / 2 - covered);
		} else if (rowTop < el.scrollTop + covered) {
			el.scrollTop = rowTop - covered;
		} else if (rowTop + ROW > el.scrollTop + el.clientHeight) {
			el.scrollTop = rowTop + ROW - el.clientHeight;
		}
		scrollTop = el.scrollTop;
	}

	/** Move the keyboard focus to a row once it is rendered. */
	async function focusRow(id: string) {
		await tick();
		containerEl?.querySelector<HTMLElement>(`[data-node-id="${CSS.escape(id)}"]`)?.focus();
	}

	// --- Search ---
	let searchQuery = $state('');
	let searchHits = $state<SearchHit[]>([]);
	let searchActive = $state(false);   // a completed search is being displayed
	let searchPending = $state(false);
	let activeHitIdx = $state(-1);
	let searchInputEl: HTMLInputElement | undefined = $state();
	let searchDebounce: ReturnType<typeof setTimeout> | null = null;
	// Monotonic token: a response is applied only if no newer search (or a
	// clear, or a message switch) started after it. Prevents a slow response
	// from repopulating results for a query no longer in the input.
	let searchToken = 0;

	// Debounced backend search. Searches the parsed message in the store, so
	// it finds fields even in segments the tree hasn't lazily expanded yet.
	$effect(() => {
		const q = searchQuery.trim();
		const msgId = messageId;
		if (searchDebounce) clearTimeout(searchDebounce);
		const token = ++searchToken;
		if (!q || !searchEnabled) {
			searchHits = [];
			searchActive = false;
			searchPending = false;
			activeHitIdx = -1;
			return;
		}
		searchPending = true;
		searchDebounce = setTimeout(async () => {
			let hits: SearchHit[] = [];
			try {
				hits = await searchMessage(msgId, q);
			} catch {
				hits = [];
			}
			if (token !== searchToken) return; // stale response
			searchHits = hits;
			searchActive = true;
			activeHitIdx = -1;
			searchPending = false;
		}, 250);
	});

	async function gotoHit(idx: number) {
		if (idx < 0 || idx >= searchHits.length) return;
		activeHitIdx = idx;
		const hit = searchHits[idx];
		await navigateToTarget(hit.segment_idx, hit.field_position === null
			? null
			: { field: hit.field_position, repetition: null, component: null });
	}

	function nextHit() { if (searchHits.length) void gotoHit((activeHitIdx + 1) % searchHits.length); }
	function prevHit() { if (searchHits.length) void gotoHit((activeHitIdx - 1 + searchHits.length) % searchHits.length); }

	function clearSearch() {
		searchQuery = '';
		searchHits = [];
		searchActive = false;
		activeHitIdx = -1;
	}

	function handleSearchKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') { clearSearch(); (e.target as HTMLElement).blur(); }
		else if (e.key === 'Enter' && e.shiftKey) { e.preventDefault(); prevHit(); }
		else if (e.key === 'Enter') { e.preventDefault(); nextHit(); }
	}

	// Ctrl+F / Cmd+F while the tree has focus jumps to the search box.
	// Monaco keeps its own find widget for the raw-text panel.
	function handleTreeKeydown(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
			e.preventDefault();
			e.stopPropagation();
			searchInputEl?.focus();
			searchInputEl?.select();
		}
	}

	// Expected segment structure for the current message type (ghost rows of
	// the full standard-structure view). Fetched once per (messageType,
	// version) while the toggle is on; HL7 v2 only.
	let expectedSegs = $state<ExpectedSegment[]>([]);
	let expectedKey = '';
	$effect(() => {
		const key = showSchemaFields && format === 'HL7v2' && messageType && version
			? `${messageType}|${version}`
			: '';
		if (key === expectedKey) return;
		expectedKey = key;
		if (!key) {
			expectedSegs = [];
			return;
		}
		getExpectedSegments(messageType, version)
			.then((s) => {
				// Ignore stale responses: the user may have switched tab or
				// version while this lookup was in flight.
				if (key === expectedKey) expectedSegs = s;
			})
			.catch(() => {
				if (key === expectedKey) expectedSegs = [];
			});
	});

	/** Root list = real segments, interleaved with ghost rows for expected
	 *  segments that are absent from the message, each at its place in the
	 *  standard structure (see placeAbsentSegments). */
	function buildRootNodes(): VNode[] {
		const real: VNode[] = roots.map((r) => ({ ...r, _expanded: false }));
		if (!showSchemaFields || expectedSegs.length === 0) return real;

		const placed = placeAbsentSegments(real.map((r) => segmentTypeFromNode(r) ?? ''), expectedSegs);
		const ghostsBefore = new Map<number, VNode[]>();
		for (const p of placed) {
			const e = expectedSegs[p.defIdx];
			const parts: string[] = [];
			if (e.group) parts.push(e.group);
			parts.push(e.required ? tr('tree.expectedRequired') : tr('tree.expectedOptional'));
			if (e.repeats) parts.push(tr('tree.expectedRepeating'));
			if (e.choice) parts.push(tr('tree.expectedChoice'));
			const ghost: VNode = {
				id: `ghost.${e.code}`,
				label: e.code,
				value_preview: parts.join(' · '),
				node_type: 'segment',
				depth: 0,
				has_children: true,
				is_truncated: false,
				child_count: 0,
				placeholder: true,
				_expanded: false,
				_isPlaceholder: true,
			};
			ghostsBefore.set(p.before, [...(ghostsBefore.get(p.before) ?? []), ghost]);
		}

		const out: VNode[] = [];
		real.forEach((r, i) => {
			out.push(...(ghostsBefore.get(i) ?? []), r);
		});
		out.push(...(ghostsBefore.get(real.length) ?? []));
		return out;
	}

	// (Re)build the root list when the message is re-parsed, when
	// showSchemaFields toggles (expanded segments pick up / drop placeholder
	// rows) and when the expected structure arrives (ghost rows appear).
	// What was expanded and selected stays so: an edit used to collapse the
	// whole tree and leave the inspector on a node that no longer existed.
	let rebuildToken = 0;
	$effect(() => {
		void roots;
		void showSchemaFields;
		void expectedSegs;
		untrack(() => void rebuild());
	});

	async function rebuild() {
		const token = ++rebuildToken;
		const expanded = visibleNodes.filter((n) => n._expanded).map((n) => ({ id: n.id, label: n.label }));
		const selected = selectedNodeId;
		visibleNodes = buildRootNodes();
		// Each expansion is a backend call: a tree opened far and wide is
		// not worth restoring node by node.
		if (expanded.length <= 300) {
			for (const prev of expanded) {
				if (token !== rebuildToken) return;
				const node = visibleNodes.find((n) => n.id === prev.id);
				// Same id and label: the same segment (an inserted line shifts
				// the ids of what follows; those are left closed).
				if (node && node.label === prev.label && node.has_children && !node._expanded) {
					await toggleNode(node);
				}
			}
		}
		if (token !== rebuildToken || selected === null) return;
		const node = visibleNodes.find((n) => n.id === selected);
		if (node) {
			onNodeSelect?.(node); // the inspector gets the new value
		} else {
			selectedNodeId = null;
			onNodeSelect?.(null);
		}
	}

	// Reset search when the displayed message changes — hits reference node
	// ids of the previous message.
	let lastMessageId = $state('');
	$effect(() => {
		if (messageId !== lastMessageId) {
			lastMessageId = messageId;
			clearSearch();
		}
	});

	// Track the last processed stamp to avoid duplicate navigation
	let lastNavStamp = $state(0);

	// Navigate to a specific segment + optional field when requested
	$effect(() => {
		if (!navigateTo || navigateTo.stamp === lastNavStamp) return;
		lastNavStamp = navigateTo.stamp;
		const { segmentIdx, target } = navigateTo;
		untrack(() => void navigateToTarget(segmentIdx, target));
	});

	/** Expand `id` (when collapsed) and return its node as now listed. */
	async function expandById(id: string): Promise<VNode | undefined> {
		const node = visibleNodes.find((n) => n.id === id);
		if (node && node.has_children && !node._expanded) await toggleNode(node);
		return visibleNodes.find((n) => n.id === id);
	}

	/** Select a segment, or a field, repetition or component in it (the
	 *  deepest of them the tree has), scroll it into view and tell the
	 *  inspector. */
	async function navigateToTarget(segmentIdx: number, target: FieldTarget | null) {
		const segId = `seg${segmentIdx}`;
		let found = visibleNodes.find((n) => n.id === segId);
		if (!found) return;

		if (target && target.field > 0) {
			await expandById(segId);
			const fieldId = `${segId}.f${target.field}`;
			const fieldNode = visibleNodes.find((n) => n.id === fieldId);
			if (fieldNode) {
				found = fieldNode;
				let parentId = fieldId;
				if (target.repetition !== null && fieldNode.has_children) {
					const opened = await expandById(fieldId);
					const repNode = visibleNodes.find((n) => n.id === `${fieldId}.r${target.repetition}`);
					if (repNode) {
						found = repNode;
						parentId = repNode.id;
					} else if (opened && !visibleNodes.some((n) => n.id.startsWith(`${fieldId}.r`))) {
						parentId = fieldId; // the field does not repeat after all
					}
				}
				if (target.component !== null) {
					await expandById(parentId);
					const comp = visibleNodes.find((n) => n.id === `${parentId}.c${target.component}`);
					if (comp) found = comp;
				}
			}
		}

		selectedNodeId = found.id;
		onNodeSelect?.(found);
		await scrollToRow(visibleNodes.findIndex((n) => n.id === found!.id), true);
	}

	/** Extract segment-type code (e.g. "PID") from a segment node's label. */
	function segmentTypeFromNode(node: TreeNode): string | null {
		// Exactly three characters: "PIDX (2)" is no PID.
		const m = node.label.match(/^([A-Z][A-Z0-9]{2})(?:\s|$)/);
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
			const info = await getSegmentSchema(segType, version);
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
					value_preview: f.data_type,
					node_type: 'field' as const,
					depth: segNode.depth + 1,
					has_children: f.has_components,
					is_truncated: false,
					child_count: 0,
					placeholder: true,
					_expanded: false,
					_isPlaceholder: true,
					_dataType: f.data_type,
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
			const msgId = messageId;
			let childNodes: VNode[];
			if (node._isPlaceholder) {
				childNodes = await expandPlaceholder(node);
			} else {
				if (!node._children) {
					// FHIR resources live in a separate backend store: asking the
					// HL7 command for their children fails with "Message not found"
					// and the node silently never expands.
					const children = format === 'HL7v2'
						? await getTreeChildren(msgId, node.id)
						: await getFhirTreeChildren(msgId, node.id);
					node._children = children;
				}
				childNodes = await mergeSchemaPlaceholders(node, node._children!);
			}
			// The list may have changed while the children loaded (a
			// re-parse, another expansion): insert where the node is now.
			const at = visibleNodes.indexOf(node);
			if (msgId !== messageId || at === -1 || visibleNodes[at]._expanded) return;
			visibleNodes = [
				...visibleNodes.slice(0, at),
				{ ...node, _expanded: true },
				...childNodes,
				...visibleNodes.slice(at + 1),
			];
		}
	}

	/** Children of a ghost/placeholder node, entirely from the schema
	 *  catalogue: segment → its field definitions, composite field → its
	 *  components (e.g. OBX-16 → XCN.1, XCN.2, …). */
	async function expandPlaceholder(node: VNode): Promise<VNode[]> {
		try {
			if (node.node_type === 'segment') {
				const code = segmentTypeFromNode(node);
				if (!code) return [];
				const schema = await getSegmentSchema(code, version);
				return (schema?.fields ?? []).map((f) => ({
					id: `${node.id}.f${f.position}`,
					label: `${code}-${f.position} ${f.name}`,
					value_preview: f.data_type,
					node_type: 'field' as const,
					depth: node.depth + 1,
					has_children: f.has_components,
					is_truncated: false,
					child_count: 0,
					placeholder: true,
					_expanded: false,
					_isPlaceholder: true,
					_dataType: f.data_type,
				}));
			}
			if (node.node_type === 'field' && node._dataType) {
				const comps = await getCompositeComponents(node._dataType, version);
				const base = node.label.split(' ')[0]; // "OBX-16"
				return comps.map((c) => ({
					id: `${node.id}.c${c.position}`,
					label: `${base}.${c.position} ${c.name}`,
					value_preview: c.data_type,
					node_type: 'component' as const,
					depth: node.depth + 1,
					has_children: false,
					is_truncated: false,
					child_count: 0,
					placeholder: true,
					_expanded: false,
					_isPlaceholder: true,
				}));
			}
		} catch {
			// schema lookup failed — render as childless
		}
		return [];
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

	/** Parse a node id like "seg3", "seg3.f5", "seg3.f5.r2.c1" into navigation parts. */
	function parseNodeId(id: string): { segmentIdx: number | null; target: FieldTarget } {
		let segmentIdx: number | null = null;
		const target: FieldTarget = { field: 0, repetition: null, component: null };
		for (const p of id.split('.')) {
			if (/^seg\d+$/.test(p)) segmentIdx = parseInt(p.slice(3));
			else if (/^f\d+$/.test(p)) target.field = parseInt(p.slice(1));
			else if (/^r\d+$/.test(p)) target.repetition = parseInt(p.slice(1));
			else if (/^c\d+$/.test(p)) target.component = parseInt(p.slice(1));
		}
		return { segmentIdx, target };
	}

	/** For a ghost root, find the real segment (line index) it should follow:
	 *  walk visibleNodes backwards from the ghost to the nearest real root. */
	function insertGhostSegment(node: VNode) {
		if (!onInsertSegment) return;
		const code = segmentTypeFromNode(node);
		if (!code) return;
		const idx = visibleNodes.findIndex((n) => n.id === node.id);
		let afterIdx: number | null = null;
		for (let i = idx - 1; i >= 0; i--) {
			const n = visibleNodes[i];
			// A real segment is recognised by its id ("seg<N>", which only
			// top-level segment nodes have), not by depth: the backend numbers
			// real segments depth 1 while ghost rows are depth 0, so any depth
			// test missed them and every insert went to the wrong line.
			if (!n._isPlaceholder) {
				const m = n.id.match(/^seg(\d+)$/);
				if (m) { afterIdx = parseInt(m[1]); break; }
			}
		}
		onInsertSegment(code, afterIdx);
	}

	function showInEditor(node: TreeNode) {
		const { segmentIdx, target } = parseNodeId(node.id);
		if (segmentIdx === null) return;
		onNavigateToEditor?.(segmentIdx, target);
	}

	/**
	 * Keyboard navigation (WAI-ARIA tree): Up/Down move, Home/End and
	 * PageUp/PageDown jump, Right opens a node or enters it, Left closes it
	 * or goes to its parent. Only the rows in view exist, so moving by Tab
	 * alone would stop at the edge of the rendered window.
	 */
	async function handleRowKeydown(e: KeyboardEvent) {
		const row = e.target;
		if (!(row instanceof HTMLElement) || !row.matches('[data-node-id]')) return;
		const idx = visibleNodes.findIndex((n) => n.id === row.getAttribute('data-node-id'));
		if (idx < 0) return;
		const node = visibleNodes[idx];
		const page = Math.max(1, Math.floor(viewportHeight / ROW) - 1);
		let next = -1;
		switch (e.key) {
			case 'ArrowDown': next = Math.min(visibleNodes.length - 1, idx + 1); break;
			case 'ArrowUp': next = Math.max(0, idx - 1); break;
			case 'Home': next = 0; break;
			case 'End': next = visibleNodes.length - 1; break;
			case 'PageDown': next = Math.min(visibleNodes.length - 1, idx + page); break;
			case 'PageUp': next = Math.max(0, idx - page); break;
			case 'ArrowRight':
				if (!node.has_children) return;
				if (!node._expanded) {
					e.preventDefault();
					await toggleNode(node);
					return;
				}
				next = idx + 1;
				break;
			case 'ArrowLeft':
				if (node._expanded) {
					e.preventDefault();
					await toggleNode(node);
					return;
				}
				for (let i = idx - 1; i >= 0; i--) {
					if (visibleNodes[i].depth < node.depth) { next = i; break; }
				}
				break;
			default:
				return;
		}
		e.preventDefault();
		if (next < 0 || next >= visibleNodes.length) return;
		const target = visibleNodes[next];
		selectNode(target);
		await scrollToRow(next);
		await focusRow(target.id);
	}

	/** The row the Tab key enters the tree on: the selected one, else the first rendered. */
	let tabStopId = $derived(
		selectedNodeId !== null && renderedNodes.some((n) => n.id === selectedNodeId)
			? selectedNodeId
			: renderedNodes[0]?.id ?? null,
	);
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="tree-container" bind:this={containerEl} onkeydown={handleTreeKeydown} onscroll={handleScroll}>
	{#if visibleNodes.length === 0}
		<div class="tree-empty">{tr('tree.noMessage')}</div>
	{:else}
		{#if searchEnabled}
		<div class="tree-search">
			<div class="search-row">
				<span class="search-icon">&#128269;</span>
				<input
					class="search-input"
					type="text"
					bind:this={searchInputEl}
					bind:value={searchQuery}
					placeholder={tr('tree.searchPlaceholder')}
					onkeydown={handleSearchKeydown}
					spellcheck="false"
				/>
				{#if searchQuery}
					<button class="search-clear" onclick={clearSearch} title={tr('tree.searchClear')}>&times;</button>
				{/if}
			</div>
			{#if searchActive && !searchPending}
				<div class="search-status">
					{#if searchHits.length === 0}
						<span class="no-results">{tr('tree.searchNoResults')}</span>
					{:else}
						<span>{tr('tree.searchResults', { count: searchHits.length })}</span>
						<span class="search-nav">
							<button class="nav-btn" onclick={prevHit} title="Shift+Enter">&#9650;</button>
							<button class="nav-btn" onclick={nextHit} title="Enter">&#9660;</button>
						</span>
					{/if}
				</div>
				{#if searchHits.length > 0}
					<div class="search-hits">
						{#each searchHits as hit, i (hit.node_id + ':' + i)}
							<button
								class="search-hit"
								class:active={i === activeHitIdx}
								onclick={() => gotoHit(i)}
							>
								<span class="hit-kind" class:kind-name={hit.match_kind === 'name'} class:kind-segment={hit.match_kind === 'segment'}>
									{hit.match_kind === 'value' ? '=' : hit.match_kind === 'name' ? 'Aa' : '§'}
								</span>
								<span class="hit-label">{hit.label}</span>
								{#if hit.snippet}<span class="hit-snippet">{hit.snippet}</span>{/if}
							</button>
						{/each}
					</div>
				{/if}
			{/if}
		</div>
		{/if}
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<div
			class="tree-list"
			role="tree"
			tabindex={-1}
			bind:this={listEl}
			onkeydown={handleRowKeydown}
			style="height: {visibleNodes.length * ROW + 8}px; padding-top: {range.start * ROW + 4}px"
		>
			{#each renderedNodes as node (node.id)}
				<TreeNodeRow
					{node}
					tabIndex={node.id === tabStopId ? 0 : -1}
					isSelected={selectedNodeId === node.id}
					isExpanded={node._expanded ?? false}
					isPlaceholder={node._isPlaceholder ?? false}
					onToggle={() => toggleNode(node)}
					onSelect={() => selectNode(node)}
					onExpandTruncated={() => expandTruncated(node)}
					onShowInEditor={onNavigateToEditor && !node._isPlaceholder ? () => showInEditor(node) : undefined}
					onInsertSegment={onInsertSegment && node._isPlaceholder && node.id.startsWith('ghost.') && node.node_type === 'segment'
						? () => insertGhostSegment(node)
						: undefined}
					onShowInGrid={onShowInGrid && format === 'HL7v2' && !node._isPlaceholder && node.node_type === 'segment' && segmentTypeFromNode(node)
						? () => onShowInGrid(segmentTypeFromNode(node) ?? '')
						: undefined}
				/>
			{/each}
		</div>
	{/if}
</div>

<style>
	.tree-container {
		position: relative; /* offsetTop of the list is measured from here */
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
		box-sizing: border-box;
		padding-bottom: 4px;
	}

	/* --- Search bar --- */
	.tree-search {
		position: sticky;
		top: 0;
		z-index: 10;
		background-color: var(--color-bg-secondary);
		border-bottom: 1px solid var(--color-border);
		padding: 4px 6px;
	}

	.search-row {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.search-icon {
		font-size: 11px;
		opacity: 0.6;
		flex-shrink: 0;
	}

	.search-input {
		flex: 1;
		min-width: 0;
		padding: 3px 6px;
		border: 1px solid var(--color-border);
		border-radius: 3px;
		background: var(--color-bg-tertiary);
		color: var(--color-text-primary);
		font-family: inherit;
		font-size: 11px;
	}

	.search-input:focus {
		outline: none;
		border-color: var(--color-accent);
	}

	.search-clear {
		background: none;
		border: none;
		color: var(--color-text-secondary);
		font-size: 14px;
		cursor: pointer;
		padding: 0 4px;
		flex-shrink: 0;
	}

	.search-clear:hover { color: var(--color-text-primary); }

	.search-status {
		display: flex;
		align-items: center;
		justify-content: space-between;
		font-size: 10px;
		color: var(--color-text-secondary);
		padding: 3px 2px 1px;
	}

	.no-results { font-style: italic; }

	.search-nav { display: flex; gap: 2px; }

	.nav-btn {
		background: none;
		border: 1px solid var(--color-border);
		border-radius: 3px;
		color: var(--color-text-secondary);
		font-size: 8px;
		cursor: pointer;
		padding: 1px 5px;
	}

	.nav-btn:hover { color: var(--color-text-primary); background: var(--color-bg-tertiary); }

	.search-hits {
		max-height: 180px;
		overflow-y: auto;
		margin-top: 3px;
		border-top: 1px solid var(--color-border);
	}

	.search-hit {
		display: flex;
		align-items: center;
		gap: 6px;
		width: 100%;
		padding: 2px 4px;
		background: none;
		border: none;
		border-left: 2px solid transparent;
		color: var(--color-text-primary);
		font-family: inherit;
		font-size: 11px;
		text-align: left;
		cursor: pointer;
		white-space: nowrap;
		overflow: hidden;
	}

	.search-hit:hover { background: var(--color-bg-tertiary); }

	.search-hit.active {
		background: var(--color-bg-tertiary);
		border-left-color: var(--color-accent);
	}

	.hit-kind {
		flex-shrink: 0;
		width: 18px;
		text-align: center;
		font-size: 9px;
		color: var(--color-accent);
		border: 1px solid var(--color-border);
		border-radius: 3px;
	}

	.hit-kind.kind-name { color: var(--color-field); }
	.hit-kind.kind-segment { color: var(--color-segment); }

	.hit-label {
		flex-shrink: 0;
		font-weight: 600;
		color: var(--color-field);
	}

	.hit-snippet {
		overflow: hidden;
		text-overflow: ellipsis;
		color: var(--color-text-secondary);
		opacity: 0.8;
	}
</style>
