import { fileStat } from '$lib/ipc/parser';
import { dialogStore } from '$lib/stores/dialog.svelte';
import { t } from '$lib/i18n';
import { baseName, withExtension } from '$lib/paths';

/**
 * The file a save dialog's answer should be written to: `path` with its
 * extension added when the user typed a name without one. The dialog
 * only asked about replacing the name as typed, so when the name with the
 * extension already exists this asks before it is replaced; null means
 * do not save.
 */
export async function saveTarget(path: string, ext: string, alsoAccept: string[] = []): Promise<string | null> {
	const target = withExtension(path, ext, alsoAccept);
	if (target === path) return path;
	let exists = false;
	try {
		exists = (await fileStat(target)) !== null;
	} catch {
		exists = true; // cannot tell: ask rather than replace blindly
	}
	if (exists && !(await dialogStore.confirm(t('file.replaceAddedExt', { file: baseName(target) })))) return null;
	return target;
}
