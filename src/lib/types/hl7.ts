/** Tree node types matching Rust backend */
export type TreeNodeType = 'message' | 'segment' | 'field' | 'repetition' | 'component' | 'subcomponent';

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
	/** The code a coded element is matched against its HL7 table by — first
	 *  component, first repetition, split on the message's own delimiters
	 *  ("ADT" for MSH-9 = "ADT^A01"). Present whenever the element has a
	 *  table, listed in it or not. HL7 v2 only. */
	code?: string;
	/** Meaning of that code from its HL7 table ("Male" for PID-8 = M), when listed. */
	code_desc?: string;
	/** A row of the standard structure that is absent from the message
	 *  (Show Schema Fields): its value is the data type, not a value. */
	placeholder?: boolean;
}

/** Result from parse_message IPC command */
export interface ParseResult {
	message_id: string;
	message_type: string;
	format: string;
	version: string;
	truncated_text: string;
	/** The whole decoded file, set by open_file: what the tab holds and saves. */
	full_text?: string;
	tree_roots: TreeNode[];
	truncation_count: number;
	file_size_bytes: number;
	segment_count: number;
	/** Charset a non-UTF-8 file was decoded with; absent for UTF-8. */
	source_charset?: string;
	/** open_file: the file is not a message BridgeLab can parse, and why.
	 *  It opens as text; everything else in the result is empty. */
	parse_error?: string;
	/** open_file: the file was not decoded as its MSH-18 says. `unsupported`:
	 *  BridgeLab cannot decode the declared charset (text and fields may be
	 *  wrong); `not_utf8`: UTF-8 declared, other bytes found. */
	charset_warning?: { kind: 'unsupported' | 'not_utf8'; declared: string };
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
