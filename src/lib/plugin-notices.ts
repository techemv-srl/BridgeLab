import { t } from '$lib/i18n';
import type { PluginInfo, PluginNotice } from '$lib/ipc/plugins';

/**
 * Plugin pack problems in the user's language. The backend describes each
 * one as a notice (a code, its values, the rule or PHI field it is about)
 * next to the English sentence the CLI prints; Settings showed that
 * English in every language. A notice whose code has no translation here
 * (a newer backend) falls back to the English sentence.
 */
export function noticeText(n: PluginNotice, english: string): string {
	const key = `plugins.notice.${n.code}`;
	const body = t(key, n.args);
	if (body === key) return english;
	const subject = n.rule !== undefined
		? t('plugins.subject.rule', { id: n.rule })
		: n.phi_field !== undefined ? t('plugins.subject.phiField', { field: n.phi_field }) : '';
	return subject ? `${subject}: ${body}` : body;
}

/** The pack's warnings, in the user's language where possible. */
export function pluginWarnings(p: PluginInfo): string[] {
	const notices = p.notices ?? [];
	return p.warnings.map((w, i) => (notices[i] ? noticeText(notices[i], w) : w));
}

/** Why the pack did not load, in the user's language where possible. */
export function pluginError(p: PluginInfo): string | null {
	if (!p.error) return null;
	return p.error_notice ? noticeText(p.error_notice, p.error) : p.error;
}

/**
 * The note added to an HL7 v2 report when packs are over the Community
 * cap (rule PLUGIN-CAP), in the user's language. The backend writes it in
 * English: "2 plugin packs not applied: the Community edition runs 3 packs
 * at a time (Settings → Plugins)"; its numbers are read back from there.
 */
export function capNoteText(message: string): string {
	const m = /^(\d+) plugin packs? not applied: .*? (\d+) packs at a time/.exec(message);
	if (!m) return message;
	const count = Number(m[1]);
	return t(count === 1 ? 'plugins.capNoteOne' : 'plugins.capNote', { count, max: m[2] });
}
