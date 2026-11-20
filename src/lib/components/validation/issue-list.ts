// The Validation panel's list: which issues it shows, in which order, and
// the key that ties each row to its issue across re-validations.

import type { ValidationIssue } from '$lib/ipc/validation';

/**
 * Rows drawn at a time. A report of thousands of issues drawn as one
 * button each blocked the window for seconds (a minute at 30,000), and
 * again at every re-validation after an edit. The next page is drawn when
 * the end of the list comes into view, as in the interpretation panel.
 */
export const ISSUE_PAGE = 200;

const SEVERITY_ORDER: Record<string, number> = { error: 0, warning: 1, info: 2 };

/**
 * The whole report, filtered to one severity (or 'all') and sorted by
 * severity or by segment. Only the drawing is paged: the counts, the
 * filter and the order always cover every issue.
 */
export function listedIssues(issues: ValidationIssue[], severity: string, sort: string): ValidationIssue[] {
	const filtered = severity === 'all' ? [...issues] : issues.filter((i) => i.severity === severity);
	if (sort === 'severity') {
		filtered.sort((a, b) => (SEVERITY_ORDER[a.severity] ?? 3) - (SEVERITY_ORDER[b.severity] ?? 3));
	} else if (sort === 'segment') {
		// Issues about no segment (the whole message) go last, also after
		// segment 999 of a long message.
		const seg = (i: ValidationIssue) => i.segment_idx ?? Number.MAX_SAFE_INTEGER;
		filtered.sort((a, b) => seg(a) - seg(b));
	}
	return filtered;
}

/**
 * Each issue with a key that stays the same when the report is made again
 * after an edit, so the rows already drawn are updated in place rather
 * than drawn anew. The key is what the issue is about (rule, segment,
 * field), numbered when that repeats. It used to end with the row's index,
 * so one issue fixed near the top gave every row below a new key.
 */
export function keyedIssues(issues: ValidationIssue[]): { key: string; issue: ValidationIssue }[] {
	const seen = new Map<string, number>();
	return issues.map((issue) => {
		const about = JSON.stringify([issue.rule_id, issue.segment_idx, issue.segment_type, issue.field_position]);
		const n = seen.get(about) ?? 0;
		seen.set(about, n + 1);
		return { key: `${about}#${n}`, issue };
	});
}
