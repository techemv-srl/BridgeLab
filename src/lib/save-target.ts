import { allowSaveWithExtension, fileStat } from '$lib/ipc/parser';
import { dialogStore } from '$lib/stores/dialog.svelte';
import { t } from '$lib/i18n';
import { baseName, withExtension } from '$lib/paths';

export interface SaveTargetOptions {
	/** Other endings that count as the right extension (`.json` for a pack). */
	alsoAccept?: string[];
	/** The backend writes the file (the test-case pack): the frontend needs
	 *  no permission to write it. */
	backendWrites?: boolean;
}

/**
 * The file a save dialog's answer should be written to: `path` with its
 * extension added when the user typed a name without one. The dialog
 * only asked about replacing the name as typed, so when the name with the
 * extension already exists this asks before it is replaced; null means
 * do not save. Rejects when the frontend cannot be allowed to write the
 * name with the extension.
 */
export async function saveTarget(path: string, ext: string, opts: SaveTargetOptions = {}): Promise<string | null> {
	const target = withExtension(path, ext, opts.alsoAccept ?? []);
	if (target === path) return path;
	let exists = false;
	try {
		exists = (await fileStat(target)) !== null;
	} catch {
		exists = true; // cannot tell: ask rather than replace blindly
	}
	if (exists && !(await dialogStore.confirm(t('file.replaceAddedExt', { file: baseName(target) })))) return null;
	if (opts.backendWrites) return target;
	// The dialog lets the frontend write only the exact path it returned,
	// so the file system plugin refused the name with the extension
	// ("forbidden path") and nothing was saved. The backend allows that
	// one file, and returns it.
	return await allowSaveWithExtension(path, ext.replace(/^\./, ''));
}
