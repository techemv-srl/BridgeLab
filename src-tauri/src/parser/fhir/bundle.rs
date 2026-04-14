use serde::Serialize;
use serde_json::Value;

/// A FHIR Bundle entry summary.
#[derive(Debug, Clone, Serialize)]
pub struct BundleEntry {
    pub index: usize,
    pub full_url: Option<String>,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub display_name: String,
    pub summary: String,
    pub request_method: Option<String>,
    pub request_url: Option<String>,
    pub response_status: Option<String>,
    /// References found inside this resource (e.g., "Patient/123", "urn:uuid:abc")
    pub references: Vec<String>,
}

/// A reference edge between two bundle entries.
#[derive(Debug, Clone, Serialize)]
pub struct ReferenceEdge {
    pub from_index: usize,
    pub to_index: Option<usize>, // None = dangling reference
    pub reference: String,
    pub field_path: String,
}

/// Bundle analysis result.
#[derive(Debug, Clone, Serialize)]
pub struct BundleAnalysis {
    pub bundle_type: String,
    pub total: Option<u64>,
    pub entry_count: usize,
    pub entries: Vec<BundleEntry>,
    pub references: Vec<ReferenceEdge>,
    pub resource_type_counts: Vec<(String, usize)>,
    pub dangling_references: usize,
}

/// Analyze a FHIR Bundle JSON and return a structured summary.
pub fn analyze_bundle(bundle_json: &Value) -> Result<BundleAnalysis, String> {
    // Verify it's a Bundle
    let resource_type = bundle_json.get("resourceType")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing resourceType".to_string())?;

    if resource_type != "Bundle" {
        return Err(format!("Expected Bundle, got {}", resource_type));
    }

    let bundle_type = bundle_json.get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let total = bundle_json.get("total").and_then(|v| v.as_u64());

    let entries_arr = bundle_json.get("entry")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut entries: Vec<BundleEntry> = Vec::new();
    let mut full_url_to_index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for (idx, entry) in entries_arr.iter().enumerate() {
        let full_url = entry.get("fullUrl").and_then(|v| v.as_str()).map(String::from);
        let resource = entry.get("resource");

        let (resource_type, resource_id, display_name, summary) = match resource {
            Some(r) => {
                let rt = r.get("resourceType").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
                let rid = r.get("id").and_then(|v| v.as_str()).map(String::from);
                let display = build_display_name(&rt, r);
                let sum = build_summary(&rt, r);
                (rt, rid, display, sum)
            }
            None => ("(empty)".to_string(), None, "(no resource)".to_string(), String::new()),
        };

        // Index by fullUrl for reference resolution
        if let Some(ref url) = full_url {
            full_url_to_index.insert(url.clone(), idx);
        }
        // Also index by ResourceType/id shorthand
        if let (rt_non_empty, Some(ref rid)) = (!resource_type.is_empty(), &resource_id) {
            if rt_non_empty {
                full_url_to_index.insert(format!("{}/{}", resource_type, rid), idx);
            }
        }

        // Extract request info
        let request = entry.get("request");
        let request_method = request.and_then(|r| r.get("method")).and_then(|v| v.as_str()).map(String::from);
        let request_url = request.and_then(|r| r.get("url")).and_then(|v| v.as_str()).map(String::from);

        let response = entry.get("response");
        let response_status = response.and_then(|r| r.get("status")).and_then(|v| v.as_str()).map(String::from);

        // Extract references
        let mut refs: Vec<String> = Vec::new();
        if let Some(r) = resource {
            extract_references(r, "", &mut refs);
        }

        entries.push(BundleEntry {
            index: idx,
            full_url,
            resource_type,
            resource_id,
            display_name,
            summary,
            request_method,
            request_url,
            response_status,
            references: refs,
        });
    }

    // Build reference edges
    let mut edges: Vec<ReferenceEdge> = Vec::new();
    let mut dangling = 0usize;

    for entry in &entries {
        for r in &entry.references {
            let target = full_url_to_index.get(r).copied();
            if target.is_none() {
                dangling += 1;
            }
            edges.push(ReferenceEdge {
                from_index: entry.index,
                to_index: target,
                reference: r.clone(),
                field_path: String::new(), // Could be enhanced with path tracking
            });
        }
    }

    // Count resource types
    let mut type_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for e in &entries {
        *type_map.entry(e.resource_type.clone()).or_insert(0) += 1;
    }
    let mut resource_type_counts: Vec<(String, usize)> = type_map.into_iter().collect();
    resource_type_counts.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(BundleAnalysis {
        bundle_type,
        total,
        entry_count: entries.len(),
        entries,
        references: edges,
        resource_type_counts,
        dangling_references: dangling,
    })
}

