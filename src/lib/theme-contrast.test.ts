import { describe, it, expect } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';

/** The custom properties of one block of app.css (`:root` or the light theme). */
function themeVars(selector: string): Record<string, string> {
	const css = fs.readFileSync(path.resolve(__dirname, '../app.css'), 'utf8');
	const start = css.indexOf(`${selector} {`);
	const block = css.slice(start, css.indexOf('}', start));
	return Object.fromEntries([...block.matchAll(/(--[\w-]+):\s*([^;]+);/g)].map((m) => [m[1], m[2].trim()]));
}

/** WCAG 2 contrast ratio of two #rrggbb colours. */
function contrast(a: string, b: string): number {
	const lum = (hex: string) => {
		const [r, g, bl] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255)
			.map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
		return 0.2126 * r + 0.7152 * g + 0.0722 * bl;
	};
	const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x);
	return (hi + 0.05) / (lo + 0.05);
}

describe('light theme contrast', () => {
	const light = themeVars("[data-theme='light']");

	it('secondary text reads at 4.5:1 or more on every background', () => {
		for (const bg of ['--color-bg-primary', '--color-bg-secondary', '--color-bg-tertiary']) {
			expect(contrast(light['--color-text-secondary'], light[bg]), bg).toBeGreaterThanOrEqual(4.5);
		}
	});

	it('does not dim secondary text further (the tree dimmed it to 2-3:1)', () => {
		expect(light['--text-dim']).toBe('1');
		expect(themeVars(':root')['--text-dim']).toBeUndefined();
	});

	it('links and the status bar read at 4.5:1 or more', () => {
		for (const bg of ['--color-bg-primary', '--color-bg-secondary']) {
			expect(contrast(light['--color-accent'], light[bg]), `accent on ${bg}`).toBeGreaterThanOrEqual(4.5);
		}
		// The status bar: the primary background colour as text on the accent.
		expect(contrast(light['--color-bg-primary'], light['--color-accent'])).toBeGreaterThanOrEqual(4.5);
	});

	it('placeholder text reads at 4.5:1 or more on every background, undimmed', () => {
		const css = fs.readFileSync(path.resolve(__dirname, '../app.css'), 'utf8');
		const rule = css.slice(css.indexOf('::placeholder {'), css.indexOf('}', css.indexOf('::placeholder {')));
		expect(rule).toContain('color: var(--color-placeholder)');
		expect(rule).toContain('opacity: 1');
		for (const bg of ['--color-bg-primary', '--color-bg-secondary', '--color-bg-tertiary']) {
			expect(contrast(light['--color-placeholder'], light[bg]), bg).toBeGreaterThanOrEqual(4.5);
		}
	});
});
