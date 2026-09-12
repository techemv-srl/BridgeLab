//! Validation of a resource against a StructureDefinition snapshot.
//!
//! What is checked:
//!
//! - **Cardinality** — a required element that is absent, or a `0..1`
//!   element that repeats.
//! - **Types** — the JSON shape against the declared type, including
//!   choice elements, where `value[x]` must appear as exactly one of
//!   `valueQuantity`, `valueString`, …
//! - **Fixed values and patterns** — `fixed[x]` exactly, `pattern[x]` as a
//!   subset of the instance.
//! - **Unknown elements** — a key the profile does not define. This is the
//!   check that catches `Observation.valueQuantity` written where the
//!   profile says `value[x]`, and plain typos.
//!
//! What is not: terminology. A `required` binding needs the ValueSet
//! expanded, which means either shipping the terminology packages or
//! calling a server — neither fits an offline desktop tool, so bindings are
//! left alone rather than half-checked.

use std::collections::HashSet;

use serde_json::{Map, Value};

use super::model::{Profile, ProfileElement, ProfileIndex};
use crate::parser::fhir::FhirValidationIssue;

/// How deep to follow complex types into their own definitions. FHIR data
/// types nest (Reference → Identifier → Period), and a bound keeps a cyclic
/// or pathological profile from hanging the UI.
const MAX_DEPTH: usize = 6;

/// Validate `resource` against `profile`, following child types through
/// `index` where the profile itself does not constrain them.
pub fn validate_against(
    resource: &Value,
    profile: &Profile,
    index: &ProfileIndex,
) -> Vec<FhirValidationIssue> {
    let mut issues = Vec::new();
    let root = profile.type_name.clone();
    walk(
        resource,
        profile,
        &root,
        &root,
        index,
        0,
        &mut Vec::new(),
        &mut issues,
    );
    issues
}

/// Validate against every profile the resource declares in `meta.profile`,
/// plus the base definition of its resource type.
///
/// Returns `None` when nothing applicable is installed, so the caller can
/// stay quiet rather than claim a resource passed checks that never ran.
pub fn validate_declared(
    resource: &Value,
    index: &ProfileIndex,
) -> Option<Vec<FhirValidationIssue>> {
    let resource_type = resource.get("resourceType")?.as_str()?;

    let mut applied = Vec::new();
    if let Some(base) = index.base_for_type(resource_type) {
        applied.push(base);
    }
    if let Some(declared) = resource
        .get("meta")
        .and_then(|m| m.get("profile"))
        .and_then(|p| p.as_array())
    {
        for url in declared.iter().filter_map(|u| u.as_str()) {
            match index.get(url) {
                Some(profile) => applied.push(profile),
                None => {
                    // Saying the profile is missing is more useful than
                    // silently validating against the base only.
                    return Some(vec![FhirValidationIssue {
                        severity: "warning".into(),
                        message: format!(
                            "Profile {} is declared but not installed — install the \
                             package that defines it to check conformance",
                            url
                        ),
                        path: "meta.profile".into(),
                    }]);
                }
            }
        }
    }

    if applied.is_empty() {
        return None;
    }

    let mut issues = Vec::new();
    for profile in applied {
        issues.extend(validate_against(resource, profile, index));
    }
    Some(dedup_issues(issues))
}

