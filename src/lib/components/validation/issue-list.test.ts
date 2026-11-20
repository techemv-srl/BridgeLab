import { describe, expect, it } from 'vitest';
import type { ValidationIssue } from '$lib/ipc/validation';
import { ISSUE_PAGE, keyedIssues, listedIssues } from './issue-list';

function issue(partial: Partial<ValidationIssue>): ValidationIssue {
	return {
		severity: 'error', message: 'm', segment_idx: null, segment_type: null, field_position: null, rule_id: 'R',
		...partial,
	};
}

/** A large report: one missing OBX-11 per OBX, and a warning and an
 *  info about the message as a whole. */
function bigReport(obx: number): ValidationIssue[] {
	const out: ValidationIssue[] = [issue({ severity: 'info', rule_id: 'MSH-005' })];
	for (let i = 0; i < obx; i++) {
		out.push(issue({ rule_id: 'REQ-OBX-11', segment_idx: i + 3, segment_type: 'OBX', field_position: 11, message: 'OBX-11 required' }));
	}
	out.push(issue({ severity: 'warning', rule_id: 'W', segment_idx: 1, segment_type: 'PID', field_position: 7 }));
	return out;
}

describe('listedIssues', () => {
	it('lists the whole report, whatever the page size', () => {
		const report = bigReport(30000);
		const all = listedIssues(report, 'all', 'severity');
		expect(all).toHaveLength(30002);
		expect(listedIssues(report, 'error', 'severity')).toHaveLength(30000);
		expect(listedIssues(report, 'warning', 'severity')).toHaveLength(1);
		expect(ISSUE_PAGE).toBeLessThan(all.length);
	});

	it('groups by severity, errors first, keeping the report order within each', () => {
		const report = [
			issue({ severity: 'info', message: 'i1' }),
			issue({ severity: 'warning', message: 'w1' }),
			issue({ severity: 'error', message: 'e1' }),
			issue({ severity: 'info', message: 'i2' }),
			issue({ severity: 'error', message: 'e2' }),
		];
		expect(listedIssues(report, 'all', 'severity').map((i) => i.message)).toEqual(['e1', 'e2', 'w1', 'i1', 'i2']);
	});

	it('sorts by segment with whole-message issues last, also past segment 999', () => {
		const report = [
			issue({ message: 'message', segment_idx: null }),
			issue({ message: 'obx', segment_idx: 1500 }),
			issue({ message: 'pid', segment_idx: 1 }),
		];
		expect(listedIssues(report, 'all', 'segment').map((i) => i.message)).toEqual(['pid', 'obx', 'message']);
	});

	it('never reorders the report it is given', () => {
		const report = bigReport(3);
		const before = [...report];
		listedIssues(report, 'all', 'severity');
		expect(report).toEqual(before);
	});
});

describe('keyedIssues', () => {
	it('gives every row its own key, also to issues that repeat', () => {
		const report = [...bigReport(1000), ...bigReport(1000)];
		const keys = keyedIssues(report).map((r) => r.key);
		expect(new Set(keys).size).toBe(report.length);
	});

	it('keeps the keys of the rows below an issue fixed by an edit', () => {
		// Re-validating after an edit that fixed the PID-7 warning at the
		// top: the OBX rows are the same rows, so they keep their keys and
		// are updated in place rather than drawn again.
		const before = [issue({ rule_id: 'W', segment_idx: 1, field_position: 7 }), ...bigReport(500).slice(1, -1)];
		const after = before.slice(1).map((i) => ({ ...i }));
		const keysBefore = keyedIssues(before).slice(1).map((r) => r.key);
		const keysAfter = keyedIssues(after).map((r) => r.key);
		expect(keysAfter).toEqual(keysBefore);
	});

	it('tells apart FHIR issues by their path', () => {
		const a = issue({ rule_id: 'FHIR', segment_type: 'Patient.name[0]' });
		const b = issue({ rule_id: 'FHIR', segment_type: 'Patient.gender' });
		const [ka, kb] = keyedIssues([a, b]).map((r) => r.key);
		expect(keyedIssues([b])[0].key).toBe(kb);
		expect(ka).not.toBe(kb);
	});
});
