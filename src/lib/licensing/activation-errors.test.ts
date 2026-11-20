import { describe, expect, it } from 'vitest';
import { setLocale, t } from '$lib/i18n';
import { activationNetworkError } from './activation-errors';

describe('activation network errors', () => {
	it('tells a refused certificate from an unreachable server', () => {
		setLocale('en');
		const cert = activationNetworkError('ERR_SERVER_CERTIFICATE: invalid peer certificate: UnknownIssuer', t);
		expect(cert).toContain('does not trust its certificate');
		expect(cert).toContain('invalid peer certificate: UnknownIssuer');
		expect(cert).toContain('offline license key');
		expect(activationNetworkError('Error: ERR_SERVER_UNREACHABLE', t)).toBe(t('act.serverUnreachable'));
		expect(activationNetworkError('The license server answered HTTP 502', t)).toBeNull();
	});

	it('has the certificate text in every language', () => {
		for (const locale of ['en', 'it', 'fr', 'es', 'de'] as const) {
			setLocale(locale);
			const text = activationNetworkError('ERR_SERVER_CERTIFICATE: DETAIL', t);
			expect(text, locale).not.toBe('act.serverCertificate');
			expect(text, locale).toContain('DETAIL');
		}
		setLocale('en');
	});
});
