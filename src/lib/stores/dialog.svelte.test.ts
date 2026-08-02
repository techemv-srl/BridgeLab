import { describe, it, expect, beforeEach } from 'vitest';
import { dialogStore } from './dialog.svelte';

beforeEach(() => {
	// Drain any dialog left over from a previous test
	dialogStore.close(false);
});

describe('show / close', () => {
	it('resolves true when confirmed', async () => {
		const p = dialogStore.show({ message: 'proceed?' });
		expect(dialogStore.active?.message).toBe('proceed?');
		dialogStore.close(true);
		await expect(p).resolves.toBe(true);
		expect(dialogStore.active).toBeNull();
	});

	it('resolves false when cancelled', async () => {
		const p = dialogStore.show({ kind: 'confirm', message: 'delete?', showCancel: true });
		dialogStore.close(false);
		await expect(p).resolves.toBe(false);
	});

	it('defaults kind to info', () => {
		void dialogStore.show({ message: 'hello' });
		expect(dialogStore.active?.kind).toBe('info');
		dialogStore.close(true);
	});

	it('close without an active dialog is a no-op', () => {
		expect(() => dialogStore.close(true)).not.toThrow();
		expect(dialogStore.active).toBeNull();
	});

	it('assigns increasing ids to successive dialogs', () => {
		void dialogStore.show({ message: 'one' });
		const first = dialogStore.active!.id;
		dialogStore.close(true);
		void dialogStore.show({ message: 'two' });
		expect(dialogStore.active!.id).toBeGreaterThan(first);
		dialogStore.close(true);
	});
});

describe('shortcuts', () => {
	it('info sets kind and title', async () => {
		const p = dialogStore.info('saved', 'Done');
		expect(dialogStore.active).toMatchObject({ kind: 'info', message: 'saved', title: 'Done' });
		dialogStore.close(true);
		await p;
	});

	it('error carries details for the expandable section', async () => {
		const p = dialogStore.error('save failed', 'Error', 'disk full');
		expect(dialogStore.active).toMatchObject({
			kind: 'error', message: 'save failed', details: 'disk full',
		});
		dialogStore.close(true);
		await p;
	});

	it('warning sets kind', async () => {
		const p = dialogStore.warning('careful');
		expect(dialogStore.active?.kind).toBe('warning');
		dialogStore.close(true);
		await p;
	});

	it('confirm shows cancel and returns the choice', async () => {
		const p = dialogStore.confirm('sure?');
		expect(dialogStore.active?.showCancel).toBe(true);
		dialogStore.close(true);
		await expect(p).resolves.toBe(true);
	});
});
