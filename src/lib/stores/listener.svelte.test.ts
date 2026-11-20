import { describe, it, expect, beforeEach, vi } from 'vitest';

// The backend's event stream: `emit` delivers to whatever subscribed.
const handlers = new Map<string, ((ev: { payload: unknown }) => void)[]>();
vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn(async (name: string, cb: (ev: { payload: unknown }) => void) => {
		handlers.set(name, [...(handlers.get(name) ?? []), cb]);
		return () => handlers.set(name, (handlers.get(name) ?? []).filter((h) => h !== cb));
	}),
}));
vi.mock('$lib/ipc/communication', () => ({
	mllpListenStatus: vi.fn(async () => ({ running: false, port: null, bind_address: null })),
	mllpListenStart: vi.fn(async (c: { port: number; bind_address: string }) => ({ running: true, port: c.port, bind_address: c.bind_address })),
	mllpListenStop: vi.fn(async () => ({ running: false, port: null, bind_address: null })),
}));

import { listen } from '@tauri-apps/api/event';
import { mllpListenStart, mllpListenStatus } from '$lib/ipc/communication';
import { listenerStore, CONSOLE_CAP, CONSOLE_CONTENT_BUDGET, AUTO_OPEN_BURST, AUTO_OPEN_BYTES } from './listener.svelte';

function emit(name: string, payload: unknown) {
	for (const h of handlers.get(name) ?? []) h({ payload });
}

let seq = 0;
function received(ack: string | null, over: Record<string, unknown> = {}) {
	seq++;
	emit('mllp:received', {
		content: `MSH|^~\\&|A|B|C|D|20240101||ADT^A01|${seq}|P|2.5\rPID|1`,
		source_addr: '127.0.0.1:5000',
		received_at: '2026-11-18T10:00:00Z',
		bytes: 60,
		ack_code: ack,
		encoding: 'UTF-8',
		...over,
	});
}

const config = { port: 23410, bind_address: '127.0.0.1', auto_ack: true, ack_code: 'AE', read_timeout_secs: 30, encoding: 'auto' };

beforeEach(() => {
	listenerStore.dispose();
	handlers.clear();
	vi.mocked(listen).mockClear();
	vi.mocked(mllpListenStatus).mockClear();
});

