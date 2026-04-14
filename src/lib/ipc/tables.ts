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
