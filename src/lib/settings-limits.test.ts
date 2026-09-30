import { describe, it, expect } from 'vitest';
import { clampSetting } from './settings-limits';

describe('clampSetting', () => {
	it('keeps values in range', () => {
		expect(clampSetting('fontSize', 400)).toBe(32);
		expect(clampSetting('fontSize', -5)).toBe(8);
		expect(clampSetting('fontSize', 14)).toBe(14);
		expect(clampSetting('autoParseDelay', '20000')).toBe(5000);
		expect(clampSetting('foldThreshold', 0)).toBe(0);
	});

	it('falls back to the default on an empty or unparsable value', () => {
		expect(clampSetting('tabSize', null)).toBe(4);
		expect(clampSetting('tabSize', '')).toBe(4);
		expect(clampSetting('tabSize', 'null')).toBe(4);
		expect(clampSetting('fontSize', 'huge')).toBe(13);
	});

	it('rounds fractions', () => {
		expect(clampSetting('tabSize', 2.6)).toBe(3);
	});
});
