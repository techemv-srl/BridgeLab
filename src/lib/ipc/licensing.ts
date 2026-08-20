import { invoke } from '@tauri-apps/api/core';

export interface LicenseStatus {
	is_valid: boolean;
	license_type: 'trial' | 'free' | 'professional' | 'enterprise' | 'expired';
	days_remaining: number | null;
	licensee: string;
	email: string;
	features: string[];
	message: string;
	/** Set when the license was obtained via online activation. */
	activation_code: string | null;
	/** License expiry (RFC-3339); null for perpetual licenses and trials. */
	expires_at: string | null;
}

export interface TelemetrySettings {
	enabled: boolean;
	installation_id: string;
	last_sent: string | null;
	counters: Record<string, number>;
}

export async function checkLicense(): Promise<LicenseStatus> {
	return invoke('check_license');
}

export async function activateLicense(
	key: string, licensee: string, email: string,
): Promise<LicenseStatus> {
	return invoke('activate_license', { key, licensee, email });
}

export async function deactivateLicense(): Promise<LicenseStatus> {
	return invoke('deactivate_license');
}

/** Exchange an activation code (BL-PRO-XXXX-XXXX-XXXX) for a signed license. */
export async function activateLicenseOnline(code: string): Promise<LicenseStatus> {
	return invoke('activate_license_online', { code });
}

/** Authoritative (Rust-side) activation-code detection. */
export async function isActivationCode(input: string): Promise<boolean> {
	return invoke('is_activation_code', { input });
}

/**
 * Frontend twin of the Rust detection — used for instant UI hints only;
 * the Rust side decides which activation path actually runs.
 */
export const ACTIVATION_CODE_RE = /^BL-(PRO|ENT)-[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$/;
export function looksLikeActivationCode(input: string): boolean {
	return ACTIVATION_CODE_RE.test(input.trim().toUpperCase().replace(/ /g, ''));
}

export async function getTelemetrySettings(): Promise<TelemetrySettings> {
	return invoke('get_telemetry_settings');
}

export async function setTelemetryEnabled(enabled: boolean): Promise<void> {
	return invoke('set_telemetry_enabled', { enabled });
}

export interface TelemetrySendResult {
	message: string;
	/** The exact JSON payload that was transmitted. */
	payload: unknown;
}

export async function sendTelemetryNow(): Promise<TelemetrySendResult> {
	return invoke('send_telemetry_now');
}

export async function getTelemetryPreview(): Promise<unknown> {
	return invoke('get_telemetry_preview');
}

export async function getHardwareId(): Promise<string> {
	return invoke('get_hardware_id');
}

export async function getAvailableFeatures(): Promise<string[]> {
	return invoke('get_available_features');
}

/**
 * Check whether an IPC error is a feature-gate upgrade prompt.
 * Returns `{ feature, tier }` if yes, `null` if it's a regular error.
 */
export function parseUpgradeError(err: unknown): { feature: string; tier: string } | null {
	const msg = String(err);
	const m = msg.match(/UPGRADE_REQUIRED:(\w+):(\w+):/);
	return m ? { feature: m[1], tier: m[2] } : null;
}
