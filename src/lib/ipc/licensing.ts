import { invoke } from '@tauri-apps/api/core';

export interface LicenseStatus {
	is_valid: boolean;
	license_type: 'trial' | 'free' | 'professional' | 'enterprise' | 'expired';
	days_remaining: number | null;
	licensee: string;
	email: string;
	features: string[];
	message: string;
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
