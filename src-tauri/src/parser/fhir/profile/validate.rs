//! Validation of a resource against a StructureDefinition snapshot.
//!
//! What is checked:
//!
//! - **Cardinality** — a required element that is absent, or a `0..1`
//!   element that repeats; and in JSON, the shape that goes with it: a
//!   repeating element is an array even with one item, a single-valued one
//!   never is.
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
use crate::parser::fhir::{FhirFormat, FhirValidationIssue};

/// How deep to follow complex types into their own definitions. FHIR data
/// types nest (Reference → Identifier → Period), and a bound keeps a cyclic
/// or pathological profile from hanging the UI.
const MAX_DEPTH: usize = 6;

/// What every level of the walk shares.
struct Context<'a> {
    index: &'a ProfileIndex,
    /// The resource was written in JSON. Only JSON has arrays, so only
    /// there can a repeating element be written as a single value, or a
    /// single-valued one as a list; a resource read from XML got its arrays
    /// from the definitions while it was converted.
    json: bool,
}

/// Validate `resource`, written in JSON, against `profile`, following
/// child types through `index` where the profile itself does not
/// constrain them.
pub fn validate_against(
    resource: &Value,
    profile: &Profile,
    index: &ProfileIndex,
) -> Vec<FhirValidationIssue> {
    against(resource, profile, &Context { index, json: true })
}

fn against(resource: &Value, profile: &Profile, cx: &Context) -> Vec<FhirValidationIssue> {
    let mut issues = Vec::new();
    let root = profile.type_name.clone();
    walk(
        resource,
        profile,
        &root,
        &root,
        cx,
        0,
        &mut Vec::new(),
        &mut issues,
    );
    issues
}

/// Validate a resource written in JSON against every profile it declares
/// in `meta.profile`, plus the base definition of its resource type.
///
/// Returns `None` when nothing applicable is installed, so the caller can
/// stay quiet rather than claim a resource passed checks that never ran.
///
/// A declared profile that cannot be applied — not installed, or installed
/// at a different version than the one pinned — is reported and the rest
/// still runs: the base definition can still catch a misspelled element,
/// and one absent package must not hide that.
pub fn validate_declared(
    resource: &Value,
    index: &ProfileIndex,
) -> Option<Vec<FhirValidationIssue>> {
    validate_declared_status(resource, index, &FhirFormat::Json).map(|(issues, _)| issues)
}

/// As [`validate_declared`] for a resource read from `format`, also telling
/// whether any definition was actually applied: `false` when the only
/// findings are "declared but not installed" notes, so a caller never
/// reports such a resource as checked.
pub fn validate_declared_status(
    resource: &Value,
    index: &ProfileIndex,
    format: &FhirFormat,
) -> Option<(Vec<FhirValidationIssue>, bool)> {
    use crate::parser::fhir::profile::model::Lookup;

    let cx = Context { index, json: *format == FhirFormat::Json };

    let resource_type = resource.get("resourceType")?.as_str()?;

    let mut applied = Vec::new();
    let mut issues = Vec::new();
    if let Some(base) = index.base_for_type(resource_type) {
        applied.push(base);
    }
    if let Some(declared) = resource
        .get("meta")
        .and_then(|m| m.get("profile"))
        .and_then(|p| p.as_array())
    {
        for canonical in declared.iter().filter_map(|u| u.as_str()) {
            let skipped = match index.lookup(canonical) {
                Lookup::Found(profile) => {
                    match applicability(profile, resource_type, index) {
                        Some(true) => applied.push(profile),
                        Some(false) => issues.push(other_type(canonical, profile, resource_type, "meta.profile".into())),
                        None => issues.push(undetermined(canonical, profile, resource_type, "meta.profile".into())),
                    }
                    continue;
                }
                Lookup::Missing => format!(
                    "Profile {} is declared but not installed — install the package \
                     that defines it to check conformance",
                    canonical
                ),
                Lookup::WrongVersion { installed } => wrong_version_message(canonical, &installed),
            };
            issues.push(FhirValidationIssue {
                severity: "warning".into(),
                message: skipped,
                path: "meta.profile".into(),
                rule_id: None,
            });
        }
    }

    if applied.is_empty() && issues.is_empty() {
        return None;
    }

    let checked = !applied.is_empty();
    for profile in applied {
        issues.extend(against(resource, profile, &cx));
    }
    Some((dedup_issues(issues), checked))
}

/// Whether `profile` can apply to a resource of `resource_type`: it
/// constrains that type or one the type derives from (Resource, which every
/// resource is; DomainResource; in R5 CanonicalResource, MetadataResource).
/// A Patient that declares an Observation profile was walked as if it were
/// an Observation ("Observation.status is required").
///
/// `Some(true)` applies, `Some(false)` provably does not, `None` cannot be
/// told: the profile is on an abstract type, and no installed definition of
/// the resource's type says whether it derives from that.
fn applicability(profile: &Profile, resource_type: &str, index: &ProfileIndex) -> Option<bool> {
    let target = profile.type_name.as_str();
    if target == resource_type || target == "Resource" {
        return Some(true);
    }
    // Read in the profile's own FHIR release: R4's CapabilityStatement
    // derives from DomainResource, R5's from CanonicalResource.
    let (ancestors, complete) = ancestors_of(resource_type, &profile.fhir_version, index);
    if ancestors.iter().any(|a| a == target) {
        return Some(true);
    }
    if complete {
        return Some(false);
    }
    match target {
        // In R4 and R5 every resource but these three derives from it.
        "DomainResource" => Some(!matches!(resource_type, "Bundle" | "Binary" | "Parameters")),
        "CanonicalResource" | "MetadataResource" => None,
        // A concrete type is no other type's ancestor.
        _ => Some(false),
    }
}

/// The types `resource_type` derives from in FHIR release `release`, read
/// up the `baseDefinition` chain of its installed definitions of that
/// release (the built-in R4 core holds every R4 resource's), nearest first,
/// and whether the chain was read up to Resource, its root. A definition
/// not installed is still named by its canonical URL; the chain stops there.
fn ancestors_of(resource_type: &str, release: &str, index: &ProfileIndex) -> (Vec<String>, bool) {
    const BASE: &str = "http://hl7.org/fhir/StructureDefinition/";
    let mut out: Vec<String> = Vec::new();
    let Some(mut current) = index.base_for_type_in(resource_type, release) else {
        return (out, false);
    };
    // Bounded: a definition chain is a few steps long, and a cycle in
    // hand-made definitions must not hang the validation.
    for _ in 0..16 {
        if current.type_name == "Resource" {
            return (out, true);
        }
        let Some(url) = current.base.as_deref() else {
            return (out, false);
        };
        match index.get_in_release(url, release) {
            Some(next) => {
                out.push(next.type_name.clone());
                current = next;
            }
            None => {
                let name = url.strip_prefix(BASE).unwrap_or(url).to_string();
                let root = name == "Resource";
                out.push(name);
                return (out, root);
            }
        }
    }
    (out, false)
}

