import { invoke } from '@tauri-apps/api/core';
import type { ParseResult, TreeNode, FieldContent } from '$lib/types/hl7';

/** Parse an HL7 message from raw text content */
export async function parseMessage(content: string, source?: string): Promise<ParseResult> {
	return invoke<ParseResult>('parse_message', { content, source: source ?? null });
}

/** Get child tree nodes for a given parent node */
export async function getTreeChildren(messageId: string, nodeId: string): Promise<TreeNode[]> {
	return invoke<TreeNode[]>('get_tree_children', { messageId, nodeId });
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
	return invoke<ParseResult>('open_file', { path });
}

/** Save message content to a file.
 *  Pass `content` to save the current editor text (preferred when user edited the message).
 *  Pass `messageId` to save the original parsed content from the message store. */
export async function saveFile(args: {
	path: string;
	content?: string;
	messageId?: string;
}): Promise<{ path: string; bytes_written: number }> {
	return invoke('save_file', {
		messageId: args.messageId ?? null,
		path: args.path,
		content: args.content ?? null,
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