/// Walk one object level.
///
/// `profile_path` is where we are in the profile's element paths;
/// `report_path` is where we are in the instance, which differs once array
/// indices come into play.
#[allow(clippy::too_many_arguments)]
fn walk(
    instance: &Value,
    profile: &Profile,
    profile_path: &str,
    report_path: &str,
    index: &ProfileIndex,
    depth: usize,
    seen_types: &mut Vec<String>,
    issues: &mut Vec<FhirValidationIssue>,
) {
    let Some(object) = instance.as_object() else {
        return;
    };
    let children = profile.children_of(profile_path);
    if children.is_empty() {
        return;
    }

    let mut known: HashSet<String> = HashSet::new();
    // Present in every resource and element but not in the snapshot.
    known.insert("resourceType".into());

    for element in &children {
        // Slices constrain a subset of the same element; the parent already
        // covers presence and cardinality, so checking the slice as if it
        // were its own element would double-report.
        if element.slice_name.is_some() {
            known.insert(element.base_name().to_string());
            continue;
        }

        let (key, values) = match resolve_element(object, element) {
            Resolved::Missing => {
                // A choice element has no bare form in the JSON encoding:
                // only `valueQuantity`, `valueString` and friends are legal,
                // so a plain `value` must stay unknown and be reported.
                if element.is_choice() {
                    for t in &element.types {
                        known.insert(element.choice_key(t));
                    }
                } else {
                    known.insert(element.base_name().to_string());
                }
                if element.min > 0 {
                    issues.push(issue(
                        "error",
                        format!(
                            "{} is required ({}){}",
                            element.path,
                            element.cardinality(),
                            describe(element)
                        ),
                        format!("{}.{}", report_path, element.base_name()),
                    ));
                }
                continue;
            }
            Resolved::Ambiguous(keys) => {
                for k in &keys {
                    known.insert(k.clone());
                }
                issues.push(issue(
                    "error",
                    format!(
                        "{} is a choice and allows only one of its forms, but {} are present",
                        element.path,
                        keys.join(", ")
                    ),
                    format!("{}.{}", report_path, element.base_name()),
                ));
                continue;
            }
            Resolved::Found { key, values } => (key, values),
        };

        known.insert(key.clone());
        if element.is_choice() {
            for t in &element.types {
                known.insert(element.choice_key(t));
            }
        }
        // FHIR carries a primitive's id and extensions in a sibling key
        // prefixed with '_'; it belongs to the same element.
        known.insert(format!("_{}", key));

        check_cardinality(element, values.len(), report_path, &key, issues);

        for (i, value) in values.iter().enumerate() {
            let child_report = if values.len() > 1 || object.get(&key).is_some_and(|v| v.is_array())
            {
                format!("{}.{}[{}]", report_path, key, i)
            } else {
                format!("{}.{}", report_path, key)
            };

            // Which of a choice element's types applies is decided by the
            // key the instance used, not by the order they are declared in.
            let applicable = applicable_type(element, &key);

            check_type(element, applicable, value, &child_report, issues);
            check_fixed(element, value, &child_report, issues);

            recurse(
                value,
                element,
                applicable,
                profile,
                &child_report,
                index,
                depth,
                seen_types,
                issues,
            );
        }
    }

    report_unknown(object, &known, profile, report_path, issues);
}

/// The declared type that applies to the value found under `key`.
///
/// For a choice element the key carries the type (`valueQuantity` →
/// `Quantity`); taking the first declared type instead would validate a
/// Quantity against whatever the definition happened to list first, which
/// for `UsageContext.value[x]` is CodeableConcept.
fn applicable_type<'a>(element: &'a ProfileElement, key: &str) -> Option<&'a str> {
    if element.is_choice() {
        return element
            .types
            .iter()
            .find(|t| element.choice_key(t) == key)
            .map(|s| s.as_str());
    }
    element.types.first().map(|s| s.as_str())
}

/// Descend into an element's children, either within this profile or
/// through the definition of the element's own type.
#[allow(clippy::too_many_arguments)]
fn recurse(
    value: &Value,
    element: &ProfileElement,
    applicable: Option<&str>,
    profile: &Profile,
    report_path: &str,
    index: &ProfileIndex,
    depth: usize,
    seen_types: &mut Vec<String>,
    issues: &mut Vec<FhirValidationIssue>,
) {
    if !value.is_object() || depth >= MAX_DEPTH {
        return;
    }

    // Constraints the profile states itself take precedence.
    if !profile.children_of(&element.path).is_empty() {
        walk(
            value,
            profile,
            &element.path,
            report_path,
            index,
            depth + 1,
            seen_types,
            issues,
        );
        return;
    }

    // Otherwise follow the element's declared type, when we have it.
    let Some(type_code) = applicable else {
        return;
    };
    // A contained or referenced Resource carries its own type.
    let type_code = if type_code == "Resource" || type_code == "DomainResource" {
        match value.get("resourceType").and_then(|v| v.as_str()) {
            Some(rt) => rt,
            None => return,
        }
    } else {
        type_code
    };

    if seen_types.contains(&type_code.to_string()) {
        return; // already expanded on this branch: stop before looping
    }
    let Some(type_profile) = index.base_for_type(type_code) else {
        return;
    };

    seen_types.push(type_code.to_string());
    let root = type_profile.type_name.clone();
    walk(
        value,
        type_profile,
        &root,
        report_path,
        index,
        depth + 1,
        seen_types,
        issues,
    );
    seen_types.pop();
}

enum Resolved {
    Missing,
    Ambiguous(Vec<String>),
    Found { key: String, values: Vec<Value> },
}

