import { describe, expect, it } from 'vitest';
import { setLocale, t } from '$lib/i18n';
import { ackFamily, ackLabel, basicAuthorization, httpBodyFor, isLocalBindAddress, listenHost, listenSettingsProblem, replyLabel, replyProblemText, sendErrorText, unfollowedRedirectText, wireSize } from './comm-helpers';

describe('send errors', () => {
	const kinds = [
		'certUntrusted', 'certExpired', 'certWrongHost', 'tlsFailed', 'refused',
		'hostNotFound', 'timedOut', 'proxyRefused', 'proxyAuth', 'reset',
	];

	it('says why in the user\'s language, with the technical reason', () => {
		setLocale('it');
		const text = sendErrorText(
			'The server\'s certificate is not trusted: …',
			{ kind: 'certUntrusted', detail: 'invalid peer certificate: UnknownIssuer' },
			t,
		);
		expect(text).toContain('Il certificato del server non è attendibile');
		expect(text).toContain('UnknownIssuer');
		setLocale('en');
	});

	it('has a text for every kind the backend names, in every language', () => {
		for (const locale of ['en', 'it', 'fr', 'es', 'de'] as const) {
			setLocale(locale);
			for (const kind of kinds) {
				const text = sendErrorText('fallback', { kind, detail: 'DETAIL' }, t);
				expect(text, `${locale} ${kind}`).not.toBe('fallback');
				expect(text, `${locale} ${kind}`).toContain('DETAIL');
			}
		}
		setLocale('en');
	});

	it('falls back to the backend message', () => {
		expect(sendErrorText('Request failed: boom', null, t)).toBe('Request failed: boom');
		expect(sendErrorText('Request failed: boom', { kind: 'other', detail: 'x' }, t)).toBe('Request failed: boom');
		expect(sendErrorText(null, undefined, t)).toBe('');
	});
});

describe('MLLP replies that accept nothing sent', () => {
	it('does not label the ACK of another message by its code', () => {
		const other = { kind: 'other_message' as const, acknowledged: 'SOMEONE_ELSE', sent: 'MSG1' };
		expect(replyLabel('AA', other, t)).toBe('ACK of another message');
		expect(replyLabel('AA', null, t)).toBe('ACK (Accept)');
		expect(replyLabel(undefined, { kind: 'not_an_ack' }, t)).toBe('Response');
		// An AA naming no message is not labelled an acceptance either.
		expect(replyLabel('AA', { kind: 'unmatched', sent: 'MSG1' }, t)).toBe('ACK naming no message');
	});

	it('says why in every language', () => {
		for (const locale of ['en', 'it', 'fr', 'es', 'de'] as const) {
			setLocale(locale);
			const other = replyProblemText({ kind: 'other_message', acknowledged: 'SOMEONE_ELSE', sent: 'MSG1' }, t);
			expect(other, locale).toContain('SOMEONE_ELSE');
			expect(other, locale).toContain('MSG1');
			expect(replyProblemText({ kind: 'not_an_ack' }, t), locale).not.toBe('comm.replyNotAck');
			expect(replyLabel('AA', { kind: 'other_message', acknowledged: 'A', sent: 'B' }, t), locale).not.toBe('comm.replyLabelOtherMessage');
			expect(replyProblemText({ kind: 'unmatched', sent: 'MSG1' }, t), locale).toContain('MSG1');
			expect(replyProblemText({ kind: 'unmatched', sent: 'MSG1' }, t), locale).toContain('MSA-2');
			expect(replyLabel('AA', { kind: 'unmatched', sent: 'MSG1' }, t), locale).not.toBe('comm.replyLabelUnmatched');
		}
		setLocale('en');
	});
});

describe('an HTTP redirect left unfollowed', () => {
	it('blames the setting when Follow Redirects was off, not another server', () => {
		expect(unfollowedRedirectText(302, 'http://h/b', false, t)).toContain("'Follow Redirects' is off");
		expect(unfollowedRedirectText(302, 'http://other/b', true, t)).toContain('another server');
		expect(unfollowedRedirectText(200, 'http://h/b', false, t)).toBeNull();
		expect(unfollowedRedirectText(304, undefined, false, t)).toBeNull();
	});
});

