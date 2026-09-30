import { readFileSync } from 'node:fs';
import { afterEach, describe, expect, it, vi } from 'vitest';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
import { setLocale, type Locale } from '$lib/i18n';
import en from '$lib/i18n/en.json';
import itJson from '$lib/i18n/it.json';
import fr from '$lib/i18n/fr.json';
import es from '$lib/i18n/es.json';
import de from '$lib/i18n/de.json';
import { SAMPLE_CATEGORY_KEYS, sampleCategory, sampleDescription, sampleName } from './samples';

// The catalogue as the backend declares it: parsed from the Rust source so a
// sample added there without translations fails here.
const rust = readFileSync(new URL('../../../src-tauri/src/samples.rs', import.meta.url), 'utf8');
const consts = new Map([...rust.matchAll(/^const (\w+): &str = "([^"]+)";/gm)].map((m) => [m[1], m[2]]));
const samples = [
	...rust.matchAll(/sample!\("([^"]+)", "([^"]+)", "[^"]+", "[^"]+", (\w+),\s*"([^"]+)"/g),
].map((m) => ({ id: m[1], name: m[2], category: consts.get(m[3])!, description: m[4] }));

const locales: Record<Locale, Record<string, string>> = { en, it: itJson, fr, es, de };

afterEach(() => setLocale('en'));

describe('sample library translations', () => {
	it('parses the whole catalogue from samples.rs', () => {
		expect(samples.length).toBeGreaterThanOrEqual(13);
		for (const s of samples) expect(s.category, s.id).toBeTruthy();
	});

	it('has a name and a description for every sample in every locale', () => {
		for (const [loc, dict] of Object.entries(locales)) {
			for (const s of samples) {
				expect(dict[`samples.name.${s.id}`], `${loc} name ${s.id}`).toBeTruthy();
				expect(dict[`samples.desc.${s.id}`], `${loc} desc ${s.id}`).toBeTruthy();
			}
		}
	});

	it('has every backend category in every locale', () => {
		const cats = new Set(samples.map((s) => s.category));
		expect(new Set(Object.keys(SAMPLE_CATEGORY_KEYS))).toEqual(cats);
		for (const [loc, dict] of Object.entries(locales)) {
			for (const c of cats) expect(dict[`samples.cat.${SAMPLE_CATEGORY_KEYS[c]}`], `${loc} ${c}`).toBeTruthy();
		}
	});

	it('keeps the English texts identical to the backend', () => {
		for (const s of samples) {
			expect(sampleName(s)).toBe(s.name);
			expect(sampleDescription(s)).toBe(s.description);
			expect(sampleCategory(s.category)).toBe(s.category);
		}
	});

	it('translates in another locale', () => {
		setLocale('it');
		expect(sampleName({ id: 'adt-a01-251', name: 'Inpatient admission' })).toBe('Ricovero ordinario');
		expect(sampleCategory('Admission / Discharge / Transfer')).toBe('Ricovero / Dimissione / Trasferimento');
	});

	it('falls back to the backend text for an unknown sample or category', () => {
		setLocale('de');
		const s = { id: 'zzz-unknown', name: 'Custom sample', description: 'Not translated' };
		expect(sampleName(s)).toBe('Custom sample');
		expect(sampleDescription(s)).toBe('Not translated');
		expect(sampleCategory('Something new')).toBe('Something new');
	});
});
