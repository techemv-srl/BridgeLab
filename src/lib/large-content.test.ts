import { describe, it, expect, vi, beforeEach } from 'vitest';

const { confirm, info } = vi.hoisted(() => ({
	confirm: vi.fn(async (_message: string, _title?: string) => true),
	info: vi.fn(async (_message: string, _title?: string) => {}),
}));
vi.mock('$lib/stores/dialog.svelte', () => ({ dialogStore: { confirm, info } }));

import {
	ASK_ABOVE_BYTES, REFUSE_ABOVE_BYTES, SEND_ASK_ABOVE_BYTES,
	mayLoadPastedText, maySendMessage, pasteCopies, utf8Length,
} from './large-content';
import { readFileSync } from 'node:fs';

const MiB = 1024 * 1024;

beforeEach(() => {
	confirm.mockClear();
	info.mockClear();
});

describe('large content', () => {
	it('counts UTF-8 bytes like TextEncoder', () => {
		// Lone surrogates are written as U+FFFD: a high one followed by
		// anything but a low one is not a pair.
		for (const s of ['', 'MSH|^~\\&|', 'Müller', '李小龍', 'a😀b', '\u{10FFFF}', 'x\uD800y', '\uD800李', 'x\uDC00y', 'end\uD800', '\uD800\uD800\uDC00']) {
			expect(utf8Length(s)).toBe(new TextEncoder().encode(s).length);
		}
	});

	it('uses the limits of opening a file and of the listener', () => {
		// Kept equal to the backend's constants: a paste and a file of the
		// same size get the same answer, and a send is asked about where
		// BridgeLab's own listener would refuse it.
		const fileio = readFileSync('src-tauri/src/commands/fileio.rs', 'utf8');
		expect(fileio).toContain('pub const OPEN_CONFIRM_BYTES: u64 = 16 * 1024 * 1024;');
		expect(fileio).toContain('pub const OPEN_MAX_BYTES: u64 = 64 * 1024 * 1024;');
		const listener = readFileSync('src-tauri/src/communication/mllp_listener.rs', 'utf8');
		expect(listener).toContain('const MAX_MSG_BYTES: usize = 10 * 1024 * 1024;');
		expect([ASK_ABOVE_BYTES, REFUSE_ABOVE_BYTES, SEND_ASK_ABOVE_BYTES]).toEqual([16 * MiB, 64 * MiB, 10 * MiB]);
	});

	it('pastes up to 16 MB at once, asks up to 64 MB, refuses beyond', async () => {
		expect(await mayLoadPastedText('x'.repeat(ASK_ABOVE_BYTES))).toBe(true);
		expect(confirm).not.toHaveBeenCalled();

		confirm.mockResolvedValueOnce(false);
		expect(await mayLoadPastedText('x'.repeat(ASK_ABOVE_BYTES + 1))).toBe(false);
		expect(confirm).toHaveBeenCalledOnce();
		expect(await mayLoadPastedText('x'.repeat(20 * MiB))).toBe(true);
		expect(confirm).toHaveBeenCalledTimes(2);

		expect(await mayLoadPastedText('x'.repeat(REFUSE_ABOVE_BYTES + 1))).toBe(false);
		expect(info).toHaveBeenCalledOnce();
		expect(confirm).toHaveBeenCalledTimes(2);
	});

	it('counts multi-byte text by its bytes, not its length', async () => {
		// 6M CJK characters are 18 MB in UTF-8.
		expect(await mayLoadPastedText('李'.repeat(6 * MiB))).toBe(true);
		expect(confirm).toHaveBeenCalledOnce();
	});

	it('counts a paste once per cursor, unless Monaco spreads its lines', async () => {
		expect(pasteCopies('abc', 1)).toBe(1);
		expect(pasteCopies('abc', 7)).toBe(7);
		// One line per cursor: one line to each, the text written once.
		expect(pasteCopies('a\nb\nc', 3)).toBe(1);
		expect(pasteCopies('a\r\nb\r\nc\n', 3)).toBe(1);
		// A Windows clipboard ends in CRLF: both are dropped, as Monaco does.
		expect(pasteCopies('a\r\nb\r\nc\r\n', 3)).toBe(1);
		expect(pasteCopies('a\rb\r', 2)).toBe(1);
		expect(pasteCopies('a\rb', 2)).toBe(1);
		expect(pasteCopies('a\nb\nc', 2)).toBe(2);
		// 3 MB at seven cursors is 21 MB: asked about.
		expect(await mayLoadPastedText('x'.repeat(3 * MiB), 7)).toBe(true);
		expect(confirm).toHaveBeenCalledOnce();
		// 10 MB at seven cursors is 70 MB: refused.
		expect(await mayLoadPastedText('x'.repeat(10 * MiB), 7)).toBe(false);
		expect(info).toHaveBeenCalledOnce();
	});

	it('asks before a send over 10 MB only', async () => {
		expect(await maySendMessage(SEND_ASK_ABOVE_BYTES)).toBe(true);
		expect(confirm).not.toHaveBeenCalled();
		confirm.mockResolvedValueOnce(false);
		expect(await maySendMessage(SEND_ASK_ABOVE_BYTES + 1)).toBe(false);
		expect(confirm).toHaveBeenCalledOnce();
	});
});

describe('sizes in the interface language', () => {
	it('writes the decimal separator of the language', async () => {
		const { setLocale } = await import('$lib/i18n');
		const { oneDecimal, formatSize } = await import('./large-content');
		setLocale('it');
		expect(oneDecimal(20)).toBe('20,0');
		expect(formatSize(1536)).toBe('1,5 KB');
		setLocale('en');
		expect(oneDecimal(20)).toBe('20.0');
		expect(formatSize(20 * 1024 * 1024)).toBe('20.0 MB');
		expect(formatSize(512)).toBe('512 B');
	});
});