/// Build a human-readable name for a FHIR resource.
fn build_display_name(resource_type: &str, r: &Value) -> String {
    match resource_type {
        "Patient" => {
            if let Some(names) = r.get("name").and_then(|v| v.as_array()) {
                if let Some(n) = names.first() {
                    let family = n.get("family").and_then(|v| v.as_str()).unwrap_or("");
                    let given = n.get("given").and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !family.is_empty() || !given.is_empty() {
                        return format!("{} {}", given, family).trim().to_string();
                    }
                }
            }
            r.get("id").and_then(|v| v.as_str()).unwrap_or("Patient").to_string()
        }
        "Observation" => {
            let code_text = r.get("code")
                .and_then(|c| c.get("text"))
                .and_then(|v| v.as_str())
                .or_else(|| r.get("code")
                    .and_then(|c| c.get("coding"))
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|c| c.get("display"))
                    .and_then(|v| v.as_str()))
                .unwrap_or("Observation");
            code_text.to_string()
        }
        "Encounter" => {
            let class = r.get("class")
                .and_then(|c| c.get("code"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let status = r.get("status").and_then(|v| v.as_str()).unwrap_or("");
            format!("{} {}", class, status).trim().to_string()
        }
        "Condition" | "DiagnosticReport" | "Procedure" => {
            r.get("code")
                .and_then(|c| c.get("text"))
                .and_then(|v| v.as_str())
                .unwrap_or(resource_type)
                .to_string()
        }
        _ => {
            r.get("id").and_then(|v| v.as_str())
                .map(|id| format!("{}/{}", resource_type, id))
                .unwrap_or_else(|| resource_type.to_string())
        }
    }
}

/// Build a short summary for a FHIR resource (1 line).
fn build_summary(resource_type: &str, r: &Value) -> String {
    match resource_type {
        "Patient" => {
            let gender = r.get("gender").and_then(|v| v.as_str()).unwrap_or("");
            let bd = r.get("birthDate").and_then(|v| v.as_str()).unwrap_or("");
            format!("{} {}", gender, bd).trim().to_string()
        }
        "Observation" => {
            let status = r.get("status").and_then(|v| v.as_str()).unwrap_or("");
            let value = r.get("valueQuantity")
                .and_then(|v| {
                    let val = v.get("value")?.as_f64()?;
                    let unit = v.get("unit").and_then(|u| u.as_str()).unwrap_or("");
                    Some(format!("{} {}", val, unit))
                })
                .or_else(|| r.get("valueString").and_then(|v| v.as_str()).map(String::from))
                .unwrap_or_default();
            format!("{} {}", status, value).trim().to_string()
        }
        _ => {
            let status = r.get("status").and_then(|v| v.as_str()).unwrap_or("");
            status.to_string()
        }
    }
}

/// Recursively extract all "reference" strings from a JSON value.
fn extract_references(value: &Value, _path: &str, refs: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                // FHIR Reference: { "reference": "Patient/123" }
                if key == "reference" {
                    if let Some(s) = val.as_str() {
                        refs.push(s.to_string());
                    }
                } else {
                    extract_references(val, key, refs);
                }
            }
        }
        Value::Array(arr) => {
            for v in arr {
                extract_references(v, _path, refs);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_simple_bundle() {
        let bundle = serde_json::json!({
            "resourceType": "Bundle",
            "type": "collection",
            "entry": [
                {
                    "fullUrl": "urn:uuid:patient-1",
                    "resource": {
                        "resourceType": "Patient",
                        "id": "p1",
                        "name": [{"family": "Doe", "given": ["John"]}],
                        "gender": "male"
                    }
                },
                {
                    "fullUrl": "urn:uuid:obs-1",
                    "resource": {
                        "resourceType": "Observation",
                        "id": "o1",
                        "status": "final",
                        "code": {"text": "Heart Rate"},
                        "subject": {"reference": "urn:uuid:patient-1"}
                    }
                }
            ]
        });

        let analysis = analyze_bundle(&bundle).unwrap();
        assert_eq!(analysis.entry_count, 2);
        assert_eq!(analysis.bundle_type, "collection");
        assert_eq!(analysis.references.len(), 1);
        assert_eq!(analysis.references[0].to_index, Some(0));
        assert_eq!(analysis.dangling_references, 0);
    }

    #[test]
    fn test_dangling_reference() {
        let bundle = serde_json::json!({
            "resourceType": "Bundle",
            "type": "collection",
            "entry": [
                {
                    "resource": {
                        "resourceType": "Observation",
                        "subject": {"reference": "Patient/missing"}
                    }
                }
            ]
        });
        let analysis = analyze_bundle(&bundle).unwrap();
        assert_eq!(analysis.dangling_references, 1);
        assert_eq!(analysis.references[0].to_index, None);
    }

    #[test]
    fn test_patient_display_name() {
        let r = serde_json::json!({
            "resourceType": "Patient",
            "name": [{"family": "Smith", "given": ["Jane", "A"]}]
        });
        assert_eq!(build_display_name("Patient", &r), "Jane Smith");
    }

    #[test]
    fn test_resource_type_counts() {
        let bundle = serde_json::json!({
            "resourceType": "Bundle",
            "type": "collection",
            "entry": [
                {"resource": {"resourceType": "Patient"}},
                {"resource": {"resourceType": "Observation"}},
                {"resource": {"resourceType": "Observation"}},
            ]
        });
        let analysis = analyze_bundle(&bundle).unwrap();
        assert_eq!(analysis.resource_type_counts[0], ("Observation".to_string(), 2));
    }

    #[test]
    fn test_rejects_non_bundle() {
        let not_bundle = serde_json::json!({"resourceType": "Patient"});
        assert!(analyze_bundle(&not_bundle).is_err());
    }
}
