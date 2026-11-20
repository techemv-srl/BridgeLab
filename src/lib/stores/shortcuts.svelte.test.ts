import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('$lib/ipc/database', () => ({
	getPreference: vi.fn(async () => null),
	setPreference: vi.fn(async () => undefined),
}));

import { getPreference, setPreference } from '$lib/ipc/database';
import {
	displayKeys,
	monacoKeys,
	SHORTCUTS,
	shortcutStore,
	shortcutCapture,
	eventToKeys,
	matchesKeys,
	bindingProblem,
	withoutDuplicates,
	isBrowserKey,
	dispatchShortcut,
} from './shortcuts.svelte';

/** Minimal stand-in — node has no KeyboardEvent constructor. */
function kbd(partial: Partial<KeyboardEvent>): KeyboardEvent {
	return {
		ctrlKey: false, shiftKey: false, altKey: false, metaKey: false, key: '',
		...partial,
	} as KeyboardEvent;
}

beforeEach(() => {
	shortcutStore.resetDefaults();
	vi.mocked(getPreference).mockReset().mockResolvedValue(null);
	vi.mocked(setPreference).mockReset().mockResolvedValue(undefined);
});

describe('defaults and mutation', () => {
	it('the default map covers every declared shortcut', () => {
		for (const s of SHORTCUTS) {
			expect(shortcutStore.get(s.id)).toBe(s.defaultKeys);
		}
	});

	it('get returns empty string for unknown ids', () => {
		expect(shortcutStore.get('nope.nothing')).toBe('');
	});

	it('set overrides a binding and resetDefaults restores it', () => {
		shortcutStore.set('file.open', 'Ctrl+Shift+O');
		expect(shortcutStore.get('file.open')).toBe('Ctrl+Shift+O');
		shortcutStore.resetDefaults();
		expect(shortcutStore.get('file.open')).toBe('Ctrl+O');
	});
});

describe('conflict lookup', () => {
	it('finds the shortcut currently bound to a combination', () => {
		expect(shortcutStore.findByKeys('Ctrl+O')?.id).toBe('file.open');
	});

	it('excludes the shortcut being edited', () => {
		expect(shortcutStore.findByKeys('Ctrl+O', 'file.open')).toBeNull();
	});

	it('never reports Monaco-native shortcuts as conflicts', () => {
		// Ctrl+F belongs to editor.find (Monaco) — not a blocking conflict
		expect(shortcutStore.findByKeys('Ctrl+F')).toBeNull();
		expect(shortcutStore.findMonacoConflict('Ctrl+F')?.id).toBe('editor.find');
	});

	it('returns null for empty combinations', () => {
		expect(shortcutStore.findByKeys('')).toBeNull();
		expect(shortcutStore.findMonacoConflict('')).toBeNull();
	});

	it('warns for the editor keys the manual names (Ctrl+D, Ctrl+/)', () => {
		expect(shortcutStore.findMonacoConflict('Ctrl+D')?.id).toBe('editor.addNextMatch');
		expect(shortcutStore.findMonacoConflict('Ctrl+/')?.id).toBe('editor.toggleComment');
		expect(shortcutStore.findMonacoConflict('Alt+C')?.id).toBe('editor.copySegment');
	});

	it('no default app key is one of the editor keys listed', () => {
		const editor = new Set(SHORTCUTS.filter((s) => s.isMonaco).flatMap((s) => [s.defaultKeys, s.macKeys]));
		for (const s of SHORTCUTS.filter((x) => !x.isMonaco && x.defaultKeys)) expect(editor.has(s.defaultKeys), s.id).toBe(false);
	});
});

