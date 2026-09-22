pub mod bundle;
pub mod xml;
pub mod fhirpath;
pub mod profile;

use std::collections::HashSet;

use serde::Serialize;
use serde_json::Value;

use crate::parser::hl7::message::{TreeNode, TreeNodeType};

/// Detected FHIR format.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FhirFormat {
    Json,
    Xml,
}

/// Parsed FHIR resource with tree-ready structure.
#[derive(Debug, Clone)]
pub struct FhirResource {
    /// The raw content
    pub raw: String,
    /// Detected format
    pub format: FhirFormat,
    /// Resource type (e.g., "Patient", "Observation", "Bundle")
    pub resource_type: String,
    /// FHIR version if detected (from meta.profile or fhirVersion)
    pub fhir_version: String,
    /// Parsed JSON value (for JSON resources)
    pub json_value: Option<Value>,
}

/// Validation issue for FHIR resources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FhirValidationIssue {
    pub severity: String,
    pub message: String,
    pub path: String,
}

/// Detect if content is a FHIR resource. Returns the format if detected.
/// Strip a leading UTF-8 byte-order mark: `str::trim` does not treat U+FEFF
/// as whitespace, and files saved by Windows tools frequently start with it.
pub fn strip_bom(content: &str) -> &str {
    content.strip_prefix('\u{feff}').unwrap_or(content)
}

pub fn detect_fhir(content: &str) -> Option<FhirFormat> {
    let trimmed = strip_bom(content).trim();

    // JSON detection
    if trimmed.starts_with('{') {
        if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
            if val.get("resourceType").is_some() {
                return Some(FhirFormat::Json);
            }
        }
    }

    // XML detection
    if trimmed.starts_with('<') || trimmed.starts_with("<?xml") {
        // Look for common FHIR root elements
        let fhir_types = [
            "<Patient", "<Observation", "<Bundle", "<Encounter",
            "<Condition", "<Procedure", "<MedicationRequest",
            "<DiagnosticReport", "<AllergyIntolerance", "<Immunization",
            "<Organization", "<Practitioner", "<Location",
            "<Medication", "<CarePlan", "<Goal", "<Device",
            "<DocumentReference", "<Composition", "<ValueSet",
            "<CodeSystem", "<StructureDefinition", "<CapabilityStatement",
            "<OperationOutcome",
        ];
        for ft in &fhir_types {
            if trimmed.contains(ft) {
                return Some(FhirFormat::Xml);
            }
        }
        // Also check xmlns
        if trimmed.contains("xmlns=\"http://hl7.org/fhir\"") {
            return Some(FhirFormat::Xml);
        }
    }

    None
}

