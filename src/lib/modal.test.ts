import { describe, it, expect } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
import { isModalOpen, MODAL_SELECTOR } from './modal';

function svelteFiles(dir: string): string[] {
	return fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
		const p = path.join(dir, e.name);
		return e.isDirectory() ? svelteFiles(p) : p.endsWith('.svelte') ? [p] : [];
	});
}

describe('isModalOpen', () => {
	it('asks the document for a modal', () => {
		const seen: string[] = [];
		const doc = (found: boolean) => ({ querySelector: (s: string) => { seen.push(s); return found ? ({} as Element) : null; } });
		expect(isModalOpen(doc(true))).toBe(true);
		expect(isModalOpen(doc(false))).toBe(false);
		expect(seen).toEqual([MODAL_SELECTOR, MODAL_SELECTOR]);
		expect(isModalOpen(undefined)).toBe(false);
	});

	it('every dialog in the components draws something the selector finds', () => {
		// The shortcuts stand down only behind what MODAL_SELECTOR matches: a
		// new dialog drawn some other way would let them act behind it.
		const markers = ['modal-overlay', 'modal-backdrop', 'aria-modal="true"'];
		// Not modal: the update notice is a banner over the window's edge.
		const notModal = ['UpdateBanner.svelte'];
		const files = svelteFiles(path.resolve(__dirname, 'components'))
			.filter((f) => /role="(alert)?dialog"/.test(fs.readFileSync(f, 'utf8')))
			.filter((f) => !notModal.includes(path.basename(f)));
		expect(files.length).toBeGreaterThan(5);
		for (const f of files) {
			const text = fs.readFileSync(f, 'utf8');
			expect(markers.some((m) => text.includes(m)), path.basename(f)).toBe(true);
		}
	});
});
