import { invoke } from '@tauri-apps/api/core';

export interface PluginInfo {
	/** `<kind>/<id>`: what the on/off switch refers to. */
	key: string;
	id: string;
	name: string;
	description: string;
	author: string;
	version: string;
	enabled: boolean;
	/** Enabled but inactive: beyond the Community cap on active plugins. */
	gated: boolean;
	kind: 'validation' | 'fhir' | 'anonymization';
	path: string;
	rule_count: number;
	error: string | null;
	/** Problems that did not stop the pack loading (typos, broken patterns). */
	warnings: string[];
}

export async function listPlugins(): Promise<PluginInfo[]> {
	return invoke('list_plugins');
}

export async function reloadPlugins(): Promise<PluginInfo[]> {
	return invoke('reload_plugins');
}

/** Switch a pack on or off; the backend saves the choice where the CLI reads it too. */
export async function setPluginEnabled(key: string, enabled: boolean): Promise<void> {
	return invoke('set_plugin_enabled', { key, enabled });
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
