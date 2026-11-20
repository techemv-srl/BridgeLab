import { messageStore, type FileStamp, type MessageTab } from './messages.svelte';
import { dialogStore } from './dialog.svelte';
import { openFile, fileStat, canonicalPath } from '$lib/ipc/parser';
import { getRecentFiles, addRecentFile, removeRecentFile, clearRecentFiles, type RecentFile } from '$lib/ipc/database';
import { t, getLocale } from '$lib/i18n';
import type { ParseResult } from '$lib/types/hl7';
import { unparsedMessage } from '$lib/unparsed';
import { baseName, normalizePath, samePath } from '$lib/paths';
import { hasExtension, saveAsDialog } from '$lib/save-format';
import { saveTarget } from '$lib/save-target';

/** Same file version: modification time and size both unchanged. */
export function sameStamp(a: FileStamp | null | undefined, b: FileStamp | null | undefined): boolean {
	if (a === undefined || b === undefined) return a === b;
	if (a === null || b === null) return a === b;
	return a.modified_ms === b.modified_ms && a.size === b.size;
}

/** The file's stamp; null when it is gone, undefined when that cannot be
 *  told (no backend, no permission): never report a change on a guess. */
async function stampOf(path: string): Promise<FileStamp | null | undefined> {
	try {
		return await fileStat(path);
	} catch {
		return undefined;
	}
}

/** The file with symlinks resolved; null when not known (no backend, gone). */
async function realPathOf(path: string): Promise<string | null> {
	try {
		return await canonicalPath(path);
	} catch {
		return null;
	}
}

/** Stands for "the file existed" when its real stamp is not known (a
 *  session tab whose file is gone): Save then asks before recreating it. */
const EXISTED: FileStamp = { modified_ms: -1, size: -1 };

/** Why open_file would not read a file: `confirm`, larger than what opens
 *  without asking (it opens once the user agreed); `too_large`, larger
 *  than anything that opens; `not_regular`, a device, pipe or socket. */
export interface OpenRefusal {
	kind: 'confirm' | 'too_large' | 'not_regular';
	size: number;
	limit: number;
}

/** The refusal in open_file's error, or null for any other error. The
 *  sizes come from its text (`<size> bytes, limit <limit> bytes`), so the
 *  limits are set in one place, in the backend (fileio.rs). */
export function openRefusal(e: unknown): OpenRefusal | null {
	const text = String(e);
	if (text.includes('not a regular file: ')) return { kind: 'not_regular', size: 0, limit: 0 };
	const m = /(too large to open|large file, open only if confirmed): (\d+) bytes, limit (\d+) bytes/.exec(text);
	if (!m) return null;
	return { kind: m[1] === 'too large to open' ? 'too_large' : 'confirm', size: Number(m[2]), limit: Number(m[3]) };
}

/** Bytes as megabytes, written the way the user's language writes numbers. */
function megabytes(bytes: number): string {
	return new Intl.NumberFormat(getLocale(), { maximumFractionDigits: 1 }).format(bytes / (1024 * 1024));
}

/** True for a URL given where a file path belongs (a link dropped on the
 *  window). The scheme has two letters or more: "C:/x" is a Windows path,
 *  and the paths of dropped files are absolute. */
export function isWebLink(path: string): boolean {
	return /^[a-z][a-z0-9+.-]+:\//i.test(path);
}

/** What to tell the user about a file open_file refused. */
export function refusalMessage(path: string, r: OpenRefusal): string {
	if (r.kind === 'not_regular') return t('file.notRegular', { path });
	return t('file.tooLarge', { name: baseName(path), size: megabytes(r.size), limit: megabytes(r.limit) });
}

/**
 * File open/save operations plus the recent-files list they maintain.
 * `suppressAutoParse` lets the caller mute the editor's next auto-parse
 * when content is set programmatically (file open sets the text).
 */
class FileOpsStore {
	recentFiles = $state<RecentFile[]>([]);
	private checking = false;

	async refreshRecent(): Promise<void> {
		try {
			this.recentFiles = await getRecentFiles(20);
		} catch { /* DB might not be available */ }
	}

