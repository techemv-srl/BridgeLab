/** Tree node types matching Rust backend */
export type TreeNodeType = 'message' | 'segment' | 'field' | 'component' | 'subcomponent';

/** Tree node from the backend */
export interface TreeNode {
	id: string;
	label: string;
	value_preview: string;
	node_type: TreeNodeType;
	depth: number;
	has_children: boolean;
	is_truncated: boolean;
	child_count: number;
}

/** Result from parse_message IPC command */
export interface ParseResult {
	message_id: string;
	message_type: string;
	format: string;
	version: string;
	truncated_text: string;
	tree_roots: TreeNode[];
	truncation_count: number;
	file_size_bytes: number;
	segment_count: number;
}

/** Result from get_field_content IPC command */
export interface FieldContent {
	full_text: string;
	byte_length: number;
}

/** Result from save_file IPC command */
export interface SaveResult {
	path: string;
	bytes_written: number;
}
