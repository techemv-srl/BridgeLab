use serde::Serialize;
use tauri::State;

use crate::licensing::feature_gate;
use crate::message_store::MessageStore;
use crate::parser::fhir;
use crate::parser::hl7::lexer::Hl7Lexer;
use crate::parser::hl7::message::{TreeNode, TreeNodeType};
use crate::parser::truncation;
use crate::utils::error::BridgeLabError;

#[derive(Debug, Serialize)]
pub struct ParseResult {
    pub message_id: String,
    pub message_type: String,
    pub format: String,
    pub version: String,
    pub truncated_text: String,
    pub tree_roots: Vec<TreeNode>,
    pub truncation_count: u32,
    pub file_size_bytes: u64,
    pub segment_count: usize,
}

#[derive(Debug, Serialize)]
pub struct FieldContent {
    pub full_text: String,
    pub byte_length: u64,
}

/// Parse an HL7 message from raw text content.
#[tauri::command]
pub fn parse_message(
    content: String,
    _source: Option<String>,
    store: State<'_, MessageStore>,
) -> Result<ParseResult, BridgeLabError> {
    // Strip UTF-8 BOM if present
    let content = if content.starts_with('\u{FEFF}') {
        content[3..].to_string()
    } else {
        content
    };
    let data = content.into_bytes();
    let file_size = data.len() as u64;

    let lexer = Hl7Lexer::new().with_truncation_threshold(100);
    let msg = lexer
        .parse(data)
        .map_err(|e| BridgeLabError::ParseError(e))?;

    let message_id = uuid::Uuid::new_v4().to_string();
    let truncation_count = msg
        .segments
        .iter()
        .flat_map(|s| &s.fields)
        .filter(|f| f.is_truncated)
        .count() as u32;

    let truncated_text = truncation::build_truncated_text(&msg, 50);
    let tree_roots = build_segment_tree_nodes(&msg);
    let segment_count = msg.segments.len();
    let version = msg.version.clone();
    let message_type = msg.message_type.clone();

    store.insert(message_id.clone(), msg);

    Ok(ParseResult {
        message_id,
        message_type,
        format: "HL7v2".to_string(),
        version,
        truncated_text,
        tree_roots,
        truncation_count,
        file_size_bytes: file_size,
        segment_count,
    })
}

/// Get child tree nodes for a given parent node.
#[tauri::command]
pub fn get_tree_children(
    message_id: String,
    node_id: String,
    store: State<'_, MessageStore>,
) -> Result<Vec<TreeNode>, BridgeLabError> {
    let msg = store
        .get(&message_id)
        .ok_or_else(|| BridgeLabError::MessageNotFound(message_id.clone()))?;

    let parts: Vec<&str> = node_id.split('.').collect();

    match parts.len() {
        // "seg.N" -> return fields of segment N
        1 if parts[0].starts_with("seg") => {
            let seg_idx: usize = parts[0]
                .trim_start_matches("seg")
                .parse()
                .map_err(|_| BridgeLabError::ParseError("Invalid segment index".to_string()))?;
            let segment = msg
                .segments
                .get(seg_idx)
                .ok_or_else(|| BridgeLabError::ParseError("Segment not found".to_string()))?;

            let nodes: Vec<TreeNode> = segment
                .fields
                .iter()
                .map(|field| {
                    let value = field.span.as_str(&msg.raw);
                    let preview = if field.is_truncated {
                        let preview: String = value.chars().take(50).collect();
                        format!("{}{{...}}", preview)
                    } else {
                        value.to_string()
                    };

                    let has_components = field
                        .repetitions
                        .first()
                        .map(|r| r.components.len() > 1)
                        .unwrap_or(false);

                    TreeNode {
                        id: format!("seg{}.f{}", seg_idx, field.position),
                        label: format!(
                            "{}-{}",
                            segment.segment_type, field.position
                        ),
                        value_preview: preview,
                        node_type: TreeNodeType::Field,
                        depth: 2,
                        has_children: has_components,
                        is_truncated: field.is_truncated,
                        child_count: if has_components {
                            field.repetitions[0].components.len()
                        } else {
                            0
                        },
                    }
                })
                .collect();

            Ok(nodes)
        }
        // "seg.N.fM" -> return components of field M in segment N
        1 if parts[0].contains(".f") => {
            // This shouldn't match the split pattern, handle in len 2
            Err(BridgeLabError::ParseError("Invalid node ID format".to_string()))
        }
        2 => {
            let seg_idx: usize = parts[0]
                .trim_start_matches("seg")
                .parse()
                .map_err(|_| BridgeLabError::ParseError("Invalid segment index".to_string()))?;
            let field_pos: usize = parts[1]
                .trim_start_matches('f')
                .parse()
                .map_err(|_| BridgeLabError::ParseError("Invalid field position".to_string()))?;

            let segment = msg
                .segments
                .get(seg_idx)
                .ok_or_else(|| BridgeLabError::ParseError("Segment not found".to_string()))?;
            let field = segment
                .fields
                .iter()
                .find(|f| f.position == field_pos)
                .ok_or_else(|| BridgeLabError::ParseError("Field not found".to_string()))?;

            let rep = field
                .repetitions
                .first()
                .ok_or_else(|| BridgeLabError::ParseError("No repetitions".to_string()))?;

            let nodes: Vec<TreeNode> = rep
                .components
                .iter()
                .enumerate()
                .map(|(i, comp)| {
                    let value = comp.span.as_str(&msg.raw).to_string();
                    let has_subs = !comp.subcomponents.is_empty();

                    TreeNode {
                        id: format!("seg{}.f{}.c{}", seg_idx, field_pos, i + 1),
                        label: format!("{}-{}.{}", segment.segment_type, field_pos, i + 1),
                        value_preview: value,
                        node_type: TreeNodeType::Component,
                        depth: 3,
                        has_children: has_subs,
                        is_truncated: false,
                        child_count: comp.subcomponents.len(),
                    }
                })
                .collect();

            Ok(nodes)
        }
        _ => Ok(Vec::new()),
    }
}

