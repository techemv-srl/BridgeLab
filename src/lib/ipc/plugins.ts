import { invoke } from '@tauri-apps/api/core';

export interface PluginInfo {
	id: string;
	name: string;
	description: string;
	author: string;
	version: string;
	enabled: boolean;
	kind: 'validation' | 'anonymization';
	path: string;
	rule_count: number;
	error: string | null;
}

export async function listPlugins(): Promise<PluginInfo[]> {
	return invoke('list_plugins');
}

export async function reloadPlugins(): Promise<PluginInfo[]> {
	return invoke('reload_plugins');
}

export async function setPluginEnabled(id: string, enabled: boolean): Promise<void> {
	return invoke('set_plugin_enabled', { id, enabled });
}

export async function applyPluginOverrides(overrides: Record<string, boolean>): Promise<void> {
	return invoke('apply_plugin_overrides', { overrides });
}

export async function getPluginsDir(): Promise<string> {
	return invoke('get_plugins_dir');
}

export async function openPluginsFolder(): Promise<void> {
	return invoke('open_plugins_folder');
}
