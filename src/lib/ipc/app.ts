import { invoke } from '@tauri-apps/api/core';

/** Build code of the running app, shown next to the version so two builds
 *  of the same version can be told apart: commit date and short hash, e.g.
 *  `20261117.762cc1c`. Empty outside Tauri (web/Vite mode). */
export async function getBuildId(): Promise<string> {
	try {
		return await invoke<string>('get_build_id');
	} catch {
		return '';
	}
}
