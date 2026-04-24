import { invoke } from '@tauri-apps/api/core';

export interface VersionOption {
	key: string;
	label: string;
	tier: 'free' | 'pro';
}

export interface MessageOption {
	code: string;
	event: string;
	description: string;
	tier: 'free' | 'pro';
}

export async function listVersions(): Promise<VersionOption[]> {
	return invoke('hl7_schema_list_versions');
}

export async function listMessages(versionKey: string): Promise<MessageOption[]> {
	return invoke('hl7_schema_list_messages', { versionKey });
}

export async function exportXsd(versionKey: string, messageCode: string): Promise<string> {
	return invoke('hl7_schema_export_xsd', { versionKey, messageCode });
}
