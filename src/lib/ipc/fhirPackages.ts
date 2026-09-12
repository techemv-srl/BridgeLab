// FHIR profile packages — IPC bindings.
//
// A package is a FHIR NPM `.tgz` (hl7.fhir.r4.core, a national IG, a site's
// own profiles). Installing one distils its StructureDefinitions into
// <config>/BridgeLab/fhir-packages/ so startup does not re-read the archive.

import { invoke } from '@tauri-apps/api/core';

export interface FhirPackageInfo {
	name: string;
	version: string;
	title: string;
	fhir_version: string;
	profile_count: number;
}

export async function listFhirPackages(): Promise<FhirPackageInfo[]> {
	return invoke('fhir_packages_list');
}

export async function reloadFhirPackages(): Promise<FhirPackageInfo[]> {
	return invoke('fhir_packages_reload');
}

/** Install from a `.tgz` on disk. Slow for a core package — show progress. */
export async function installFhirPackage(path: string): Promise<FhirPackageInfo[]> {
	return invoke('fhir_packages_install', { path });
}

export async function removeFhirPackage(name: string, version: string): Promise<FhirPackageInfo[]> {
	return invoke('fhir_packages_remove', { name, version });
}

export async function fhirPackagesDir(): Promise<string> {
	return invoke('fhir_packages_dir');
}
