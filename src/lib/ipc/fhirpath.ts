import { invoke } from '@tauri-apps/api/core';

export interface FhirPathResult {
	expression: string;
	results: unknown[];
	count: number;
	error: string | null;
}

export async function evaluateFhirPath(messageId: string, expression: string): Promise<FhirPathResult> {
	return invoke('evaluate_fhirpath', { messageId, expression });
}
