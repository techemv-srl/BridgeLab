import { invoke } from '@tauri-apps/api/core';

export interface TestCase {
	id: string;
	name: string;
	description: string;
	category: string;
	tags: string;
	content: string;
	expected_message_type: string;
	expected_validation_result: string;
	created_at: string;
	updated_at: string;
}

export async function saveTestCase(
	testCase: Partial<TestCase> & {
		name: string;
		content: string;
	}
): Promise<TestCase> {
	return invoke('save_test_case', {
		id: testCase.id ?? null,
		name: testCase.name,
		description: testCase.description ?? '',
		category: testCase.category ?? 'general',
		tags: testCase.tags ?? '',
		content: testCase.content,
		expectedMessageType: testCase.expected_message_type ?? '',
		expectedValidationResult: testCase.expected_validation_result ?? 'valid',
	});
}

export async function getTestCases(category?: string): Promise<TestCase[]> {
	return invoke('get_test_cases', { category: category ?? null });
}

export async function deleteTestCase(id: string): Promise<void> {
	return invoke('delete_test_case', { id });
}

// --- Test case packs (export / import) ---

export type PhiScanKind = 'hl7' | 'fhir' | 'unparsed';

export interface PhiScan {
	id: string;
	name: string;
	kind: PhiScanKind;
	/** PHI fields holding a value, e.g. "PID-5 Patient Name" (HL7 v2 only). */
	fields: string[];
}

/** Test cases (all when `ids` is empty) that need a look before sharing. */
export async function scanTestCasesPhi(ids: string[]): Promise<PhiScan[]> {
	return invoke('scan_test_cases_phi', { ids });
}

export interface PackExportResult {
	exported: number;
	anonymized: number;
}

/** Write a pack; `anonymize` masks HL7 v2 PHI in the file only (Pro). */
export async function exportTestCases(ids: string[], path: string, anonymize: boolean): Promise<PackExportResult> {
	return invoke('export_test_cases', { ids, path, anonymize });
}

export type ImportStatus = 'new' | 'identical' | 'conflict';
export type ConflictChoice = 'skip' | 'overwrite' | 'copy';

export interface ImportPlanItem {
	index: number;
	id: string;
	name: string;
	category: string;
	status: ImportStatus;
	existing_name: string | null;
}

export interface ImportPreview {
	app_version: string;
	exported_at: string;
	items: ImportPlanItem[];
	/** Room left under the Community cap; null when unlimited. */
	room: number | null;
	/** Identifies the previewed file; the import refuses a changed one. */
	fingerprint: string;
}

export async function previewTestCaseImport(path: string): Promise<ImportPreview> {
	return invoke('preview_test_case_import', { path });
}

export interface PackImportResult {
	added: number;
	updated: number;
	skipped: number;
}

/** Import the previewed pack; `choices` maps a conflict's pack index to what to do. */
export async function importTestCases(
	path: string,
	fingerprint: string,
	choices: Record<number, ConflictChoice>,
): Promise<PackImportResult> {
	return invoke('import_test_cases', { path, fingerprint, choices });
}

/** Counts an import will produce, for the preview summary. */
export function importCounts(items: ImportPlanItem[], choices: Record<number, ConflictChoice>) {
	let add = 0, update = 0, skip = 0;
	for (const it of items) {
		if (it.status === 'new') add++;
		else if (it.status === 'identical') skip++;
		else {
			const c = choices[it.index] ?? 'skip';
			if (c === 'overwrite') update++;
			else if (c === 'copy') add++;
			else skip++;
		}
	}
	return { add, update, skip };
}