/// Find the instance values for an element, resolving choice types.
fn resolve_element(object: &Map<String, Value>, element: &ProfileElement) -> Resolved {
    if !element.is_choice() {
        let name = element.base_name();
        return match object.get(name) {
            None => Resolved::Missing,
            Some(v) => Resolved::Found {
                key: name.to_string(),
                values: as_list(v),
            },
        };
    }

    let present: Vec<String> = element
        .types
        .iter()
        .map(|t| element.choice_key(t))
        .filter(|k| object.contains_key(k))
        .collect();

    match present.len() {
        0 => Resolved::Missing,
        1 => {
            let key = present.into_iter().next().unwrap();
            let values = as_list(&object[&key]);
            Resolved::Found { key, values }
        }
        _ => Resolved::Ambiguous(present),
    }
}

fn as_list(v: &Value) -> Vec<Value> {
    match v {
        Value::Array(items) => items.clone(),
        other => vec![other.clone()],
    }
}

fn check_cardinality(
    element: &ProfileElement,
    count: usize,
    report_path: &str,
    key: &str,
    issues: &mut Vec<FhirValidationIssue>,
) {
    if (count as u32) < element.min {
        issues.push(issue(
            "error",
            format!(
                "{} needs {} but found {}",
                element.path,
                element.cardinality(),
                count
            ),
            format!("{}.{}", report_path, key),
        ));
    }
    if let Some(max) = element.max {
        if count as u32 > max {
            issues.push(issue(
                "error",
                format!(
                    "{} allows {} but found {}",
                    element.path,
                    element.cardinality(),
                    count
                ),
                format!("{}.{}", report_path, key),
            ));
        }
    }
}

/// Check the JSON shape against the element's declared type. Only the
/// primitives can be judged from JSON alone; complex types are objects and
/// their contents are checked by recursion.
fn check_type(
    element: &ProfileElement,
    declared: Option<&str>,
    value: &Value,
    report_path: &str,
    issues: &mut Vec<FhirValidationIssue>,
) {
    let Some(declared) = declared else { return };

    let ok = match declared {
        "boolean" => value.is_boolean(),
        "integer" | "positiveInt" | "unsignedInt" | "integer64" => value.is_i64() || value.is_u64(),
        "decimal" => value.is_number(),
        "string" | "code" | "uri" | "url" | "canonical" | "oid" | "id" | "uuid" | "markdown"
        | "base64Binary" | "xhtml" | "date" | "dateTime" | "instant" | "time"
        | "System.String" => value.is_string(),
        // Everything else is a complex type or a BackboneElement.
        _ => value.is_object(),
    };

    if !ok {
        issues.push(issue(
            "error",
            format!(
                "{} must be a {} but the value is {}",
                element.path,
                declared,
                json_kind(value)
            ),
            report_path.to_string(),
        ));
    }
}

fn check_fixed(
    element: &ProfileElement,
    value: &Value,
    report_path: &str,
    issues: &mut Vec<FhirValidationIssue>,
) {
    if let Some(fixed) = &element.fixed {
        if value != fixed {
            issues.push(issue(
                "error",
                format!(
                    "{} is fixed to {} but the value is {}",
                    element.path,
                    render(fixed),
                    render(value)
                ),
                report_path.to_string(),
            ));
        }
    }
    if let Some(pattern) = &element.pattern {
        if !matches_pattern(value, pattern) {
            issues.push(issue(
                "error",
                format!(
                    "{} must match the pattern {}",
                    element.path,
                    render(pattern)
                ),
                report_path.to_string(),
            ));
        }
    }
}

/// A pattern constrains the parts it states and leaves the rest free; for
/// an array, every pattern entry must be matched by some instance entry.
fn matches_pattern(value: &Value, pattern: &Value) -> bool {
    match (value, pattern) {
        (Value::Object(v), Value::Object(p)) => p
            .iter()
            .all(|(k, pv)| v.get(k).is_some_and(|vv| matches_pattern(vv, pv))),
        (Value::Array(v), Value::Array(p)) => p
            .iter()
            .all(|pv| v.iter().any(|vv| matches_pattern(vv, pv))),
        (v, p) => v == p,
    }
}

fn report_unknown(
    object: &Map<String, Value>,
    known: &HashSet<String>,
    profile: &Profile,
    report_path: &str,
    issues: &mut Vec<FhirValidationIssue>,
) {
    for key in object.keys() {
        if known.contains(key) {
            continue;
        }
        // A '_'-prefixed sibling belongs to the primitive it decorates.
        if let Some(base) = key.strip_prefix('_') {
            if known.contains(base) {
                continue;
            }
        }
        issues.push(issue(
            "error",
            format!(
                "'{}' is not defined by {} — check the element name and its type suffix",
                key, profile.name
            ),
            if report_path.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", report_path, key)
            },
        ));
    }
}

