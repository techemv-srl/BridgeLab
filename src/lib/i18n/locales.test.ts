import { describe, it, expect } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
import en from './en.json';
import it_ from './it.json';
import fr from './fr.json';
import es from './es.json';
import de from './de.json';

const locales: Record<string, Record<string, string>> = { it: it_, fr, es, de };

function svelteFiles(dir: string): string[] {
	return fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
		const p = path.join(dir, e.name);
		return e.isDirectory() ? svelteFiles(p) : p.endsWith('.svelte') ? [p] : [];
	});
}

describe('translations', () => {
	it('every language has every key, with the same placeholders', () => {
		for (const [code, map] of Object.entries(locales)) {
			for (const [key, text] of Object.entries(en)) {
				expect(map[key], `${code}: ${key}`).toBeTypeOf('string');
				const holes = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
				expect(holes(map[key]), `${code}: ${key}`).toEqual(holes(text));
			}
		}
	});

	it('no text names a modifier key itself: keys go through displayKeys (⌘ and ⌥ on macOS)', () => {
		for (const [code, map] of Object.entries({ en, ...locales })) {
			for (const [key, text] of Object.entries(map)) {
				if (key === 'shortcuts.needsModifier') continue; // shown on Windows and Linux only
				expect(text, `${code}: ${key}`).not.toMatch(/\b(Ctrl|Strg)\+|\bAlt\+/);
			}
		}
	});

	it('the components show no English labels of their own', () => {
		const root = path.resolve(__dirname, '../components');
		const english = [/aria-label="Close"/, /title="MLLP /, /placeholder="Bearer token"/, /name: 'All Files'/, /name: 'FHIR package'/];
		for (const f of svelteFiles(root)) {
			const text = fs.readFileSync(f, 'utf8');
			for (const re of english) expect(re.test(text), `${path.basename(f)}: ${re}`).toBe(false);
		}
	});
});
