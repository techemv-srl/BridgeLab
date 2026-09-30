import { invoke } from '@tauri-apps/api/core';
import type { ParseResult, TreeNode, FieldContent } from '$lib/types/hl7';
import { trackMessage } from '$lib/stores/message-gc';

/** Parse an HL7 message from raw text content */
export async function parseMessage(content: string, source?: string): Promise<ParseResult> {
	const result = await invoke<ParseResult>('parse_message', { content, source: source ?? null });
	trackMessage(result.message_id);
	return result;
}

/** Get child tree nodes for a given parent node */
export async function getTreeChildren(messageId: string, nodeId: string): Promise<TreeNode[]> {
	return invoke<TreeNode[]>('get_tree_children', { messageId, nodeId });
}

/** A single search match. node_id follows the tree scheme ("seg3", "seg3.f5"). */
export interface SearchHit {
	node_id: string;
	segment_idx: number;
	field_position: number | null;
	label: string;
	snippet: string;
	match_kind: 'segment' | 'name' | 'value';
}

/** Case-insensitive search across segment types, schema field names and field values. */
export async function searchMessage(messageId: string, query: string): Promise<SearchHit[]> {
	return invoke<SearchHit[]>('search_message', { messageId, query });
}

/** Get full content of a specific field (for expanding truncated fields) */
export async function getFieldContent(
	messageId: string,
	segmentIdx: number,
	fieldIdx: number
): Promise<FieldContent> {
	return invoke<FieldContent>('get_field_content', { messageId, segmentIdx, fieldIdx });
}

/** Open a file from disk and parse it */
export async function openFile(path: string): Promise<ParseResult> {
	const result = await invoke<ParseResult>('open_file', { path });
	trackMessage(result.message_id);
	return result;
}

/** Save message content to a file.
 *  Pass `content` to save the current editor text (preferred when user edited the message).
 *  Pass `messageId` to save the original parsed content from the message store. */
export async function saveFile(args: {
	path: string;
	content?: string;
	messageId?: string;
	/** Charset the tab's file was opened in (non-UTF-8 files only). */
	charset?: string | null;
}): Promise<{ path: string; bytes_written: number }> {
	return invoke('save_file', {
		messageId: args.messageId ?? null,
		path: args.path,
		content: args.content ?? null,
		charset: args.charset ?? null,
	});
}

/** Expand a truncated field inline - returns full text with that field expanded */
export async function expandFieldInline(
	messageId: string,
	segmentIdx: number,
	fieldIdx: number,
): Promise<string> {
	return invoke<string>('expand_field_inline', { messageId, segmentIdx, fieldIdx });
}

/** Expand ALL truncated fields - returns full original message text */
export async function expandAllFields(messageId: string): Promise<string> {
	return invoke<string>('expand_all_fields', { messageId });
}

/** Re-truncate all fields - returns text with all fields truncated */
export async function collapseAllFields(messageId: string): Promise<string> {
	return invoke<string>('collapse_all_fields', { messageId });
}

/** The file's modification time and size; null when it does not exist.
 *  Rejects when that cannot be told (no permission...). */
export async function fileStat(path: string): Promise<{ modified_ms: number; size: number } | null> {
	return invoke('file_stat', { path });
}

/** The file `path` names with symlinks and `.`/`..` resolved; null when
 *  it cannot be resolved (it does not exist). */
export async function canonicalPath(path: string): Promise<string | null> {
	return invoke('canonical_path', { path });
}

/** Files to open that arrived before the frontend was listening (launch
 *  arguments, forwarded launches, Finder opens). Drained: returns them
 *  once; later ones arrive as `app://open-files` events. */
export async function getLaunchFiles(): Promise<string[]> {
	return invoke('get_launch_files');
}

// --- Segment grid ---

export interface SegmentCount {
	segment_type: string;
	count: number;
}

export interface GridColumn {
	position: number;
	/** Field name from the catalogue of the message's version; empty if unknown. */
	name: string;
	data_type: string;
}

export interface GridCell {
	value: string;
	truncated: boolean;
	code_desc: string | null;
}

export interface GridRow {
	/** Segment index in the message: tree node `seg{N}`, editor line N+1. */
	segment_idx: number;
	cells: GridCell[];
}

export interface SegmentGrid {
	segment_type: string;
	segment_name: string;
	version: string;
	columns: GridColumn[];
	rows: GridRow[];
}

export async function getSegmentCounts(messageId: string): Promise<SegmentCount[]> {
	return invoke('get_segment_counts', { messageId });
}

export async function getSegmentGrid(messageId: string, segmentType: string): Promise<SegmentGrid> {
	return invoke('get_segment_grid', { messageId, segmentType });
}

/** The segment the grid opens on: the requested one if present, otherwise
 *  the most repeated one (MSH never), otherwise the first. */
export function defaultGridSegment(counts: SegmentCount[], requested?: string | null): string | null {
	if (requested && counts.some((c) => c.segment_type === requested)) return requested;
	const candidates = counts.filter((c) => c.segment_type !== 'MSH');
	if (candidates.length === 0) return counts[0]?.segment_type ?? null;
	return candidates.reduce((best, c) => (c.count > best.count ? c : best)).segment_type;
}
