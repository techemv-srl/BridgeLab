import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('$lib/ipc/database', () => ({
	getPreference: vi.fn(async () => null),
	setPreference: vi.fn(async () => undefined),
}));

import { getPreference, setPreference } from '$lib/ipc/database';
import {
	SHORTCUTS,
	shortcutStore,
	shortcutCapture,
	eventToKeys,
	matchesKeys,
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

	it('treats meta as Ctrl (macOS)', () => {
		expect(eventToKeys(kbd({ metaKey: true, key: 'o' }))).toBe('Ctrl+O');
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

	it('accepts meta in place of Ctrl', () => {
		expect(matchesKeys(kbd({ metaKey: true, key: 'o' }), 'Ctrl+O')).toBe(true);
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
