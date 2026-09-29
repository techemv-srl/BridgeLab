import { describe, expect, it, vi } from 'vitest';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
import { importCounts, type ImportPlanItem } from './testcases';

const item = (index: number, status: ImportPlanItem['status']): ImportPlanItem =>
	({ index, id: `id${index}`, name: `n${index}`, category: 'general', status, existing_name: status === 'conflict' ? 'old' : null });

describe('importCounts', () => {
	it('adds new cases, skips identical ones and follows the conflict choices', () => {
		const items = [item(0, 'new'), item(1, 'identical'), item(2, 'conflict'), item(3, 'conflict'), item(4, 'conflict')];
		expect(importCounts(items, { 2: 'overwrite', 3: 'copy' })).toEqual({ add: 2, update: 1, skip: 2 });
	});

	it('skips a conflict with no choice', () => {
		expect(importCounts([item(0, 'conflict')], {})).toEqual({ add: 0, update: 0, skip: 1 });
	});
});
