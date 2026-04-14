import { invoke } from '@tauri-apps/api/core';

export interface BundleEntry {
	index: number;
	full_url: string | null;
	resource_type: string;
	resource_id: string | null;
	display_name: string;
	summary: string;
	request_method: string | null;
	request_url: string | null;
	response_status: string | null;
	references: string[];
}

export interface ReferenceEdge {
	from_index: number;
	to_index: number | null;
	reference: string;
	field_path: string;
}

export interface BundleAnalysis {
	bundle_type: string;
	total: number | null;
	entry_count: number;
	entries: BundleEntry[];
	references: ReferenceEdge[];
	resource_type_counts: [string, number][];
	dangling_references: number;
}

export async function analyzeFhirBundle(messageId: string): Promise<BundleAnalysis> {
	return invoke('analyze_fhir_bundle', { messageId });
}

export async function getFhirBundleEntry(messageId: string, entryIndex: number): Promise<string> {
	return invoke('get_fhir_bundle_entry', { messageId, entryIndex });
}
