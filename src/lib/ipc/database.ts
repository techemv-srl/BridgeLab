import { invoke } from '@tauri-apps/api/core';

export interface RecentFile {
	path: string;
	filename: string;
	message_type: string;
	version: string;
	file_size: number;
	opened_at: string;
}

export interface Preference {
	key: string;
	value: string;
}

export async function getRecentFiles(limit: number = 20): Promise<RecentFile[]> {
	return invoke('get_recent_files', { limit });
}

export async function addRecentFile(
	path: string,
	filename: string,
	messageType: string,
	version: string,
	fileSize: number,
): Promise<void> {
	return invoke('add_recent_file', {
		path,
		filename,
		messageType,
		version,
		fileSize,
	});
}

export async function removeRecentFile(path: string): Promise<void> {
	return invoke('remove_recent_file', { path });
}

export async function clearRecentFiles(): Promise<void> {
	return invoke('clear_recent_files');
}

export async function getPreference(key: string): Promise<string | null> {
	return invoke('get_preference', { key });
}

export async function setPreference(key: string, value: string): Promise<void> {
	return invoke('set_preference', { key, value });
}

export async function getAllPreferences(): Promise<Preference[]> {
	return invoke('get_all_preferences');
}