/// The same element can be reported by both the base definition and a
/// profile constraining it; report each finding once.
fn dedup_issues(issues: Vec<FhirValidationIssue>) -> Vec<FhirValidationIssue> {
    let mut seen: HashSet<(String, String)> = HashSet::new();
    issues
        .into_iter()
        .filter(|i| seen.insert((i.path.clone(), i.message.clone())))
        .collect()
}

fn describe(element: &ProfileElement) -> String {
    if element.short.is_empty() {
        String::new()
    } else {
        format!(": {}", element.short)
    }
}

fn json_kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

fn render(v: &Value) -> String {
    match v {
        Value::String(s) => format!("'{}'", s),
        other => other.to_string(),
    }
}

fn issue(severity: &str, message: String, path: String) -> FhirValidationIssue {
    FhirValidationIssue {
        severity: severity.into(),
        message,
        path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::fhir::profile::model::{from_structure_definition, ProfilePackage};
    use serde_json::json;

    fn index_with(sds: Vec<Value>) -> ProfileIndex {
        let profiles = sds
            .iter()
            .filter_map(from_structure_definition)
            .collect::<Vec<_>>();
        let mut index = ProfileIndex::default();
        index.add_package(ProfilePackage {
            name: "test".into(),
            version: "1.0".into(),
            title: String::new(),
            fhir_version: "4.0.1".into(),
            profiles,
        });
        index
    }

    fn patient_sd() -> Value {
        json!({
            "resourceType": "StructureDefinition",
            "url": "http://hl7.org/fhir/StructureDefinition/Patient",
            "name": "Patient", "kind": "resource", "type": "Patient",
            "derivation": "specialization",
            "snapshot": { "element": [
                {"path": "Patient", "min": 0, "max": "*"},
                {"path": "Patient.id", "min": 0, "max": "1", "type": [{"code": "id"}]},
                {"path": "Patient.meta", "min": 0, "max": "1", "type": [{"code": "Meta"}]},
                {"path": "Patient.active", "min": 0, "max": "1", "type": [{"code": "boolean"}]},
                {"path": "Patient.name", "min": 0, "max": "*", "type": [{"code": "HumanName"}]},
                {"path": "Patient.gender", "min": 0, "max": "1", "type": [{"code": "code"}]},
                {"path": "Patient.deceased[x]", "min": 0, "max": "1",
                 "type": [{"code": "boolean"}, {"code": "dateTime"}]}
            ]}
        })
    }

    fn human_name_sd() -> Value {
        json!({
            "resourceType": "StructureDefinition",
            "url": "http://hl7.org/fhir/StructureDefinition/HumanName",
            "name": "HumanName", "kind": "complex-type", "type": "HumanName",
            "derivation": "specialization",
            "snapshot": { "element": [
                {"path": "HumanName", "min": 0, "max": "*"},
                {"path": "HumanName.use", "min": 0, "max": "1", "type": [{"code": "code"}]},
                {"path": "HumanName.family", "min": 0, "max": "1", "type": [{"code": "string"}]},
                {"path": "HumanName.given", "min": 0, "max": "*", "type": [{"code": "string"}]}
            ]}
        })
    }

    fn meta_sd() -> Value {
        json!({
            "resourceType": "StructureDefinition",
            "url": "http://hl7.org/fhir/StructureDefinition/Meta",
            "name": "Meta", "kind": "complex-type", "type": "Meta",
            "derivation": "specialization",
            "snapshot": { "element": [
                {"path": "Meta", "min": 0, "max": "*"},
                {"path": "Meta.profile", "min": 0, "max": "*", "type": [{"code": "canonical"}]}
            ]}
        })
    }

    fn validate(resource: &Value, index: &ProfileIndex) -> Vec<FhirValidationIssue> {
        let profile = index.base_for_type("Patient").expect("Patient profile");
        validate_against(resource, profile, index)
    }

    #[test]
    fn a_conforming_resource_produces_nothing() {
        let index = index_with(vec![patient_sd(), human_name_sd(), meta_sd()]);
        let patient = json!({
            "resourceType": "Patient",
            "id": "p1",
            "active": true,
            "gender": "female",
            "name": [{"use": "official", "family": "Smith", "given": ["Jane"]}]
        });
        assert_eq!(validate(&patient, &index), vec![]);
    }

    #[test]
    fn flags_an_unknown_element() {
        let index = index_with(vec![patient_sd(), human_name_sd()]);
        let patient = json!({"resourceType": "Patient", "genderr": "female"});
        let issues = validate(&patient, &index);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("'genderr' is not defined"), "{}", issues[0].message);
    }

    #[test]
    fn flags_a_choice_element_written_with_its_type_suffix_where_none_applies() {
        // `Patient.name` is not a choice, so `nameHumanName` is simply wrong.
        let index = index_with(vec![patient_sd(), human_name_sd()]);
        let patient = json!({"resourceType": "Patient", "nameHumanName": {"family": "Smith"}});
        let issues = validate(&patient, &index);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("nameHumanName"));
    }

    #[test]
    fn accepts_every_form_of_a_choice_element_and_rejects_two_at_once() {
        let index = index_with(vec![patient_sd(), human_name_sd()]);

        for value in [json!({"deceasedBoolean": true}), json!({"deceasedDateTime": "2020-01-01"})] {
            let mut patient = json!({"resourceType": "Patient"});
            patient.as_object_mut().unwrap().extend(
                value.as_object().unwrap().clone(),
            );
            assert_eq!(validate(&patient, &index), vec![], "{:?}", patient);
        }

        let both = json!({
            "resourceType": "Patient",
            "deceasedBoolean": true,
            "deceasedDateTime": "2020-01-01"
        });
        let issues = validate(&both, &index);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("only one of its forms"), "{}", issues[0].message);
    }

    #[test]
    fn flags_a_value_of_the_wrong_json_type() {
        let index = index_with(vec![patient_sd(), human_name_sd()]);
        let patient = json!({"resourceType": "Patient", "active": "yes"});
        let issues = validate(&patient, &index);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("must be a boolean"), "{}", issues[0].message);
    }

    #[test]
    fn flags_cardinality_in_both_directions() {
        let constrained = json!({
            "resourceType": "StructureDefinition",
            "url": "http://acme.org/StructureDefinition/AcmePatient",
            "name": "AcmePatient", "kind": "resource", "type": "Patient",
            "derivation": "constraint",
            "baseDefinition": "http://hl7.org/fhir/StructureDefinition/Patient",
            "snapshot": { "element": [
                {"path": "Patient", "min": 0, "max": "*"},
                {"path": "Patient.name", "min": 1, "max": "1",
                 "type": [{"code": "HumanName"}], "short": "exactly one name"}
            ]}
        });
        let index = index_with(vec![patient_sd(), human_name_sd(), constrained.clone()]);
        let profile = index.get("http://acme.org/StructureDefinition/AcmePatient").unwrap();

        let none = json!({"resourceType": "Patient"});
        let issues = validate_against(&none, profile, &index);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("required"), "{}", issues[0].message);

        let two = json!({
            "resourceType": "Patient",
            "name": [{"family": "A"}, {"family": "B"}]
        });
        let issues = validate_against(&two, profile, &index);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("allows 1..1 but found 2"), "{}", issues[0].message);
    }

    #[test]
    fn follows_complex_types_into_their_own_definitions() {
        let index = index_with(vec![patient_sd(), human_name_sd()]);
        let patient = json!({
            "resourceType": "Patient",
            "name": [{"family": "Smith", "nosuchfield": "x"}]
        });
        let issues = validate(&patient, &index);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("nosuchfield"), "{}", issues[0].message);
        // The path points at the offending entry, not just the resource.
        assert_eq!(issues[0].path, "Patient.name[0].nosuchfield");
    }

    #[test]
    fn checks_fixed_values_and_patterns() {
        let sd = json!({
            "resourceType": "StructureDefinition",
            "url": "http://acme.org/StructureDefinition/FinalObs",
            "name": "FinalObs", "kind": "resource", "type": "Observation",
            "derivation": "constraint",
            "snapshot": { "element": [
                {"path": "Observation", "min": 0, "max": "*"},
                {"path": "Observation.status", "min": 1, "max": "1",
                 "type": [{"code": "code"}], "fixedCode": "final"},
                {"path": "Observation.code", "min": 1, "max": "1",
                 "type": [{"code": "CodeableConcept"}],
                 "patternCodeableConcept": {"coding": [{"code": "1234-5"}]}}
            ]}
        });
        let index = index_with(vec![sd]);
        let profile = index.get("http://acme.org/StructureDefinition/FinalObs").unwrap();

        let good = json!({
            "resourceType": "Observation",
            "status": "final",
            "code": {"text": "anything", "coding": [{"system": "http://loinc.org", "code": "1234-5"}]}
        });
        assert_eq!(validate_against(&good, profile, &index), vec![]);

        let bad = json!({
            "resourceType": "Observation",
            "status": "preliminary",
            "code": {"coding": [{"code": "9999-9"}]}
        });
        let issues = validate_against(&bad, profile, &index);
        assert_eq!(issues.len(), 2);
        assert!(issues.iter().any(|i| i.message.contains("fixed to 'final'")));
        assert!(issues.iter().any(|i| i.message.contains("match the pattern")));
    }

    #[test]
    fn primitive_extensions_are_not_unknown_elements() {
        let index = index_with(vec![patient_sd(), human_name_sd()]);
        let patient = json!({
            "resourceType": "Patient",
            "gender": "female",
            "_gender": {"extension": [{"url": "http://acme.org/x", "valueString": "y"}]}
        });
        assert_eq!(validate(&patient, &index), vec![]);
    }

    #[test]
    fn validate_declared_reports_a_profile_that_is_not_installed() {
        let index = index_with(vec![patient_sd(), human_name_sd(), meta_sd()]);
        let patient = json!({
            "resourceType": "Patient",
            "meta": {"profile": ["http://acme.org/StructureDefinition/Nope"]}
        });
        let issues = validate_declared(&patient, &index).expect("base profile applies");
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("not installed"), "{}", issues[0].message);
    }

    #[test]
    fn validate_declared_says_nothing_when_no_profile_is_installed() {
        let index = ProfileIndex::default();
        let patient = json!({"resourceType": "Patient"});
        assert!(validate_declared(&patient, &index).is_none());
    }

    #[test]
    fn validate_declared_applies_both_the_base_and_the_declared_profile() {
        let constrained = json!({
            "resourceType": "StructureDefinition",
            "url": "http://acme.org/StructureDefinition/AcmePatient",
            "name": "AcmePatient", "kind": "resource", "type": "Patient",
            "derivation": "constraint",
            "snapshot": { "element": [
                {"path": "Patient", "min": 0, "max": "*"},
                {"path": "Patient.meta", "min": 0, "max": "1", "type": [{"code": "Meta"}]},
                {"path": "Patient.gender", "min": 1, "max": "1", "type": [{"code": "code"}]}
            ]}
        });
        let index = index_with(vec![patient_sd(), human_name_sd(), meta_sd(), constrained]);
        let patient = json!({
            "resourceType": "Patient",
            "meta": {"profile": ["http://acme.org/StructureDefinition/AcmePatient"]},
            "active": "not a boolean"
        });
        let issues = validate_declared(&patient, &index).unwrap();
        // Base: active has the wrong type. Profile: gender is required.
        assert!(issues.iter().any(|i| i.message.contains("must be a boolean")));
        assert!(issues.iter().any(|i| i.message.contains("Patient.gender is required")));
    }

    #[test]
    fn recursion_stops_on_a_self_referential_type() {
        // A type whose element is of its own type would recurse forever
        // without the visited-type guard.
        let looping = json!({
            "resourceType": "StructureDefinition",
            "url": "http://acme.org/StructureDefinition/Node",
            "name": "Node", "kind": "complex-type", "type": "Node",
            "derivation": "specialization",
            "snapshot": { "element": [
                {"path": "Node", "min": 0, "max": "*"},
                {"path": "Node.child", "min": 0, "max": "*", "type": [{"code": "Node"}]}
            ]}
        });
        let holder = json!({
            "resourceType": "StructureDefinition",
            "url": "http://acme.org/StructureDefinition/Tree",
            "name": "Tree", "kind": "resource", "type": "Tree",
            "derivation": "specialization",
            "snapshot": { "element": [
                {"path": "Tree", "min": 0, "max": "*"},
                {"path": "Tree.root", "min": 0, "max": "1", "type": [{"code": "Node"}]}
            ]}
        });
        let index = index_with(vec![looping, holder]);
        let profile = index.base_for_type("Tree").unwrap();
        let deep = json!({
            "resourceType": "Tree",
            "root": {"child": [{"child": [{"child": [{"child": []}]}]}]}
        });
        // Terminates, and says nothing wrong about a well-formed tree.
        assert_eq!(validate_against(&deep, profile, &index), vec![]);
    }
}
