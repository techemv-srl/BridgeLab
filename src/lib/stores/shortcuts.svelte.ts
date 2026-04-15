import { getPreference, setPreference } from '$lib/ipc/database';

/** Category for grouping shortcuts in UI. */
export type ShortcutCategory = 'file' | 'edit' | 'view' | 'tools' | 'editor';

/** A single keyboard shortcut binding. */
export interface ShortcutDef {
	id: string;
	label: string;
	category: ShortcutCategory;
	defaultKeys: string; // e.g. "Ctrl+O", "F5"
	/** Marker: true if this action is handled by Monaco editor (not our global handler) */
	isMonaco?: boolean;
}

/** Current shortcut mapping: id -> key combination. */
export type ShortcutMap = Record<string, string>;

/** The list of all shortcuts in the app. */
export const SHORTCUTS: ShortcutDef[] = [
	// File
	{ id: 'file.open', label: 'Open File', category: 'file', defaultKeys: 'Ctrl+O' },
	{ id: 'file.save', label: 'Save', category: 'file', defaultKeys: 'Ctrl+S' },
	{ id: 'file.saveAs', label: 'Save As', category: 'file', defaultKeys: 'Ctrl+Shift+S' },
	{ id: 'file.closeTab', label: 'Close Tab', category: 'file', defaultKeys: 'Ctrl+W' },
	{ id: 'file.newFromTemplate', label: 'New from Template', category: 'file', defaultKeys: 'Ctrl+N' },
	{ id: 'file.testCases', label: 'Test Case Library', category: 'file', defaultKeys: 'Ctrl+L' },

	// Edit
	{ id: 'edit.settings', label: 'Settings', category: 'edit', defaultKeys: 'Ctrl+,' },

	// View
	{ id: 'view.toggleTree', label: 'Toggle Tree Panel', category: 'view', defaultKeys: 'Ctrl+B' },
	{ id: 'view.toggleValidation', label: 'Toggle Validation Panel', category: 'view', defaultKeys: 'Ctrl+J' },
	{ id: 'view.toggleCommunication', label: 'Toggle Communication Panel', category: 'view', defaultKeys: 'Ctrl+K' },
	{ id: 'view.toggleFhirPath', label: 'Toggle FHIRPath Panel', category: 'view', defaultKeys: 'Ctrl+P' },

	// Tools
	{ id: 'tools.reparse', label: 'Re-parse Message', category: 'tools', defaultKeys: '' },
	{ id: 'tools.validate', label: 'Validate', category: 'tools', defaultKeys: 'F6' },

	// Editor (Monaco native - informational only)
	{ id: 'editor.find', label: 'Find', category: 'editor', defaultKeys: 'Ctrl+F', isMonaco: true },
	{ id: 'editor.replace', label: 'Replace', category: 'editor', defaultKeys: 'Ctrl+H', isMonaco: true },
	{ id: 'editor.undo', label: 'Undo', category: 'editor', defaultKeys: 'Ctrl+Z', isMonaco: true },
	{ id: 'editor.redo', label: 'Redo', category: 'editor', defaultKeys: 'Ctrl+Y', isMonaco: true },
	{ id: 'editor.copy', label: 'Copy', category: 'editor', defaultKeys: 'Ctrl+C', isMonaco: true },
	{ id: 'editor.paste', label: 'Paste', category: 'editor', defaultKeys: 'Ctrl+V', isMonaco: true },
	{ id: 'editor.selectAll', label: 'Select All', category: 'editor', defaultKeys: 'Ctrl+A', isMonaco: true },
	{ id: 'editor.goToLine', label: 'Go to Line', category: 'editor', defaultKeys: 'Ctrl+G', isMonaco: true },
	{ id: 'editor.commandPalette', label: 'Command Palette', category: 'editor', defaultKeys: 'F1', isMonaco: true },
];

/** Reactive current shortcut map. Svelte 5 $state. */
class ShortcutStore {
	map = $state<ShortcutMap>(this.buildDefault());
	loaded = $state(false);

	private buildDefault(): ShortcutMap {
		const m: ShortcutMap = {};
		for (const s of SHORTCUTS) m[s.id] = s.defaultKeys;
		return m;
	}

	async loadFromPrefs(): Promise<void> {
		try {
			const saved = await getPreference('shortcuts_json');
			if (saved) {
				const parsed = JSON.parse(saved) as ShortcutMap;
				this.map = { ...this.buildDefault(), ...parsed };
			}
		} catch { /* use defaults */ }
		this.loaded = true;
	}

	async save(): Promise<void> {
		try {
			await setPreference('shortcuts_json', JSON.stringify(this.map));
		} catch { /* web mode */ }
	}

	get(id: string): string {
		return this.map[id] ?? '';
	}

	set(id: string, keys: string): void {
		this.map = { ...this.map, [id]: keys };
	}

	resetDefaults(): void {
		this.map = this.buildDefault();
	}

	/** Find a shortcut by its current key combination (returns first match, excluding Monaco). */
	findByKeys(keys: string, excludeId?: string): ShortcutDef | null {
		if (!keys) return null;
		for (const s of SHORTCUTS) {
			if (s.id === excludeId) continue;
			if (s.isMonaco) continue; // Monaco shortcuts can conflict, but we don't block them
			if (this.map[s.id] === keys) return s;
		}
		return null;
	}

	/** Find Monaco conflict (if a user-assigned shortcut matches a Monaco default). */
	findMonacoConflict(keys: string): ShortcutDef | null {
		if (!keys) return null;
		for (const s of SHORTCUTS) {
			if (!s.isMonaco) continue;
			if (this.map[s.id] === keys) return s;
		}
		return null;
	}
}

export const shortcutStore = new ShortcutStore();

/** Normalize a KeyboardEvent to a shortcut string like "Ctrl+Shift+K". */
export function eventToKeys(e: KeyboardEvent): string {
	const parts: string[] = [];
	if (e.ctrlKey || e.metaKey) parts.push('Ctrl');
	if (e.shiftKey) parts.push('Shift');
	if (e.altKey) parts.push('Alt');

	let key = e.key;
	// Handle F1-F12
	if (/^F\d+$/.test(key)) {
		parts.push(key);
	} else if (key === ' ') {
		parts.push('Space');
	} else if (key.length === 1) {
		parts.push(key.toUpperCase());
	} else {
		// Ignore bare modifier key events (user hasn't pressed a "real" key yet)
		if (['Control', 'Shift', 'Alt', 'Meta'].includes(key)) return '';
		parts.push(key);
	}

	return parts.join('+');
}

/** Check if a KeyboardEvent matches a shortcut string like "Ctrl+O". */
export function matchesKeys(e: KeyboardEvent, keys: string): boolean {
	if (!keys) return false;
	const parts = keys.split('+').map(p => p.trim());

	const wantCtrl = parts.includes('Ctrl');
	const wantShift = parts.includes('Shift');
	const wantAlt = parts.includes('Alt');

	// Main key is the last non-modifier part
	const key = parts.filter(p => !['Ctrl', 'Shift', 'Alt', 'Meta'].includes(p)).pop() ?? '';

	const hasCtrl = e.ctrlKey || e.metaKey;
	if (wantCtrl !== hasCtrl) return false;
	if (wantShift !== e.shiftKey) return false;
	if (wantAlt !== e.altKey) return false;

	// Normalize key for comparison
	if (key === 'Space') return e.key === ' ';
	if (/^F\d+$/.test(key)) return e.key === key;
	return e.key.toLowerCase() === key.toLowerCase();
}
