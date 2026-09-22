import { invoke } from '@tauri-apps/api/core';

export interface FieldDef {
	position: number;
	name: string;
	data_type: string;
	max_length: number | null;
	required: boolean;
	repeating: boolean;
	description: string;
	/** HL7 value table behind the field's value (first component for composites). */
	table_id: string | null;
	/** Data type of the element that table belongs to (MSG.1 is an `ID` even
	 *  though MSH-9 is `MSG`): decides whether the table is closed. */
	table_data_type: string | null;
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
	/** HL7 value table backing this coded field (e.g. "0001" for PID-8);
	 *  for a composite field, its first component's table (MSH-9 → 0076). */
	table_id: string | null;
	/** Components of a composite field, each with its own table where coded. */
	components: ComponentInfo[];
}

export interface ComponentInfo {
	position: number;
	name: string;
	data_type: string;
	max_length: number | null;
	table_id: string | null;
}

export interface TableValue {
	code: string;
	description: string;
}

export interface ValueTable {
	id: string;
	name: string;
	/** True when the standard lists every legal value (the element is an `ID`):
	 *  a value outside the table is non-standard. False for user-defined tables
	 *  (`IS`…), whose values are suggestions. */
	exhaustive: boolean;
	values: TableValue[];
}

/** Fetch the values of an HL7 value table (e.g. "0001" Administrative Sex).
 *  `dataType` is that of the element the table is shown for; it decides
 *  whether the table is exhaustive. */
export async function getHl7Table(tableId: string, dataType?: string): Promise<ValueTable | null> {
	return invoke('get_hl7_table', { tableId, dataType: dataType ?? null });
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
