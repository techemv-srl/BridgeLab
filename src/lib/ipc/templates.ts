import { invoke } from '@tauri-apps/api/core';

export interface MessageTemplate {
	id: string;
	name: string;
	message_type: string;
	description: string;
	category: string;
	content: string;
}

export async function getTemplates(): Promise<MessageTemplate[]> {
	return invoke('get_templates');
}

export async function getTemplatesGrouped(): Promise<[string, MessageTemplate[]][]> {
	return invoke('get_templates_grouped');
}
