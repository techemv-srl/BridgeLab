import { describe, it, expect, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => undefined) }));
import { invoke } from '@tauri-apps/api/core';
import { trackMessage, setLiveIds, sweep } from './message-gc';

describe('message GC', () => {
	it('releases only messages no tab uses, after the grace period', () => {
		let live = ['a'];
		setLiveIds(() => live);
		trackMessage('a');
		trackMessage('b');
		expect(sweep(Date.now())).toEqual([]); // b is too recent
		expect(sweep(Date.now() + 20_000)).toEqual(['b']);
		expect(invoke).toHaveBeenCalledWith('release_messages', { ids: ['b'] });
		live = [];
		// a was in use until now: its grace period starts from the last sweep that saw it.
		expect(sweep(Date.now() + 25_000)).toEqual([]);
		expect(sweep(Date.now() + 60_000)).toEqual(['a']);
	});
});
