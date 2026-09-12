import { invoke } from '@tauri-apps/api/core';

export interface TraceEntry {
	name: string;
	values: unknown[];
}

export interface FhirPathResult {
	expression: string;
	results: unknown[];
	count: number;
	error: string | null;
	/** Values captured by `trace('label')` calls, in evaluation order. */
	trace?: TraceEntry[];
}

export async function evaluateFhirPath(messageId: string, expression: string): Promise<FhirPathResult> {
	return invoke('evaluate_fhirpath', { messageId, expression });
}
