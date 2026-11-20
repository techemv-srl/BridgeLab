import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
	mllpListenStart, mllpListenStop, mllpListenStatus,
	type ListenerConfig, type ListenerStatus, type MllpReceivedEvent, type MllpIdleEvent,
} from '$lib/ipc/communication';
import { ackFamily } from '$lib/components/communication/comm-helpers';
import { t } from '$lib/i18n';
import { ASK_ABOVE_BYTES } from '$lib/large-content';

/** One row of the listener console: a received message, a listener error,
 *  or an ordinary event (a connection that sent nothing was closed). */
export interface ConsoleEntry {
	id: number;
	time: string;            // local HH:MM:SS
	kind: 'msg' | 'error' | 'note';
	peer?: string;
	bytes?: number;
	ack?: string | null;     // ACK code sent, null = auto-ACK off
	encoding?: string;
	snippet?: string;        // first line, always retained
	content?: string;        // full message for click-to-open; evicted by byte budget
	text?: string;           // error or note text
}

/** The outcome filters over the console (and the send history). */
export type AckFilter = 'all' | 'AA' | 'AE' | 'AR' | 'noack' | 'fail';
export const ACK_FILTERS: AckFilter[] = ['all', 'AA', 'AE', 'AR', 'noack', 'fail'];

/** Rows the console keeps, newest first, so an unattended listener cannot
 *  grow without bound. */
export const CONSOLE_CAP = 200;
/** Total bytes of full message contents retained for click-to-open. The
 *  backend accepts payloads up to 10 MiB each, so the row cap alone could
 *  pin gigabytes in renderer state during an unattended high-volume
 *  session. Oldest rows lose `content` first (the snippet row stays). */
export const CONSOLE_CONTENT_BUDGET = 32 * 1024 * 1024;

/** Received messages opened in a tab of their own while the user does
 *  nothing; later ones only reach the console (and the History) until the
 *  user is back. Every tab costs the window: 500 at once froze it for
 *  about half a minute, 1000 for three minutes. Enough for any message
 *  someone is there to read. */
export const AUTO_OPEN_BURST = 20;

/** What the received messages opened in tabs by themselves may come to,
 *  in bytes, while the user does nothing: the size a file opens at without
 *  a question. A message of 10 MB takes the window a second or two and
 *  hundreds of MB of memory; twenty of them in a row would take several
 *  GB. A message alone always opens (the listener accepts up to 10 MB). */
export const AUTO_OPEN_BYTES = ASK_ABOVE_BYTES;

/** Console rows: `fail` is a listener error row, `noack` a message received
 *  with auto-ACK off. The commit-mode codes count with AA, AE and AR. A
 *  note counts under `all` only. */
export function consoleMatches(e: ConsoleEntry, f: AckFilter): boolean {
	switch (f) {
		case 'all': return true;
		case 'fail': return e.kind === 'error';
		case 'noack': return e.kind === 'msg' && !e.ack;
		default: return e.kind === 'msg' && ackFamily(e.ack) === f;
	}
}

function zeroCounts(): Record<AckFilter, number> {
	return { all: 0, AA: 0, AE: 0, AR: 0, noack: 0, fail: 0 };
}

function localTime(iso: string): string {
	try { return new Date(iso).toLocaleTimeString(); } catch { return iso; }
}

function snippetOf(content: string): string {
	return (content.split(/\r|\n/, 1)[0] ?? '').substring(0, 80);
}

/**
 * The MLLP listener as the app sees it: status, the console and its
 * counts, and opening received messages in a tab. The backend keeps
 * listening and acknowledging whatever the frontend shows; this lived in
 * the Communication panel, so closing the panel stopped received messages
 * from opening in a tab or reaching the console, and reopening it showed
 * an empty console and "0 received". Here it lives as long as the app,
 * subscribed once; the panel only shows it.
 */
class ListenerStore {
	status = $state<ListenerStatus>({ running: false, port: null, bind_address: null });
	/** The last listener error (start refused, or a connection failure). */
	error = $state<string | null>(null);
	/** Console rows, newest first, at most CONSOLE_CAP. Replaced, never
	 *  changed in place. */
	entries = $state.raw<ConsoleEntry[]>([]);
	/** Rows per outcome since the console was last cleared, the ones past
	 *  the row cap included: computed from the kept rows, "AE 12" out of
	 *  300 read "AE 0" once the AE rows had been dropped. */
	counts = $state<Record<AckFilter, number>>(zeroCounts());
	/** Messages received since the console was last cleared. */
	received = $state(0);
	/** Open each received message in a new tab (the console toggle), up to
	 *  AUTO_OPEN_BURST while the user does nothing. */
	autoOpen = $state(true);
	/** Received messages not opened in a tab since the burst cap was
	 *  reached: the notice that says so shows while this is above 0. */
	heldBack = $state(0);
	/** What the listener was last started with: the panel's form shows it
	 *  again after the panel was closed and reopened. */
	lastConfig = $state<ListenerConfig | null>(null);

	private nextId = 0;
	/** Tabs opened by the listener since the user last did something. */
	private autoOpened = 0;
	/** What those tabs' messages came to, in bytes. */
	private autoOpenedBytes = 0;
	private opener: ((content: string) => void) | null = null;
	private subscribed: Promise<void> | null = null;
	private unlisten: UnlistenFn[] = [];

