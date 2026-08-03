import { invoke } from '@tauri-apps/api/core';

export interface FieldDef {
	position: number;
	name: string;
	data_type: string;
	max_length: number | null;
	required: boolean;
	repeating: boolean;
	description: string;
}

export interface SegmentInfo {
	code: string;
	name: string;
	description: string;
	fields: FieldDef[];
}

export interface FieldInfo {
	segment_code: string;
	position: number;
	name: string;
	data_type: string;
	max_length: number | null;
	required: boolean;
	repeating: boolean;
	description: string;
	/** HL7 value table backing this coded field (e.g. "0001" for PID-8). */
	table_id: string | null;
}

export interface TableValue {
	code: string;
	description: string;
}

export interface ValueTable {
	id: string;
	name: string;
	/** False for deliberately partial tables (e.g. 0076): absence of a value is not evidence it is non-standard. */
	exhaustive: boolean;
	values: TableValue[];
}

/** Fetch the values of an HL7 value table (e.g. "0001" Administrative Sex). */
export async function getHl7Table(tableId: string): Promise<ValueTable | null> {
	return invoke('get_hl7_table', { tableId });
}

export async function getSegmentInfo(
	segmentType: string,
	version: string,
): Promise<SegmentInfo | null> {
	return invoke('get_segment_info', { segmentType, version });
}

export async function getFieldInfo(
	segmentType: string,
	fieldPosition: number,
	version: string,
): Promise<FieldInfo | null> {
	return invoke('get_field_info', { segmentType, fieldPosition, version });
}

// --- Schema-catalogue-backed structure info (all shipped HL7 versions) ---

export interface ExpectedSegment {
	code: string;
	required: boolean;
	repeats: boolean;
	/** Group path (" / "-joined), empty for top-level segments. */
	group: string;
	/** True when the segment is one alternative of an HL7 choice block. */
	choice: boolean;
}

/** Expected segment sequence for a message type (empty if unknown). */
export async function getExpectedSegments(
	messageType: string, version: string,
): Promise<ExpectedSegment[]> {
	return invoke('get_expected_segments', { messageType, version });
}

export interface SchemaFieldInfo {
	position: number;
	name: string;
	data_type: string;
	required: boolean;
	repeats: boolean;
	has_components: boolean;
}

export interface SegmentSchemaInfo {
	code: string;
	name: string;
	fields: SchemaFieldInfo[];
}

export async function getSegmentSchema(
	segment: string, version: string,
): Promise<SegmentSchemaInfo | null> {
	return invoke('get_segment_schema', { segment, version });
}

export interface CompositeComponentInfo {
	position: number;
	name: string;
	data_type: string;
}

/** Components of a composite data type (empty for primitives / unknown). */
export async function getCompositeComponents(
	dataType: string, version: string,
): Promise<CompositeComponentInfo[]> {
	return invoke('get_composite_components', { dataType, version });
}
