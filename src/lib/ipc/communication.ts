import { invoke } from '@tauri-apps/api/core';

export interface MllpSendResult {
	success: boolean;
	response: string;
	response_time_ms: number;
	error: string | null;
}

export interface MllpReceivedMessage {
	content: string;
	source_addr: string;
	received_at: string;
}

export interface HttpResult {
	success: boolean;
	status_code: number;
	status_text: string;
	headers: Record<string, string>;
	body: string;
	response_time_ms: number;
	error: string | null;
}

export interface ConnectionProfile {
	id: string;
	name: string;
	profile_type: 'mllp' | 'http';
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
}

// --- MLLP ---
export interface MllpSendOptions {
	/** TCP connect timeout (seconds). */
	timeoutSecs?: number;
	/** ACK read timeout (seconds); defaults to timeoutSecs backend-side. */
	responseTimeoutSecs?: number;
	encoding?: string;
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
	});
}

export async function mllpReceive(
	port: number, timeoutSecs?: number, autoAck?: boolean,
): Promise<MllpReceivedMessage> {
	return invoke('mllp_receive', { port, timeoutSecs, autoAck });
}

// --- Persistent MLLP listener ---
export interface ListenerConfig {
	port: number;
	bind_address: string;
	auto_ack: boolean;
	ack_code: string;       // 'AA' | 'AE' | 'AR'
	read_timeout_secs: number;
	encoding: string;       // 'UTF-8' | 'ISO-8859-1' | 'windows-1252' | etc.
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
export async function generateAck(
	ackCode: string, messageControlId: string, textMessage?: string,
): Promise<string> {
	return invoke('generate_ack', { ackCode, messageControlId, textMessage });
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