/// The finding for a declared profile on an abstract type the resource's
/// type may or may not derive from, with nothing installed to tell: not
/// applied, and said so in the words ("was skipped") that make Run all and
/// `bridgelab-cli test` report the resource as not checked against it.
fn undetermined(canonical: &str, profile: &Profile, resource_type: &str, path: String) -> FhirValidationIssue {
    issue(
        "warning",
        format!(
            "Profile {} constrains {}, and no installed definition of {} says whether a {} is one, \
             so this profile was skipped",
            canonical, profile.type_name, resource_type, resource_type
        ),
        path,
    )
}

/// The finding for a declared profile of another resource type: the
/// resource cannot conform to it, which is an error, and it is not
/// applied.
fn other_type(canonical: &str, profile: &Profile, resource_type: &str, path: String) -> FhirValidationIssue {
    issue(
        "error",
        format!(
            "Profile {} is declared, but it constrains {}, not {}: a {} cannot conform to it, so it was not applied",
            canonical, profile.type_name, resource_type, resource_type
        ),
        path,
    )
}

/// How many resources may nest inside one another before the walk stops:
/// a Bundle of Bundles of contained resources is legal, a runaway one is
/// not worth hanging the UI over.
const MAX_NESTING: usize = 4;

/// Validate a resource found inside another one, against its own base
/// definition and whatever it declares in `meta.profile`, reporting under
/// `report_path`. Mirrors [`validate_declared`] for the document root.
fn validate_nested(
    resource: &Value,
    report_path: &str,
    cx: &Context,
    depth: usize,
    issues: &mut Vec<FhirValidationIssue>,
) {
    use crate::parser::fhir::profile::model::Lookup;

    let index = cx.index;

    // `depth` counts element levels; each nested resource sits a few of
    // those below its parent, so the element budget alone bounds nesting.
    // The explicit cap keeps a pathological document from spending it all
    // on resources-in-resources before any element is looked at.
    if depth / 2 >= MAX_NESTING {
        return;
    }
    let Some(resource_type) = resource.get("resourceType").and_then(|v| v.as_str()) else {
        return;
    };

    let mut applied = Vec::new();
    if let Some(base) = index.base_for_type(resource_type) {
        applied.push(base);
    }
    if let Some(declared) = resource
        .get("meta")
        .and_then(|m| m.get("profile"))
        .and_then(|p| p.as_array())
    {
        for canonical in declared.iter().filter_map(|u| u.as_str()) {
            match index.lookup(canonical) {
                Lookup::Found(profile) => match applicability(profile, resource_type, index) {
                    Some(true) => applied.push(profile),
                    Some(false) => issues.push(other_type(
                        canonical,
                        profile,
                        resource_type,
                        format!("{}.meta.profile", report_path),
                    )),
                    None => issues.push(undetermined(
                        canonical,
                        profile,
                        resource_type,
                        format!("{}.meta.profile", report_path),
                    )),
                },
                Lookup::Missing => issues.push(issue(
                    "warning",
                    format!(
                        "Profile {} is declared but not installed — install the package \
                         that defines it to check conformance",
                        canonical
                    ),
                    format!("{}.meta.profile", report_path),
                )),
                // Same words as at the top level: Run all and `bridgelab-cli
                // test` recognise a profile that was not applied by them, so
                // a nested resource pinned to a missing version is "not
                // checked", never a pass.
                Lookup::WrongVersion { installed } => issues.push(issue(
                    "warning",
                    wrong_version_message(canonical, &installed),
                    format!("{}.meta.profile", report_path),
                )),
            }
        }
    }

    for profile in applied {
        let root = profile.type_name.clone();
        // A fresh branch: the cycle guard is per branch, and this resource's
        // element paths start over from its own type.
        walk(
            resource,
            profile,
            &root,
            report_path,
            cx,
            depth + 1,
            &mut Vec::new(),
            issues,
        );
    }
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
    cx: &Context,
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
                    // A primitive may be present only through its
                    // extensions (`_birthDate` with no `birthDate`).
                    let sibling = format!("_{}", element.base_name());
                    if let Some(ext) = object.get(&sibling) {
                        known.insert(sibling.clone());
                        if cx.json {
                            check_shape(element, profile, cx.index, ext, report_path, &sibling, issues);
                        }
                        check_primitive_sibling(ext, &format!("{}.{}", report_path, sibling), cx, depth, seen_types, issues);
                        continue;
                    }
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
        let sibling = format!("_{}", key);
        if let Some(ext) = object.get(&sibling) {
            if cx.json {
                check_shape(element, profile, cx.index, ext, report_path, &sibling, issues);
            }
            check_primitive_sibling(ext, &format!("{}.{}", report_path, sibling), cx, depth, seen_types, issues);
        }
        known.insert(sibling);

        check_cardinality(element, values.len(), report_path, &key, issues);
        if cx.json {
            check_shape(element, profile, cx.index, &object[&key], report_path, &key, issues);
        }

        // FHIR's JSON has no empty values: a missing element is left out,
        // not written as "", [] or {}.
        if object.get(&key).is_some_and(|v| v.as_array().is_some_and(|a| a.is_empty())) {
            issues.push(issue(
                "error",
                format!("{} is an empty array; leave the element out instead", element.path),
                format!("{}.{}", report_path, key),
            ));
        }

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
            // A resource's own id is an `id` (at most 64 letters, digits,
            // '-' and '.'), although the definition types it as a string.
            let applicable = if profile.kind == "resource"
                && element.path == format!("{}.id", profile.type_name)
                && value.is_string()
            {
                Some("id")
            } else {
                applicable
            };

            let empty = match value {
                Value::String(t) => t.is_empty().then_some("an empty string"),
                Value::Object(o) => o.is_empty().then_some("an empty object"),
                _ => None,
            };
            if let Some(what) = empty {
                issues.push(issue(
                    "error",
                    format!("{} is {}; leave the element out instead", element.path, what),
                    child_report.clone(),
                ));
                continue;
            }

            check_type(element, applicable, value, &child_report, issues);
            check_fixed(element, value, &child_report, issues);

            recurse(
                value,
                element,
                applicable,
                profile,
                &child_report,
                cx,
                depth,
                seen_types,
                issues,
            );
        }
    }

    report_unknown(object, &known, profile, report_path, issues);
}

