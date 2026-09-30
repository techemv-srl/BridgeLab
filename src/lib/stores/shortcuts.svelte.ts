import { getPreference, setPreference } from '$lib/ipc/database';

/** Category for grouping shortcuts in UI. */
export type ShortcutCategory = 'file' | 'edit' | 'view' | 'tools' | 'editor';

/** A single keyboard shortcut binding. */
export interface ShortcutDef {
	id: string;
	label: string;
	category: ShortcutCategory;
	defaultKeys: string; // e.g. "Ctrl+O", "F5"
	/** Handled by the Monaco editor itself: listed for reference, not
	 *  rebindable (the editor keeps its own keys). */
	isMonaco?: boolean;
	/** The editor's key on macOS when it is not simply Ctrl → ⌘ ("Control"
	 *  is the real Control key, ⌃). */
	macKeys?: string;
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
	{ id: 'view.toggleSegmentGrid', label: 'Toggle Segment Grid', category: 'view', defaultKeys: 'Ctrl+Shift+G' },

	// Tools
	{ id: 'tools.reparse', label: 'Re-parse Message', category: 'tools', defaultKeys: '' },
	{ id: 'tools.validate', label: 'Validate', category: 'tools', defaultKeys: 'F6' },

	// Editor (Monaco native - informational only)
	{ id: 'editor.find', label: 'Find', category: 'editor', defaultKeys: 'Ctrl+F', isMonaco: true },
	{ id: 'editor.replace', label: 'Replace', category: 'editor', defaultKeys: 'Ctrl+H', macKeys: 'Ctrl+Alt+F', isMonaco: true },
	{ id: 'editor.undo', label: 'Undo', category: 'editor', defaultKeys: 'Ctrl+Z', isMonaco: true },
	{ id: 'editor.redo', label: 'Redo', category: 'editor', defaultKeys: 'Ctrl+Y', macKeys: 'Ctrl+Shift+Z', isMonaco: true },
	{ id: 'editor.copy', label: 'Copy', category: 'editor', defaultKeys: 'Ctrl+C', isMonaco: true },
	{ id: 'editor.paste', label: 'Paste', category: 'editor', defaultKeys: 'Ctrl+V', isMonaco: true },
	{ id: 'editor.selectAll', label: 'Select All', category: 'editor', defaultKeys: 'Ctrl+A', isMonaco: true },
	{ id: 'editor.goToLine', label: 'Go to Line', category: 'editor', defaultKeys: 'Ctrl+G', macKeys: 'Control+G', isMonaco: true },
	{ id: 'editor.commandPalette', label: 'Command Palette', category: 'editor', defaultKeys: 'Ctrl+Shift+P', isMonaco: true },
];

/** Global flag: true while the ShortcutsEditor is capturing a key combo.
 *  The app-level keydown handler must stand down during capture, or pressing
 *  Ctrl+O to *assign* it also opens the file picker. */
export const shortcutCapture = $state({ active: false });

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
				const map = { ...this.buildDefault() };
				// Keep only app bindings that are still allowed: a bare key
				// saved by an older version would swallow typing.
				for (const s of SHORTCUTS) {
					const keys = parsed[s.id];
					if (!s.isMonaco && typeof keys === 'string' && (keys === '' || bindingProblem(keys) === null)) map[s.id] = keys;
				}
				this.map = map;
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
			if (monacoKeys(s) === keys) return s;
		}
		return null;
	}
}

export const shortcutStore = new ShortcutStore();

/** Keys the app keeps for itself: F1 always opens the manual. */
const RESERVED_KEYS = ['F1'];

/**
 * Why `keys` cannot be an app shortcut, or null when it can: a reserved
 * key, or a key that types text (a letter, digit, Space, Shift+letter...),
 * which would fire while typing in the editor. Function keys F2-F12 and
 * anything with Ctrl or Alt are fine.
 */
export function bindingProblem(keys: string): 'reserved' | 'needsModifier' | null {
	if (!keys) return null;
	if (RESERVED_KEYS.includes(keys)) return 'reserved';
	const parts = keys.split('+');
	if (parts.includes('Ctrl') || parts.includes('Alt')) return null;
	const key = parts[parts.length - 1];
	if (/^F([2-9]|1[0-2])$/.test(key)) return null;
	return 'needsModifier';
}

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

/** True on macOS, where Ctrl in a binding is the Command key. */
export function isMacPlatform(): boolean {
	if (typeof navigator === 'undefined') return false;
	return /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
}

/**
 * A binding as the user's keyboard labels it: "Ctrl+Shift+K" on Windows
 * and Linux, "⌘⇧K" on macOS (bindings are stored with "Ctrl", which
 * matches Command there, see matchesKeys).
 */
export function displayKeys(keys: string, mac = isMacPlatform()): string {
	if (!keys || !mac) return keys;
	const symbols: Record<string, string> = { Ctrl: '⌘', Shift: '⇧', Alt: '⌥', Meta: '⌘', Control: '⌃' };
	return keys.split('+').map((p) => symbols[p] ?? p).join('');
}

/** The key the editor (Monaco) uses for one of its own commands on this
 *  platform: on macOS Replace is ⌘⌥F (⌘H hides the app), Redo ⌘⇧Z and Go
 *  to Line ⌃G (⌘G is Find Next). */
export function monacoKeys(s: ShortcutDef, mac = isMacPlatform()): string {
	return mac && s.macKeys ? s.macKeys : s.defaultKeys;
}
