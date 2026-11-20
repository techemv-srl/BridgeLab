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

// --- Session persistence (Notepad++-style restore) ---

export interface SessionTab {
	tab_order: number;
	label: string;
	file_path: string | null;
	content: string;
	is_modified: boolean;
	is_active: boolean;
	cursor_line: number;
	cursor_column: number;
}

export async function saveSession(tabs: SessionTab[]): Promise<void> {
	return invoke('save_session', { tabs });
}

export async function loadSession(): Promise<SessionTab[]> {
	return invoke('load_session');
}

export async function clearSession(): Promise<void> {
	return invoke('clear_session');
}

/** Hand the backend the latest tabs: the whole session (`session`, restore
 *  on) or only the unsaved tabs (restore off). Held in memory only, and
 *  written once, as the session or as a recovery copy, if the system ends
 *  the app without the window's close request. An empty list of unsaved
 *  tabs drops it. */
export async function setExitSnapshot(tabs: SessionTab[], session: boolean): Promise<void> {
	return invoke('set_exit_snapshot', { tabs, session });
}

/** The unsaved tabs a run the system ended left behind (empty when none). */
export async function loadRecoveryTabs(): Promise<SessionTab[]> {
	return invoke('load_recovery_tabs');
}

/** Delete the recovery copy. */
export async function clearRecoveryTabs(): Promise<void> {
	return invoke('clear_recovery_tabs');
}

/** Tell the backend the window has written what it holds, after it said
 *  the system is ending the app (`app://os-ending`). */
export async function osEndingReady(): Promise<void> {
	return invoke('os_ending_ready');
}

/** The database file that could not be opened at startup, and why: this
 *  run keeps everything in memory only. */
export interface DatabaseFallback {
	/** Empty when not even the data folder is known. */
	path: string;
	reason: string;
	/** The file is fine, but another BridgeLab window uses it and this
	 *  launch could not hand its files over (Linux without D-Bus). */
	second_instance?: boolean;
}

/** Why this run has no database file, or null when it has one. Answers
 *  once per run. */
export async function takeDatabaseFallback(): Promise<DatabaseFallback | null> {
	return invoke('take_database_fallback');
}