/// The `_name` sibling of a primitive: an Element (id and extensions), or
/// for a repeating primitive an array of them with `null` for the values
/// that have none. Its extensions are checked like any other.
fn check_primitive_sibling(
    value: &Value,
    report_path: &str,
    cx: &Context,
    depth: usize,
    seen_types: &mut Vec<String>,
    issues: &mut Vec<FhirValidationIssue>,
) {
    let items: Vec<(String, &Value)> = match value {
        Value::Array(arr) => arr.iter().enumerate().map(|(i, v)| (format!("{}[{}]", report_path, i), v)).collect(),
        v => vec![(report_path.to_string(), v)],
    };
    for (path, item) in items {
        match item {
            Value::Null if value.is_array() => {}
            Value::Object(_) => {
                if depth >= MAX_DEPTH || seen_types.iter().any(|t| t == "Element") {
                    continue;
                }
                let Some(element) = cx.index.base_for_type("Element") else { continue };
                seen_types.push("Element".into());
                let root = element.type_name.clone();
                walk(item, element, &root, &path, cx, depth + 1, seen_types, issues);
                seen_types.pop();
            }
            other => issues.push(issue(
                "error",
                format!(
                    "{} holds a primitive's id and extensions, so it must be an object, not {}",
                    path.rsplit('.').next().unwrap_or(&path),
                    json_kind(other)
                ),
                path.clone(),
            )),
        }
    }
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
    cx: &Context,
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
            cx,
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
    // A nested resource — a Bundle entry, a contained resource, a
    // Parameters part — is a root of its own: its type comes from the
    // instance and its meta.profile declarations apply to it exactly as
    // they would at the top of a document.
    if type_code == "Resource" || type_code == "DomainResource" {
        validate_nested(value, report_path, cx, depth, issues);
        return;
    }

    if seen_types.contains(&type_code.to_string()) {
        return; // already expanded on this branch: stop before looping
    }
    let Some(type_profile) = cx.index.base_for_type(type_code) else {
        return;
    };

    seen_types.push(type_code.to_string());
    let root = type_profile.type_name.clone();
    walk(
        value,
        type_profile,
        &root,
        report_path,
        cx,
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

/// FHIR's JSON writes an element that can repeat as an array, even when it
/// holds one item, and one that cannot as a bare value. The values read
/// the same either way, which is why nothing else here notices, but a
/// server parsing `"name": {…}` or `"gender": ["male"]` refuses the
/// resource. `key` is the element's JSON key, or its `_key` sibling, which
/// takes the same shape.
///
/// The messages leave out the cardinality: the base definition and a
/// profile narrowing it give the same finding, reported once.
fn check_shape(
    element: &ProfileElement,
    profile: &Profile,
    index: &ProfileIndex,
    value: &Value,
    report_path: &str,
    key: &str,
    issues: &mut Vec<FhirValidationIssue>,
) {
    let Some(repeats) = repeats_in_json(element, profile, index) else {
        return;
    };
    let message = match (value, repeats) {
        (Value::Array(_), true) => return,
        // An empty array has a finding of its own.
        (Value::Array(items), false) if items.is_empty() => return,
        (Value::Array(_), false) => format!(
            "{} does not repeat, so in JSON '{}' must be a single value, not an array",
            element.path, key
        ),
        (_, true) => format!(
            "{} repeats, so in JSON '{}' must be an array, even with one item",
            element.path, key
        ),
        (_, false) => return,
    };
    issues.push(issue("error", message, format!("{}.{}", report_path, key)));
}

/// Whether JSON writes `element` as an array: whether it repeats in the
/// base definition it comes from. `None` when that cannot be told, and then
/// the shape is not judged rather than guessed.
fn repeats_in_json(element: &ProfileElement, profile: &Profile, index: &ProfileIndex) -> Option<bool> {
    if let Some(repeats) = element.repeats_in_base() {
        return Some(repeats);
    }
    // A base definition is its own base.
    if !profile.is_constraint() {
        return (element.max == Some(1)).then_some(false);
    }
    // A profile whose package kept no base maximums, with a maximum of one:
    // the same element in the definition of the type it constrains says
    // whether that is the element's own or a narrowed one.
    let base = index.base_for_type(&profile.type_name)?;
    let same = base
        .elements
        .iter()
        .find(|e| e.path == element.path && e.slice_name.is_none())?;
    match same.max {
        Some(1) => Some(false),
        _ => same.repeats_in_base(),
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
        return;
    }

    // The JSON kind is right; now the value itself, where the type says
    // what a value looks like.
    if let Some(text) = value.as_str() {
        if let Some((severity, why)) = primitive_format_problem(declared, text, &element.path) {
            issues.push(issue(
                severity,
                format!("{} {}", element.path, why),
                report_path.to_string(),
            ));
        }
    }
    if let Some(n) = value.as_i64() {
        let bad = match declared {
            "positiveInt" if n < 1 => Some("must be a positive integer (1 or more)"),
            "unsignedInt" if n < 0 => Some("must not be negative"),
            _ => None,
        };
        if let Some(why) = bad {
            issues.push(issue(
                "error",
                format!("{} {} but is {}", element.path, why, n),
                report_path.to_string(),
            ));
        }
    }
}

/// Lexical rules for the primitive types, as the specification states them.
///
/// Returns the severity and a description of what is wrong, or `None` when
/// the value is well-formed. Everything here is a regular expression from
/// the datatypes page, with one addition: a *warning* on an endpoint with
/// no scheme. A relative url is legal in general — the core package itself
/// stores bare type names in a `url`-typed extension — but an element whose
/// whole purpose is to be connected to, written as `https//host`, is a typo
/// nobody wants to find in production. So that check is confined to those
/// elements.
fn primitive_format_problem(
    declared: &str,
    text: &str,
    element_path: &str,
) -> Option<(&'static str, String)> {
    use regex::Regex;
    use std::sync::LazyLock;

    const YEAR: &str = r"([0-9]([0-9]([0-9][1-9]|[1-9]0)|[1-9]00)|[1-9]000)";
    const MONTH: &str = r"(0[1-9]|1[0-2])";
    const DAY: &str = r"(0[1-9]|[1-2][0-9]|3[0-1])";
    const TIME: &str = r"([01][0-9]|2[0-3]):[0-5][0-9]:([0-5][0-9]|60)(\.[0-9]+)?";
    const TZ: &str = r"(Z|(\+|-)((0[0-9]|1[0-3]):[0-5][0-9]|14:00))";

    static DATE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(&format!("^{YEAR}(-{MONTH}(-{DAY})?)?$")).unwrap());
    static DATE_TIME: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!("^{YEAR}(-{MONTH}(-{DAY}(T{TIME}{TZ})?)?)?$")).unwrap()
    });
    static INSTANT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(&format!("^{YEAR}-{MONTH}-{DAY}T{TIME}{TZ}$")).unwrap());
    static TIME_ONLY: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!("^{TIME}$")).unwrap());
    static CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[^\s]+( [^\s]+)*$").unwrap());
    static ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z0-9\-\.]{1,64}$").unwrap());
    static OID: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^urn:oid:[0-2](\.(0|[1-9][0-9]*))+$").unwrap());
    static UUID: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^urn:uuid:[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
            .unwrap()
    });

    let shown = || {
        if text.chars().count() > 40 {
            format!("{}…", text.chars().take(40).collect::<String>())
        } else {
            text.to_string()
        }
    };
    let bad = |what: &str| Some(("error", format!("is a {} but '{}' is not a valid {}", declared, shown(), what)));

    // The regular expressions allow 31 days in every month.
    let impossible_day = || {
        let (y, m, d) = (text.get(0..4)?, text.get(5..7)?, text.get(8..10)?);
        let (y, m, d) = (y.parse::<i32>().ok()?, m.parse::<u32>().ok()?, d.parse::<u32>().ok()?);
        chrono::NaiveDate::from_ymd_opt(y, m, d).is_none().then_some(())
    };

    match declared {
        "date" if !DATE.is_match(text) => bad("date (YYYY, YYYY-MM or YYYY-MM-DD)"),
        "date" | "dateTime" | "instant" if impossible_day().is_some() => {
            bad("date (that day does not exist)")
        }
        "dateTime" if !DATE_TIME.is_match(text) => {
            bad("dateTime (a date, or a date with a time and a time zone)")
        }
        "instant" if !INSTANT.is_match(text) => bad("instant (full date, time and time zone)"),
        "time" if !TIME_ONLY.is_match(text) => bad("time (hh:mm:ss)"),
        "code" if !CODE.is_match(text) => {
            bad("code (no leading, trailing or doubled whitespace)")
        }
        "id" if !ID.is_match(text) => bad("id (letters, digits, '-' and '.', at most 64)"),
        "oid" if !OID.is_match(text) => bad("oid (urn:oid:…)"),
        "uuid" if !UUID.is_match(text) => bad("uuid (urn:uuid: with lowercase hex)"),
        "uri" | "url" | "canonical" if text.chars().any(char::is_whitespace) => {
            bad("URI (it contains whitespace)")
        }
        "url" if is_endpoint_element(element_path) && !looks_absolute(text) => Some((
            "warning",
            format!(
                "is an endpoint and '{}' has no scheme — nothing can connect to it as written",
                shown()
            ),
        )),
        _ => None,
    }
}

