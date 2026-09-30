import { describe, it, expect } from 'vitest';
import { hl7ToolTarget, type ToolTargetDeps } from './tool-target';
import type { ParseResult } from '$lib/types/hl7';

const result = (id: string) => ({ message_id: id, format: 'HL7v2' }) as unknown as ParseResult;
const hl7 = (s: string) => /^(MSH|FHS|BHS)/.test(s);
const fhir = (s: string) => s.startsWith('{') || s.startsWith('<');

function deps(screen: { id: string; content: string } | null, parse = async (c: string) => result(c.slice(0, 12))): ToolTargetDeps {
	return { parse, current: () => screen, looksLikeHl7: hl7, looksLikeFhir: fhir };
}

describe('hl7ToolTarget', () => {
	it('parses the text in the editor and binds the result to its tab', async () => {
		const tab = { id: 't1', content: 'MSH|^~\\&|NEW' };
		const r = await hl7ToolTarget(tab, deps(tab));
		expect(r).toEqual({ ok: true, tabId: 't1', result: result('MSH|^~\\&|NEW') });
	});

	it('refuses text that is no longer an HL7 message instead of using the old parse', async () => {
		let parsed = false;
		const tab = { id: 't1', content: 'PID|1||123||Doe^John' };
		const r = await hl7ToolTarget(tab, deps(tab, async (c) => { parsed = true; return result(c); }));
		expect(r).toEqual({ ok: false, reason: 'notHl7' });
		expect(parsed).toBe(false);
		expect(await hl7ToolTarget({ id: 't1', content: '{"resourceType":"Patient"}' }, deps(tab))).toEqual({ ok: false, reason: 'fhir' });
		expect(await hl7ToolTarget({ id: 't1', content: '  ' }, deps(tab))).toEqual({ ok: false, reason: 'empty' });
		expect(await hl7ToolTarget(null, deps(null))).toEqual({ ok: false, reason: 'empty' });
	});

	it('is refused when the user switches tabs or edits while the parse runs', async () => {
		const tab = { id: 't1', content: 'MSH|^~\\&|A' };
		expect(await hl7ToolTarget(tab, deps({ id: 't2', content: 'MSH|^~\\&|B' }))).toEqual({ ok: false, reason: 'changed' });
		expect(await hl7ToolTarget(tab, deps({ id: 't1', content: 'MSH|^~\\&|A2' }))).toEqual({ ok: false, reason: 'changed' });
		expect(await hl7ToolTarget(tab, deps(null))).toEqual({ ok: false, reason: 'changed' });
	});

	it('reports a parse error', async () => {
		const tab = { id: 't1', content: 'MSH|broken' };
		const r = await hl7ToolTarget(tab, deps(tab, async () => { throw 'bad header'; }));
		expect(r).toEqual({ ok: false, reason: 'parseFailed', error: 'bad header' });
	});
});
