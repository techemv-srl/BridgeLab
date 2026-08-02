import { describe, it, expect, beforeEach } from 'vitest';
import { messageStore } from './messages.svelte';
import type { ParseResult } from '$lib/types/hl7';

// The store never inspects the parse result's internals — an opaque
// placeholder keeps these tests decoupled from the ParseResult shape.
const fakeParse = () => ({ marker: Math.random() }) as unknown as ParseResult;

beforeEach(() => {
	messageStore.closeAllTabs();
});

describe('newTab', () => {
	it('creates an empty Untitled tab and activates it', () => {
		const id = messageStore.newTab();
		expect(messageStore.tabs).toHaveLength(1);
		expect(messageStore.activeTabId).toBe(id);
		expect(messageStore.activeTab?.label).toBe('Untitled');
		expect(messageStore.activeTab?.content).toBe('');
		expect(messageStore.activeTab?.isModified).toBe(false);
	});

	it('assigns unique ids across calls', () => {
		const a = messageStore.newTab();
		const b = messageStore.newTab();
		expect(a).not.toBe(b);
	});
});

describe('openMessage', () => {
	it('derives the label from a unix path', () => {
		messageStore.openMessage(fakeParse(), '/data/msgs/adt_a01.hl7', 'MSH|...');
		expect(messageStore.activeTab?.label).toBe('adt_a01.hl7');
	});

	it('derives the label from a windows path', () => {
		messageStore.openMessage(fakeParse(), 'C:\\data\\msgs\\oru_r01.hl7', 'MSH|...');
		expect(messageStore.activeTab?.label).toBe('oru_r01.hl7');
	});

	it('reactivates the existing tab instead of duplicating an open file', () => {
		const first = messageStore.openMessage(fakeParse(), '/tmp/a.hl7', 'MSH|1');
		messageStore.newTab();
		const second = messageStore.openMessage(fakeParse(), '/tmp/a.hl7', 'MSH|2');
		expect(second).toBe(first);
		expect(messageStore.tabs).toHaveLength(2);
		expect(messageStore.activeTabId).toBe(first);
	});

	it('always creates a new tab for pasted content (no file path)', () => {
		messageStore.openMessage(fakeParse(), null, 'MSH|1');
		messageStore.openMessage(fakeParse(), null, 'MSH|1');
		expect(messageStore.tabs).toHaveLength(2);
	});
});

describe('content and parse updates', () => {
	it('updateContent marks the tab modified', () => {
		const id = messageStore.newTab();
		messageStore.updateContent(id, 'MSH|edited');
		expect(messageStore.activeTab?.content).toBe('MSH|edited');
		expect(messageStore.activeTab?.isModified).toBe(true);
	});

	it('updateParseResult keeps editor content unless truncatedText is given', () => {
		const id = messageStore.newTab();
		messageStore.updateContent(id, 'user is typing');
		messageStore.updateParseResult(id, fakeParse());
		expect(messageStore.activeTab?.content).toBe('user is typing');

		messageStore.updateParseResult(id, fakeParse(), 'truncated view');
		expect(messageStore.activeTab?.content).toBe('truncated view');
	});

	it('updateCursor tracks position', () => {
		const id = messageStore.newTab();
		messageStore.updateCursor(id, 7, 42);
		expect(messageStore.activeTab?.cursorLine).toBe(7);
		expect(messageStore.activeTab?.cursorColumn).toBe(42);
	});

	it('ignores updates for unknown tab ids', () => {
		messageStore.newTab();
		messageStore.updateContent('tab-does-not-exist', 'x');
		expect(messageStore.activeTab?.content).toBe('');
	});
});

describe('markSaved', () => {
	it('clears the modified flag', () => {
		const id = messageStore.newTab();
		messageStore.updateContent(id, 'MSH|x');
		messageStore.markSaved(id);
		expect(messageStore.activeTab?.isModified).toBe(false);
		expect(messageStore.activeTab?.filePath).toBeNull();
	});

	it('adopts the new path and label on save-as', () => {
		const id = messageStore.newTab();
		messageStore.markSaved(id, '/exports/renamed.hl7');
		expect(messageStore.activeTab?.filePath).toBe('/exports/renamed.hl7');
		expect(messageStore.activeTab?.label).toBe('renamed.hl7');
	});
});

