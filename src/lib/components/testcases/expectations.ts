/**
 * Test case expectations, the same rules as the CLI's `test` command
 * (src-tauri/src/test_packs.rs `type_matches`).
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