/// Elements that exist to be connected to: MessageHeader's source and
/// destination endpoints, Endpoint.address, a Subscription channel.
fn is_endpoint_element(path: &str) -> bool {
    path.ends_with(".endpoint") || path == "Endpoint.address"
}

/// `scheme:` before any `/` — `https://x`, `urn:uuid:…`, `mailto:a@b`.
fn looks_absolute(s: &str) -> bool {
    match s.find(':') {
        Some(i) => {
            let scheme = &s[..i];
            !scheme.is_empty()
                && !scheme.contains('/')
                && scheme.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                && scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
        }
        None => false,
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
        rule_id: None,
    }
}

/// The warning for a declared profile pinned to a version that is not
/// installed. Its ending, "was skipped", is what Run all and `bridgelab-cli
/// test` look for to report the resource as not checked.
fn wrong_version_message(canonical: &str, installed: &[String]) -> String {
    let (url, wanted) = canonical.split_once('|').unwrap_or((canonical, ""));
    let installed = installed
        .iter()
        .map(|v| if v.is_empty() { "an unversioned copy" } else { v.as_str() })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Profile {} is pinned to version {} but only {} is installed — \
         conformance against another version would answer a different \
         question, so this profile was skipped",
        url, wanted, installed
    )
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
        // bridgelab-cli's "not checked" rule and the Test Case Library's
        // (testcases/expectations.ts) look for this path and wording.
        assert_eq!(issues[0].path, "meta.profile");
        assert!(issues[0].message.contains("is declared but not installed"), "{}", issues[0].message);
    }

    /// A Patient declaring a profile of Observation is not walked as an
    /// Observation: one error says the profile is for another type, and the
    /// Patient base still runs.
    #[test]
    fn a_profile_of_another_resource_type_is_not_applied() {
        let observation = json!({
            "resourceType": "StructureDefinition",
            "url": "http://acme.org/StructureDefinition/AcmeObs",
            "name": "AcmeObs", "kind": "resource", "type": "Observation",
            "derivation": "constraint",
            "snapshot": { "element": [
                {"path": "Observation", "min": 0, "max": "*"},
                {"path": "Observation.status", "min": 1, "max": "1", "type": [{"code": "code"}]}
            ]}
        });
        let index = index_with(vec![patient_sd(), human_name_sd(), meta_sd(), observation]);
        let patient = json!({
            "resourceType": "Patient",
            "meta": {"profile": ["http://acme.org/StructureDefinition/AcmeObs"]},
            "gender": "male"
        });
        let issues = validate_declared(&patient, &index).unwrap();
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(issues[0].severity, "error");
        assert_eq!(issues[0].path, "meta.profile");
        assert!(issues[0].message.contains("constrains Observation, not Patient"), "{}", issues[0].message);
        assert!(!issues.iter().any(|i| i.message.contains("Observation.status")));
    }

    /// A DomainResource profile applies only to resources derived from
    /// DomainResource: not to a Bundle, Binary or Parameters, nor to a type
    /// whose definition says it derives from Resource. A Resource profile
    /// applies to every resource.
    #[test]
    fn a_domain_resource_profile_applies_only_to_domain_resources() {
        let abstract_profile = |name: &str, base: &str| json!({
            "resourceType": "StructureDefinition",
            "url": format!("http://acme.org/StructureDefinition/{name}"),
            "name": name, "kind": "resource", "type": base, "abstract": true,
            "derivation": "constraint",
            "snapshot": { "element": [{"path": base, "min": 0, "max": "*"}]}
        });
        let widget = json!({
            "resourceType": "StructureDefinition",
            "url": "http://acme.org/StructureDefinition/Widget",
            "name": "Widget", "kind": "resource", "type": "Widget",
            "derivation": "specialization",
            "baseDefinition": "http://hl7.org/fhir/StructureDefinition/Resource",
            "snapshot": { "element": [{"path": "Widget", "min": 0, "max": "*"}]}
        });
        let index = index_with(vec![
            patient_sd(), human_name_sd(), meta_sd(), bundle_sd(), widget,
            abstract_profile("AnyDomain", "DomainResource"), abstract_profile("AnyResource", "Resource"),
        ]);
        let declaring = |resource_type: &str, profile: &str| {
            let mut r = json!({"resourceType": resource_type, "meta": {"profile": [format!("http://acme.org/StructureDefinition/{profile}")]}});
            if resource_type == "Bundle" {
                r["type"] = json!("collection");
            }
            validate_declared(&r, &index).unwrap_or_default()
        };
        let mismatch = |issues: &[FhirValidationIssue]| issues.iter().any(|i| i.path == "meta.profile" && i.message.contains("constrains DomainResource"));

        assert!(!mismatch(&declaring("Patient", "AnyDomain")), "{:?}", declaring("Patient", "AnyDomain"));
        for not_domain in ["Bundle", "Binary", "Parameters", "Widget"] {
            let issues = declaring(not_domain, "AnyDomain");
            assert!(mismatch(&issues), "{not_domain}: {issues:?}");
            assert!(issues.iter().any(|i| i.severity == "error" && i.message.contains(&format!("not {not_domain}"))));
        }
        for any in ["Patient", "Bundle", "Binary", "Widget"] {
            assert!(!declaring(any, "AnyResource").iter().any(|i| i.message.contains("constrains")), "{any}");
        }
        // The chain is read where it is installed, the list used where not.
        let domain = index.get("http://acme.org/StructureDefinition/AnyDomain").unwrap();
        assert_eq!(applicability(domain, "Widget", &index), Some(false));
        assert_eq!(applicability(domain, "Observation", &index), Some(true));
        assert_eq!(applicability(domain, "Binary", &index), Some(false));
    }

    /// R5 puts CanonicalResource between CapabilityStatement and
    /// DomainResource: a profile on it applies to a CapabilityStatement,
    /// and is a mismatch for a Patient, read from the R5 definitions. With
    /// no definition of the resource's type in that release it cannot be
    /// told, and the profile is skipped without a mismatch reported.
    #[test]
    fn a_profile_on_an_intermediate_type_applies_to_the_types_derived_from_it() {
        let base = "http://hl7.org/fhir/StructureDefinition/";
        let sd = |name: &str, parent: Option<&str>, derivation: &str, extra: Vec<Value>| {
            let mut elements = vec![json!({"path": name, "min": 0, "max": "*"})];
            elements.extend(extra);
            let mut v = json!({
                "resourceType": "StructureDefinition",
                "url": format!("{base}{name}"),
                "name": name, "kind": "resource", "type": name,
                "derivation": derivation,
                "snapshot": {"element": elements}
            });
            if let Some(p) = parent {
                v["baseDefinition"] = json!(format!("{base}{p}"));
            }
            v
        };
        let mut canonical_profile = sd(
            "CanonicalResource",
            Some("CanonicalResource"),
            "constraint",
            vec![json!({"path": "CanonicalResource.url", "min": 1, "max": "1", "type": [{"code": "uri"}]})],
        );
        canonical_profile["url"] = json!("http://acme.org/StructureDefinition/MustHaveUrl");
        let r5 = vec![
            sd("Resource", None, "specialization", vec![]),
            sd("DomainResource", Some("Resource"), "specialization", vec![]),
            sd("CanonicalResource", Some("DomainResource"), "specialization", vec![]),
            sd("CapabilityStatement", Some("CanonicalResource"), "specialization",
               vec![json!({"path": "CapabilityStatement.url", "min": 0, "max": "1", "type": [{"code": "uri"}]})]),
            sd("Patient", Some("DomainResource"), "specialization", vec![]),
            canonical_profile,
        ];
        let mut index = ProfileIndex::default();
        index.add_package(ProfilePackage {
            name: "r5".into(),
            version: "5.0.0".into(),
            title: String::new(),
            fhir_version: "5.0.0".into(),
            profiles: r5.iter().filter_map(from_structure_definition).collect(),
        });
        let profile = index.get("http://acme.org/StructureDefinition/MustHaveUrl").unwrap();
        assert_eq!(applicability(profile, "CapabilityStatement", &index), Some(true));
        assert_eq!(applicability(profile, "Patient", &index), Some(false));
        // No R5 definition of the type: not provable either way.
        assert_eq!(applicability(profile, "Questionnaire", &index), None);

        let declaring = |resource_type: &str| {
            validate_declared(
                &json!({"resourceType": resource_type, "meta": {"profile": ["http://acme.org/StructureDefinition/MustHaveUrl"]}}),
                &index,
            )
            .unwrap_or_default()
        };
        let cs = declaring("CapabilityStatement");
        assert!(!cs.iter().any(|i| i.message.contains("constrains")), "{cs:?}");
        assert!(cs.iter().any(|i| i.message.contains("url") && i.message.contains("required")), "the profile ran: {cs:?}");
        let patient = declaring("Patient");
        assert!(patient.iter().any(|i| i.severity == "error" && i.message.contains("constrains CanonicalResource, not Patient")), "{patient:?}");
        let questionnaire = declaring("Questionnaire");
        assert!(questionnaire.iter().any(|i| i.severity == "warning" && i.message.contains("was skipped")), "{questionnaire:?}");
        assert!(!questionnaire.iter().any(|i| i.severity == "error"), "{questionnaire:?}");
    }

    #[test]
    fn validate_declared_keeps_checking_the_base_when_a_declared_profile_is_missing() {
        let index = index_with(vec![patient_sd(), human_name_sd(), meta_sd()]);
        let patient = json!({
            "resourceType": "Patient",
            "meta": {"profile": ["http://acme.org/StructureDefinition/Nope"]},
            "active": "not a boolean"
        });
        let issues = validate_declared(&patient, &index).unwrap();
        // The missing profile is reported, and so is the base finding that
        // used to be swallowed by an early return.
        assert!(issues.iter().any(|i| i.message.contains("not installed")), "{issues:?}");
        assert!(issues.iter().any(|i| i.message.contains("must be a boolean")), "{issues:?}");
    }

    fn acme_patient_sd(version: &str, required: &str) -> Value {
        json!({
            "resourceType": "StructureDefinition",
            "url": "http://acme.org/StructureDefinition/AcmePatient",
            "version": version,
            "name": "AcmePatient", "kind": "resource", "type": "Patient",
            "derivation": "constraint",
            "snapshot": { "element": [
                {"path": "Patient", "min": 0, "max": "*"},
                {"path": "Patient.meta", "min": 0, "max": "1", "type": [{"code": "Meta"}]},
                {"path": format!("Patient.{required}"), "min": 1, "max": "1",
                 "type": [{"code": if required == "active" { "boolean" } else { "code" }}]}
            ]}
        })
    }

    #[test]
    fn validate_declared_honours_a_version_pin() {
        // Two versions of the same profile: 1.0.0 requires gender, 2.0.0
        // requires active. Which one runs must follow the pin, not luck.
        let mut index = index_with(vec![patient_sd(), human_name_sd(), meta_sd()]);
        index.add_package(ProfilePackage {
            name: "acme.ig".into(),
            version: "1.0.0".into(),
            title: String::new(),
            fhir_version: "4.0.1".into(),
            profiles: vec![from_structure_definition(&acme_patient_sd("1.0.0", "gender")).unwrap()],
        });
        index.add_package(ProfilePackage {
            name: "acme.ig".into(),
            version: "2.0.0".into(),
            title: String::new(),
            fhir_version: "4.0.1".into(),
            profiles: vec![from_structure_definition(&acme_patient_sd("2.0.0", "active")).unwrap()],
        });
        let declaring = |canonical: &str| {
            json!({"resourceType": "Patient", "meta": {"profile": [canonical]}})
        };
        let url = "http://acme.org/StructureDefinition/AcmePatient";
        let messages = |canonical: &str| {
            validate_declared(&declaring(canonical), &index)
                .unwrap()
                .into_iter()
                .map(|i| i.message)
                .collect::<Vec<_>>()
        };

        let pinned_old = messages(&format!("{url}|1.0.0"));
        assert!(pinned_old.iter().any(|m| m.contains("Patient.gender is required")), "{pinned_old:?}");
        assert!(!pinned_old.iter().any(|m| m.contains("Patient.active")), "{pinned_old:?}");

        let pinned_new = messages(&format!("{url}|2.0.0"));
        assert!(pinned_new.iter().any(|m| m.contains("Patient.active is required")), "{pinned_new:?}");
        assert!(!pinned_new.iter().any(|m| m.contains("Patient.gender")), "{pinned_new:?}");

        // Unpinned: the newest installed version answers.
        let unpinned = messages(url);
        assert!(unpinned.iter().any(|m| m.contains("Patient.active is required")), "{unpinned:?}");

        // Pinned to a version nobody installed: say so, name what is there,
        // and do not quietly validate against something else.
        let pinned_absent = messages(&format!("{url}|3.0.0"));
        assert_eq!(pinned_absent.len(), 1, "{pinned_absent:?}");
        assert!(pinned_absent[0].contains("pinned to version 3.0.0"), "{pinned_absent:?}");
        assert!(pinned_absent[0].contains("2.0.0, 1.0.0"), "{pinned_absent:?}");
        // The wording the "not checked" rule of bridgelab-cli and of the
        // Test Case Library looks for.
        assert!(pinned_absent[0].contains("was skipped"), "{pinned_absent:?}");
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

    fn bundle_sd() -> Value {
        json!({
            "resourceType": "StructureDefinition",
            "url": "http://hl7.org/fhir/StructureDefinition/Bundle",
            "name": "Bundle", "kind": "resource", "type": "Bundle",
            "derivation": "specialization",
            "snapshot": { "element": [
                {"path": "Bundle", "min": 0, "max": "*"},
                {"path": "Bundle.type", "min": 1, "max": "1", "type": [{"code": "code"}]},
                {"path": "Bundle.entry", "min": 0, "max": "*", "type": [{"code": "BackboneElement"}]},
                {"path": "Bundle.entry.fullUrl", "min": 0, "max": "1", "type": [{"code": "uri"}]},
                {"path": "Bundle.entry.resource", "min": 0, "max": "1", "type": [{"code": "Resource"}]}
            ]}
        })
    }

    #[test]
    fn a_bundle_entry_is_validated_against_its_own_declared_profile() {
        let index = index_with(vec![
            patient_sd(), human_name_sd(), meta_sd(), bundle_sd(),
            acme_patient_sd("1.0.0", "gender"),
        ]);
        let bundle = json!({
            "resourceType": "Bundle", "type": "message",
            "entry": [
                {"fullUrl": "urn:uuid:aaaaaaaa-0000-4000-8000-000000000001",
                 "resource": {"resourceType": "Patient",
                              "meta": {"profile": ["http://acme.org/StructureDefinition/AcmePatient"]},
                              "active": "not a boolean"}},
                {"fullUrl": "urn:uuid:aaaaaaaa-0000-4000-8000-000000000002",
                 "resource": {"resourceType": "Patient",
                              "meta": {"profile": ["http://acme.org/StructureDefinition/Nope"]},
                              "genderr": "female"}},
                {"fullUrl": "urn:uuid:aaaaaaaa-0000-4000-8000-000000000003",
                 "resource": {"resourceType": "Patient",
                              "meta": {"profile": ["http://acme.org/StructureDefinition/AcmePatient|9.9.9"]}}}
            ]
        });
        let issues = validate_declared(&bundle, &index).unwrap();
        let at = |path: &str| issues.iter().filter(|i| i.path == path).map(|i| i.message.clone()).collect::<Vec<_>>();
        // Entry 0: the profile it declares (gender required) and the base (active type).
        assert!(at("Bundle.entry[0].resource.gender").iter().any(|m| m.contains("is required")), "{issues:?}");
        assert!(at("Bundle.entry[0].resource.active").iter().any(|m| m.contains("must be a boolean")), "{issues:?}");
        // Entry 1: its profile is missing — said under its own path — and the base still runs.
        assert!(at("Bundle.entry[1].resource.meta.profile").iter().any(|m| m.contains("not installed")), "{issues:?}");
        assert!(at("Bundle.entry[1].resource.genderr").iter().any(|m| m.contains("not defined")), "{issues:?}");
        // Entry 2: pinned to a version nobody installed. The same words as at
        // the top level ("was skipped"), so Run all and `bridgelab-cli test`
        // report it as not checked rather than as a pass.
        let pinned = at("Bundle.entry[2].resource.meta.profile");
        assert!(pinned.iter().any(|m| m.contains("pinned to version 9.9.9") && m.contains("was skipped")), "{issues:?}");
        // The bundle itself is fine.
        assert!(!issues.iter().any(|i| i.path == "Bundle.type"), "{issues:?}");
    }

    #[test]
    fn primitive_values_are_checked_for_their_lexical_form() {
        let endpoint = json!({
            "resourceType": "StructureDefinition",
            "url": "http://acme.org/StructureDefinition/Thing",
            "name": "Thing", "kind": "resource", "type": "Thing",
            "derivation": "specialization",
            "snapshot": { "element": [
                {"path": "Thing", "min": 0, "max": "*"},
                {"path": "Thing.id", "min": 0, "max": "1", "type": [{"code": "id"}]},
                {"path": "Thing.endpoint", "min": 0, "max": "1", "type": [{"code": "url"}]},
                {"path": "Thing.link", "min": 0, "max": "1", "type": [{"code": "url"}]},
                {"path": "Thing.when", "min": 0, "max": "1", "type": [{"code": "dateTime"}]},
                {"path": "Thing.stamp", "min": 0, "max": "1", "type": [{"code": "instant"}]},
                {"path": "Thing.day", "min": 0, "max": "1", "type": [{"code": "date"}]},
                {"path": "Thing.kind", "min": 0, "max": "1", "type": [{"code": "code"}]},
                {"path": "Thing.rank", "min": 0, "max": "1", "type": [{"code": "positiveInt"}]}
            ]}
        });
        let index = index_with(vec![endpoint]);
        let profile = index.base_for_type("Thing").unwrap();
        let good = json!({
            "resourceType": "Thing", "id": "abc-1.2",
            "endpoint": "https://fhir.example/Endpoint/x",
            // A relative url is legal wherever the element is not an endpoint.
            "link": "string",
            "when": "2018-05-24T00:00:00+00:00", "stamp": "2018-05-24T10:20:30.5Z",
            "day": "2018-05", "kind": "final", "rank": 1
        });
        assert_eq!(validate_against(&good, profile, &index), vec![]);

        let bad = json!({
            "resourceType": "Thing", "id": "has space",
            "endpoint": "https//fhir.example/Endpoint/x",
            "when": "2018-05-24T00:00:00", "stamp": "2018-05-24",
            "day": "2018-13-40", "kind": " final", "rank": 0
        });
        let issues = validate_against(&bad, profile, &index);
        let msg = |path: &str| issues.iter().find(|i| i.path == path).map(|i| (i.severity.as_str(), i.message.clone()));
        assert!(msg("Thing.id").unwrap().1.contains("not a valid id"), "{issues:?}");
        assert_eq!(msg("Thing.endpoint").unwrap().0, "warning", "{issues:?}");
        assert!(msg("Thing.endpoint").unwrap().1.contains("no scheme"), "{issues:?}");
        assert!(msg("Thing.when").unwrap().1.contains("time zone"), "{issues:?}");
        assert!(msg("Thing.stamp").unwrap().1.contains("instant"), "{issues:?}");
        assert!(msg("Thing.day").unwrap().1.contains("not a valid date"), "{issues:?}");
        assert!(msg("Thing.kind").unwrap().1.contains("whitespace"), "{issues:?}");
        assert!(msg("Thing.rank").unwrap().1.contains("positive"), "{issues:?}");
    }

    #[test]
    fn a_primitive_extension_sibling_is_checked_against_the_core() {
        let mut index = ProfileIndex::default();
        index.add_builtin(crate::parser::fhir::profile::package::builtin().expect("built-in core"));
        let bad = json!({"resourceType": "Patient", "id": "pj", "name": [{"family": "x", "_given": [null, 3]}],
            "birthDate": "1970-01-01",
            "_birthDate": {"extension": [{"valueString": "x"}], "bogus": 1},
            "_gender": 5});
        let (issues, _) = validate_declared_status(&bad, &index, &FhirFormat::Json).unwrap();
        let text: Vec<String> = issues.iter().map(|i| format!("{} | {}", i.path, i.message)).collect();
        assert!(text.iter().any(|t| t.contains("_birthDate") && t.contains("url is required")), "{:#?}", text);
        assert!(text.iter().any(|t| t.contains("'bogus'")), "{:#?}", text);
        assert!(text.iter().any(|t| t.contains("_gender") && t.contains("must be an object")), "{:#?}", text);
        assert!(text.iter().any(|t| t.contains("_given[1]") && t.contains("must be an object")), "{:#?}", text);

        let good = json!({"resourceType": "Patient", "id": "pj",
            "_birthDate": {"extension": [{"url": "http://hl7.org/fhir/StructureDefinition/patient-birthTime",
                "valueDateTime": "1970-01-01T10:00:00Z"}]}});
        let (issues, _) = validate_declared_status(&good, &index, &FhirFormat::Json).unwrap();
        assert!(issues.iter().all(|i| i.severity != "error"), "{:#?}", issues);
    }

    #[test]
    fn impossible_dates_bad_ids_and_empty_values_are_reported() {
        let mut index = ProfileIndex::default();
        index.add_builtin(crate::parser::fhir::profile::package::builtin().expect("built-in core"));
        let errors = |r: Value| -> Vec<String> {
            validate_declared_status(&r, &index, &FhirFormat::Json)
                .unwrap()
                .0
                .into_iter()
                .filter(|i| i.severity == "error")
                .map(|i| format!("{} | {}", i.path, i.message))
                .collect()
        };
        let e = errors(json!({"resourceType": "Patient", "id": "p", "birthDate": "2019-02-29"}));
        assert!(e.iter().any(|m| m.contains("birthDate") && m.contains("does not exist")), "{e:#?}");
        let e = errors(json!({"resourceType": "Observation", "id": "o", "status": "final", "code": {"text": "x"},
            "effectiveDateTime": "2020-02-30T10:00:00Z", "issued": "2021-04-31T00:00:00Z"}));
        assert_eq!(e.iter().filter(|m| m.contains("does not exist")).count(), 2, "{e:#?}");
        let e = errors(json!({"resourceType": "Patient", "id": "has spaces and is far too long to be an id ever, really, truly"}));
        assert!(e.iter().any(|m| m.contains("Patient.id") && m.contains("not a valid id")), "{e:#?}");
        let e = errors(json!({"resourceType": "Patient", "id": "p", "name": [], "maritalStatus": {},
            "telecom": [{"value": ""}]}));
        assert!(e.iter().any(|m| m.contains("name") && m.contains("empty array")), "{e:#?}");
        assert!(e.iter().any(|m| m.contains("maritalStatus") && m.contains("empty object")), "{e:#?}");
        assert!(e.iter().any(|m| m.contains("telecom") && m.contains("empty string")), "{e:#?}");
        // A leap day and an ordinary resource stay clean.
        let e = errors(json!({"resourceType": "Patient", "id": "p-1.x", "birthDate": "2020-02-29",
            "name": [{"family": "x"}]}));
        assert!(e.is_empty(), "{e:#?}");
    }

    fn core() -> ProfileIndex {
        let mut index = ProfileIndex::default();
        index.add_builtin(crate::parser::fhir::profile::package::builtin().expect("built-in core"));
        index
    }

    fn errors_in(resource: &Value, index: &ProfileIndex, format: &FhirFormat) -> Vec<(String, String)> {
        validate_declared_status(resource, index, format)
            .unwrap()
            .0
            .into_iter()
            .filter(|i| i.severity == "error")
            .map(|i| (i.path, i.message))
            .collect()
    }

    #[test]
    fn json_array_shape_follows_whether_the_element_repeats() {
        let index = core();
        // A repeating element given as one value, and single-valued ones
        // given as arrays: what a FHIR server refuses to read.
        let wrong = json!({"resourceType": "Patient", "id": "p",
            "name": {"family": "X", "given": "Jane"},
            "gender": ["male"], "active": [true],
            "telecom": [{"system": "phone", "value": "1"}],
            "birthDate": "1980-01-01", "_birthDate": [{"id": "b"}]});
        let e = errors_in(&wrong, &index, &FhirFormat::Json);
        let at = |path: &str| e.iter().filter(|(p, _)| p == path).map(|(_, m)| m.as_str()).collect::<Vec<_>>();
        assert_eq!(
            at("Patient.name"),
            vec!["Patient.name repeats, so in JSON 'name' must be an array, even with one item"],
            "{e:#?}"
        );
        assert_eq!(
            at("Patient.name.given"),
            vec!["HumanName.given repeats, so in JSON 'given' must be an array, even with one item"],
            "{e:#?}"
        );
        assert_eq!(
            at("Patient.gender"),
            vec!["Patient.gender does not repeat, so in JSON 'gender' must be a single value, not an array"],
            "{e:#?}"
        );
        assert_eq!(at("Patient.active").len(), 1, "{e:#?}");
        assert!(at("Patient._birthDate")[0].contains("must be a single value"), "{e:#?}");
        // One finding per element: nothing else is wrong with the values.
        assert_eq!(e.len(), 5, "{e:#?}");

        let right = json!({"resourceType": "Patient", "id": "p",
            "name": [{"family": "X", "given": ["Jane"]}],
            "gender": "male", "active": true,
            "birthDate": "1980-01-01", "_birthDate": {"id": "b"}});
        assert_eq!(errors_in(&right, &index, &FhirFormat::Json), vec![]);

        // The same in a Bundle entry and a contained resource.
        let bundle = json!({"resourceType": "Bundle", "type": "collection", "entry": {
            "resource": {"resourceType": "Observation", "id": "o", "status": ["final"], "code": {"text": "x"},
                "contained": [{"resourceType": "Patient", "id": "c", "name": {"family": "Y"}}]}}});
        let e = errors_in(&bundle, &index, &FhirFormat::Json);
        let paths: Vec<&str> = e.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&"Bundle.entry"), "{e:#?}");
        assert!(paths.contains(&"Bundle.entry.resource.status"), "{e:#?}");
        assert!(paths.contains(&"Bundle.entry.resource.contained[0].name"), "{e:#?}");
    }

    #[test]
    fn the_json_shape_is_the_base_definitions_not_a_narrowed_profiles() {
        // A profile that narrows Patient.name to 1..1 leaves it an array.
        let narrowed = |with_base: bool| {
            let mut name = json!({"path": "Patient.name", "min": 1, "max": "1", "type": [{"code": "HumanName"}]});
            if with_base {
                name["base"] = json!({"path": "Patient.name", "min": 0, "max": "*"});
            }
            json!({
                "resourceType": "StructureDefinition",
                "url": "http://acme.org/StructureDefinition/OneName",
                "name": "OneName", "kind": "resource", "type": "Patient",
                "derivation": "constraint",
                "baseDefinition": "http://hl7.org/fhir/StructureDefinition/Patient",
                "snapshot": { "element": [{"path": "Patient", "min": 0, "max": "*"}, name]}
            })
        };
        for with_base in [true, false] {
            let index = index_with(vec![patient_sd(), human_name_sd(), narrowed(with_base)]);
            let profile = index.get("http://acme.org/StructureDefinition/OneName").unwrap();
            let one = json!({"resourceType": "Patient", "name": [{"family": "A"}]});
            assert_eq!(validate_against(&one, profile, &index), vec![], "with base: {with_base}");
            let bare = json!({"resourceType": "Patient", "name": {"family": "A"}});
            let issues = validate_against(&bare, profile, &index);
            assert_eq!(issues.len(), 1, "with base: {with_base}: {issues:?}");
            assert!(issues[0].message.contains("must be an array"), "{issues:?}");
        }
    }

    #[test]
    fn a_profile_and_its_base_report_a_wrong_shape_once() {
        let required_gender = json!({
            "resourceType": "StructureDefinition",
            "url": "http://acme.org/StructureDefinition/Gendered",
            "name": "Gendered", "kind": "resource", "type": "Patient",
            "derivation": "constraint",
            "baseDefinition": "http://hl7.org/fhir/StructureDefinition/Patient",
            "snapshot": { "element": [
                {"path": "Patient", "min": 0, "max": "*"},
                {"path": "Patient.gender", "min": 1, "max": "1", "type": [{"code": "code"}],
                 "base": {"path": "Patient.gender", "min": 0, "max": "1"}}
            ]}
        });
        let index = index_with(vec![patient_sd(), human_name_sd(), meta_sd(), required_gender]);
        let patient = json!({"resourceType": "Patient", "gender": ["male"],
            "meta": {"profile": ["http://acme.org/StructureDefinition/Gendered"]}});
        let issues = validate_declared(&patient, &index).unwrap();
        let shape: Vec<_> = issues.iter().filter(|i| i.path == "Patient.gender").collect();
        assert_eq!(shape.len(), 1, "{issues:#?}");
        assert!(shape[0].message.contains("must be a single value"), "{issues:#?}");
    }

    #[test]
    fn xml_has_no_arrays_so_its_shape_is_not_judged() {
        // What the XML converter produces before the definitions give it
        // arrays: the validator must not blame the document for it.
        let index = core();
        let converted = json!({"resourceType": "Patient", "id": "p", "name": {"family": "X"}, "gender": ["male"]});
        assert_eq!(errors_in(&converted, &index, &FhirFormat::Xml), vec![]);
        assert_eq!(errors_in(&converted, &index, &FhirFormat::Json).len(), 2);

        let xml = r#"<Patient xmlns="http://hl7.org/fhir"><id value="p"/><name><family value="X"/><given value="Jane"/></name><gender value="male"/></Patient>"#;
        let resource = crate::parser::fhir::parse_fhir_xml(xml).unwrap();
        let json = resource.json_value.as_ref().unwrap();
        assert_eq!(errors_in(json, &index, &resource.format), vec![]);
    }

    #[test]
    fn bundled_examples_validate_clean() {
        // The FHIR templates the app offers and the fixtures the CI and the
        // manual tests use: no profile finding, the shape check included.
        let index = core();
        let mut resources: Vec<(String, Value)> = crate::templates::get_builtin_templates()
            .into_iter()
            .filter(|t| t.message_type == "FHIR")
            .map(|t| (t.id, serde_json::from_str(&t.content).unwrap()))
            .collect();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/fhir");
        for entry in std::fs::read_dir(&dir).expect("tests/fixtures/fhir").flatten() {
            let text = std::fs::read_to_string(entry.path()).unwrap();
            resources.push((entry.path().display().to_string(), serde_json::from_str(&text).unwrap()));
        }
        assert!(resources.len() >= 7, "{}", resources.len());
        for (name, resource) in &resources {
            assert_eq!(errors_in(resource, &index, &FhirFormat::Json), vec![], "{name}");
        }
    }
}
