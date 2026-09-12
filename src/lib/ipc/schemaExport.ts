import { invoke } from '@tauri-apps/api/core';

export interface VersionOption {
	key: string;
	label: string;
	tier: 'free' | 'pro';
	/**
	 * Set when the version reuses another release's definitions (HL7 v2.7.1
	 * is a technical correction of v2.7 and ships identical tables), so the
	 * dropdown can say so rather than imply distinct data.
	 */
	aliased_from: string | null;
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