describe('listenerStore', () => {
	it('subscribes once, however often it is initialised', async () => {
		const open = vi.fn();
		await Promise.all([listenerStore.init(open), listenerStore.init(open)]);
		await listenerStore.init(open);
		expect(vi.mocked(listen).mock.calls.map((c) => c[0])).toEqual(['mllp:received', 'mllp:listen_error', 'mllp:listen_stopped', 'mllp:connection_idle']);
		received('AA');
		expect(open).toHaveBeenCalledOnce();
		expect(listenerStore.entries).toHaveLength(1);
	});

	it('opens and logs received messages with no panel open at all', async () => {
		// The panel used to own the subscription: closed, nothing opened and
		// the console stayed empty while the backend kept ACKing.
		const open = vi.fn();
		await listenerStore.init(open);
		await listenerStore.start(config);
		received('AE');
		received('AE');
		received('AE');
		expect(open).toHaveBeenCalledTimes(3);
		expect(open.mock.calls[0][0]).toContain('ADT^A01');
		expect(listenerStore.received).toBe(3);
		expect(listenerStore.entries.map((e) => e.ack)).toEqual(['AE', 'AE', 'AE']);
		expect(listenerStore.counts.AE).toBe(3);
		expect(listenerStore.status).toEqual({ running: true, port: 23410, bind_address: '127.0.0.1' });
		expect(listenerStore.lastConfig).toEqual(config);
	});

	it('only logs when "open received messages in a new tab" is off, and a row click opens', async () => {
		const open = vi.fn();
		await listenerStore.init(open);
		listenerStore.autoOpen = false;
		received('AA');
		expect(open).not.toHaveBeenCalled();
		const row = listenerStore.entries[0];
		listenerStore.open(row.content!);
		expect(open).toHaveBeenCalledWith(row.content);
	});

	it('counts every row since the last Clear, not only the rows the console keeps', async () => {
		await listenerStore.init(() => {});
		for (let i = 0; i < 12; i++) received('AE');
		for (let i = 0; i < 288; i++) received('AA');
		// The chips read "All 200 · AA 200 · AE 0".
		expect(listenerStore.entries).toHaveLength(CONSOLE_CAP);
		expect(listenerStore.received).toBe(300);
		expect(listenerStore.counts).toEqual({ all: 300, AA: 288, AE: 12, AR: 0, noack: 0, fail: 0 });

		listenerStore.clear();
		expect(listenerStore.entries).toHaveLength(0);
		expect(listenerStore.received).toBe(0);
		expect(listenerStore.counts).toEqual({ all: 0, AA: 0, AE: 0, AR: 0, noack: 0, fail: 0 });
	});

	it('counts commit codes with their family, no ACK and listener errors', async () => {
		await listenerStore.init(() => {});
		received('CA');
		received('CR');
		received(null);
		emit('mllp:listen_error', 'connection reset by peer');
		expect(listenerStore.counts).toEqual({ all: 4, AA: 1, AE: 0, AR: 1, noack: 1, fail: 1 });
		expect(listenerStore.received).toBe(3);
		expect(listenerStore.error).toBe('connection reset by peer');
		expect(listenerStore.entries[0]).toMatchObject({ kind: 'error', text: 'connection reset by peer' });
	});

	it('logs a connection closed for sending nothing as an ordinary row, not an error', async () => {
		await listenerStore.init(() => {});
		emit('mllp:connection_idle', { source_addr: '127.0.0.1:51000', idle_secs: 30 });
		expect(listenerStore.counts).toEqual({ all: 1, AA: 0, AE: 0, AR: 0, noack: 0, fail: 0 });
		expect(listenerStore.error).toBeNull();
		expect(listenerStore.received).toBe(0);
		expect(listenerStore.entries[0]).toMatchObject({ kind: 'note', peer: '127.0.0.1:51000' });
		expect(listenerStore.entries[0].text).toContain('127.0.0.1:51000');
		expect(listenerStore.entries[0].text).toContain('30');
	});

	it('shows the port the system gave when port 0 was asked for', async () => {
		vi.mocked(mllpListenStart).mockResolvedValueOnce({ running: true, port: 43127, bind_address: '127.0.0.1' });
		await listenerStore.start({ ...config, port: 0 });
		expect(listenerStore.status.port).toBe(43127);
		// The form keeps what was asked for: a new Start picks a port again.
		expect(listenerStore.lastConfig?.port).toBe(0);
	});

	it('shows a listener that was already running', async () => {
		vi.mocked(mllpListenStatus).mockResolvedValueOnce({ running: true, port: 2576, bind_address: '0.0.0.0' });
		await listenerStore.init(() => {});
		expect(listenerStore.status).toEqual({ running: true, port: 2576, bind_address: '0.0.0.0' });
		await listenerStore.stop();
		expect(listenerStore.status.running).toBe(false);
	});

	it('shows a listener that ended on its own as stopped, with the reason', async () => {
		// It used to keep "Listening" and Stop while every connection was refused.
		await listenerStore.init(() => {});
		await listenerStore.start(config);
		expect(listenerStore.status.running).toBe(true);
		vi.mocked(mllpListenStatus).mockClear();
		emit('mllp:listen_stopped', 'accept failed: Bad file descriptor (os error 9)');
		expect(listenerStore.status.running).toBe(false);
		expect(listenerStore.error).toContain('accept failed: Bad file descriptor');
		expect(listenerStore.entries[0]).toMatchObject({ kind: 'error' });
		expect(listenerStore.counts.fail).toBe(1);
		// The backend has the last word: the event may be from a listener
		// already replaced by a new Start.
		await vi.waitFor(() => expect(mllpListenStatus).toHaveBeenCalledOnce());
	});

	it('a stale stopped event does not hide a listener started since', async () => {
		await listenerStore.init(() => {});
		await listenerStore.start(config);
		vi.mocked(mllpListenStatus).mockResolvedValueOnce({ running: true, port: 23410, bind_address: '127.0.0.1' });
		await listenerStore.stopped('accept failed: Bad file descriptor (os error 9)');
		expect(listenerStore.status).toEqual({ running: true, port: 23410, bind_address: '127.0.0.1' });
	});

	it('releases old full contents past the byte budget, keeping the rows', async () => {
		await listenerStore.init(() => {});
		const big = CONSOLE_CONTENT_BUDGET / 2 + 1;
		received('AA', { bytes: big });
		received('AA', { bytes: big });
		received('AA', { bytes: big });
		expect(listenerStore.entries.map((e) => e.content !== undefined)).toEqual([true, false, false]);
		expect(listenerStore.entries.every((e) => e.snippet?.startsWith('MSH'))).toBe(true);
	});
});