describe('dispatchShortcut', () => {
	/** A key press that records whether it was consumed. */
	const press = (partial: Partial<KeyboardEvent>) => {
		const e = kbd(partial) as KeyboardEvent & { consumed: string[] };
		e.consumed = [];
		Object.assign(e, {
			preventDefault: () => e.consumed.push('default'),
			stopPropagation: () => e.consumed.push('propagation'),
		});
		return e;
	};
	const keys: Record<string, string> = { 'file.closeTab': 'Ctrl+W', 'file.open': 'Ctrl+O' };

	it('runs the bound action and takes the key', () => {
		const close = vi.fn();
		const e = press({ metaKey: true, key: 'w' });
		expect(dispatchShortcut(e, { 'file.closeTab': close }, false, (id) => keys[id], true)).toBe('file.closeTab');
		expect(close).toHaveBeenCalledOnce();
		expect(e.consumed).toEqual(['default', 'propagation']);
	});

	it('behind a dialog, takes the key without running the action (⌘W must not close the window)', () => {
		const close = vi.fn();
		const e = press({ metaKey: true, key: 'w' });
		expect(dispatchShortcut(e, { 'file.closeTab': close }, true, (id) => keys[id], true)).toBe('file.closeTab');
		expect(close).not.toHaveBeenCalled();
		expect(e.consumed).toEqual(['default', 'propagation']);
	});

	it("leaves typing and other keys to the dialog's fields", () => {
		const open = vi.fn();
		for (const k of [{ key: 'w' }, { shiftKey: true, key: 'W' }, { metaKey: true, key: 'a' }, { key: 'Escape' }]) {
			const e = press(k);
			expect(dispatchShortcut(e, { 'file.open': open }, true, (id) => keys[id], true)).toBeNull();
			expect(e.consumed).toEqual([]);
		}
		expect(open).not.toHaveBeenCalled();
	});
});

describe('isBrowserKey', () => {
	it('cancels the WebView2 page keys on Windows and Linux, with the Control key only', () => {
		expect(isBrowserKey(kbd({ ctrlKey: true, key: 'r' }), false)).toBe(true);
		expect(isBrowserKey(kbd({ ctrlKey: true, key: 'p' }), false)).toBe(true);
		expect(isBrowserKey(kbd({ key: 'F5' }), false)).toBe(true);
		expect(isBrowserKey(kbd({ key: 'F7' }), false)).toBe(true);
		expect(isBrowserKey(kbd({ metaKey: true, key: 'r' }), false)).toBe(false);
		expect(isBrowserKey(kbd({ ctrlKey: true, key: 'o' }), false)).toBe(false);
	});

	it('leaves every key to the app menu and text fields on macOS', () => {
		// Hide (⌘H), Full Screen (⌃⌘F), and Control+H (delete backward).
		expect(isBrowserKey(kbd({ metaKey: true, key: 'h' }), true)).toBe(false);
		expect(isBrowserKey(kbd({ metaKey: true, ctrlKey: true, key: 'f' }), true)).toBe(false);
		expect(isBrowserKey(kbd({ ctrlKey: true, key: 'h' }), true)).toBe(false);
	});
});

