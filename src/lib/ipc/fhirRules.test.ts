import { describe, it, expect, afterEach } from 'vitest';
import { describeRuleError } from './fhirRules';
import { parseUpgradeError } from './licensing';
import { setLocale, t } from '$lib/i18n';

describe('describeRuleError', () => {
	afterEach(() => setLocale('en'));

	const refusal = {
		code: 'bad_pattern',
		params: { id: 'r1', detail: 'unclosed group' },
		message: "Rule 'r1': the pattern does not compile (unclosed group)",
	};

	it('shows a refusal in the interface language, with its values', () => {
		setLocale('it');
		expect(describeRuleError(refusal, t, parseUpgradeError)).toBe("Regola 'r1': l'espressione regolare non è valida (unclosed group).");
		setLocale('de');
		const dup = { code: 'duplicate_id', params: { id: 'r2' }, message: "Two rules share the id 'r2'" };
		expect(describeRuleError(dup, t, parseUpgradeError)).toContain("'r2'");
		expect(describeRuleError(dup, t, parseUpgradeError)).toContain('Zwei Regeln');
	});

	it('every refusal code the backend sends has a string in every language', () => {
		const codes = [
			'no_id', 'no_message', 'bad_severity', 'bad_pattern', 'bad_fhirpath', 'path_needs_check', 'both_forms',
			'no_form', 'duplicate_id', 'changed_on_disk', 'invalid_pack', 'invalid_pack_unsaved', 'could_not_read',
			'could_not_write', 'no_resource',
		];
		for (const locale of ['en', 'it', 'es', 'de', 'fr'] as const) {
			setLocale(locale);
			for (const code of codes) {
				expect(t(`fhirRules.refusal.${code}`), `${locale} ${code}`).not.toBe(`fhirRules.refusal.${code}`);
			}
		}
	});

	it('passes English on when there is no translation, and still spots an upgrade notice', () => {
		const other = { code: 'other', params: {}, message: 'Could not determine the config directory' };
		expect(describeRuleError(other, t, parseUpgradeError)).toBe('Could not determine the config directory');
		const upgrade = { code: 'other', params: {}, message: 'UPGRADE_REQUIRED:fhir_rules_builder:pro:Pro needed' };
		expect(describeRuleError(upgrade, t, parseUpgradeError)).toBe(t('upgrade.required', { tier: 'pro' }));
		expect(describeRuleError({ code: 'brand_new', params: {}, message: 'New reason' }, t, parseUpgradeError)).toBe('New reason');
		expect(describeRuleError('plain string error', t, parseUpgradeError)).toBe('plain string error');
	});
});
