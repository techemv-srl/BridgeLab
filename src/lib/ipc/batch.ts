import { invoke } from '@tauri-apps/api/core';

export interface BatchFileResult {
	path: string;
	file_name: string;
	message_type: string;
	version: string;
	segment_count: number;
	error_count: number;
	warning_count: number;
	parse_error: string | null;
}

export interface BatchReport {
	results: BatchFileResult[];
	/** Files skipped because the backend file cap was hit. */
	skipped: number;
}

/** Validate every message file in `paths` (files and/or directories). Pro feature. */
export async function batchValidate(paths: string[]): Promise<BatchReport> {
	return invoke('batch_validate', { paths });
}

export interface GeneratedMessage {
	content: string;
	label: string;
}

/** Generate synthetic test messages ("ADT^A01" | "ADT^A08" | "ORU^R01" | "ORM^O01" | "mixed"). */
export async function generateTestMessages(
	kind: string,
	count: number,
	seed?: number,
): Promise<GeneratedMessage[]> {
	return invoke('generate_test_messages', { kind, count, seed: seed ?? null });
}
