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
