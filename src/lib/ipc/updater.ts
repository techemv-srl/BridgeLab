/** Check for available updates via Tauri's updater plugin. */
export async function checkForUpdates(): Promise<UpdateInfo | null> {
	try {
		const { check } = await import('@tauri-apps/plugin-updater');
		const update = await check();
		if (update) {
			return {
				available: true,
				version: update.version,
				currentVersion: update.currentVersion,
				notes: update.body ?? '',
				date: update.date ?? '',
				update,
			};
		}
		return null;
	} catch (e) {
		console.error('[Updater] check failed:', e);
		return null;
	}
}

export interface UpdateInfo {
	available: boolean;
	version: string;
	currentVersion: string;
	notes: string;
	date: string;
	update: any;
}

/** Install a pending update. */
export async function installUpdate(update: any): Promise<void> {
	await update.downloadAndInstall();
	const { relaunch } = await import('@tauri-apps/plugin-process');
	await relaunch();
}