/// Parse a FHIR JSON resource.
pub fn parse_fhir_json(content: &str) -> Result<FhirResource, String> {
    let value: Value = serde_json::from_str(content)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    let resource_type = value
        .get("resourceType")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing resourceType field".to_string())?
        .to_string();

    let fhir_version = value
        .get("meta")
        .and_then(|m| m.get("versionId"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(FhirResource {
        raw: content.to_string(),
        format: FhirFormat::Json,
        resource_type,
        fhir_version,
        json_value: Some(value),
    })
}

/// Parse a FHIR XML resource (basic parsing for tree view).
pub fn parse_fhir_xml(content: &str) -> Result<FhirResource, String> {
    let trimmed = content.trim();

    // Full XML -> JSON conversion so validation and tree building work on
    // XML resources exactly like on JSON ones.
    let (resource_type, json) = xml::fhir_xml_to_json(trimmed)?;

    let fhir_version = json
        .get("meta")
        .and_then(|m| m.get("versionId"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(FhirResource {
        raw: content.to_string(),
        format: FhirFormat::Xml,
        resource_type,
        fhir_version,
        json_value: Some(json),
    })
}


/// Build tree nodes from a FHIR JSON resource.
pub fn build_fhir_tree_nodes(resource: &FhirResource) -> Vec<TreeNode> {
    match &resource.json_value {
        Some(value) => build_json_tree(value, &resource.resource_type, 0),
        None => build_xml_tree_simple(&resource.raw, &resource.resource_type),
    }
}

/// Build tree from JSON value, returning top-level property nodes.
fn build_json_tree(value: &Value, _resource_type: &str, _depth: u32) -> Vec<TreeNode> {
    let mut nodes = Vec::new();

    if let Value::Object(map) = value {
        for (i, (key, val)) in map.iter().enumerate() {
            let (preview, has_children, child_count) = describe_json_value(val);
            nodes.push(TreeNode {
                id: format!("fhir.{}", key),
                label: key.clone(),
                value_preview: preview,
                node_type: TreeNodeType::Field,
                depth: 1,
                has_children,
                is_truncated: false,
                child_count,
                code: None,
                code_desc: None,
            });
            // Safety: cap at 200 top-level nodes
            if i >= 200 {
                break;
            }
        }
    }

    nodes
}

/// Describe a JSON value for tree preview.
fn describe_json_value(val: &Value) -> (String, bool, usize) {
    match val {
        Value::Null => ("null".into(), false, 0),
        Value::Bool(b) => (b.to_string(), false, 0),
        Value::Number(n) => (n.to_string(), false, 0),
        Value::String(s) => {
            let preview: String = s.chars().take(80).collect();
            let display = if s.len() > 80 {
                format!("\"{}...\"", preview)
            } else {
                format!("\"{}\"", preview)
            };
            (display, false, 0)
        }
        Value::Array(arr) => {
            (format!("[{} items]", arr.len()), !arr.is_empty(), arr.len())
        }
        Value::Object(map) => {
            let type_hint = map
                .get("resourceType")
                .or_else(|| map.get("system"))
                .or_else(|| map.get("code"))
                .and_then(|v| v.as_str());
            let preview = match type_hint {
                Some(hint) => format!("{{{}...}}", hint),
                None => format!("{{{} properties}}", map.len()),
            };
            (preview, !map.is_empty(), map.len())
        }
    }
}

/// Get children of a FHIR tree node by path.
pub fn get_fhir_children(resource: &FhirResource, node_id: &str) -> Vec<TreeNode> {
    let json = match &resource.json_value {
        Some(v) => v,
        None => return Vec::new(),
    };

    // Navigate to the value at the given path
    let path = node_id.strip_prefix("fhir.").unwrap_or(node_id);
    let parts: Vec<&str> = path.split('.').collect();

    let mut current = json;
    for part in &parts {
        if let Ok(idx) = part.parse::<usize>() {
            current = match current.get(idx) {
                Some(v) => v,
                None => return Vec::new(),
            };
        } else {
            current = match current.get(*part) {
                Some(v) => v,
                None => return Vec::new(),
            };
        }
    }

    // Build children based on value type
    match current {
        Value::Object(map) => {
            map.iter()
                .enumerate()
                .map(|(_, (key, val))| {
                    let (preview, has_children, child_count) = describe_json_value(val);
                    TreeNode {
                        id: format!("{}.{}", node_id, key),
                        label: key.clone(),
                        value_preview: preview,
                        node_type: TreeNodeType::Component,
                        depth: (parts.len() as u32) + 2,
                        has_children,
                        is_truncated: false,
                        child_count,
                        code: None,
                        code_desc: None,
                    }
                })
                .collect()
        }
        Value::Array(arr) => {
            arr.iter()
                .enumerate()
                .take(500) // Cap to prevent huge arrays
                .map(|(i, val)| {
                    let (preview, has_children, child_count) = describe_json_value(val);
                    TreeNode {
                        id: format!("{}.{}", node_id, i),
                        label: format!("[{}]", i),
                        value_preview: preview,
                        node_type: TreeNodeType::Component,
                        depth: (parts.len() as u32) + 2,
                        has_children,
                        is_truncated: false,
                        child_count,
                        code: None,
                        code_desc: None,
                    }
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Basic XML tree for display (without full XML parsing dependency).
fn build_xml_tree_simple(xml: &str, resource_type: &str) -> Vec<TreeNode> {
    vec![TreeNode {
        id: "fhir.root".into(),
        label: resource_type.to_string(),
        value_preview: format!("XML {} resource ({} bytes)", resource_type, xml.len()),
        node_type: TreeNodeType::Segment,
        depth: 1,
        has_children: false,
        is_truncated: false,
        child_count: 0,
        code: None,
        code_desc: None,
    }]
}

/// Basic FHIR JSON validation.
///
/// Runs on the root resource and on every resource nested inside it —
/// Bundle entries and `contained` resources. A Bundle whose entries were
/// never looked at reports itself clean while carrying a broken
/// Observation, and "clean" is exactly the conclusion the person who opened
/// it draws.
pub fn validate_fhir_json(resource: &FhirResource) -> Vec<FhirValidationIssue> {
    let mut issues = Vec::new();

    let json = match &resource.json_value {
        Some(v) => v,
        None => {
            issues.push(FhirValidationIssue {
                severity: "error".into(),
                message: "No JSON content available for validation".into(),
                path: "".into(),
            });
            return issues;
        }
    };

    validate_resource_at(json, "", Nesting::Root, &mut issues);
    issues
}

/// Where a resource sits, which decides what an absent `id` means.
#[derive(Clone, Copy)]
enum Nesting {
    Root,
    /// A Bundle entry: whether it needs an id depends on its fullUrl — a
    /// `urn:uuid:` or `urn:oid:` *is* its identity inside the bundle — so
    /// that is judged where the fullUrl is, in [`validate_bundle`].
    Entry,
    Contained,
}

fn push(issues: &mut Vec<FhirValidationIssue>, severity: &str, message: String, path: String) {
    issues.push(FhirValidationIssue {
        severity: severity.into(),
        message,
        path,
    });
}

fn at(prefix: &str, path: &str) -> String {
    format!("{}{}", prefix, path)
}

fn validate_resource_at(
    json: &Value,
    prefix: &str,
    nesting: Nesting,
    issues: &mut Vec<FhirValidationIssue>,
) {
    let resource_type = json.get("resourceType").and_then(|v| v.as_str());
    if resource_type.is_none() {
        push(issues, "error", "Missing required field: resourceType".into(), at(prefix, "resourceType"));
    }

    match nesting {
        Nesting::Root => {
            if json.get("id").is_none() {
                push(issues, "info", "Resource has no 'id' field".into(), at(prefix, "id"));
            }
            if json.get("meta").is_none() {
                push(issues, "info", "Resource has no 'meta' field (recommended)".into(), at(prefix, "meta"));
            }
        }
        Nesting::Entry => {}
        Nesting::Contained => {
            if json.get("id").is_none() {
                push(
                    issues,
                    "warning",
                    "Contained resource has no 'id' — nothing can reference it as #id".into(),
                    at(prefix, "id"),
                );
            }
        }
    }

    // meta.profile declarations: canonical URLs are surfaced and malformed
    // entries flagged. Whether conformance against the profile is actually
    // checked depends on the installed packages, so the caller reports that
    // rather than this function claiming either way.
    if let Some(profiles) = json.get("meta").and_then(|m| m.get("profile")) {
        match profiles.as_array() {
            Some(list) => {
                for (i, p) in list.iter().enumerate() {
                    let path = at(prefix, &format!("meta.profile[{}]", i));
                    match p.as_str() {
                        Some(url) if is_absolute_uri(url) => {
                            push(issues, "info", format!("Declares profile {}", url), path);
                        }
                        Some(url) => push(
                            issues,
                            "warning",
                            format!("meta.profile entry '{}' is not an absolute canonical URI", url),
                            path,
                        ),
                        None => push(issues, "warning", "meta.profile entries must be strings".into(), path),
                    }
                }
            }
            None => push(
                issues,
                "warning",
                "meta.profile must be an array of canonical URLs".into(),
                at(prefix, "meta.profile"),
            ),
        }
    }

    // Resource-type-specific validations. Structure and cardinality come
    // from the built-in R4 core definitions; what stays here is what a
    // StructureDefinition cannot say (terminology) and the Bundle rules.
    match resource_type {
        Some("Patient") => validate_patient(json, prefix, issues),
        Some("Bundle") => validate_bundle(json, prefix, issues),
        _ => {}
    }

    if let Some(Value::Array(contained)) = json.get("contained") {
        for (j, inner) in contained.iter().enumerate() {
            let inner_prefix = at(prefix, &format!("contained[{}].", j));
            validate_resource_at(inner, &inner_prefix, Nesting::Contained, issues);
        }
    }
}

/// FHIR `canonical` is URI-based, not HTTP-only: `urn:oid:...`, `urn:uuid:...`
/// and other absolute URIs are all valid profile declarations. Accept any
/// `scheme:rest` with an RFC 3986 scheme and a non-empty, whitespace-free
/// remainder; reject relative references.
fn is_absolute_uri(s: &str) -> bool {
    let Some((scheme, rest)) = s.split_once(':') else {
        return false;
    };
    let scheme_ok = !scheme.is_empty()
        && scheme.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false)
        && scheme.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.');
    scheme_ok && !rest.is_empty() && !s.chars().any(|c| c.is_whitespace())
}

fn validate_patient(json: &Value, prefix: &str, issues: &mut Vec<FhirValidationIssue>) {
    // Patient should have a name
    if json.get("name").is_none() {
        push(issues, "warning", "Patient resource should have a 'name' field".into(), at(prefix, "name"));
    }

    // Check gender values
    if let Some(gender) = json.get("gender").and_then(|v| v.as_str()) {
        let valid = ["male", "female", "other", "unknown"];
        if !valid.contains(&gender) {
            push(
                issues,
                "error",
                format!("Invalid gender value '{}'. Must be one of: male, female, other, unknown", gender),
                at(prefix, "gender"),
            );
        }
    }

    // Cardinality and the lexical form of birthDate are the built-in R4
    // core's job; what it deliberately leaves alone is terminology, which
    // is why the gender check above stays here.
}

/// A transient identity: the urn is the resource's name inside the bundle
/// and the resource need not carry an id of its own.
fn is_transient_full_url(url: &str) -> bool {
    url.starts_with("urn:uuid:") || url.starts_with("urn:oid:")
}

/// Bundle rules from the specification's "Resource URL & Uniqueness rules":
/// every entry carries a fullUrl (POST requests excepted), a persistent
/// fullUrl agrees with the resource's own type and id, fullUrls are unique
/// outside history bundles — and then every entry is validated as the
/// resource it is, and every Reference inside the bundle is checked to
/// resolve against it.
fn validate_bundle(json: &Value, prefix: &str, issues: &mut Vec<FhirValidationIssue>) {
    let bundle_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if bundle_type.is_empty() {
        push(issues, "error", "Bundle must have 'type' field".into(), at(prefix, "type"));
    }

    let Some(Value::Array(entries)) = json.get("entry") else {
        return;
    };

    let mut seen_full_urls: HashSet<String> = HashSet::new();
    for (i, entry) in entries.iter().enumerate() {
        let entry_prefix = at(prefix, &format!("entry[{}].", i));
        let full_url = entry.get("fullUrl").and_then(|v| v.as_str());
        let is_post = entry
            .get("request")
            .and_then(|r| r.get("method"))
            .and_then(|m| m.as_str())
            .is_some_and(|m| m.eq_ignore_ascii_case("POST"));

        let Some(resource) = entry.get("resource") else {
            push(
                issues,
                "warning",
                format!("Bundle entry[{}] has no 'resource' field", i),
                at(&entry_prefix, "resource"),
            );
            continue;
        };
        let resource_type = resource.get("resourceType").and_then(|v| v.as_str());
        let id = resource.get("id").and_then(|v| v.as_str());

        match full_url {
            None => {
                if !is_post {
                    push(
                        issues,
                        "warning",
                        format!("Bundle entry[{}] has no fullUrl — required except on a POST request", i),
                        at(&entry_prefix, "fullUrl"),
                    );
                }
            }
            Some(url) => {
                if !is_absolute_uri(url) {
                    push(
                        issues,
                        "warning",
                        format!("fullUrl '{}' must be an absolute URL or a urn", url),
                        at(&entry_prefix, "fullUrl"),
                    );
                }
                if !seen_full_urls.insert(url.to_string()) && bundle_type != "history" {
                    push(
                        issues,
                        "warning",
                        format!("fullUrl '{}' appears more than once in this Bundle (bdl-7)", url),
                        at(&entry_prefix, "fullUrl"),
                    );
                }
                if !is_transient_full_url(url) {
                    match (resource_type, id) {
                        (Some(rt), Some(id)) if !url.ends_with(&format!("/{}/{}", rt, id)) => push(
                            issues,
                            "warning",
                            format!("fullUrl '{}' disagrees with the resource's own identity {}/{}", url, rt, id),
                            at(&entry_prefix, "fullUrl"),
                        ),
                        (_, None) => push(
                            issues,
                            "warning",
                            format!(
                                "Resource has no 'id', but its fullUrl '{}' is persistent (not a urn) — \
                                 the two are required to agree",
                                url
                            ),
                            at(&entry_prefix, "resource.id"),
                        ),
                        _ => {}
                    }
                }
            }
        }

        validate_resource_at(resource, &at(&entry_prefix, "resource."), Nesting::Entry, issues);
    }

    check_bundle_references(json, entries, bundle_type, prefix, issues);
}

/// A reference inside a bundle must reach something. Local references —
/// `urn:uuid:…`, `Type/id`, `#contained` — are checked against the bundle
/// itself; an absolute URL may point outside, except in a document, which
/// the specification requires to be self-contained.
fn check_bundle_references(
    bundle: &Value,
    entries: &[Value],
    bundle_type: &str,
    prefix: &str,
    issues: &mut Vec<FhirValidationIssue>,
) {
    let mut full_urls: HashSet<&str> = HashSet::new();
    let mut identities: HashSet<String> = HashSet::new();
    for entry in entries {
        if let Some(url) = entry.get("fullUrl").and_then(|v| v.as_str()) {
            full_urls.insert(url);
        }
        if let Some(resource) = entry.get("resource") {
            if let (Some(rt), Some(id)) = (
                resource.get("resourceType").and_then(|v| v.as_str()),
                resource.get("id").and_then(|v| v.as_str()),
            ) {
                identities.insert(format!("{}/{}", rt, id));
            }
        }
    }
    // A relative reference ("Patient/1") names an entry by its identity, and
    // an entry's fullUrl ends in that identity. An absolute one names an
    // entry only by matching its fullUrl exactly: "https://b.example/Patient/1"
    // is not the entry "https://a.example/fhir/Patient/1", even though the
    // two end the same way.
    let resolves_relative = |target: &str| -> bool {
        identities.contains(target)
            || full_urls.iter().any(|u| u.ends_with(&format!("/{}", target)))
    };
    let resolves_absolute = |target: &str| -> bool { full_urls.contains(target) };
    // Outside the types that carry a related set of resources, an
    // unresolved local reference is a note rather than a defect: a searchset
    // legitimately returns only part of a graph.
    let severity = match bundle_type {
        "message" | "document" | "transaction" | "batch" | "collection" => "warning",
        _ => "info",
    };
    let _ = bundle;

    for (i, entry) in entries.iter().enumerate() {
        let Some(resource) = entry.get("resource") else { continue };
        let resource_prefix = at(prefix, &format!("entry[{}].resource.", i));
        let mut refs = Vec::new();
        collect_references(resource, "", &mut refs);
        for (path, target) in refs {
            let ok = if let Some(local_id) = target.strip_prefix('#') {
                // "#x" names a resource contained in the entry's resource —
                // also from inside another contained resource, which cannot
                // contain anything itself.
                resource
                    .get("contained")
                    .and_then(|c| c.as_array())
                    .is_some_and(|c| c.iter().any(|r| r.get("id").and_then(|v| v.as_str()) == Some(local_id)))
            } else if target.starts_with("urn:") {
                resolves_absolute(&target)
            } else if !target.contains("://") {
                resolves_relative(&target)
            } else {
                // Absolute: external unless the document must be self-contained.
                bundle_type != "document" || resolves_absolute(&target)
            };
            if !ok {
                push(
                    issues,
                    severity,
                    format!("Reference '{}' does not resolve to anything in this Bundle", target),
                    at(&resource_prefix, &path),
                );
            }
        }
    }
}

/// Every `reference` string below `value`, with the JSON path it sits at —
/// including those inside `contained` resources, whose references resolve
/// against the same bundle and the same containing resource.
fn collect_references(value: &Value, path: &str, out: &mut Vec<(String, String)>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let child = if path.is_empty() { k.clone() } else { format!("{}.{}", path, k) };
                if k == "reference" {
                    if let Some(s) = v.as_str() {
                        out.push((child, s.trim().to_string()));
                        continue;
                    }
                }
                collect_references(v, &child, out);
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                collect_references(v, &format!("{}[{}]", path, i), out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_fhir_json() {
        let json = r#"{"resourceType": "Patient", "id": "123"}"#;
        assert_eq!(detect_fhir(json), Some(FhirFormat::Json));
    }

    #[test]
    fn test_detect_fhir_xml() {
        let xml = r#"<Patient xmlns="http://hl7.org/fhir"><id value="123"/></Patient>"#;
        assert_eq!(detect_fhir(xml), Some(FhirFormat::Xml));
    }

    #[test]
    fn test_detect_not_fhir() {
        assert_eq!(detect_fhir("MSH|^~\\&|"), None);
        assert_eq!(detect_fhir("Hello world"), None);
    }

    #[test]
    fn test_detect_fhir_with_utf8_bom() {
        // Windows editors prepend U+FEFF; str::trim does not strip it.
        let json = "\u{feff}{\"resourceType\": \"Patient\", \"id\": \"123\"}";
        assert_eq!(detect_fhir(json), Some(FhirFormat::Json));
        let xml = "\u{feff}<Patient xmlns=\"http://hl7.org/fhir\"><id value=\"123\"/></Patient>";
        assert_eq!(detect_fhir(xml), Some(FhirFormat::Xml));
        assert_eq!(strip_bom("\u{feff}abc"), "abc");
        assert_eq!(strip_bom("abc"), "abc");
        // A BOM-prefixed document must also parse, not just be detected.
        assert!(parse_fhir_json(strip_bom(json)).is_ok());
    }

    #[test]
    fn test_parse_fhir_json() {
        let json = r#"{"resourceType": "Patient", "id": "123", "name": [{"family": "Doe"}]}"#;
        let resource = parse_fhir_json(json).unwrap();
        assert_eq!(resource.resource_type, "Patient");
        assert_eq!(resource.format, FhirFormat::Json);
    }

    #[test]
    fn test_fhir_tree_nodes() {
        let json = r#"{"resourceType": "Patient", "id": "123", "active": true}"#;
        let resource = parse_fhir_json(json).unwrap();
        let nodes = build_fhir_tree_nodes(&resource);
        assert_eq!(nodes.len(), 3); // resourceType, id, active
    }

    #[test]
    fn test_validate_patient() {
        let json = r#"{"resourceType": "Patient", "gender": "invalid_value"}"#;
        let resource = parse_fhir_json(json).unwrap();
        let issues = validate_fhir_json(&resource);
        assert!(issues.iter().any(|i| i.path == "gender" && i.severity == "error"));
    }

    #[test]
    fn test_fhir_children() {
        let json = r#"{"resourceType": "Patient", "name": [{"family": "Doe", "given": ["John"]}]}"#;
        let resource = parse_fhir_json(json).unwrap();
        let children = get_fhir_children(&resource, "fhir.name");
        assert_eq!(children.len(), 1); // One array element
        let grandchildren = get_fhir_children(&resource, "fhir.name.0");
        assert_eq!(grandchildren.len(), 2); // family, given
    }

    #[test]
    fn test_meta_profile_surfaced_and_linted() {
        let json = r#"{"resourceType": "Patient", "id": "x", "name": [{"family": "D"}],
            "meta": {"profile": ["http://hl7.org/fhir/StructureDefinition/Patient",
                                 "urn:oid:2.16.840.1.113883.2.9.10.1.1",
                                 "not-a-url"]}}"#;
        let resource = parse_fhir_json(json).unwrap();
        let issues = validate_fhir_json(&resource);
        assert!(issues.iter().any(|i| i.path == "meta.profile[0]"
            && i.severity == "info"
            && i.message.contains("Declares profile")));
        // canonical is URI-based, not HTTP-only: urn: schemes are valid
        assert!(issues.iter().any(|i| i.path == "meta.profile[1]"
            && i.severity == "info"
            && i.message.contains("Declares profile")));
        assert!(issues.iter().any(|i| i.path == "meta.profile[2]"
            && i.severity == "warning"
            && i.message.contains("not an absolute canonical URI")));
    }

    #[test]
    fn test_meta_profile_must_be_array() {
        let json = r#"{"resourceType": "Patient", "id": "x", "name": [{"family": "D"}],
            "meta": {"profile": "http://example.org/p"}}"#;
        let resource = parse_fhir_json(json).unwrap();
        let issues = validate_fhir_json(&resource);
        assert!(issues.iter().any(|i| i.path == "meta.profile" && i.severity == "warning"));
    }

    fn issues_for(json: &str) -> Vec<FhirValidationIssue> {
        validate_fhir_json(&parse_fhir_json(json).unwrap())
    }

    fn has(issues: &[FhirValidationIssue], path: &str, severity: &str, text: &str) -> bool {
        issues
            .iter()
            .any(|i| i.path == path && i.severity == severity && i.message.contains(text))
    }

    #[test]
    fn bundle_entries_are_validated_as_the_resources_they_are() {
        // A message bundle the way NHS and IHE write them: urn:uuid
        // fullUrls, no ids anywhere. The first Patient has a bad gender,
        // the second declares a profile.
        let issues = issues_for(
            r#"{"resourceType": "Bundle", "type": "message", "entry": [
                {"fullUrl": "urn:uuid:aaaaaaaa-0000-4000-8000-000000000001",
                 "resource": {"resourceType": "Patient", "gender": "malle"}},
                {"fullUrl": "urn:uuid:aaaaaaaa-0000-4000-8000-000000000002",
                 "resource": {"resourceType": "Observation", "status": "final", "code": {"text": "x"},
                              "meta": {"profile": ["http://example.org/StructureDefinition/x"]},
                              "subject": {"reference": "urn:uuid:aaaaaaaa-0000-4000-8000-000000000001"}}}
            ]}"#,
        );
        assert!(has(&issues, "entry[0].resource.gender", "error", "Invalid gender"), "{issues:?}");
        assert!(has(&issues, "entry[1].resource.meta.profile[0]", "info", "Declares profile"), "{issues:?}");
        // A urn:uuid entry needs no id of its own: nothing about ids.
        assert!(!issues.iter().any(|i| i.path.ends_with(".id")), "{issues:?}");
        // The reference resolves against the other entry's fullUrl.
        assert!(!issues.iter().any(|i| i.message.contains("does not resolve")), "{issues:?}");
    }

    #[test]
    fn a_persistent_full_url_must_agree_with_the_resource_identity() {
        let issues = issues_for(
            r#"{"resourceType": "Bundle", "type": "collection", "entry": [
                {"fullUrl": "http://srv/fhir/Patient/1", "resource": {"resourceType": "Patient", "id": "1", "name": [{}]}},
                {"fullUrl": "http://srv/fhir/Patient/2", "resource": {"resourceType": "Patient", "id": "9", "name": [{}]}},
                {"fullUrl": "http://srv/fhir/Patient/3", "resource": {"resourceType": "Patient", "name": [{}]}},
                {"fullUrl": "Patient/4", "resource": {"resourceType": "Patient", "id": "4", "name": [{}]}}
            ]}"#,
        );
        assert!(!issues.iter().any(|i| i.path.starts_with("entry[0].")), "{issues:?}");
        assert!(has(&issues, "entry[1].fullUrl", "warning", "disagrees"), "{issues:?}");
        assert!(has(&issues, "entry[2].resource.id", "warning", "persistent"), "{issues:?}");
        assert!(has(&issues, "entry[3].fullUrl", "warning", "absolute"), "{issues:?}");
    }

    #[test]
    fn a_missing_full_url_is_reported_except_on_a_post() {
        let issues = issues_for(
            r#"{"resourceType": "Bundle", "type": "transaction", "entry": [
                {"resource": {"resourceType": "Patient", "name": [{}]}, "request": {"method": "POST", "url": "Patient"}},
                {"resource": {"resourceType": "Patient", "name": [{}]}}
            ]}"#,
        );
        assert!(!issues.iter().any(|i| i.path == "entry[0].fullUrl"), "{issues:?}");
        assert!(has(&issues, "entry[1].fullUrl", "warning", "no fullUrl"), "{issues:?}");
    }

    #[test]
    fn duplicate_full_urls_are_reported_outside_history_bundles() {
        let two = r#"[{"fullUrl": "urn:uuid:aaaaaaaa-0000-4000-8000-000000000001", "resource": {"resourceType": "Patient", "name": [{}]}},
                       {"fullUrl": "urn:uuid:aaaaaaaa-0000-4000-8000-000000000001", "resource": {"resourceType": "Patient", "name": [{}]}}]"#;
        let dup = issues_for(&format!(r#"{{"resourceType": "Bundle", "type": "collection", "entry": {two}}}"#));
        assert!(has(&dup, "entry[1].fullUrl", "warning", "more than once"), "{dup:?}");
        let history = issues_for(&format!(r#"{{"resourceType": "Bundle", "type": "history", "entry": {two}}}"#));
        assert!(!history.iter().any(|i| i.message.contains("more than once")), "{history:?}");
    }

    #[test]
    fn dangling_references_inside_a_bundle_are_reported() {
        let issues = issues_for(
            r##"{"resourceType": "Bundle", "type": "message", "entry": [
                {"fullUrl": "urn:uuid:aaaaaaaa-0000-4000-8000-000000000001",
                 "resource": {"resourceType": "DiagnosticReport", "id": "r", "status": "final",
                              "result": [{"reference": "urn:uuid:aaaaaaaa-0000-4000-8000-00000000dead"},
                                         {"reference": "Observation/o1"}],
                              "subject": {"reference": "http://elsewhere.example/fhir/Patient/p9"},
                              "performer": [{"reference": "#org"}],
                              "contained": [{"resourceType": "Organization", "id": "org"}]}},
                {"fullUrl": "http://srv/fhir/Observation/o1",
                 "resource": {"resourceType": "Observation", "id": "o1", "status": "final", "code": {"text": "x"}}}
            ]}"##,
        );
        assert!(has(&issues, "entry[0].resource.result[0].reference", "warning", "does not resolve"), "{issues:?}");
        // Observation/o1 is there, under a persistent fullUrl.
        assert!(!issues.iter().any(|i| i.path == "entry[0].resource.result[1].reference"), "{issues:?}");
        // An absolute reference may point outside a message bundle.
        assert!(!issues.iter().any(|i| i.path == "entry[0].resource.subject.reference"), "{issues:?}");
        // #org is contained right there.
        assert!(!issues.iter().any(|i| i.path == "entry[0].resource.performer[0].reference"), "{issues:?}");

        // In a document nothing may point outside.
        let doc = issues_for(
            r#"{"resourceType": "Bundle", "type": "document", "entry": [
                {"fullUrl": "urn:uuid:aaaaaaaa-0000-4000-8000-000000000001",
                 "resource": {"resourceType": "Composition", "id": "c", "subject": {"reference": "http://elsewhere.example/fhir/Patient/p9"}}}
            ]}"#,
        );
        assert!(has(&doc, "entry[0].resource.subject.reference", "warning", "does not resolve"), "{doc:?}");

        // A searchset legitimately returns part of a graph: a note, not a defect.
        let search = issues_for(
            r#"{"resourceType": "Bundle", "type": "searchset", "entry": [
                {"fullUrl": "http://srv/fhir/Observation/o1",
                 "resource": {"resourceType": "Observation", "id": "o1", "status": "final", "code": {"text": "x"},
                              "subject": {"reference": "Patient/p1"}}}
            ]}"#,
        );
        assert!(has(&search, "entry[0].resource.subject.reference", "info", "does not resolve"), "{search:?}");
    }

    #[test]
    fn an_absolute_reference_matches_an_entry_only_by_its_exact_full_url() {
        // Same Type/id, different server: not the same resource. In a
        // document that is a reference outside the bundle.
        let doc = |target: &str| issues_for(&format!(
            r#"{{"resourceType": "Bundle", "type": "document", "entry": [
                {{"fullUrl": "https://a.example/fhir/Patient/1", "resource": {{"resourceType": "Patient", "id": "1", "name": [{{}}]}}}},
                {{"fullUrl": "https://a.example/fhir/Composition/c",
                 "resource": {{"resourceType": "Composition", "id": "c", "subject": {{"reference": "{target}"}}}}}}
            ]}}"#
        ));
        let dangling = |issues: &[FhirValidationIssue]| issues.iter().any(|i| i.path == "entry[1].resource.subject.reference");
        assert!(dangling(&doc("https://b.example/Patient/1")), "different server must not resolve");
        assert!(!dangling(&doc("https://a.example/fhir/Patient/1")), "the exact fullUrl resolves");
        assert!(!dangling(&doc("Patient/1")), "a relative reference resolves by identity");
        assert!(dangling(&doc("Patient/2")), "a relative reference to nothing does not");
    }

    #[test]
    fn references_inside_contained_resources_are_checked_too() {
        let issues = issues_for(
            r##"{"resourceType": "Bundle", "type": "message", "entry": [
                {"fullUrl": "urn:uuid:aaaaaaaa-0000-4000-8000-000000000001",
                 "resource": {"resourceType": "DiagnosticReport", "id": "r", "status": "final",
                              "contained": [
                                  {"resourceType": "Organization", "id": "org"},
                                  {"resourceType": "Observation", "id": "obs", "status": "final", "code": {"text": "x"},
                                   "performer": [{"reference": "#org"}],
                                   "subject": {"reference": "#missing"},
                                   "specimen": {"reference": "urn:uuid:aaaaaaaa-0000-4000-8000-00000000dead"}}
                              ]}}
            ]}"##,
        );
        let at = |p: &str| issues.iter().any(|i| i.path == p && i.message.contains("does not resolve"));
        assert!(!at("entry[0].resource.contained[1].performer[0].reference"), "{issues:?}");
        assert!(at("entry[0].resource.contained[1].subject.reference"), "{issues:?}");
        assert!(at("entry[0].resource.contained[1].specimen.reference"), "{issues:?}");
    }

    #[test]
    fn contained_resources_are_validated_too() {
        let issues = issues_for(
            r#"{"resourceType": "Observation", "id": "o", "status": "final", "code": {"text": "x"},
                "contained": [{"resourceType": "Patient", "id": "p", "name": [{}], "gender": "yes"},
                              {"resourceType": "Organization"}]}"#,
        );
        assert!(has(&issues, "contained[0].gender", "error", "Invalid gender"), "{issues:?}");
        assert!(has(&issues, "contained[1].id", "warning", "reference it as #id"), "{issues:?}");
    }
}
