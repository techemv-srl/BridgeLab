import { describe, expect, it } from 'vitest';
import { FIELD_PREVIEW_CHARS, previewText } from './text-preview';

describe('previewText', () => {
	it('keeps a value up to the limit whole', () => {
		expect(previewText('PID|1')).toEqual({ text: 'PID|1', cut: false });
		const atLimit = 'A'.repeat(FIELD_PREVIEW_CHARS);
		expect(previewText(atLimit)).toEqual({ text: atLimit, cut: false });
		expect(previewText('')).toEqual({ text: '', cut: false });
	});

	it('shows only the start of a multi-megabyte value', () => {
		const b64 = 'QUJD'.repeat((5 * 1024 * 1024) / 4);
		const p = previewText(b64);
		expect(p.cut).toBe(true);
		expect(p.text.length).toBe(FIELD_PREVIEW_CHARS);
		expect(b64.startsWith(p.text)).toBe(true);
	});

	it('never splits a surrogate pair at the cut', () => {
		// '😀' is two UTF-16 units: a cut after its first half would draw a
		// broken character.
		const p = previewText('ab😀cd', 3);
		expect(p).toEqual({ text: 'ab', cut: true });
		expect(previewText('ab😀cd', 4)).toEqual({ text: 'ab😀', cut: true });
	});
});