/// A single match returned by `search_message`. `node_id` follows the same
/// scheme as the tree ("seg{N}" / "seg{N}.f{P}") so the frontend can reuse
/// the existing expand-and-scroll navigation.
#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub node_id: String,
    pub segment_idx: usize,
    pub field_position: Option<usize>,
    pub label: String,
    pub snippet: String,
    /// "segment" | "name" | "value" — what part of the node matched.
    pub match_kind: String,
}

/// Maximum hits returned; beyond this the query is too broad to be useful
/// and the UI would choke rendering thousands of rows.
const MAX_SEARCH_HITS: usize = 200;
/// Per-field scan cap. Truncated fields can hold multi-MB base64 blobs;
/// matches that deep in a blob aren't navigable anyway.
const MAX_FIELD_SCAN_CHARS: usize = 10_000;

/// Case-insensitive search across segment types, schema field names and
/// field values. Searches the *parsed* message in the store, so it finds
/// fields the tree hasn't lazily loaded yet.
#[tauri::command]
pub fn search_message(
    message_id: String,
    query: String,
    store: State<'_, MessageStore>,
) -> Result<Vec<SearchHit>, BridgeLabError> {
    let msg = store
        .get(&message_id)
        .ok_or_else(|| BridgeLabError::MessageNotFound(message_id))?;
    Ok(search_in_message(&msg, &query))
}

fn search_in_message(
    msg: &crate::parser::hl7::message::Hl7Message,
    query: &str,
) -> Vec<SearchHit> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }

    let mut hits: Vec<SearchHit> = Vec::new();

    'segments: for (seg_idx, segment) in msg.segments.iter().enumerate() {
        if segment.segment_type.to_lowercase().contains(&q) {
            let preview: String = segment.span.as_str(&msg.raw).chars().take(60).collect();
            hits.push(SearchHit {
                node_id: format!("seg{}", seg_idx),
                segment_idx: seg_idx,
                field_position: None,
                label: format!("{} ({})", segment.segment_type, seg_idx),
                snippet: preview,
                match_kind: "segment".into(),
            });
            if hits.len() >= MAX_SEARCH_HITS {
                break 'segments;
            }
        }

        for field in &segment.fields {
            let value = field.span.as_str(&msg.raw);
            let scan: String = value.chars().take(MAX_FIELD_SCAN_CHARS).collect::<String>().to_lowercase();
            let value_match = scan.contains(&q);

            let field_name = crate::parser::hl7::tables::get_field_info(
                &segment.segment_type,
                field.position,
                &msg.version,
            )
            .map(|i| i.name)
            .unwrap_or_default();
            let name_match = !field_name.is_empty() && field_name.to_lowercase().contains(&q);

            if value_match || name_match {
                let label = if field_name.is_empty() {
                    format!("{}-{}", segment.segment_type, field.position)
                } else {
                    format!("{}-{} {}", segment.segment_type, field.position, field_name)
                };
                hits.push(SearchHit {
                    node_id: format!("seg{}.f{}", seg_idx, field.position),
                    segment_idx: seg_idx,
                    field_position: Some(field.position),
                    label,
                    snippet: value.chars().take(60).collect(),
                    match_kind: if value_match { "value".into() } else { "name".into() },
                });
                if hits.len() >= MAX_SEARCH_HITS {
                    break 'segments;
                }
            }
        }
    }

    hits
}

