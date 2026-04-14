import { invoke } from '@tauri-apps/api/core';

export interface PhiLocation {
	segment_idx: number;
	segment_type: string;
	field_position: number;
	field_name: string;
	sensitivity: 'high' | 'medium' | 'low';
	current_value: string;
}

export interface AnonymizeResult {
	anonymized_text: string;
	phi_fields_masked: number;
}

export async function detectPhi(messageId: string): Promise<PhiLocation[]> {
	return invoke('detect_phi', { messageId });
}

export async function anonymizeMessage(messageId: string): Promise<AnonymizeResult> {
	return invoke('anonymize_message', { messageId });
}

export async function getMessageFullText(messageId: string): Promise<string> {
	return invoke('get_message_full_text', { messageId });
}

export async function getMessageTruncatedText(
	messageId: string, threshold?: number,
): Promise<string> {
	return invoke('get_message_truncated_text', { messageId, threshold });
}

export async function exportAsJson(messageId: string): Promise<string> {
	return invoke('export_as_json', { messageId });
}

export async function exportAsCsv(messageId: string): Promise<string> {
	return invoke('export_as_csv', { messageId });
}