describe('closeTab', () => {
	it('activates the tab that took the closed tab position', () => {
		const a = messageStore.newTab();
		const b = messageStore.newTab();
		const c = messageStore.newTab();
		messageStore.setActiveTab(b);
		const next = messageStore.closeTab(b);
		expect(next).toBe(c);
		expect(messageStore.tabs.map((t) => t.id)).toEqual([a, c]);
	});

	it('falls back to the previous tab when the last tab is closed', () => {
		const a = messageStore.newTab();
		const b = messageStore.newTab();
		const next = messageStore.closeTab(b);
		expect(next).toBe(a);
	});

	it('returns null when the only tab is closed', () => {
		const a = messageStore.newTab();
		expect(messageStore.closeTab(a)).toBeNull();
		expect(messageStore.tabs).toHaveLength(0);
	});

	it('keeps the active tab when a background tab is closed', () => {
		const a = messageStore.newTab();
		const b = messageStore.newTab();
		expect(messageStore.closeTab(a)).toBe(b);
		expect(messageStore.activeTabId).toBe(b);
	});

	it('is a no-op for unknown ids', () => {
		const a = messageStore.newTab();
		expect(messageStore.closeTab('nope')).toBe(a);
		expect(messageStore.tabs).toHaveLength(1);
	});
});

describe('bulk close and activation', () => {
	it('closeOtherTabs keeps only the given tab', () => {
		messageStore.newTab();
		const keep = messageStore.newTab();
		messageStore.newTab();
		messageStore.closeOtherTabs(keep);
		expect(messageStore.tabs.map((t) => t.id)).toEqual([keep]);
		expect(messageStore.activeTabId).toBe(keep);
	});

	it('setActiveTab ignores unknown ids', () => {
		const a = messageStore.newTab();
		messageStore.setActiveTab('ghost');
		expect(messageStore.activeTabId).toBe(a);
	});
});

describe('session persistence', () => {
	it('serializes order, active flag and cursor', () => {
		const a = messageStore.newTab();
		messageStore.updateContent(a, 'MSH|a');
		const b = messageStore.openMessage(fakeParse(), '/tmp/b.hl7', 'MSH|b');
		messageStore.updateCursor(b, 3, 9);
		messageStore.setActiveTab(a);

		const s = messageStore.serializeSession();
		expect(s).toHaveLength(2);
		expect(s[0]).toMatchObject({
			tab_order: 0, content: 'MSH|a', is_modified: true, is_active: true,
		});
		expect(s[1]).toMatchObject({
			tab_order: 1, file_path: '/tmp/b.hl7', label: 'b.hl7',
			is_active: false, cursor_line: 3, cursor_column: 9,
		});
	});

	it('restores tabs sorted by tab_order and reactivates the right one', () => {
		const restored = messageStore.restoreSession([
			{ tab_order: 1, label: 'second', file_path: null, content: 'B', is_modified: false, is_active: true, cursor_line: 1, cursor_column: 1 },
			{ tab_order: 0, label: 'first', file_path: '/x/first.hl7', content: 'A', is_modified: true, is_active: false, cursor_line: 2, cursor_column: 5 },
		]);
		expect(restored).toBe(true);
		expect(messageStore.tabs.map((t) => t.label)).toEqual(['first', 'second']);
		expect(messageStore.activeTab?.label).toBe('second');
		expect(messageStore.tabs[0].isModified).toBe(true);
		// parseResult is re-derived after restore, never persisted
		expect(messageStore.tabs.every((t) => t.parseResult === null)).toBe(true);
	});

	it('returns false for an empty session and leaves the store untouched', () => {
		messageStore.newTab();
		expect(messageStore.restoreSession([])).toBe(false);
		expect(messageStore.tabs).toHaveLength(1);
	});

	it('round-trips serialize → restore', () => {
		const a = messageStore.newTab();
		messageStore.updateContent(a, 'MSH|roundtrip');
		const snapshot = messageStore.serializeSession();

		messageStore.closeAllTabs();
		messageStore.restoreSession(snapshot);
		expect(messageStore.tabs).toHaveLength(1);
		expect(messageStore.activeTab?.content).toBe('MSH|roundtrip');
		expect(messageStore.activeTab?.isModified).toBe(true);
	});
});
