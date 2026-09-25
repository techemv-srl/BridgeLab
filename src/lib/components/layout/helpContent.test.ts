import { describe, expect, it } from 'vitest';
import { generateManualHtml, generateManualParts, manualRoute, parseManualQuery } from './helpContent';

describe('manual route', () => {
	const live = [
		{ label: 'Save', keys: 'Ctrl+S' },
		{ label: 'Find & replace', keys: 'Ctrl+/' },
	];

	it('round-trips locale and live shortcuts through the query string', () => {
		const route = manualRoute('it', live);
		expect(route.startsWith('manual?')).toBe(true);
		const parsed = parseManualQuery(route.slice(route.indexOf('?')));
		expect(parsed.locale).toBe('it');
		expect(parsed.liveShortcuts).toEqual([
			{ label: 'Save', keys: 'Ctrl+S' },
			{ label: 'Find &amp; replace', keys: 'Ctrl+/' },
		]);
	});

	it('escapes markup arriving through the query', () => {
		const q = new URLSearchParams({ lang: 'en', keys: JSON.stringify([{ label: '<img src=x onerror=1>', keys: 'F1' }]) });
		const { liveShortcuts } = parseManualQuery('?' + q.toString());
		expect(liveShortcuts?.[0].label).toBe('&lt;img src=x onerror=1&gt;');
	});

	it('ignores malformed keys and unknown locales', () => {
		const parsed = parseManualQuery('?lang=xx&keys=%7Bnot-json');
		expect(parsed.liveShortcuts).toBeUndefined();
		const parts = generateManualParts(parsed.locale, parsed.liveShortcuts);
		expect(parts.lang).toBe('en');
		expect(parts.title).toBe('BridgeLab User Manual');
	});

	it('renders the same sections as the standalone document', () => {
		for (const lang of ['en', 'it', 'fr', 'es', 'de']) {
			const parts = generateManualParts(lang);
			expect(parts.body).toContain('<aside class="toc">');
			expect(parts.body.match(/<section id=/g)?.length).toBeGreaterThan(5);
			expect(generateManualHtml(lang)).toContain(parts.body);
		}
	});
});
