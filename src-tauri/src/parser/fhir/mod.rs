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
#[derive(Debug, Clone, Serialize)]
pub struct FhirValidationIssue {
    pub severity: String,
    pub message: String,
    pub path: String,
}

/// Detect if content is a FHIR resource. Returns the format if detected.
pub fn detect_fhir(content: &str) -> Option<FhirFormat> {
    let trimmed = content.trim();

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

    // Extract resource type from root element
    let resource_type = extract_xml_root_element(trimmed)
        .ok_or_else(|| "Could not detect FHIR resource type from XML".to_string())?;

    Ok(FhirResource {
        raw: content.to_string(),
        format: FhirFormat::Xml,
        resource_type,
        fhir_version: String::new(),
        json_value: None,
    })
}

/// Extract the root element name from XML content.
fn extract_xml_root_element(xml: &str) -> Option<String> {
    // Skip XML declaration if present
    let content = if xml.starts_with("<?xml") {
        xml.find("?>").map(|i| &xml[i + 2..]).unwrap_or(xml)
    } else {
        xml
    };

    // Find first element
    let trimmed = content.trim();
    if !trimmed.starts_with('<') {
        return None;
    }

    let end = trimmed[1..]
        .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .map(|i| i + 1)?;

    let element_name = &trimmed[1..end];
    if element_name.is_empty() || element_name.starts_with('!') || element_name.starts_with('?') {
        return None;
    }

    Some(element_name.to_string())
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
    }]
}

/// Basic FHIR JSON validation.
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

    // resourceType must be present
    if json.get("resourceType").is_none() {
        issues.push(FhirValidationIssue {
            severity: "error".into(),
            message: "Missing required field: resourceType".into(),
            path: "resourceType".into(),
        });
    }

    // id should be present
    if json.get("id").is_none() {
        issues.push(FhirValidationIssue {
            severity: "info".into(),
            message: "Resource has no 'id' field".into(),
            path: "id".into(),
        });
    }

    // meta is recommended
    if json.get("meta").is_none() {
        issues.push(FhirValidationIssue {
            severity: "info".into(),
            message: "Resource has no 'meta' field (recommended)".into(),
            path: "meta".into(),
        });
    }

    // Resource-type-specific validations
    match resource.resource_type.as_str() {
        "Patient" => validate_patient(json, &mut issues),
        "Observation" => validate_observation(json, &mut issues),
        "Bundle" => validate_bundle(json, &mut issues),
        _ => {}
    }

    issues
}

fn validate_patient(json: &Value, issues: &mut Vec<FhirValidationIssue>) {
    // Patient should have a name
    if json.get("name").is_none() {
        issues.push(FhirValidationIssue {
            severity: "warning".into(),
            message: "Patient resource should have a 'name' field".into(),
            path: "name".into(),
        });
    }

    // Check gender values
    if let Some(gender) = json.get("gender").and_then(|v| v.as_str()) {
        let valid = ["male", "female", "other", "unknown"];
        if !valid.contains(&gender) {
            issues.push(FhirValidationIssue {
                severity: "error".into(),
                message: format!("Invalid gender value '{}'. Must be one of: male, female, other, unknown", gender),
                path: "gender".into(),
            });
        }
    }

    // birthDate format check
    if let Some(bd) = json.get("birthDate").and_then(|v| v.as_str()) {
        if !bd.chars().all(|c| c.is_ascii_digit() || c == '-') || bd.len() < 4 {
            issues.push(FhirValidationIssue {
                severity: "warning".into(),
                message: format!("birthDate '{}' may not be valid (expected YYYY-MM-DD)", bd),
                path: "birthDate".into(),
            });
        }
    }
}

fn validate_observation(json: &Value, issues: &mut Vec<FhirValidationIssue>) {
    // status is required
    if json.get("status").is_none() {
        issues.push(FhirValidationIssue {
            severity: "error".into(),
            message: "Observation must have 'status' field".into(),
            path: "status".into(),
        });
    }

    // code is required
    if json.get("code").is_none() {
        issues.push(FhirValidationIssue {
            severity: "error".into(),
            message: "Observation must have 'code' field".into(),
            path: "code".into(),
        });
    }
}

fn validate_bundle(json: &Value, issues: &mut Vec<FhirValidationIssue>) {
    // type is required
    if json.get("type").is_none() {
        issues.push(FhirValidationIssue {
            severity: "error".into(),
            message: "Bundle must have 'type' field".into(),
            path: "type".into(),
        });
    }

    // Check entries have resource
    if let Some(Value::Array(entries)) = json.get("entry") {
        for (i, entry) in entries.iter().enumerate() {
            if entry.get("resource").is_none() {
                issues.push(FhirValidationIssue {
                    severity: "warning".into(),
                    message: format!("Bundle entry[{}] has no 'resource' field", i),
                    path: format!("entry[{}].resource", i),
                });
            }
        }
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
}
