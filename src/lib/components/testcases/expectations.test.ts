import { describe, expect, it } from 'vitest';
import { caseOutcome, expectsInvalid, fhirNotCheckedReasons, runSummaryOf, typeMatches } from './expectations';

// The backend's messages (parser/fhir/profile/validate.rs and the
// validate_fhir command), as the report carries them.
const NOT_INSTALLED = 'Profile http://example.org/StructureDefinition/NotInstalled is declared but not installed — install the package that defines it to check conformance';
const WRONG_VERSION = 'Profile http://hl7.org/fhir/StructureDefinition/Patient is pinned to version 9.9 but only 4.0.1 is installed — conformance against another version would answer a different question, so this profile was skipped';
const NO_DEFINITION = "Profile conformance was not checked: no installed package defines the resource type 'Foo'. Built-in: FHIR R4 core; add others under Tools → FHIR profile packages…";

describe('fhirNotCheckedReasons', () => {
	it('names a declared profile that is not installed (a Patient on an unknown profile)', () => {
		const report = { profiles_applied: true, issues: [{ severity: 'warning', message: NOT_INSTALLED, path: 'meta.profile' }] };
		expect(fhirNotCheckedReasons(report)).toEqual([
			'Profile http://example.org/StructureDefinition/NotInstalled is declared but not installed',
		]);
	});

	it('covers a pinned version that was skipped, and nested resources', () => {
		const report = {
			profiles_applied: true,
			issues: [
				{ message: WRONG_VERSION, path: 'meta.profile' },
				{ message: NOT_INSTALLED, path: 'Bundle.entry[0].resource.meta.profile' },
				{ message: NOT_INSTALLED, path: 'Bundle.entry[1].resource.meta.profile' },
			],
		};
		expect(fhirNotCheckedReasons(report)).toEqual([
			'Profile http://hl7.org/fhir/StructureDefinition/Patient is pinned to version 9.9 but only 4.0.1 is installed',
			'Profile http://example.org/StructureDefinition/NotInstalled is declared but not installed',
		]);
	});

	it('treats a Bundle entry pinned to a missing version as not checked (same words as the top level)', () => {
		const report = {
			profiles_applied: true,
			issues: [{ message: WRONG_VERSION, path: 'Bundle.entry[2].resource.meta.profile' }],
		};
		expect(fhirNotCheckedReasons(report)).toHaveLength(1);
		expect(caseOutcome([], fhirNotCheckedReasons(report))).toBe('notChecked');
	});

	it('names a resource type no installed package defines', () => {
		const report = { profiles_applied: false, issues: [{ message: NO_DEFINITION, path: 'resourceType' }] };
		expect(fhirNotCheckedReasons(report)).toEqual([
			"Profile conformance was not checked: no installed package defines the resource type 'Foo'",
		]);
		expect(fhirNotCheckedReasons({ profiles_applied: false, issues: [] })).toEqual(['Profile conformance was not checked']);
	});

	it('is empty for a fully checked resource, whatever else it reports', () => {
		const report = {
			profiles_applied: true,
			issues: [
				{ message: 'Patient.gender: value not in the required value set', path: 'gender' },
				{ message: 'Unknown element', path: 'meta.profile' },
				{ message: NOT_INSTALLED, path: 'text' },
			],
		};
		expect(fhirNotCheckedReasons(report)).toEqual([]);
	});
});

describe('caseOutcome', () => {
	it('fails on unmet expectations even when also not checked', () => {
		expect(caseOutcome(['Expected valid, found 1 validation errors'], ['Profile x is declared but not installed'])).toBe('fail');
	});

	it('is not checked, never a pass, when expectations are met but a profile was not applied', () => {
		expect(caseOutcome([], ['Profile x is declared but not installed'])).toBe('notChecked');
	});

	it('passes only when nothing is left', () => {
		expect(caseOutcome([], [])).toBe('pass');
	});
});

describe('runSummaryOf', () => {
	it('counts not-checked cases apart from passed ones, like "4/8 passed, 1 not checked"', () => {
		expect(runSummaryOf(['pass', 'pass', 'pass', 'pass', 'fail', 'fail', 'fail', 'notChecked'])).toEqual({ passed: 4, notChecked: 1, total: 8 });
		expect(runSummaryOf(['pass'])).toEqual({ passed: 1, notChecked: 0, total: 1 });
		expect(runSummaryOf([])).toBeNull();
	});
});

describe('typeMatches', () => {
	it('compares only the components the expectation gives', () => {
		expect(typeMatches('ADT', 'ADT^A01')).toBe(true);
		expect(typeMatches('ADT^A01', 'ADT^A01^ADT_A01')).toBe(true);
		expect(typeMatches('adt^a01', 'ADT^A01')).toBe(true);
		expect(typeMatches('ADT^A01^ADT_A01', 'ADT^A01^ADT_A01')).toBe(true);
		expect(typeMatches('ADT^A01^ADT_A01', 'ADT^A01')).toBe(false);
		expect(typeMatches('ADT^A04', 'ADT^A01^ADT_A01')).toBe(false);
		expect(typeMatches('AD', 'ADT^A01')).toBe(false);
		expect(typeMatches('', 'anything')).toBe(true);
	});
});

describe('expectsInvalid', () => {
	it('accepts any case of "invalid"', () => {
		expect(expectsInvalid('invalid')).toBe(true);
		expect(expectsInvalid('Invalid')).toBe(true);
		expect(expectsInvalid('valid')).toBe(false);
		expect(expectsInvalid(undefined)).toBe(false);
	});
});