describe('one key, one action', () => {
	/** App actions (not the editor's own keys) bound to `keys`. */
	const holders = (keys: string) =>
		SHORTCUTS.filter((s) => !s.isMonaco && shortcutStore.get(s.id) === keys).map((s) => s.id);

	it('no default key is on two app actions', () => {
		const keys = SHORTCUTS.filter((s) => !s.isMonaco && s.defaultKeys).map((s) => s.defaultKeys);
		expect(new Set(keys).size).toBe(keys.length);
	});

	it('assign takes the key from the action that had it', () => {
		const taken = shortcutStore.assign('tools.validate', 'Ctrl+O');
		expect(taken.map((s) => s.id)).toEqual(['file.open']);
		expect(shortcutStore.get('tools.validate')).toBe('Ctrl+O');
		expect(shortcutStore.get('file.open')).toBe('');
	});

	it('assign with a free key or no key takes nothing', () => {
		expect(shortcutStore.assign('tools.validate', 'Ctrl+Alt+V')).toEqual([]);
		// Unbinding must not clear the other unbound actions.
		shortcutStore.set('file.save', '');
		expect(shortcutStore.assign('tools.validate', '')).toEqual([]);
		expect(shortcutStore.get('tools.reparse')).toBe('');
		expect(shortcutStore.get('file.open')).toBe('Ctrl+O');
	});

	it('assign leaves the editor keys alone', () => {
		shortcutStore.assign('tools.validate', 'Ctrl+F');
		expect(shortcutStore.get('editor.find')).toBe('Ctrl+F');
		expect(shortcutStore.get('tools.validate')).toBe('Ctrl+F');
	});

	it('resetting an entry whose default moved elsewhere moves it back', () => {
		// The sequence: Ctrl+O given to Validate with Reassign, then
		// the reset next to Open File. The reset finds the holder (to warn)
		// and, once confirmed, transfers the key instead of copying it.
		shortcutStore.assign('tools.validate', 'Ctrl+O');
		expect(shortcutStore.findByKeys('Ctrl+O', 'file.open')?.id).toBe('tools.validate');
		shortcutStore.assign('file.open', 'Ctrl+O');
		expect(holders('Ctrl+O')).toEqual(['file.open']);
		expect(shortcutStore.get('tools.validate')).toBe('');
	});

	it('a saved duplicate keeps the key on the action it ran, and only there', async () => {
		vi.mocked(getPreference).mockResolvedValue(JSON.stringify({
			...Object.fromEntries(SHORTCUTS.map((s) => [s.id, s.defaultKeys])),
			'tools.validate': 'Ctrl+O',
		}));
		await shortcutStore.loadFromPrefs();
		expect(holders('Ctrl+O')).toEqual(['file.open']);
		expect(shortcutStore.get('tools.validate')).toBe('');
	});

	it('a key the user saved wins over a newer default that has it too', async () => {
		// Settings from before Toggle Interpretation existed, with its
		// default given to Validate: the user's choice stays.
		vi.mocked(getPreference).mockResolvedValue(JSON.stringify({
			'file.open': 'Ctrl+O',
			'tools.validate': 'Ctrl+Shift+E',
		}));
		await shortcutStore.loadFromPrefs();
		expect(holders('Ctrl+Shift+E')).toEqual(['tools.validate']);
		expect(shortcutStore.get('view.toggleInterpretation')).toBe('');
	});

	it('withoutDuplicates keeps unbound and distinct keys as they are', () => {
		const map = { 'file.open': 'Ctrl+O', 'file.save': '', 'tools.reparse': '', 'tools.validate': 'F6' };
		expect(withoutDuplicates(map)).toEqual(map);
		expect(withoutDuplicates({ ...map, 'tools.reparse': 'F6' }, new Set(['tools.reparse'])))
			.toEqual({ ...map, 'tools.reparse': 'F6', 'tools.validate': '' });
	});
});

describe('preference persistence', () => {
	it('merges saved bindings over defaults', async () => {
		vi.mocked(getPreference).mockResolvedValue(
			JSON.stringify({ 'file.open': 'Ctrl+Shift+O' }),
		);
		await shortcutStore.loadFromPrefs();
		expect(shortcutStore.get('file.open')).toBe('Ctrl+Shift+O');
		// untouched bindings keep their defaults
		expect(shortcutStore.get('file.save')).toBe('Ctrl+S');
		expect(shortcutStore.loaded).toBe(true);
	});

	it('keeps defaults when the saved value is corrupt', async () => {
		vi.mocked(getPreference).mockResolvedValue('{not json');
		await shortcutStore.loadFromPrefs();
		expect(shortcutStore.get('file.open')).toBe('Ctrl+O');
		expect(shortcutStore.loaded).toBe(true);
	});

	it('save writes the current map as JSON', async () => {
		shortcutStore.set('tools.validate', 'F8');
		await shortcutStore.save();
		expect(setPreference).toHaveBeenCalledWith(
			'shortcuts_json',
			expect.stringContaining('"tools.validate":"F8"'),
		);
	});
});

