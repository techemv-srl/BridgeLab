use serde::Serialize;
use tauri::State;

use crate::anonymization::{self, ExtraPhiField, PhiLocation};
use crate::licensing::feature_gate;
use crate::message_store::MessageStore;
use crate::parser::truncation;
use crate::plugins::{self, PluginRegistry};

fn plugin_phi_rules(registry: &PluginRegistry) -> Vec<ExtraPhiField> {
    registry.active_phi_rules().into_iter().map(|r| ExtraPhiField {
        segment: r.segment,
        field: r.field,
        name: r.name,
        sensitivity: plugins::parse_sensitivity(&r.sensitivity),
    }).collect()
}

/// Detect PHI fields in an HL7 message (built-in + plugin rules).
#[tauri::command]
pub fn detect_phi(
    message_id: String,
    store: State<'_, MessageStore>,
    registry: State<'_, PluginRegistry>,
) -> Result<Vec<PhiLocation>, String> {
    let msg = store.get(&message_id)
        .ok_or_else(|| format!("Message not found: {}", message_id))?;
    let extra = plugin_phi_rules(&registry);
    Ok(anonymization::detect_phi_with_extra(&msg, &extra))
}

/// Anonymize an HL7 message and return the anonymized text (Pro feature).
#[tauri::command]
pub fn anonymize_message(
    message_id: String,
    store: State<'_, MessageStore>,
    registry: State<'_, PluginRegistry>,
) -> Result<AnonymizeResult, String> {
    feature_gate::require("anonymize_mask")?;
    let msg = store.get(&message_id)
        .ok_or_else(|| format!("Message not found: {}", message_id))?;

    let extra = plugin_phi_rules(&registry);
    let phi_count = anonymization::detect_phi_with_extra(&msg, &extra).len();
    let anonymized_text = anonymization::anonymize_message_with_extra(&msg, &extra);

    Ok(AnonymizeResult {
        anonymized_text,
        phi_fields_masked: phi_count,
    })
}

#[derive(Debug, Serialize)]
pub struct AnonymizeResult {
    pub anonymized_text: String,
    pub phi_fields_masked: usize,
}

/// Copy the full message content to a string (frontend handles clipboard).
#[tauri::command]
pub fn get_message_full_text(
    message_id: String,
    store: State<'_, MessageStore>,
) -> Result<String, String> {
    let msg = store.get(&message_id)
        .ok_or_else(|| format!("Message not found: {}", message_id))?;
    Ok(truncation::build_full_text(&msg))
}

/// Get a truncated copy of the message for email sharing.
#[tauri::command]
pub fn get_message_truncated_text(
    message_id: String,
    threshold: Option<usize>,
    store: State<'_, MessageStore>,
) -> Result<String, String> {
    let msg = store.get(&message_id)
        .ok_or_else(|| format!("Message not found: {}", message_id))?;
    let thresh = threshold.unwrap_or(100);
    Ok(anonymization::build_truncated_copy(&msg, thresh))
}

/// Export message as JSON representation (Pro feature).
#[tauri::command]
pub fn export_as_json(
    message_id: String,
    store: State<'_, MessageStore>,
) -> Result<String, String> {
    feature_gate::require("export")?;
    let msg = store.get(&message_id)
        .ok_or_else(|| format!("Message not found: {}", message_id))?;

    let mut segments = Vec::new();
    for seg in &msg.segments {
        let mut fields_json = serde_json::Map::new();
        for field in &seg.fields {
            let value = field.span.as_str(&msg.raw);
            fields_json.insert(
                format!("{}-{}", seg.segment_type, field.position),
                serde_json::Value::String(value.to_string()),
            );
        }
        let mut seg_obj = serde_json::Map::new();
        seg_obj.insert("segment_type".into(), serde_json::Value::String(seg.segment_type.clone()));
        seg_obj.insert("position".into(), serde_json::Value::Number(seg.position.into()));
        seg_obj.insert("fields".into(), serde_json::Value::Object(fields_json));
        segments.push(serde_json::Value::Object(seg_obj));
    }

    let root = serde_json::json!({
        "message_type": msg.message_type,
        "version": msg.version,
        "segments": segments,
    });

    serde_json::to_string_pretty(&root)
        .map_err(|e| format!("JSON serialization failed: {}", e))
}

/// Export message as CSV (Pro feature).
#[tauri::command]
pub fn export_as_csv(
    message_id: String,
    store: State<'_, MessageStore>,
) -> Result<String, String> {
    feature_gate::require("export")?;
    let msg = store.get(&message_id)
        .ok_or_else(|| format!("Message not found: {}", message_id))?;

    let mut csv = String::from("Segment,Position,Field,Value\n");
    for seg in &msg.segments {
        for field in &seg.fields {
            let value = field.span.as_str(&msg.raw).replace('"', "\"\"");
            csv.push_str(&format!(
                "{},{},{}-{},\"{}\"\n",
                seg.segment_type, seg.position, seg.segment_type, field.position, value
            ));
        }
    }

    Ok(csv)
}
