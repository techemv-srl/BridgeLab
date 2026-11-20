import { invoke } from '@tauri-apps/api/core';

export interface MllpSendResult {
	success: boolean;
	response: string;
	response_time_ms: number;
	error: string | null;
	/** Charset the message was encoded in on the wire. */
	encoding?: string;
	/** MSA-1 of the reply (AA/AE/AR/CA/CE/CR), when there was one. */
	ack_code?: string | null;
	/** Why the reply acknowledges nothing sent: it is not HL7, or its MSA-2
	 *  names another message. Absent when it does, or when no reply came. */
	reply_problem?: ReplyProblem | null;
}

/** Why an MLLP reply is not an acknowledgement of the message sent. */
export type ReplyProblem =
	| { kind: 'not_an_ack' }
	| { kind: 'other_message'; acknowledged: string; sent: string }
	/** An AA or CA with MSA-2 empty: it names no message. */
	| { kind: 'unmatched'; sent: string };

export interface HttpResult {
	success: boolean;
	status_code: number;
	status_text: string;
	headers: Record<string, string>;
	body: string;
	response_time_ms: number;
	error: string | null;
	/** The URL that answered, when redirects on the same server were followed. */
	final_url?: string | null;
	/** Why no answer came, when the backend could tell. */
	failure?: SendFailure | null;
}

/** Why a request got no answer: a refused certificate, a closed port… */
export interface SendFailure {
	/** certUntrusted, certExpired, certWrongHost, tlsFailed, refused,
	 *  hostNotFound, timedOut, proxyRefused, proxyAuth or reset. */
	kind: string;
	/** The technical reason, as the TLS library or the OS gave it. */
	detail: string;
}

export interface ConnectionProfile {
	id: string;
	name: string;
	profile_type: 'mllp' | 'http' | 'soap';
	host: string;
	port: number;
	timeout_secs: number;
	url: string | null;
	headers: string | null;
	auto_ack: boolean;
}

export interface HistoryEntry {
	id: string;
	profile_name: string;
	profile_type: string;
	direction: string;
	content_preview: string;
	status: string;
	response_time_ms: number;
	timestamp: string;
	/** MSA-1 of the ACK an MLLP send got back ("AA", "AE", "AR"…); null when
	 *  there was no ACK to read — a failed send, or an HTTP/SOAP request. */
	ack_code: string | null;
	/** host:port for MLLP (the peer for a received message), the URL for
	 *  HTTP/SOAP with credentials redacted; empty in older entries. */
	target?: string;
	/** Size of the message sent or received, in bytes. */
	size_bytes?: number;
	/** The message sent or received (cut past 256 KB). */
	request?: string;
	/** The ACK, HTTP response body, or SOAP Body/Fault (cut past 256 KB). */
	response?: string;
}

/** How many entries the History keeps. */
export const HISTORY_KEPT = 100;

// --- MLLP ---
export interface MllpSendOptions {
	/** TCP connect timeout (seconds). */
	timeoutSecs?: number;
	/** ACK read timeout (seconds); defaults to timeoutSecs backend-side. */
	responseTimeoutSecs?: number;
	/** 'auto' (MSH-18, else `sourceCharset`, else UTF-8) or a charset label. */
	encoding?: string;
	/** Charset the tab's file was read in, used by 'auto' when MSH-18 is empty. */
	sourceCharset?: string | null;
	/** Framing byte overrides as hex strings ("0x0B"); invalid values fall back to standard MLLP. */
	startChar?: string;
	endChar1?: string;
	endChar2?: string;
	profileName?: string;
}

export async function mllpSend(
	host: string, port: number, message: string,
	opts: MllpSendOptions = {},
): Promise<MllpSendResult> {
	return invoke('mllp_send', {
		host, port, message,
		timeoutSecs: opts.timeoutSecs,
		responseTimeoutSecs: opts.responseTimeoutSecs,
		encoding: opts.encoding,
		startChar: opts.startChar,
		endChar1: opts.endChar1,
		endChar2: opts.endChar2,
		profileName: opts.profileName,
		sourceCharset: opts.sourceCharset ?? undefined,
	});
}

/** Bytes `mllpSend` would put on the wire for `message`, in the encoding
 *  the send resolves (MSH-18's in "auto" mode); nothing is sent. */
export async function mllpWireSize(message: string, encoding?: string, sourceCharset?: string | null): Promise<number> {
	return invoke('mllp_wire_size', { message, encoding, sourceCharset: sourceCharset ?? undefined });
}

// --- Persistent MLLP listener ---
export interface ListenerConfig {
	port: number;
	bind_address: string;
	auto_ack: boolean;
	ack_code: string;       // 'AA' | 'AE' | 'AR'
	read_timeout_secs: number;
	encoding: string;       // 'auto' | 'UTF-8' | 'ISO-8859-1' | 'windows-1252' | etc.
}

/** Payload of `mllp:connection_idle`: a connection that sent nothing
 *  within the read timeout was closed. */
export interface MllpIdleEvent {
	source_addr: string;
	idle_secs: number;
}

export interface ListenerStatus {
	running: boolean;
	port: number | null;
	bind_address: string | null;
}

/** Payload of the `mllp:received` Tauri event. */
export interface MllpReceivedEvent {
	content: string;
	source_addr: string;
	received_at: string;
	/** Payload size after MLLP unframing, in bytes. */
	bytes: number;
	/** ACK code sent back ("AA"/"AE"/"AR"), or null when auto-ACK is off. */
	ack_code: string | null;
	/** Charset the payload was decoded with. */
	encoding: string;
}

export async function mllpListenStart(config: ListenerConfig): Promise<ListenerStatus> {
	return invoke('mllp_listen_start', { config });
}

export async function mllpListenStop(): Promise<ListenerStatus> {
	return invoke('mllp_listen_stop');
}

export async function mllpListenStatus(): Promise<ListenerStatus> {
	return invoke('mllp_listen_status');
}

// --- HTTP ---
export async function httpRequest(
	url: string, method: string,
	headers?: Record<string, string>, body?: string,
	timeoutSecs?: number, followRedirects?: boolean, profileName?: string,
): Promise<HttpResult> {
	return invoke('http_request', { url, method, headers, body, timeoutSecs, followRedirects, profileName });
}

// --- ACK ---
/** The ACK of `message`: sender and receiver swapped, its trigger event,
 *  processing ID, version, charset and separators mirrored, MSA-2 its
 *  MSH-10. Rejects when the message has no MSH-10. */
export async function generateAck(
	ackCode: string, message: string, textMessage?: string,
): Promise<string> {
	return invoke('generate_ack', { ackCode, message, textMessage });
}

// --- Profiles ---
export async function saveConnectionProfile(profile: ConnectionProfile): Promise<void> {
	return invoke('save_connection_profile', { profile });
}

export async function getConnectionProfiles(): Promise<ConnectionProfile[]> {
	return invoke('get_connection_profiles');
}

export async function deleteConnectionProfile(id: string): Promise<void> {
	return invoke('delete_connection_profile', { id });
}

// --- History ---
export async function getRequestHistory(limit?: number): Promise<HistoryEntry[]> {
	return invoke('get_request_history', { limit });
}

export async function clearRequestHistory(): Promise<void> {
	return invoke('clear_request_history');
}