describe('eventToKeys', () => {
	it('builds modifier chains in Ctrl+Shift+Alt order', () => {
		expect(eventToKeys(kbd({ ctrlKey: true, shiftKey: true, key: 'k' }))).toBe('Ctrl+Shift+K');
	});

	it('Command is Ctrl on macOS, and the Control key is Control there', () => {
		expect(eventToKeys(kbd({ metaKey: true, key: 'o' }), true)).toBe('Ctrl+O');
		expect(eventToKeys(kbd({ ctrlKey: true, key: 'g' }), true)).toBe('Control+G');
		expect(eventToKeys(kbd({ ctrlKey: true, metaKey: true, key: 'f' }), true)).toBe('Ctrl+Control+F');
	});

	it('the Windows/Super key is not Ctrl on Windows and Linux', () => {
		expect(eventToKeys(kbd({ metaKey: true, key: 'b' }), false)).toBe('Meta+B');
		expect(eventToKeys(kbd({ ctrlKey: true, key: 'b' }), false)).toBe('Ctrl+B');
	});

	it('records Option+letter on macOS by the key, not the character it types', () => {
		expect(eventToKeys(kbd({ altKey: true, key: '†', code: 'KeyT' }), true)).toBe('Alt+T');
		expect(eventToKeys(kbd({ altKey: true, metaKey: true, key: 'ç', code: 'KeyC' }), true)).toBe('Ctrl+Alt+C');
		// A dead key (Option+E) on a letter is that letter; elsewhere nothing.
		expect(eventToKeys(kbd({ altKey: true, key: 'Dead', code: 'KeyE' }), true)).toBe('Alt+E');
		expect(eventToKeys(kbd({ altKey: true, key: 'Dead', code: 'Backquote' }), true)).toBe('');
		// A layout that only moves ASCII letters keeps the letter typed.
		expect(eventToKeys(kbd({ ctrlKey: true, key: 'a', code: 'KeyQ' }), false)).toBe('Ctrl+A');
	});

	it('handles function keys and space', () => {
		expect(eventToKeys(kbd({ key: 'F5' }))).toBe('F5');
		expect(eventToKeys(kbd({ ctrlKey: true, key: ' ' }))).toBe('Ctrl+Space');
	});

	it('returns empty for bare modifier presses', () => {
		expect(eventToKeys(kbd({ ctrlKey: true, key: 'Control' }))).toBe('');
		expect(eventToKeys(kbd({ shiftKey: true, key: 'Shift' }))).toBe('');
	});
});

describe('matchesKeys', () => {
	it('matches modifiers exactly', () => {
		expect(matchesKeys(kbd({ ctrlKey: true, key: 'o' }), 'Ctrl+O')).toBe(true);
		expect(matchesKeys(kbd({ ctrlKey: true, shiftKey: true, key: 'o' }), 'Ctrl+O')).toBe(false);
		expect(matchesKeys(kbd({ key: 'o' }), 'Ctrl+O')).toBe(false);
	});

	it('Ctrl is Command on macOS and the Control key elsewhere, never both', () => {
		expect(matchesKeys(kbd({ metaKey: true, key: 'o' }), 'Ctrl+O', true)).toBe(true);
		expect(matchesKeys(kbd({ ctrlKey: true, key: 'o' }), 'Ctrl+O', false)).toBe(true);
		// The Super key on Linux hid the tree (Ctrl+B)...
		expect(matchesKeys(kbd({ metaKey: true, key: 'b' }), 'Ctrl+B', false)).toBe(false);
		// ...and on macOS Control+K, an editor key, opened the Communication panel.
		expect(matchesKeys(kbd({ ctrlKey: true, key: 'k' }), 'Ctrl+K', true)).toBe(false);
		// Control+Command+F is the macOS Full Screen, not Ctrl+F.
		expect(matchesKeys(kbd({ ctrlKey: true, metaKey: true, key: 'f' }), 'Ctrl+F', true)).toBe(false);
	});

	it('matches the real Control key on macOS and the Super key elsewhere when asked for', () => {
		expect(matchesKeys(kbd({ ctrlKey: true, key: 'g' }), 'Control+G', true)).toBe(true);
		expect(matchesKeys(kbd({ metaKey: true, key: 'g' }), 'Control+G', true)).toBe(false);
		expect(matchesKeys(kbd({ metaKey: true, key: 'b' }), 'Meta+B', false)).toBe(true);
	});

	it('matches a key by its place when the layout types another character', () => {
		expect(matchesKeys(kbd({ altKey: true, metaKey: true, key: '†', code: 'KeyT' }), 'Ctrl+Alt+T', true)).toBe(true);
		expect(matchesKeys(kbd({ ctrlKey: true, key: 'щ', code: 'KeyO' }), 'Ctrl+O', false)).toBe(true);
		expect(matchesKeys(kbd({ ctrlKey: true, key: 'a', code: 'KeyQ' }), 'Ctrl+Q', false)).toBe(false);
	});

	it('is case-insensitive on the main key', () => {
		expect(matchesKeys(kbd({ ctrlKey: true, key: 'O' }), 'Ctrl+O')).toBe(true);
	});

	it('matches function keys and space literally', () => {
		expect(matchesKeys(kbd({ key: 'F6' }), 'F6')).toBe(true);
		expect(matchesKeys(kbd({ key: ' ' }), 'Space')).toBe(true);
	});

	it('never matches an empty binding', () => {
		expect(matchesKeys(kbd({ key: 'x' }), '')).toBe(false);
	});
});