	/**
	 * Subscribe to the backend's listener events and read its status. Only
	 * the first call subscribes; later ones (a reload of the shell) only
	 * replace `openMessage`, so no message is ever handled twice.
	 */
	init(openMessage: (content: string) => void): Promise<void> {
		this.opener = openMessage;
		if (!this.subscribed) {
			this.subscribed = (async () => {
				// A listener may already run (the frontend was reloaded with
				// the backend alive).
				await this.refreshStatus();
				try {
					this.unlisten.push(await listen<MllpReceivedEvent>('mllp:received', (ev) => this.receive(ev.payload)));
					// Failed or refused connections and messages, and accept()
					// failures the listener retries: it is still listening.
					this.unlisten.push(await listen<string>('mllp:listen_error', (ev) => this.fail(ev.payload)));
					// The listener ended on its own and accepts nothing any more.
					this.unlisten.push(await listen<string>('mllp:listen_stopped', (ev) => void this.stopped(ev.payload)));
					// A connection that sent nothing was closed: no error.
					this.unlisten.push(await listen<MllpIdleEvent>('mllp:connection_idle', (ev) => this.idle(ev.payload)));
				} catch { /* web mode */ }
			})();
		}
		return this.subscribed;
	}

	/** Unsubscribe and forget everything (tests). */
	dispose(): void {
		for (const u of this.unlisten) u();
		this.unlisten = [];
		this.subscribed = null;
		this.opener = null;
		this.status = { running: false, port: null, bind_address: null };
		this.error = null;
		this.autoOpen = true;
		this.lastConfig = null;
		this.clear();
		this.resumeAutoOpen();
	}

	async refreshStatus(): Promise<void> {
		try { this.status = await mllpListenStatus(); } catch { /* web mode */ }
	}

	/** Start listening; rejects with the backend's error (the panel words it). */
	async start(config: ListenerConfig): Promise<void> {
		this.error = null;
		this.status = await mllpListenStart(config);
		this.lastConfig = { ...config };
	}

	async stop(): Promise<void> {
		this.status = await mllpListenStop();
	}

	/** A message the backend received and acknowledged. */
	receive(ev: MllpReceivedEvent): void {
		this.received += 1;
		this.push({
			time: localTime(ev.received_at),
			kind: 'msg',
			peer: ev.source_addr,
			bytes: ev.bytes,
			ack: ev.ack_code,
			encoding: ev.encoding,
			snippet: snippetOf(ev.content),
			content: ev.content,
		});
		if (!this.autoOpen) return;
		const bytes = ev.bytes;
		const fits = this.autoOpened === 0 || this.autoOpenedBytes + bytes <= AUTO_OPEN_BYTES;
		if (this.autoOpened < AUTO_OPEN_BURST && fits) {
			this.autoOpened += 1;
			this.autoOpenedBytes += bytes;
			this.opener?.(ev.content);
		} else {
			this.heldBack += 1;
		}
	}

	/** The user did something (a key, a click): the tabs opened so far
	 *  have someone to look at them, and the next messages open again. */
	noteActivity(): void {
		this.autoOpened = 0;
		this.autoOpenedBytes = 0;
	}

	/** The notice was read and closed: open received messages again, from
	 *  a full burst. */
	resumeAutoOpen(): void {
		this.autoOpened = 0;
		this.autoOpenedBytes = 0;
		this.heldBack = 0;
	}

	/** A listener error: shown above the console and as a red row. */
	fail(text: string): void {
		this.error = text;
		this.push({ time: new Date().toLocaleTimeString(), kind: 'error', text });
	}

	/** A connection that sent nothing within the read timeout was closed
	 *  to free its place: an ordinary row, not an error (a sender may open
	 *  its connection long before it has a message). */
	idle(ev: MllpIdleEvent): void {
		this.push({
			time: new Date().toLocaleTimeString(),
			kind: 'note',
			peer: ev.source_addr,
			text: t('comm.consoleIdle', { peer: ev.source_addr, secs: ev.idle_secs }),
		});
	}

	/** The backend's listener ended on its own (accept() failed in a way
	 *  it cannot get past). The panel showed "Listening" and Stop while
	 *  every connection was refused: it now shows the listener stopped, with
	 *  the reason. The status is read again because the event may come from
	 *  a listener a new Start has already replaced. */
	async stopped(reason: string): Promise<void> {
		this.status = { running: false, port: null, bind_address: null };
		this.fail(t('comm.listenStopped', { reason }));
		await this.refreshStatus();
	}

	/** Open a message in a tab (a click on a console row). */
	open(content: string): void {
		this.opener?.(content);
	}

	clear(): void {
		this.entries = [];
		this.counts = zeroCounts();
		this.received = 0;
	}

	private push(entry: Omit<ConsoleEntry, 'id'>): void {
		const row = { ...entry, id: this.nextId++ };
		const counts = { ...this.counts };
		for (const f of ACK_FILTERS) if (consoleMatches(row, f)) counts[f] += 1;
		this.counts = counts;
		const entries = [row, ...this.entries.slice(0, CONSOLE_CAP - 1)];
		// Enforce the byte budget newest→oldest: once cumulative content size
		// exceeds it, strip full contents from the older entries.
		let retained = 0;
		for (let i = 0; i < entries.length; i++) {
			const e = entries[i];
			if (!e.content) continue;
			retained += e.bytes ?? e.content.length;
			if (retained > CONSOLE_CONTENT_BUDGET && i > 0) {
				entries[i] = { ...e, content: undefined };
			}
		}
		this.entries = entries;
	}
}

export const listenerStore = new ListenerStore();
