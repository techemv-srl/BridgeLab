import { describe, expect, it, vi } from 'vitest';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
import { defaultGridSegment } from './parser';

const c = (segment_type: string, count: number) => ({ segment_type, count });

describe('defaultGridSegment', () => {
	it('opens on the requested segment when the message has it', () => {
		expect(defaultGridSegment([c('MSH', 1), c('OBX', 12), c('NTE', 2)], 'NTE')).toBe('NTE');
	});
	it('otherwise picks the most repeated segment, never MSH', () => {
		expect(defaultGridSegment([c('MSH', 1), c('PID', 1), c('OBX', 12), c('NTE', 2)], 'ZZZ')).toBe('OBX');
		expect(defaultGridSegment([c('MSH', 1), c('PID', 1)])).toBe('PID');
	});
	it('falls back to MSH only when it is all there is, and to null on nothing', () => {
		expect(defaultGridSegment([c('MSH', 1)])).toBe('MSH');
		expect(defaultGridSegment([])).toBeNull();
	});
});
