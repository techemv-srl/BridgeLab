import { invoke } from '@tauri-apps/api/core';
import { t } from '$lib/i18n';

/** A complete HL7 v2 example shipped with the app (see src-tauri/src/samples.rs). */
export interface Sample {
	id: string;
	name: string;
	/** MSH-9 as in the message, e.g. "ADT^A01^ADT_A01". */
	message_type: string;
	/** MSH-12. */
	version: string;
	category: string;
	description: string;
	/** Segments separated by CR. */
	content: string;
}

export async function getSamples(): Promise<Sample[]> {
	return invoke('get_samples');
}

/** Short tab label for a sample: its trigger ("ADT^A01") and version. */
export function sampleTabLabel(s: Sample): string {
	const trigger = s.message_type.split('^').slice(0, 2).join('^');
	return `${trigger} v${s.version}`;
}

/** Translation for `key`, or `fallback` when no locale knows the key. */
function trOr(key: string, fallback: string): string {
	const v = t(key);
	return v === key ? fallback : v;
}

/** The backend's category titles (src-tauri/src/samples.rs) → i18n key suffix. */
export const SAMPLE_CATEGORY_KEYS: Record<string, string> = {
	'Admission / Discharge / Transfer': 'adt',
	'Observations / Results': 'results',
	Orders: 'orders',
	'Scheduling, documents, billing, immunization': 'other',
	Acknowledgment: 'ack',
};

/** The backend's names are English: show them in the UI language
 *  (a sample the translations do not know keeps its own text). */
export const sampleName = (s: Pick<Sample, 'id' | 'name'>) => trOr(`samples.name.${s.id}`, s.name);
export const sampleDescription = (s: Pick<Sample, 'id' | 'description'>) =>
	trOr(`samples.desc.${s.id}`, s.description);
export const sampleCategory = (c: string) =>
	SAMPLE_CATEGORY_KEYS[c] ? trOr(`samples.cat.${SAMPLE_CATEGORY_KEYS[c]}`, c) : c;
