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
export async function mllpSend(
	host: string, port: number, message: string,
	timeoutSecs?: number, profileName?: string,
): Promise<MllpSendResult> {
	return invoke('mllp_send', { host, port, message, timeoutSecs, profileName });
}

export async function mllpReceive(
	port: number, timeoutSecs?: number, autoAck?: boolean,
): Promise<MllpReceivedMessage> {
	return invoke('mllp_receive', { port, timeoutSecs, autoAck });
}

// --- HTTP ---
export async function httpRequest(
	url: string, method: string,
	headers?: Record<string, string>, body?: string,
	timeoutSecs?: number, profileName?: string,
): Promise<HttpResult> {
	return invoke('http_request', { url, method, headers, body, timeoutSecs, profileName });
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