/// Get full content of a specific field (for expanding truncated fields).
#[tauri::command]
pub fn get_field_content(
    message_id: String,
    segment_idx: usize,
    field_idx: usize,
    store: State<'_, MessageStore>,
) -> Result<FieldContent, BridgeLabError> {
    let content = store
        .get_field_content(&message_id, segment_idx, field_idx)
        .ok_or_else(|| BridgeLabError::MessageNotFound(message_id))?;

    let byte_length = content.len() as u64;
    Ok(FieldContent {
        full_text: content,
        byte_length,
    })
}

/// Build top-level tree nodes (segments) for the frontend.
fn build_segment_tree_nodes(
    msg: &crate::parser::hl7::message::Hl7Message,
) -> Vec<TreeNode> {
    msg.segments
        .iter()
        .enumerate()
        .map(|(i, seg)| {
            let preview = seg.span.as_str(&msg.raw);
            let preview_short: String = preview.chars().take(80).collect();

            TreeNode {
                id: format!("seg{}", i),
                label: format!("{} ({})", seg.segment_type, i),
                value_preview: preview_short,
                node_type: TreeNodeType::Segment,
                depth: 1,
                has_children: !seg.fields.is_empty(),
                is_truncated: false,
                child_count: seg.fields.len(),
            }
        })
        .collect()
}

/// Expand a truncated field inline: returns the full message text with that field expanded.
#[tauri::command]
pub fn expand_field_inline(
    message_id: String,
    segment_idx: usize,
    field_idx: usize,
    store: State<'_, MessageStore>,
) -> Result<String, BridgeLabError> {
    let msg = store
        .get(&message_id)
        .ok_or_else(|| BridgeLabError::MessageNotFound(message_id))?;

    let sep = msg.delimiters.field as char;
    let mut result_segments: Vec<String> = Vec::new();

    for (si, seg) in msg.segments.iter().enumerate() {
        let _seg_text = seg.span.as_str(&msg.raw);
        if si == segment_idx {
            // Rebuild this segment with the target field expanded
            let mut fields: Vec<String> = Vec::new();
            let is_msh = seg.segment_type == "MSH";

            // For MSH, fields[0] = "MSH", and MSH-1 is the field separator
            if is_msh {
                fields.push(seg.segment_type.clone());
            }

            for field in &seg.fields {
                let content = field.span.as_str(&msg.raw);
                if is_msh && field.position <= 2 {
                    // MSH-1 and MSH-2 are handled specially
                    fields.push(content.to_string());
                    continue;
                }

                if field.position == field_idx {
                    // This is the field to expand - use full content
                    fields.push(content.to_string());
                } else if field.is_truncated {
                    // Keep other truncated fields truncated
                    let preview: String = content.chars().take(50).collect();
                    fields.push(format!("{}{{...{} bytes}}", preview, field.span.len()));
                } else {
                    fields.push(content.to_string());
                }
            }

            if is_msh {
                // MSH: first item is "MSH", MSH-1 is separator, then join rest with separator
                let mut line = String::new();
                for (i, f) in fields.iter().enumerate() {
                    if i == 0 {
                        line.push_str(f); // "MSH"
                    } else if i == 1 {
                        line.push_str(f); // field separator "|"
                    } else if i == 2 {
                        line.push_str(f); // encoding chars "^~\&"
                        line.push(sep);
                    } else {
                        if i > 3 { line.push(sep); }
                        line.push_str(f);
                    }
                }
                result_segments.push(line);
            } else {
                result_segments.push(format!("{}{}{}", seg.segment_type, sep, fields.join(&sep.to_string())));
            }
        } else {
            // Not the target segment - use truncated version
            let mut fields_out = Vec::new();
            for field in &seg.fields {
                let content = field.span.as_str(&msg.raw);
                if seg.segment_type == "MSH" && field.position <= 2 {
                    fields_out.push(content.to_string());
                    continue;
                }
                if field.is_truncated {
                    let preview: String = content.chars().take(50).collect();
                    fields_out.push(format!("{}{{...{} bytes}}", preview, field.span.len()));
                } else {
                    fields_out.push(content.to_string());
                }
            }
            if seg.segment_type == "MSH" {
                let mut line = seg.segment_type.clone();
                for (i, f) in fields_out.iter().enumerate() {
                    if i == 0 { line.push_str(f); }
                    else if i == 1 { line.push_str(f); line.push(sep); }
                    else { if i > 2 { line.push(sep); } line.push_str(f); }
                }
                result_segments.push(line);
            } else {
                result_segments.push(format!("{}{}{}", seg.segment_type, sep, fields_out.join(&sep.to_string())));
            }
        }
    }

    Ok(result_segments.join("\r"))
}