	async openFromDialog(suppressAutoParse: () => void): Promise<void> {
		let path: string;
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const selected = await open({
				multiple: false,
				filters: [
					{ name: t('filter.hl7'), extensions: ['hl7', 'txt', 'msg'] },
					{ name: t('filter.fhir'), extensions: ['json', 'xml'] },
					{ name: t('filter.all'), extensions: ['*'] },
				],
			});
			if (!selected) return;
			path = typeof selected === 'string' ? selected : (selected as any).path ?? String(selected);
		} catch (e) {
			// The welcome screen has no tab to surface errors in — an invisible
			// failure there looks like "the button does nothing". Always show
			// a real error dialog.
			await dialogStore.error(t('dialog.openFailed'), undefined, String(e));
			return;
		}
		await this.openPath(path, suppressAutoParse);
	}

	async openPath(rawPath: string, suppressAutoParse: () => void): Promise<void> {
		// A link dragged from a browser arrives as a "path": it was read as
		// a file name and reported as "File not found: https:/…".
		if (isWebLink(rawPath)) {
			await dialogStore.error(t('file.droppedLink', { url: rawPath }));
			return;
		}
		const path = normalizePath(rawPath);
		// The same file through a symlinked folder is the same tab: two tabs
		// on one file saved over each other's edits.
		const real = await realPathOf(path);
		// Already open: show that tab, and offer the newer file if another
		// program changed it (focusing the stale text looked like a reopen).
		const existing = messageStore.findByPath(path, real);
		if (existing) {
			messageStore.setActiveTab(existing.id);
			// Restored without its text (too large, or not loaded when asked):
			// opening the file again is how it gets loaded.
			if (existing.notLoaded) {
				await this.reload(existing.id);
				return;
			}
			await this.checkTab(existing);
			return;
		}
		try {
			const result = await this.readFile(path);
			if (!result) return;
			suppressAutoParse();
			// The tab holds the full text; the editor folds long fields
			// visually. truncated_text is only a fallback for old backends.
			const text = result.full_text ?? result.truncated_text;
			const id = messageStore.openMessage(result, path, text, real);
			messageStore.setDiskStamp(id, (await stampOf(path)) ?? undefined);
			if (result.parse_error) {
				// Open anyway, as text to fix: say why there is no tree.
				await dialogStore.warning(
					unparsedMessage(baseName(path), text, result.parse_error, result.parse_error_kind),
					t('file.notParsedTitle'),
				);
			}
			const cw = result.charset_warning;
			if (cw) {
				await dialogStore.warning(
					t(cw.kind === 'unsupported' ? 'file.charsetUnsupported' : 'file.charsetNotUtf8', {
						name: baseName(path),
						charset: cw.declared,
					}),
					t('file.charsetTitle'),
				);
			}
		} catch (e) {
			console.error('Failed to open file:', path, e);
			await this.reportOpenFailure(path, e);
			return;
		}
		// Recent-list persistence is best-effort — a DB hiccup must not read
		// as "could not open the file" for a file that just opened fine.
		try {
			const result = messageStore.activeTab?.parseResult;
			await addRecentFile(
				path, baseName(path),
				result?.message_type ?? '', result?.version ?? '',
				result?.file_size_bytes ?? 0,
			);
			this.recentFiles = await getRecentFiles(20);
		} catch {
			// DB might not be available
		}
	}

	/** A file that is not there gets a plain message (not the OS error
	 *  text), and a stale Recent Files entry can be dropped. */
	private async reportOpenFailure(path: string, e: unknown): Promise<void> {
		// A folder (dropped on the window, say) exists but is not a file.
		if (String(e).includes('not a file but a folder: ')) {
			await dialogStore.error(t('file.isFolder', { path }));
			return;
		}
		const refusal = openRefusal(e);
		if (refusal) {
			await dialogStore.error(refusalMessage(path, refusal));
			return;
		}
		if ((await stampOf(path)) !== null) {
			await dialogStore.error(t('dialog.openFailed'), undefined, `${path}\n${String(e)}`);
			return;
		}
		// A name that is not valid UTF-8 (a Latin-1 file name on Linux)
		// reaches the frontend with U+FFFD for the bytes it could not show,
		// so no file has that name: the file is there, but cannot be opened
		// by name. "File not found" would send the user looking for it.
		if (path.includes('�')) {
			await dialogStore.error(t('file.nameNotUtf8', { path }));
			return;
		}
		const recent = this.recentFiles.find((f) => samePath(f.path, path));
		if (!recent) {
			await dialogStore.error(t('file.notFound', { path }));
			return;
		}
		if (await dialogStore.confirm(t('file.notFoundRemoveRecent', { path }), t('file.notFoundTitle'))) {
			try {
				await removeRecentFile(recent.path);
				this.recentFiles = await getRecentFiles(20);
			} catch { /* DB might not be available */ }
		}
	}

	/**
	 * Tell the user when a tab's file changed on disk since it was opened or
	 * saved, once per change, and offer to load the new version. Declining
	 * keeps the tab's text; Save then asks before overwriting.
	 */
	async checkTab(tab: MessageTab): Promise<void> {
		if (!tab.filePath || tab.diskStamp === undefined) return;
		const now = await stampOf(tab.filePath);
		if (now === undefined || sameStamp(now, tab.diskStamp)) return;
		if (tab.externalNotice !== undefined && sameStamp(now, tab.externalNotice)) return;
		tab.externalNotice = now;
		if (now === null) {
			await dialogStore.warning(t('file.deletedOnDisk', { name: tab.label }), t('file.changedTitle'));
			return;
		}
		const ask = tab.isModified ? 'file.changedReloadDiscard' : 'file.changedReload';
		if (await dialogStore.confirm(t(ask, { name: tab.label }), t('file.changedTitle'))) {
			await this.reload(tab.id);
		}
	}

	/** checkTab for every file tab (the window got the focus back). */
	async checkAllTabs(): Promise<void> {
		if (this.checking) return;
		this.checking = true;
		try {
			for (const tab of [...messageStore.tabs]) {
				if (messageStore.tabs.includes(tab)) await this.checkTab(tab);
			}
		} finally {
			this.checking = false;
		}
	}

	/**
	 * open_file, asking first when the file is large: such a file takes the
	 * app to several GB of memory. Null when the user would rather not.
	 * `restored`: the file was open in the last session, and the question
	 * says so (it comes at start, unasked for).
	 */
	private async readFile(path: string, restored = false): Promise<ParseResult | null> {
		try {
			return await openFile(path);
		} catch (e) {
			const refusal = openRefusal(e);
			if (refusal?.kind !== 'confirm') throw e;
			const sizes = { name: baseName(path), size: megabytes(refusal.size), limit: megabytes(refusal.limit) };
			const question = t(restored ? 'file.largeRestore' : 'file.largeConfirm', sizes);
			if (!(await dialogStore.confirm(question, t('file.largeTitle')))) return null;
			return openFile(path, true);
		}
	}

	/** Load the tab's file as it is now on disk. */
	private async reload(tabId: string): Promise<boolean> {
		const tab = messageStore.tabs.find((x) => x.id === tabId);
		if (!tab?.filePath) return false;
		try {
			const result = await this.readFile(tab.filePath);
			if (!result) return false;
			const stamp = await stampOf(tab.filePath);
			messageStore.reloadFromDisk(tabId, result.full_text ?? result.truncated_text, result, stamp ?? undefined);
			return true;
		} catch (e) {
			const refusal = openRefusal(e);
			if (refusal) await dialogStore.error(refusalMessage(tab.filePath, refusal));
			else await dialogStore.error(t('dialog.openFailed'), undefined, `${tab.filePath}\n${String(e)}`);
			return false;
		}
	}

	/**
	 * A tab restored from the session: an unedited one shows its file as
	 * it is now (another program may have changed it while the app was
	 * closed); an edited one keeps its text. Returns false when the caller
	 * still has to parse the stored text. Missing files are collected in
	 * `missing`, to be reported together.
	 */
	/** For a restored tab that is not loaded yet: only whether its file is
	 *  still there, so every missing file is reported at launch. */
	async checkRestoredFile(tabId: string, missing: string[]): Promise<void> {
		const tab = messageStore.tabs.find((x) => x.id === tabId);
		if (!tab?.filePath) return;
		if ((await stampOf(tab.filePath)) === null) {
			missing.push(tab.filePath);
			tab.diskStamp = EXISTED;
			tab.externalNotice = null;
		}
	}

	async adoptRestoredTab(tabId: string, missing: string[]): Promise<boolean> {
		const tab = messageStore.tabs.find((x) => x.id === tabId);
		if (!tab?.filePath) return false;
		tab.realPath = await realPathOf(tab.filePath);
		const now = await stampOf(tab.filePath);
		if (now === undefined) return false;
		if (now === null) {
			missing.push(tab.filePath);
			tab.diskStamp = EXISTED;
			tab.externalNotice = null; // reported with the others
			return false;
		}
		if (tab.isModified) {
			// The file's state when these edits were made is unknown: it may
			// have changed while the app was closed. Watch it from now on,
			// and have the first save ask.
			tab.diskStamp = now;
			tab.diskUnverified = true;
			return false;
		}
		const path = tab.filePath;
		try {
			const result = await this.readFile(path, true);
			const current = messageStore.tabs.find((x) => x.id === tabId);
			// Edited while the file was being read: keep the edit.
			if (!current || current.isModified) return false;
			if (!result) {
				// Not now: the tab stays empty until the file is opened again.
				messageStore.leaveUnloaded(tabId);
				return true;
			}
			const text = result.full_text ?? result.truncated_text;
			messageStore.reloadFromDisk(tabId, text, result, (await stampOf(path)) ?? undefined);
			return true;
		} catch (e) {
			const refusal = openRefusal(e);
			if (!refusal) return false;
			// Grown past the limit, or replaced by a device or pipe, since the
			// last session: restored without its text, so it is not read (and
			// the app not taken down) at every start, and the old text is not
			// shown, or saved, as the file's.
			messageStore.leaveUnloaded(tabId);
			await dialogStore.warning(t('file.restoredEmpty', { reason: refusalMessage(path, refusal) }));
			return true;
		}
	}

	/** Before writing over the tab's file: when another program changed or
	 *  deleted it since it was opened or saved, ask. True = go ahead. */
	private async confirmOverwrite(tab: MessageTab, path: string): Promise<boolean> {
		if (tab.diskUnverified && path === tab.filePath) {
			return dialogStore.confirm(t('file.saveRestored', { name: tab.label }), t('file.changedTitle'));
		}
		if (tab.diskStamp === undefined) return true;
		const now = await stampOf(path);
		if (now === undefined || sameStamp(now, tab.diskStamp)) return true;
		const msg = now === null ? 'file.saveDeleted' : 'file.saveNewer';
		return dialogStore.confirm(t(msg, { name: tab.label }), t('file.changedTitle'));
	}

	/** The active tab once its text is known: a restored tab whose file is
	 *  still being read is waited for. Null (after saying why) when the
	 *  text could not be read, so the empty placeholder is never saved. */
	private async loadedActiveTab(): Promise<MessageTab | null> {
		const tab = messageStore.activeTab;
		if (!tab) return null;
		if (!tab.contentPending) return tab;
		await messageStore.whenLoaded(tab.id);
		const now = messageStore.tabs.find((x) => x.id === tab.id);
		if (!now) return null;
		if (now.contentPending) {
			await dialogStore.warning(t('file.notLoaded', { name: now.label }));
			return null;
		}
		return now;
	}

	async saveActive(): Promise<void> {
		const activeTab = await this.loadedActiveTab();
		if (!activeTab) return;
		// If tab has no file path (Untitled / from paste/template), fall back to Save As
		if (!activeTab.filePath) {
			await this.saveActiveAs();
			return;
		}
		const path = activeTab.filePath;
		if (!(await this.confirmOverwrite(activeTab, path))) return;
		try {
			const { saveFile } = await import('$lib/ipc/parser');
			await saveFile({
				path,
				content: activeTab.content, // save current editor text, not the parsed store
				charset: activeTab.charset,
			});
			messageStore.markSaved(activeTab.id);
			messageStore.setDiskStamp(activeTab.id, (await stampOf(path)) ?? undefined);
		} catch (e) {
			console.error('Save failed:', e);
			await dialogStore.error(t('dialog.saveFailed'), undefined, String(e));
		}
	}

	async saveActiveAs(): Promise<void> {
		const activeTab = await this.loadedActiveTab();
		if (!activeTab) return;
		try {
			const { save } = await import('@tauri-apps/plugin-dialog');
			// The filter and the default name follow the document: a FHIR
			// resource offered only "HL7 Messages" became patient.hl7 on
			// Windows, and a file with no extension on Linux.
			const { format, defaultPath, filters } = saveAsDialog(activeTab);
			const picked = await save({ defaultPath, filters });
			// The Linux (GTK) dialog adds no extension to a name typed
			// without one. The backend writes the file, so the name with
			// the extension needs no permission; it only asks before
			// replacing a file of that name.
			// A file that is already there under that exact name (the tab's
			// own extensionless file, or another one picked in the dialog,
			// which asked before replacing it) is written as it is: adding
			// the extension would save a different file than the one chosen.
			const chosenExisting = !!picked && (
				(!!activeTab.filePath && samePath(picked, activeTab.filePath)) ||
				!!(await stampOf(picked))
			);
			const path = !picked || hasExtension(picked) || chosenExisting
				? picked
				: await saveTarget(picked, format.ext, { backendWrites: true });
			if (path) {
				const { saveFile } = await import('$lib/ipc/parser');
				await saveFile({ path, content: activeTab.content, charset: activeTab.charset });
				messageStore.markSaved(activeTab.id, path);
				activeTab.realPath = await realPathOf(path);
				messageStore.setDiskStamp(activeTab.id, (await stampOf(path)) ?? undefined);
			}
		} catch (e) {
			console.error('Save As failed:', e);
			await dialogStore.error(t('dialog.saveAsFailed'), undefined, String(e));
		}
	}

	async clearRecent(): Promise<void> {
		try {
			await clearRecentFiles();
			this.recentFiles = [];
		} catch {
			// ignore in web mode
		}
	}
}

export const fileOpsStore = new FileOpsStore();
