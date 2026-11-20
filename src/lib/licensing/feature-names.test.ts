import { describe, it, expect, afterEach } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
import { featureName } from './feature-names';
import { setLocale, getLocales } from '$lib/i18n';

/** The feature tags the backend grants (feature_gate.rs). */
function backendTags(): string[] {
	const rs = fs.readFileSync(path.resolve(__dirname, '../../../src-tauri/src/licensing/feature_gate.rs'), 'utf8');
	return [...rs.matchAll(/const (?:COMMUNITY|PRO|ENTERPRISE)_FEATURES: &\[&str\] = &\[([\s\S]*?)\];/g)]
		.flatMap((m) => [...m[1].matchAll(/"([a-z0-9_]+)"/g)].map((x) => x[1]));
}

afterEach(() => setLocale('en'));

describe('featureName', () => {
	it('names every feature the backend grants, in every language', () => {
		const tags = backendTags();
		expect(tags).toContain('fhir_parse');
		for (const { code } of getLocales()) {
			setLocale(code);
			for (const tag of tags) {
				const name = featureName(tag);
				expect(name, `${code} ${tag}`).not.toBe(tag);
				expect(name, `${code} ${tag}`).not.toMatch(/^feature\./);
			}
		}
	});

	it('is translated, and shows a tag it does not know as it is', () => {
		expect(featureName('core')).toBe('Editor and tree');
		setLocale('de');
		expect(featureName('core')).toBe('Editor und Baum');
		expect(featureName('quantum_mode')).toBe('quantum_mode');
	});
});
