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
	{ id: 'view.toggleInterpretation', label: 'Toggle Interpretation', category: 'view', defaultKeys: 'Ctrl+Shift+E' },

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
	// More of the editor's own keys, so that giving one of them to an app
	// action warns (the manual promises it for Ctrl+D, say).
	{ id: 'editor.cut', label: 'Cut', category: 'editor', defaultKeys: 'Ctrl+X', isMonaco: true },
	{ id: 'editor.addNextMatch', label: 'Add Selection to Next Find Match', category: 'editor', defaultKeys: 'Ctrl+D', isMonaco: true },
	{ id: 'editor.selectAllMatches', label: 'Select All Occurrences of Find Match', category: 'editor', defaultKeys: 'Ctrl+Shift+L', isMonaco: true },
	{ id: 'editor.findNext', label: 'Find Next', category: 'editor', defaultKeys: 'F3', macKeys: 'Ctrl+G', isMonaco: true },
	{ id: 'editor.toggleComment', label: 'Toggle Line Comment', category: 'editor', defaultKeys: 'Ctrl+/', isMonaco: true },
	{ id: 'editor.deleteLine', label: 'Delete Line', category: 'editor', defaultKeys: 'Ctrl+Shift+K', isMonaco: true },
	{ id: 'editor.insertLineBelow', label: 'Insert Line Below', category: 'editor', defaultKeys: 'Ctrl+Enter', isMonaco: true },
	{ id: 'editor.insertLineAbove', label: 'Insert Line Above', category: 'editor', defaultKeys: 'Ctrl+Shift+Enter', isMonaco: true },
	{ id: 'editor.moveLineUp', label: 'Move Line Up', category: 'editor', defaultKeys: 'Alt+ArrowUp', isMonaco: true },
	{ id: 'editor.moveLineDown', label: 'Move Line Down', category: 'editor', defaultKeys: 'Alt+ArrowDown', isMonaco: true },
	{ id: 'editor.indent', label: 'Indent Line', category: 'editor', defaultKeys: 'Ctrl+]', isMonaco: true },
	{ id: 'editor.outdent', label: 'Outdent Line', category: 'editor', defaultKeys: 'Ctrl+[', isMonaco: true },
	{ id: 'editor.triggerSuggest', label: 'Trigger Suggestions', category: 'editor', defaultKeys: 'Ctrl+Space', macKeys: 'Control+Space', isMonaco: true },
	// BridgeLab's own editor actions (also in the editor's context menu),
	// fixed like the editor's keys. On macOS Option with a letter types a
	// character (Option+C is ç, Option+T is †), so there they take Command
	// as well: ⌘⌥T and ⌘⌥C.
	{ id: 'editor.showInTree', label: 'Show Segment in Tree', category: 'editor', defaultKeys: 'Alt+T', macKeys: 'Ctrl+Alt+T', isMonaco: true },
	{ id: 'editor.copySegment', label: 'Copy Segment', category: 'editor', defaultKeys: 'Alt+C', macKeys: 'Ctrl+Alt+C', isMonaco: true },
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
				const chosen = new Set<string>();
				// Keep only app bindings that are still allowed: a bare key
				// saved by an older version would swallow typing.
				for (const s of SHORTCUTS) {
					const keys = parsed[s.id];
					if (!s.isMonaco && typeof keys === 'string' && (keys === '' || bindingProblem(keys) === null)) {
						map[s.id] = keys;
						chosen.add(s.id);
					}
				}
				this.map = withoutDuplicates(map, chosen);
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

	/**
	 * Bind `keys` to `id` and take them from any other action that has
	 * them. One key must never be on two actions: the app runs only the
	 * first match, so the other would show a key that does nothing.
	 * Returns the actions the keys were taken from.
	 */
	assign(id: string, keys: string): ShortcutDef[] {
		const taken = keys
			? SHORTCUTS.filter((s) => s.id !== id && !s.isMonaco && this.map[s.id] === keys)
			: [];
		const map = { ...this.map, [id]: keys };
		for (const s of taken) map[s.id] = '';
		this.map = map;
		return taken;
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

/**
 * `map` with each key combination left on one app action only. Saved
 * settings can hold the same key twice: the per-entry reset of 2.1.0
 * skipped the conflict check, and a default added by a newer version can
 * be a key the user had already given to another action. The key stays
 * with the first action, in list order, whose binding is in `chosen`
 * (the ones the user saved), otherwise with the first action of all,
 * which is the one the key ran until now. The others are left unbound.
 */
export function withoutDuplicates(map: ShortcutMap, chosen: ReadonlySet<string> = new Set()): ShortcutMap {
	const holders = new Map<string, string[]>();
	for (const s of SHORTCUTS) {
		const keys = map[s.id];
		if (s.isMonaco || !keys) continue;
		holders.set(keys, [...(holders.get(keys) ?? []), s.id]);
	}
	const out = { ...map };
	for (const ids of holders.values()) {
		if (ids.length < 2) continue;
		const keep = ids.find((id) => chosen.has(id)) ?? ids[0];
		for (const id of ids) if (id !== keep) out[id] = '';
	}
	return out;
}

/** Keys the app keeps for itself: F1 always opens the manual. */
const RESERVED_KEYS = ['F1'];

/** Modifier names a binding can hold. "Ctrl" is the platform's main
 *  modifier: the Control key on Windows and Linux, Command on macOS.
 *  "Control" is the real Control key on macOS (⌃), "Meta" the Windows or
 *  Super key on Windows and Linux. They are told apart: the Super key used
 *  to act as Ctrl (Super+B hid the tree), and on macOS Control+K, a text
 *  editing key there, opened the Communication panel. */
const MODIFIERS = ['Ctrl', 'Control', 'Meta', 'Shift', 'Alt'];

/** True when `key` (the last part of a binding) types a character. */
function typesText(key: string): boolean {
	return key.length === 1 || key === 'Space';
}

/**
 * Why `keys` cannot be an app shortcut, or null when it can: a reserved
 * key, or a key that types text (a letter, digit, Space, Shift+letter...),
 * which would fire while typing in the editor. Function keys F2-F12 and
 * anything with Ctrl (or Control, Meta) are fine, and so is Alt, except on
 * macOS, where Option with a letter types a character (Option+C is ç).
 */
export function bindingProblem(keys: string, mac = isMacPlatform()): 'reserved' | 'needsModifier' | null {
	if (!keys) return null;
	if (RESERVED_KEYS.includes(keys)) return 'reserved';
	const parts = keys.split('+');
	const key = parts[parts.length - 1];
	if (parts.includes('Ctrl') || parts.includes('Control') || parts.includes('Meta')) return null;
	if (parts.includes('Alt') && !(mac && typesText(key))) return null;
	if (/^F([2-9]|1[0-2])$/.test(key)) return null;
	return 'needsModifier';
}

/**
 * The key of a key press as a binding names it. Mostly `e.key`; but when
 * the layout makes the key type a character outside ASCII (Option+T is †
 * on macOS, a Cyrillic layout types щ for O) or a dead key, the key's
 * place on the keyboard: T, O. The editor matches its own keys by place
 * too. A layout that only moves ASCII letters (AZERTY) keeps `e.key`, so
 * Ctrl+A there is still the key marked A.
 */
function mainKey(e: KeyboardEvent): string {
	const key = e.key;
	const nonAscii = key.length === 1 && key.charCodeAt(0) > 0x7e;
	if ((nonAscii || key === 'Dead') && e.code) {
		const m = /^(?:Key([A-Z])|Digit([0-9]))$/.exec(e.code);
		if (m) return m[1] ?? m[2];
	}
	return key;
}

/** Normalize a KeyboardEvent to a shortcut string like "Ctrl+Shift+K". */
export function eventToKeys(e: KeyboardEvent, mac = isMacPlatform()): string {
	const parts: string[] = [];
	if (mac ? e.metaKey : e.ctrlKey) parts.push('Ctrl');
	if (mac && e.ctrlKey) parts.push('Control');
	if (!mac && e.metaKey) parts.push('Meta');
	if (e.shiftKey) parts.push('Shift');
	if (e.altKey) parts.push('Alt');

	const key = mainKey(e);
	// Handle F1-F12
	if (/^F\d+$/.test(key)) {
		parts.push(key);
	} else if (key === ' ') {
		parts.push('Space');
	} else if (key.length === 1) {
		parts.push(key.toUpperCase());
	} else {
		// Ignore bare modifier key events (user hasn't pressed a "real" key
		// yet), and a dead key that is not on a letter or digit.
		if (['Control', 'Shift', 'Alt', 'Meta', 'AltGraph', 'OS', 'Dead', 'Unidentified'].includes(key)) return '';
		parts.push(key);
	}

	return parts.join('+');
}

/** Check if a KeyboardEvent matches a shortcut string like "Ctrl+O". */
export function matchesKeys(e: KeyboardEvent, keys: string, mac = isMacPlatform()): boolean {
	if (!keys) return false;
	const parts = keys.split('+').map(p => p.trim());

	// "Ctrl" is Command on macOS, where "Meta" is Command too (⌘) and
	// "Control" the Control key. Elsewhere "Control" is Ctrl.
	const wantMain = parts.includes('Ctrl') || parts.includes(mac ? 'Meta' : 'Control');
	const wantOther = parts.includes(mac ? 'Control' : 'Meta');
	const wantShift = parts.includes('Shift');
	const wantAlt = parts.includes('Alt');

	// Main key is the last non-modifier part
	const key = parts.filter(p => !MODIFIERS.includes(p)).pop() ?? '';

	if (wantMain !== (mac ? e.metaKey : e.ctrlKey)) return false;
	if (wantOther !== (mac ? e.ctrlKey : e.metaKey)) return false;
	if (wantShift !== e.shiftKey) return false;
	if (wantAlt !== e.altKey) return false;

	// Normalize key for comparison
	const pressed = mainKey(e);
	if (key === 'Space') return pressed === ' ';
	if (/^F\d+$/.test(key)) return pressed === key;
	return pressed.toLowerCase() === key.toLowerCase();
}

/**
 * Run the app action bound to the key pressed. A key that is an app
 * binding is always consumed (default prevented, propagation stopped),
 * but while a dialog is open (`modalOpen`) its action does not run: it
 * would act on the window behind. Consumed even then, or ⌘W reached the
 * macOS menu's Close Window and Ctrl+W/O/S a browser's own. A key that is
 * no app binding (typing in the dialog's fields) is left alone. Returns
 * the action's id, or null.
 */
export function dispatchShortcut(
	e: KeyboardEvent,
	actions: Record<string, () => void>,
	modalOpen: boolean,
	keysOf: (id: string) => string = (id) => shortcutStore.get(id),
	mac = isMacPlatform(),
): string | null {
	for (const [id, action] of Object.entries(actions)) {
		const keys = keysOf(id);
		if (!keys || !matchesKeys(e, keys, mac)) continue;
		e.preventDefault();
		e.stopPropagation();
		if (!modalOpen) action();
		return id;
	}
	return null;
}

/**
 * Whether a key press the page did not use would make the webview act on
 * the page itself, and must be cancelled. On Windows, WebView2 reloads on
 * Ctrl+R and F5 (losing panels, and every tab when session restore is
 * off), prints on Ctrl+P, and has find, history, downloads and caret
 * browsing keys. Only the Control key counts, not the Windows key. Never
 * on macOS: WKWebView has none of these keys, and a cancelled Command key
 * never reaches the app menu there (⌘H hides the app, ⌃⌘F is full
 * screen), nor Control+H a text field (delete backward).
 */
export function isBrowserKey(e: KeyboardEvent, mac = isMacPlatform()): boolean {
	if (mac) return false;
	const ctrl = e.ctrlKey && !e.metaKey;
	const k = e.key.toLowerCase();
	const reload = e.key === 'F5' || (ctrl && k === 'r');
	const pageAction = ctrl && !e.altKey && !e.shiftKey && ['p', 'f', 'g', 'u', 'j', 'h'].includes(k);
	const caretBrowsing = e.key === 'F7' || e.key === 'F3';
	return reload || pageAction || caretBrowsing;
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
	if (!keys) return keys;
	const arrows: Record<string, string> = { ArrowUp: '↑', ArrowDown: '↓', ArrowLeft: '←', ArrowRight: '→' };
	if (!mac) return keys.split('+').map((p) => arrows[p] ?? p).join('+');
	const symbols: Record<string, string> = { Ctrl: '⌘', Shift: '⇧', Alt: '⌥', Meta: '⌘', Control: '⌃', ...arrows };
	return keys.split('+').map((p) => symbols[p] ?? p).join('');
}

/** The key the editor (Monaco) uses for one of its own commands on this
 *  platform: on macOS Replace is ⌘⌥F (⌘H hides the app), Redo ⌘⇧Z, Go
 *  to Line ⌃G (⌘G is Find Next), and BridgeLab's Show Segment in Tree and
 *  Copy Segment ⌘⌥T and ⌘⌥C. */
export function monacoKeys(s: ShortcutDef, mac = isMacPlatform()): string {
	return mac && s.macKeys ? s.macKeys : s.defaultKeys;
}