/// Expand ALL truncated fields: returns the full original message text.
#[tauri::command]
pub fn expand_all_fields(
    message_id: String,
    store: State<'_, MessageStore>,
) -> Result<String, BridgeLabError> {
    let msg = store
        .get(&message_id)
        .ok_or_else(|| BridgeLabError::MessageNotFound(message_id))?;

    Ok(truncation::build_full_text(&msg))
}

/// Re-truncate a message: returns text with all fields truncated again.
#[tauri::command]
pub fn collapse_all_fields(
    message_id: String,
    store: State<'_, MessageStore>,
) -> Result<String, BridgeLabError> {
    let msg = store
        .get(&message_id)
        .ok_or_else(|| BridgeLabError::MessageNotFound(message_id))?;

    Ok(truncation::build_truncated_text(&msg, 50))
}

/// Parse a FHIR resource from raw text content.
#[tauri::command]
pub fn parse_fhir_message(
    content: String,
    store: State<'_, MessageStore>,
) -> Result<ParseResult, BridgeLabError> {
    let file_size = content.len() as u64;

    let format_type = fhir::detect_fhir(&content)
        .ok_or_else(|| BridgeLabError::ParseError("Content is not a valid FHIR resource".into()))?;

    let resource = match format_type {
        fhir::FhirFormat::Json => fhir::parse_fhir_json(&content)
            .map_err(|e| BridgeLabError::ParseError(e))?,
        fhir::FhirFormat::Xml => fhir::parse_fhir_xml(&content)
            .map_err(|e| BridgeLabError::ParseError(e))?,
    };

    let message_id = uuid::Uuid::new_v4().to_string();
    let resource_type = resource.resource_type.clone();
    let fhir_version = resource.fhir_version.clone();
    let tree_roots = fhir::build_fhir_tree_nodes(&resource);
    let tree_count = tree_roots.len();

    let format_str = match format_type {
        fhir::FhirFormat::Json => "FHIR JSON",
        fhir::FhirFormat::Xml => "FHIR XML",
    };

    store.insert_fhir(message_id.clone(), resource);

    Ok(ParseResult {
        message_id,
        message_type: resource_type,
        format: format_str.to_string(),
        version: fhir_version,
        truncated_text: content,
        tree_roots,
        truncation_count: 0,
        file_size_bytes: file_size,
        segment_count: tree_count,
    })
}

/// Get child tree nodes for a FHIR resource.
#[tauri::command]
pub fn get_fhir_tree_children(
    message_id: String,
    node_id: String,
    store: State<'_, MessageStore>,
) -> Result<Vec<TreeNode>, BridgeLabError> {
    let resource = store
        .get_fhir(&message_id)
        .ok_or_else(|| BridgeLabError::MessageNotFound(message_id))?;

    Ok(fhir::get_fhir_children(&resource, &node_id))
}

/// Analyze a FHIR Bundle (Pro feature).
#[tauri::command]
pub fn analyze_fhir_bundle(
    message_id: String,
    store: State<'_, MessageStore>,
) -> Result<fhir::bundle::BundleAnalysis, BridgeLabError> {
    feature_gate::require("bundle_visualizer")
        .map_err(|e| BridgeLabError::ParseError(e))?;
    let resource = store
        .get_fhir(&message_id)
        .ok_or_else(|| BridgeLabError::MessageNotFound(message_id.clone()))?;

    let json = resource.json_value
        .ok_or_else(|| BridgeLabError::ParseError("FHIR resource is not JSON".into()))?;

    fhir::bundle::analyze_bundle(&json)
        .map_err(|e| BridgeLabError::ParseError(e))
}

