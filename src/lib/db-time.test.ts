import { describe, it, expect } from 'vitest';
import { parseDbTime } from './db-time';

describe('parseDbTime', () => {
	it('reads a SQLite datetime as UTC', () => {
		expect(parseDbTime('2026-09-30 11:30:08').toISOString()).toBe('2026-09-30T11:30:08.000Z');
	});

	it('keeps an explicit offset', () => {
		expect(parseDbTime('2026-09-30T13:30:08+02:00').toISOString()).toBe('2026-09-30T11:30:08.000Z');
		expect(parseDbTime('2026-09-30T11:30:08Z').toISOString()).toBe('2026-09-30T11:30:08.000Z');
	});
});
