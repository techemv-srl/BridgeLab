// FHIR validation rules builder — IPC bindings.
//
// Rules live in a plugin pack at
// <config>/BridgeLab/plugins/fhir/user-rules.json. The builder owns that
// one file; packs written by hand in the same folder load alongside it.

import { invoke } from '@tauri-apps/api/core';
import type { FhirValidationIssue } from './validation';

/** A check applied to the values a rule's `path` selected. */
export type FhirCheck =
	| { type: 'not_empty' }
	| { type: 'regex'; pattern: string }
	| { type: 'max_length'; max: number }
	| { type: 'min_length'; min: number }
	| { type: 'one_of'; values: string[] }
	| { type: 'contains'; value: string }
	| { type: 'cardinality'; min: number | null; max: number | null };

export type Severity = 'error' | 'warning' | 'info';

export interface FhirRule {
	rule_id: string;
	severity: Severity;
	/** Resource type the rule applies to; null means every resource. */
	resource: string | null;
	/** Invariant form: a FHIRPath expression that must be true. */
	expression: string | null;
	/** Selector form: a FHIRPath expression picking values to check. */
	path: string | null;
	/** Required with `path`. */
	check: FhirCheck | null;
	message: string;
}

export interface FhirRuleSet {
	rules: FhirRule[];
	path: string;
}

export interface FhirRuleTest {
	passed: boolean;
	/** False when the rule's resource filter excluded the open resource. */
	applied: boolean;
	issues: FhirValidationIssue[];
	selected: unknown[];
}

export async function listFhirRules(): Promise<FhirRuleSet> {
	return invoke('fhir_rules_list');
}

export async function saveFhirRules(rules: FhirRule[]): Promise<FhirRuleSet> {
	return invoke('fhir_rules_save', { rules });
}

/** Validate one rule without saving it. Resolves when the rule is usable. */
export async function checkFhirRule(rule: FhirRule): Promise<void> {
	return invoke('fhir_rule_check', { rule });
}

/** Run one rule against a message already open in the app. */
export async function testFhirRule(rule: FhirRule, messageId: string): Promise<FhirRuleTest> {
	return invoke('fhir_rule_test', { rule, messageId });
}

/** A blank rule in the invariant form, which is the more common one. */
export function blankRule(): FhirRule {
	return {
		rule_id: '',
		severity: 'error',
		resource: null,
		expression: '',
		path: null,
		check: null,
		message: '',
	};
}
