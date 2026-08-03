import { messageStore } from './messages.svelte';
import { dialogStore } from './dialog.svelte';
import { openFile } from '$lib/ipc/parser';
import { getRecentFiles, addRecentFile, clearRecentFiles, type RecentFile } from '$lib/ipc/database';
import { t } from '$lib/i18n';

/**
 * File open/save operations plus the recent-files list they maintain.
 * `suppressAutoParse` lets the caller mute the editor's next auto-parse
 * when content is set programmatically (file open sets truncated text).
 */
class FileOpsStore {
	recentFiles = $state<RecentFile[]>([]);

	async refreshRecent(): Promise<void> {
		try {
			this.recentFiles = await getRecentFiles(20);
		} catch { /* DB might not be available */ }
	}

	async openFromDialog(suppressAutoParse: () => void): Promise<void> {
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
			if (selected) {
				const path = typeof selected === 'string' ? selected : (selected as any).path ?? String(selected);
				console.log('[BridgeLab] Opening file:', path);
				const result = await openFile(path);
				console.log('[BridgeLab] Parse result:', result.message_type, result.format, result.segment_count, 'segments');
				suppressAutoParse();
				messageStore.openMessage(result, path, result.truncated_text);
				// Track recent file
				const filename = path.split('/').pop()?.split('\\').pop() ?? '';
				try {
					await addRecentFile(path, filename, result.message_type, result.version, result.file_size_bytes);
					this.recentFiles = await getRecentFiles(20);
				} catch {
					// DB might not be available
				}
			}
		} catch (e) {
			console.error('[BridgeLab] Failed to open file:', e);
			// The welcome screen has no tab to surface errors in — an invisible
			// failure there looks like "the button does nothing". Always show
			// a real error dialog.
			await dialogStore.error(t('dialog.openFailed'), undefined, String(e));
		}
	}

	async openPath(path: string, suppressAutoParse: () => void): Promise<void> {
		try {
			const result = await openFile(path);
			suppressAutoParse();
			messageStore.openMessage(result, path, result.truncated_text);
		} catch (e) {
			console.error('Failed to open file:', path, e);
			await dialogStore.error(t('dialog.openFailed'), undefined, `${path}\n${String(e)}`);
			return;
		}
		// Recent-list persistence is best-effort — a DB hiccup must not read
		// as "could not open the file" for a file that just opened fine.
		try {
			const filename = path.split('/').pop()?.split('\\').pop() ?? '';
			const result = messageStore.activeTab?.parseResult;
			await addRecentFile(
				path, filename,
				result?.message_type ?? '', result?.version ?? '',
				result?.file_size_bytes ?? 0,
			);
			this.recentFiles = await getRecentFiles(20);
		} catch {
			// DB might not be available
		}
	}

	async saveActive(): Promise<void> {
		const activeTab = messageStore.activeTab;
		if (!activeTab) return;
		// If tab has no file path (Untitled / from paste/template), fall back to Save As
		if (!activeTab.filePath) {
			await this.saveActiveAs();
			return;
		}
		try {
			const { saveFile } = await import('$lib/ipc/parser');
			await saveFile({
				path: activeTab.filePath,
				content: activeTab.content, // save current editor text, not the parsed store
			});
			messageStore.markSaved(activeTab.id);
			console.log('[BridgeLab] Saved to:', activeTab.filePath);
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
				await saveFile({ path, content: activeTab.content });
				messageStore.markSaved(activeTab.id, path);
				console.log('[BridgeLab] Saved as:', path);
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