describe('listener settings', () => {
	it('refuses a cleared, fractional or out-of-range port or read timeout before Start', () => {
		expect(listenSettingsProblem(2576, 30)).toBeNull();
		// 0: the system picks a free port, which the status then shows.
		expect(listenSettingsProblem(0, 30)).toBeNull();
		expect(listenSettingsProblem(-1, 30)).toBe('comm.listenPortInvalid');
		expect(listenSettingsProblem(70000, 30)).toBe('comm.listenPortInvalid');
		expect(listenSettingsProblem(null, 30)).toBe('comm.listenPortInvalid');
		expect(listenSettingsProblem(2576, null)).toBe('comm.listenReadTimeoutInvalid');
		expect(listenSettingsProblem(2576, undefined)).toBe('comm.listenReadTimeoutInvalid');
		expect(listenSettingsProblem(2576, Number.NaN)).toBe('comm.listenReadTimeoutInvalid');
		expect(listenSettingsProblem(2576, 0)).toBe('comm.listenReadTimeoutInvalid');
		expect(listenSettingsProblem(2576, 2.5)).toBe('comm.listenReadTimeoutInvalid');
		expect(listenSettingsProblem(2576, 600)).toBeNull();
	});

	it('shows an IPv6 listening address in brackets before the port', () => {
		expect(listenHost('127.0.0.1')).toBe('127.0.0.1');
		expect(listenHost('::1')).toBe('[::1]');
		expect(listenHost('fe80::1%3')).toBe('[fe80::1%3]');
		expect(listenHost('[::1]')).toBe('[::1]');
		expect(listenHost(null)).toBe('');
	});
});

describe('HTTP Basic authentication', () => {
	it('encodes the user and password as UTF-8', () => {
		expect(basicAuthorization('alice', 'pässwörd')).toBe('Basic YWxpY2U6cMOkc3N3w7ZyZA==');
		// btoa alone threw on characters outside Latin-1.
		expect(basicAuthorization('alice', 'pw€')).toBe('Basic YWxpY2U6cHfigqw=');
		expect(basicAuthorization('Aladdin', 'open sesame')).toBe('Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==');
	});
});

describe('communication helpers', () => {
	it('groups commit-mode codes with the application ones', () => {
		expect(ackFamily('CA')).toBe('AA');
		expect(ackFamily('CE')).toBe('AE');
		expect(ackFamily('cr')).toBe('AR');
		expect(ackFamily(null)).toBeNull();
		expect(ackFamily('XX')).toBeNull();
	});

	it('labels replies by MSA-1, not by searching for "MSA|AA"', () => {
		expect(ackLabel('AR')).toBe('NACK (Application Reject)');
		expect(ackLabel('CA')).toBe('Commit ACK (Accept)');
		expect(ackLabel('CE')).toBe('Commit NACK (Error)');
		expect(ackLabel(undefined)).toBe('Response');
	});

	it('does not put the active message in a GET or DELETE body', () => {
		expect(httpBodyFor('GET', '', 'MSH|^~\\&|A')).toBeUndefined();
		expect(httpBodyFor('delete', '', 'MSH|^~\\&|A')).toBeUndefined();
		expect(httpBodyFor('POST', '', 'MSH|^~\\&|A')).toBe('MSH|^~\\&|A');
		expect(httpBodyFor('GET', ' {"q":1} ', 'MSH')).toBe('{"q":1}');
		expect(httpBodyFor('PUT', '', '')).toBeUndefined();
	});

	it('counts UTF-8 bytes with CR segment ends', () => {
		expect(wireSize('MSH|^~\\&|Müller')).toBe(17);
		expect(wireSize('MSH|a\r\nPID|1\r\n')).toBe(12);
		expect(wireSize('{\n"a": "é"\n}')).toBe(13);
		// Not an HL7 header (a letter after MSH): sent as it is, CRLF kept,
		// as the backend's to_wire_segments does.
		expect(wireSize('MSHA\r\nMSHA\r\n')).toBe(12);
		expect(wireSize('MSH \r\nx')).toBe(7);
		expect(wireSize('\uFEFF\r\nMSH|a\nPID|1')).toBe(12);
	});

	it('counts the final CR and leaves blank lines out, as the CLI sends', () => {
		// 'MSH|a\rPID|1\r'
		expect(wireSize('MSH|a\nPID|1\n\n\n')).toBe(12);
		expect(wireSize('MSH|a\rPID|1')).toBe(12);
		expect(wireSize('MSH|a\r\n\r\n  \r\nPID|1')).toBe(12);
	});
});


describe('isLocalBindAddress', () => {
	it('takes every loopback spelling as this computer only', () => {
		for (const a of ['', '  ', 'localhost', 'LOCALHOST', '127.0.0.1', '127.0.0.2', '::1', '[::1]', ' [::1] ', '0:0:0:0:0:0:0:1', '[0000::0001]', '::ffff:127.0.0.1']) {
			expect(isLocalBindAddress(a), a).toBe(true);
		}
	});
	it('takes any other address as reachable from the network', () => {
		for (const a of ['0.0.0.0', '::', '[::]', '192.168.1.10', 'fe80::1%3', '[fe80::1%3]', '::2', '10.0.0.1', 'myhost']) {
			expect(isLocalBindAddress(a), a).toBe(false);
		}
	});
});