describe('a burst of received messages', () => {
	it('opens tabs only up to the cap while nobody is at the window, and logs the rest', async () => {
		// 1000 messages opened 1000 tabs and froze the window for minutes.
		const open = vi.fn();
		await listenerStore.init(open);
		for (let i = 0; i < 1000; i++) received('AA');
		expect(AUTO_OPEN_BURST).toBe(20);
		expect(open).toHaveBeenCalledTimes(AUTO_OPEN_BURST);
		expect(listenerStore.heldBack).toBe(1000 - AUTO_OPEN_BURST);
		// Every message is still in the console and the counts.
		expect(listenerStore.received).toBe(1000);
		expect(listenerStore.counts.AA).toBe(1000);
		expect(listenerStore.entries).toHaveLength(CONSOLE_CAP);
		// A held-back one opens from its console row.
		listenerStore.open(listenerStore.entries[0].content!);
		expect(open).toHaveBeenCalledTimes(AUTO_OPEN_BURST + 1);
	});

	it('a burst of large messages stops opening tabs at the byte budget', async () => {
		// Twenty 9 MB messages opened as tabs take several GB: past 16 MB of
		// messages the rest stay in the console, as past the tab count.
		const open = vi.fn();
		await listenerStore.init(open);
		const nineMb = 9 * 1024 * 1024;
		expect(AUTO_OPEN_BYTES).toBe(16 * 1024 * 1024);
		for (let i = 0; i < 5; i++) received('AA', { bytes: nineMb });
		expect(open).toHaveBeenCalledTimes(1);
		expect(listenerStore.heldBack).toBe(4);
		// Small ones still fit next to it.
		received('AA', { bytes: 60 });
		expect(open).toHaveBeenCalledTimes(2);
		// A message alone always opens, and activity starts a new budget.
		listenerStore.noteActivity();
		received('AA', { bytes: 10 * 1024 * 1024 });
		expect(open).toHaveBeenCalledTimes(3);
		listenerStore.resumeAutoOpen();
		received('AA', { bytes: nineMb });
		expect(open).toHaveBeenCalledTimes(4);
	});

	it('opens tabs again once the user did something, up to the cap again', async () => {
		const open = vi.fn();
		await listenerStore.init(open);
		for (let i = 0; i < AUTO_OPEN_BURST + 5; i++) received('AA');
		expect(open).toHaveBeenCalledTimes(AUTO_OPEN_BURST);
		listenerStore.noteActivity();
		for (let i = 0; i < AUTO_OPEN_BURST + 5; i++) received('AA');
		expect(open).toHaveBeenCalledTimes(2 * AUTO_OPEN_BURST);
		// The notice keeps counting what was not opened until it is closed.
		expect(listenerStore.heldBack).toBe(10);
		listenerStore.resumeAutoOpen();
		expect(listenerStore.heldBack).toBe(0);
		received('AA');
		expect(open).toHaveBeenCalledTimes(2 * AUTO_OPEN_BURST + 1);
	});

	it('a user who keeps working gets every message in a tab', async () => {
		const open = vi.fn();
		await listenerStore.init(open);
		for (let i = 0; i < 3 * AUTO_OPEN_BURST; i++) {
			received('AA');
			if (i % 10 === 9) listenerStore.noteActivity();
		}
		expect(open).toHaveBeenCalledTimes(3 * AUTO_OPEN_BURST);
		expect(listenerStore.heldBack).toBe(0);
	});

	it('holds nothing back with the toggle off: nothing is opened at all', async () => {
		const open = vi.fn();
		await listenerStore.init(open);
		listenerStore.autoOpen = false;
		for (let i = 0; i < 50; i++) received('AA');
		expect(open).not.toHaveBeenCalled();
		expect(listenerStore.heldBack).toBe(0);
	});
});
