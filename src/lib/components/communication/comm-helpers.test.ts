import { describe, expect, it } from 'vitest';
import { ackFamily, ackLabel, httpBodyFor, wireSize } from './comm-helpers';

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
		expect(wireSize('MSH|^~\\&|Müller')).toBe(16);
		expect(wireSize('MSH|a\r\nPID|1\r\n')).toBe(12);
		expect(wireSize('{\n"a": "é"\n}')).toBe(13);
	});
});
