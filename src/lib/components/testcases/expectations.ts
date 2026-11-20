/**
 * Test case expectations, the same rules as the CLI's `test` command
 * (src-tauri/src/test_packs.rs `type_matches`, and bridgelab-cli's
 * not-checked rule).
 */

/**
 * Compares the components the expectation gives, case-insensitively:
 * "ADT" matches any ADT event, "ADT^A01" matches `ADT^A01` and
 * `ADT^A01^ADT_A01` (v2.3.1+ adds the message structure as a third
 * component) but not `ADT^A04`; "ADT^A01^ADT_A01" needs all three.
 */
export function typeMatches(expected: string, actual: string): boolean {
	const want = expected.trim().toUpperCase();
	if (!want) return true;
	const got = actual.trim().toUpperCase().split('^');
	return want.split('^').every((c, i) => i < got.length && got[i].trim() === c.trim());
}

/** Only an explicit "invalid" (any case) expects errors; anything else expects a clean result. */
export function expectsInvalid(expectedResult: string | null | undefined): boolean {
	return (expectedResult ?? '').trim().toLowerCase() === 'invalid';
}

/** The parts of a FHIR validation report the not-checked rule reads. */
export interface FhirCheckReport {
	profiles_applied: boolean;
	issues: { message: string; path: string }[];
}

/** A backend note cut to its first clause: the reason, without the advice
 *  after it ("— install the package that defines it…"). */
function firstClause(message: string): string {
	return message.split(/ — |\. /)[0].trim();
}

/**
 * Why a FHIR resource was not fully checked, empty when it was: no
 * installed package defines its resource type, or a profile it declares
 * in `meta.profile` (its own, or a Bundle entry's or contained
 * resource's) is not installed or not in the version it pins. This is
 * the CLI's rule (`validate_fhir` and `not_checked_notes` in
 * bridgelab-cli), so Run all and `bridgelab-cli test` agree: before, a
 * resource checked against none of the profiles it declares read as a
 * pass. The message fragments are the backend's
 * (`parser/fhir/profile/validate.rs`).
 */
export function fhirNotCheckedReasons(report: FhirCheckReport): string[] {
	const reasons: string[] = [];
	for (const i of report.issues) {
		const onProfile = i.path === 'meta.profile' || i.path.endsWith('.meta.profile');
		if (onProfile && (i.message.includes('is declared but not installed') || i.message.includes('was skipped'))) {
			reasons.push(firstClause(i.message));
		}
	}
	if (!report.profiles_applied) {
		const note = report.issues.find((i) => i.path === 'resourceType' && i.message.startsWith('Profile conformance was not checked'));
		reasons.push(firstClause(note?.message ?? 'Profile conformance was not checked'));
	}
	return [...new Set(reasons)];
}

/** A test case's result: `notChecked` when it met its expectations but
 *  its FHIR resource was not fully checked: counted apart, never a pass. */
export type CaseOutcome = 'pass' | 'fail' | 'notChecked';

/** Unmet expectations fail the case whatever else is true; met ones pass
 *  only when nothing was left unchecked. */
export function caseOutcome(problems: string[], notChecked: string[]): CaseOutcome {
	if (problems.length > 0) return 'fail';
	return notChecked.length > 0 ? 'notChecked' : 'pass';
}

/** The Run all summary: how many passed out of all the cases run, and
 *  how many of those were not checked (neither passed nor failed). */
export function runSummaryOf(outcomes: CaseOutcome[]): { passed: number; notChecked: number; total: number } | null {
	if (outcomes.length === 0) return null;
	return {
		passed: outcomes.filter((o) => o === 'pass').length,
		notChecked: outcomes.filter((o) => o === 'notChecked').length,
		total: outcomes.length,
	};
}
