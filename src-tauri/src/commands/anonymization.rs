use serde::Serialize;
use tauri::State;

use crate::anonymization::{self, ExtraPhiField, PhiLocation};
use crate::licensing::feature_gate;
use crate::message_store::MessageStore;
use crate::parser::truncation;
use crate::plugins::PluginRegistry;

pub(crate) fn plugin_phi_rules(registry: &PluginRegistry) -> Vec<ExtraPhiField> {
    registry.active_phi_fields(feature_gate::active_plugin_limit())
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
    tel: State<'_, crate::licensing::telemetry::UsageCounters>,
) -> Result<AnonymizeResult, String> {
    tel.bump_mem("anonymizations");
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
    tel: State<'_, crate::licensing::telemetry::UsageCounters>,
) -> Result<String, String> {
    tel.bump_mem("exports");
    feature_gate::require("export")?;
    let msg = store.get(&message_id)
        .ok_or_else(|| format!("Message not found: {}", message_id))?;

    crate::parser::hl7::export::to_json(&msg)
}

/// Export message as CSV (Pro feature).
#[tauri::command]
pub fn export_as_csv(
    message_id: String,
    store: State<'_, MessageStore>,
    tel: State<'_, crate::licensing::telemetry::UsageCounters>,
) -> Result<String, String> {
    tel.bump_mem("exports");
    feature_gate::require("export")?;
    let msg = store.get(&message_id)
        .ok_or_else(|| format!("Message not found: {}", message_id))?;

    Ok(crate::parser::hl7::export::to_csv(&msg))
}
