import { describe, it, expect, afterEach } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
import { noticeText, pluginWarnings, pluginError, capNoteText } from './plugin-notices';
import { setLocale } from '$lib/i18n';
import en from '$lib/i18n/en.json';
import type { PluginInfo } from '$lib/ipc/plugins';

afterEach(() => setLocale('en'));

const pack = (over: Partial<PluginInfo>): PluginInfo => ({
	key: 'validation/p', id: 'p', name: 'P', description: '', author: '', version: '1', enabled: true,
	gated: false, kind: 'validation', path: '/p.json', rule_count: 1, error: null, warnings: [], ...over,
});

describe('plugin pack problems', () => {
	it('every notice the backend makes has words in the app', () => {
		const rs = fs.readFileSync(path.resolve(__dirname, '../../src-tauri/src/plugins/mod.rs'), 'utf8');
		const codes = new Set([...rs.matchAll(/PluginNotice::new\("([a-z_]+)"\)/g)].map((m) => m[1]));
		expect(codes.size).toBeGreaterThan(15);
		for (const code of codes) expect((en as Record<string, string>)[`plugins.notice.${code}`], code).toBeTypeOf('string');
	});

	it('are worded in the user language, with the rule or field they are about', () => {
		const p = pack({
			warnings: ['rule F0: field 0 is the segment name, not a field (fields count from 1); the rule checks an empty value',
				'PHI field pid-5: segment "pid" is read as PID; write it as PID'],
			notices: [{ code: 'field_zero', rule: 'F0', args: {} },
				{ code: 'segment_case', phi_field: 'pid-5', args: { segment: 'pid', key: 'PID' } }],
		});
		expect(pluginWarnings(p)).toEqual(p.warnings);
		setLocale('es');
		expect(pluginWarnings(p)).toEqual([
			'regla F0: el campo 0 es el nombre del segmento, no un campo (los campos se cuentan desde 1); la regla comprueba un valor vacío',
			'campo PHI pid-5: el segmento "pid" se lee como PID; escríbelo como PID',
		]);
	});

	it('fall back to the English text for an unknown code or an older backend', () => {
		setLocale('de');
		expect(noticeText({ code: 'from_the_future', args: {} }, 'english text')).toBe('english text');
		expect(pluginWarnings(pack({ warnings: ['old backend'] }))).toEqual(['old backend']);
		expect(pluginError(pack({ error: 'not a regular file' }))).toBe('not a regular file');
		expect(pluginError(pack({ error: 'not a regular file', error_notice: { code: 'not_regular_file', args: {} } })))
			.toBe('keine normale Datei');
		expect(pluginError(pack({}))).toBeNull();
	});

	it('the note on packs over the cap is worded in the user language', () => {
		const one = '1 plugin pack not applied: the Community edition runs 3 packs at a time (Settings → Plugins)';
		expect(capNoteText(one)).toBe(one);
		setLocale('it');
		expect(capNoteText(one)).toBe("1 pacchetto di plugin non applicato: l'edizione Community ne esegue 3 alla volta (Impostazioni → Plugin)");
		expect(capNoteText('2 plugin packs not applied: the Community edition runs 3 packs at a time (Settings → Plugins)'))
			.toBe("2 pacchetti di plugin non applicati: l'edizione Community ne esegue 3 alla volta (Impostazioni → Plugin)");
		expect(capNoteText('something else')).toBe('something else');
	});
});
