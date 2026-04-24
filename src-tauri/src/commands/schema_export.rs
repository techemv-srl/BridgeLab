//! IPC commands for the XSD export feature (F1).

use serde::Serialize;

use crate::parser::hl7::schema::{self, Hl7Version};

/// A version as exposed to the frontend dropdown.
#[derive(Debug, Clone, Serialize)]
pub struct VersionOption {
    /// Stable key, e.g. "V2_5" — used as the argument for subsequent commands.
    pub key: String,
    /// Display label, e.g. "2.5".
    pub label: String,
}

/// A message type as exposed to the frontend dropdown.
#[derive(Debug, Clone, Serialize)]
pub struct MessageOption {
    /// XSD-safe code, e.g. "ADT_A01" — argument for `export_xsd`.
    pub code: String,
    /// HL7 event notation, e.g. "ADT^A01" — shown to the user.
    pub event: String,
    /// Human-readable description.
    pub description: String,
}

/// Enumerate HL7 versions available for XSD export.
#[tauri::command]
pub fn hl7_schema_list_versions() -> Vec<VersionOption> {
    vec![VersionOption {
        key: "V2_5".into(),
        label: "2.5".into(),
    }]
}

/// List the message structures available for the given version.
#[tauri::command]
pub fn hl7_schema_list_messages(version_key: String) -> Result<Vec<MessageOption>, String> {
    let v = parse_version(&version_key)?;
    let s = schema::load(v);
    Ok(s.messages
        .iter()
        .map(|m| MessageOption {
            code: m.code.clone(),
            event: m.event.clone(),
            description: m.description.clone(),
        })
        .collect())
}

/// Generate the XSD text for `message_code` under `version_key`.
#[tauri::command]
pub fn hl7_schema_export_xsd(version_key: String, message_code: String) -> Result<String, String> {
    let v = parse_version(&version_key)?;
    let s = schema::load(v);
    schema::xsd::generate_xsd(&s, &message_code)
}

fn parse_version(key: &str) -> Result<Hl7Version, String> {
    match key {
        "V2_5" => Ok(Hl7Version::V2_5),
        other => Err(format!("Unsupported HL7 version: {}", other)),
    }
}