describe('capture flag', () => {
	it('starts inactive', () => {
		expect(shortcutCapture.active).toBe(false);
	});
});

describe('bindingProblem', () => {
	it('refuses keys that type text and the reserved F1', () => {
		expect(bindingProblem('Q')).toBe('needsModifier');
		expect(bindingProblem('Shift+Q')).toBe('needsModifier');
		expect(bindingProblem('Space')).toBe('needsModifier');
		expect(bindingProblem('F1')).toBe('reserved');
	});

	it('accepts modified keys and F2-F12', () => {
		expect(bindingProblem('Ctrl+Q')).toBeNull();
		expect(bindingProblem('Alt+Q', false)).toBeNull();
		expect(bindingProblem('F6')).toBeNull();
		expect(bindingProblem('Shift+F12')).toBeNull();
		expect(bindingProblem('Meta+Q', false)).toBeNull();
		expect(bindingProblem('Control+Q', true)).toBeNull();
	});

	it('on macOS Option with a key that types is not enough (Option+C types ç)', () => {
		expect(bindingProblem('Alt+C', true)).toBe('needsModifier');
		expect(bindingProblem('Shift+Alt+T', true)).toBe('needsModifier');
		expect(bindingProblem('Alt+Space', true)).toBe('needsModifier');
		expect(bindingProblem('Alt+F5', true)).toBeNull();
		expect(bindingProblem('Alt+ArrowUp', true)).toBeNull();
		expect(bindingProblem('Ctrl+Alt+C', true)).toBeNull();
	});

	it('drops saved bindings that are not allowed, and editor keys', async () => {
		vi.mocked(getPreference).mockResolvedValue(JSON.stringify({
			'tools.validate': 'Q',
			'file.save': 'F1',
			'file.open': 'Ctrl+Shift+O',
			'editor.find': 'Ctrl+E',
		}));
		await shortcutStore.loadFromPrefs();
		expect(shortcutStore.get('tools.validate')).toBe('F6');
		expect(shortcutStore.get('file.save')).toBe('Ctrl+S');
		expect(shortcutStore.get('file.open')).toBe('Ctrl+Shift+O');
		expect(shortcutStore.get('editor.find')).toBe('Ctrl+F');
	});
});

describe('displayKeys', () => {
	it('shows Command-key symbols on macOS only', () => {
		expect(displayKeys('Ctrl+Shift+K', true)).toBe('⌘⇧K');
		expect(displayKeys('Ctrl+Alt+T', true)).toBe('⌘⌥T');
		expect(displayKeys('F6', true)).toBe('F6');
		expect(displayKeys('Ctrl+O', false)).toBe('Ctrl+O');
		expect(displayKeys('', true)).toBe('');
		expect(displayKeys('Control+G', true)).toBe('⌃G');
	});

	it('lists the editor keys macOS really uses', () => {
		const key = (id: string, mac: boolean) => monacoKeys(SHORTCUTS.find((s) => s.id === id)!, mac);
		expect(displayKeys(key('editor.replace', true), true)).toBe('⌘⌥F');
		expect(displayKeys(key('editor.redo', true), true)).toBe('⌘⇧Z');
		expect(displayKeys(key('editor.goToLine', true), true)).toBe('⌃G');
		expect(displayKeys(key('editor.find', true), true)).toBe('⌘F');
		expect(key('editor.replace', false)).toBe('Ctrl+H');
		expect(key('editor.goToLine', false)).toBe('Ctrl+G');
	});

	it('Show Segment in Tree and Copy Segment do not take Option+letter on macOS', () => {
		const key = (id: string, mac: boolean) => monacoKeys(SHORTCUTS.find((s) => s.id === id)!, mac);
		expect(displayKeys(key('editor.showInTree', true), true)).toBe('⌘⌥T');
		expect(displayKeys(key('editor.copySegment', true), true)).toBe('⌘⌥C');
		expect(key('editor.showInTree', false)).toBe('Alt+T');
		expect(key('editor.copySegment', false)).toBe('Alt+C');
		for (const s of SHORTCUTS) expect(bindingProblem(monacoKeys(s, true), true), s.id).toBeNull();
	});

	it('shows arrow keys as arrows', () => {
		expect(displayKeys('Alt+ArrowUp', false)).toBe('Alt+↑');
		expect(displayKeys('Alt+ArrowDown', true)).toBe('⌥↓');
	});
});
