import { invoke } from '@tauri-apps/api/core';

export interface ValidationIssue {
	severity: 'error' | 'warning' | 'info';
	message: string;
	segment_idx: number | null;
	segment_type: string | null;
	field_position: number | null;
	rule_id: string;
}

export interface ValidationReport {
	issues: ValidationIssue[];
	error_count: number;
	warning_count: number;
	info_count: number;
}

export interface FhirValidationIssue {
	severity: string;
	message: string;
	path: string;
}

export interface FhirValidationReport {
	issues: FhirValidationIssue[];
	error_count: number;
	warning_count: number;
	info_count: number;
}

export async function validateMessage(messageId: string): Promise<ValidationReport> {
	return invoke('validate_message', { messageId });
}

export async function validateFhir(content: string): Promise<FhirValidationReport> {
	return invoke('validate_fhir', { content });
}

export async function parseFhirMessage(content: string): Promise<import('$lib/types/hl7').ParseResult> {
	return invoke('parse_fhir_message', { content });
}

export async function getFhirTreeChildren(
	messageId: string,
	nodeId: string,
): Promise<import('$lib/types/hl7').TreeNode[]> {
	return invoke('get_fhir_tree_children', { messageId, nodeId });
}
