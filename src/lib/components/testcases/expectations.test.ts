import { describe, expect, it } from 'vitest';
import { expectsInvalid, typeMatches } from './expectations';

describe('typeMatches', () => {
	it('compares only the components the expectation gives', () => {
		expect(typeMatches('ADT', 'ADT^A01')).toBe(true);
		expect(typeMatches('ADT^A01', 'ADT^A01^ADT_A01')).toBe(true);
		expect(typeMatches('adt^a01', 'ADT^A01')).toBe(true);
		expect(typeMatches('ADT^A01^ADT_A01', 'ADT^A01^ADT_A01')).toBe(true);
		expect(typeMatches('ADT^A01^ADT_A01', 'ADT^A01')).toBe(false);
		expect(typeMatches('ADT^A04', 'ADT^A01^ADT_A01')).toBe(false);
		expect(typeMatches('AD', 'ADT^A01')).toBe(false);
		expect(typeMatches('', 'anything')).toBe(true);
	});
});

describe('expectsInvalid', () => {
	it('accepts any case of "invalid"', () => {
		expect(expectsInvalid('invalid')).toBe(true);
		expect(expectsInvalid('Invalid')).toBe(true);
		expect(expectsInvalid('valid')).toBe(false);
		expect(expectsInvalid(undefined)).toBe(false);
	});
});
