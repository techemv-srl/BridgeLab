import { messageStore, type FileStamp, type MessageTab } from './messages.svelte';
import { dialogStore } from './dialog.svelte';
import { openFile, fileStat, canonicalPath } from '$lib/ipc/parser';
import { getRecentFiles, addRecentFile, removeRecentFile, clearRecentFiles, type RecentFile } from '$lib/ipc/database';
import { t } from '$lib/i18n';
import { baseName, normalizePath, samePath } from '$lib/paths';

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
					{ name: 'HL7 Messages', extensions: ['hl7', 'txt', 'msg'] },
					{ name: 'FHIR Resources', extensions: ['json', 'xml'] },
					{ name: 'All Files', extensions: ['*'] },
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
		const path = normalizePath(rawPath);
		// The same file through a symlinked folder is the same tab: two tabs
		// on one file saved over each other's edits.
		const real = await realPathOf(path);
		// Already open: show that tab, and offer the newer file if another
		// program changed it (focusing the stale text looked like a reopen).
		const existing = messageStore.findByPath(path, real);
		if (existing) {
			messageStore.setActiveTab(existing.id);
			await this.checkTab(existing);
			return;
		}
		try {
			const result = await openFile(path);
			suppressAutoParse();
			// The tab holds the full text; the editor folds long fields
			// visually. truncated_text is only a fallback for old backends.
			const id = messageStore.openMessage(result, path, result.full_text ?? result.truncated_text, real);
			messageStore.setDiskStamp(id, (await stampOf(path)) ?? undefined);
			if (result.parse_error) {
				// Open anyway, as text to fix: say why there is no tree.
				await dialogStore.warning(
					t('file.openedUnparsed', { name: baseName(path), error: result.parse_error }),
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
		if ((await stampOf(path)) !== null) {
			await dialogStore.error(t('dialog.openFailed'), undefined, `${path}\n${String(e)}`);
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

	/** Load the tab's file as it is now on disk. */
	private async reload(tabId: string): Promise<boolean> {
		const tab = messageStore.tabs.find((x) => x.id === tabId);
		if (!tab?.filePath) return false;
		try {
			const result = await openFile(tab.filePath);
			const stamp = await stampOf(tab.filePath);
			messageStore.reloadFromDisk(tabId, result.full_text ?? result.truncated_text, result, stamp ?? undefined);
			return true;
		} catch (e) {
			await dialogStore.error(t('dialog.openFailed'), undefined, `${tab.filePath}\n${String(e)}`);
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
		try {
			const result = await openFile(tab.filePath);
			const text = result.full_text ?? result.truncated_text;
			const current = messageStore.tabs.find((x) => x.id === tabId);
			// Edited while the file was being read: keep the edit.
			if (!current || current.isModified) return false;
			messageStore.reloadFromDisk(tabId, text, result, (await stampOf(tab.filePath)) ?? undefined);
			return true;
		} catch {
			return false;
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

	async saveActive(): Promise<void> {
		const activeTab = messageStore.activeTab;
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
		const activeTab = messageStore.activeTab;
		if (!activeTab) return;
		try {
			const { save } = await import('@tauri-apps/plugin-dialog');
			const path = await save({
				defaultPath: activeTab.filePath ?? activeTab.label,
				filters: [
					{ name: 'HL7 Messages', extensions: ['hl7'] },
					{ name: 'All Files', extensions: ['*'] },
				],
			});
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