/// Evaluate a FHIRPath expression (Pro feature).
#[tauri::command]
pub fn evaluate_fhirpath(
    message_id: String,
    expression: String,
    store: State<'_, MessageStore>,
) -> Result<fhir::fhirpath::FhirPathResult, BridgeLabError> {
    feature_gate::require("fhirpath")
        .map_err(|e| BridgeLabError::ParseError(e))?;
    let resource = store
        .get_fhir(&message_id)
        .ok_or_else(|| BridgeLabError::MessageNotFound(message_id))?;

    let json = resource.json_value
        .ok_or_else(|| BridgeLabError::ParseError("FHIR resource is not JSON".into()))?;

    Ok(fhir::fhirpath::evaluate(&expression, &json))
}

/// Get a specific entry content from a FHIR Bundle (for inspector panel).
#[tauri::command]
pub fn get_fhir_bundle_entry(
    message_id: String,
    entry_index: usize,
    store: State<'_, MessageStore>,
) -> Result<String, BridgeLabError> {
    let resource = store
        .get_fhir(&message_id)
        .ok_or_else(|| BridgeLabError::MessageNotFound(message_id))?;

    let json = resource.json_value
        .ok_or_else(|| BridgeLabError::ParseError("FHIR resource is not JSON".into()))?;

    let entries = json.get("entry")
        .and_then(|v| v.as_array())
        .ok_or_else(|| BridgeLabError::ParseError("Bundle has no entry array".into()))?;

    let entry = entries.get(entry_index)
        .ok_or_else(|| BridgeLabError::ParseError(format!("Entry index {} out of range", entry_index)))?;

    serde_json::to_string_pretty(entry)
        .map_err(|e| BridgeLabError::ParseError(format!("Serialize failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::hl7::lexer::Hl7Lexer;

    fn sample_msg() -> crate::parser::hl7::message::Hl7Message {
        let raw = "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|20260101120000||ADT^A01|MSG001|P|2.5\rPID|1||12345^^^HOSP^MR||ROSSI^MARIO||19800101|M|||VIA ROMA 1^^MILANO^^20100^IT\rPV1|1|I|WARD^101^A".to_string();
        Hl7Lexer::new().parse(raw.into_bytes()).unwrap()
    }

    #[test]
    fn test_search_by_value() {
        let msg = sample_msg();
        let hits = search_in_message(&msg, "rossi");
        assert!(!hits.is_empty());
        let hit = hits.iter().find(|h| h.node_id == "seg1.f5").expect("PID-5 hit");
        assert_eq!(hit.match_kind, "value");
        assert!(hit.snippet.contains("ROSSI"));
        assert_eq!(hit.segment_idx, 1);
        assert_eq!(hit.field_position, Some(5));
    }

    #[test]
    fn test_search_by_segment_type() {
        let msg = sample_msg();
        let hits = search_in_message(&msg, "PV1");
        assert!(hits.iter().any(|h| h.node_id == "seg2" && h.match_kind == "segment"));
    }

    #[test]
    fn test_search_by_schema_field_name() {
        let msg = sample_msg();
        // "Patient Name" is the schema name for PID-5; the literal string
        // does not appear in the message body.
        let hits = search_in_message(&msg, "patient name");
        assert!(hits.iter().any(|h| h.node_id == "seg1.f5" && h.match_kind == "name"));
    }

    #[test]
    fn test_search_empty_query_returns_nothing() {
        let msg = sample_msg();
        assert!(search_in_message(&msg, "   ").is_empty());
    }

    #[test]
    fn test_search_case_insensitive() {
        let msg = sample_msg();
        let lower = search_in_message(&msg, "milano");
        let upper = search_in_message(&msg, "MILANO");
        assert_eq!(lower.len(), upper.len());
        assert!(!lower.is_empty());
    }

    #[test]
    fn test_search_caps_results() {
        // A query matching every field of every segment must not exceed the cap.
        let mut raw = String::from("MSH|^~\\&|A|A|A|A|20260101||ADT^A01|M1|P|2.5");
        for _ in 0..300 {
            raw.push_str("\rNTE|1||AAAA");
        }
        let msg = Hl7Lexer::new().parse(raw.into_bytes()).unwrap();
        let hits = search_in_message(&msg, "a");
        assert!(hits.len() <= MAX_SEARCH_HITS);
    }
}
