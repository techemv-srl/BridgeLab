//! IPC commands for the XSD export feature.

use serde::Serialize;

use crate::licensing::feature_gate;
use crate::parser::hl7::schema::{self, Hl7Version};

/// Free-tier whitelist: these four messages in v2.5 stay exportable in
/// the Community tier. Everything else is gated behind `xsd_export_full`.
const FREE_MESSAGE_WHITELIST_V2_5: &[&str] = &["ADT_A01", "ADT_A40", "ORM_O01", "ORU_R01"];

/// A version as exposed to the frontend dropdown.
#[derive(Debug, Clone, Serialize)]
pub struct VersionOption {
    pub key: String,
    pub label: String,
    /// "free" or "pro" — the tier required to export *any* message in this version.
    pub tier: String,
}

/// A message type as exposed to the frontend dropdown.
#[derive(Debug, Clone, Serialize)]
pub struct MessageOption {
    pub code: String,
    pub event: String,
    pub description: String,
    /// "free" or "pro" — whether this (version, message) combination is gated.
    pub tier: String,
}

#[tauri::command]
pub fn hl7_schema_list_versions() -> Vec<VersionOption> {
    vec![VersionOption {
        key: "V2_5".into(),
        label: "2.5".into(),
        tier: "free".into(),
    }]
}

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
            tier: tier_for(v, &m.code).into(),
        })
        .collect())
}

#[tauri::command]
pub fn hl7_schema_export_xsd(version_key: String, message_code: String) -> Result<String, String> {
    let v = parse_version(&version_key)?;

    // Gate: community whitelist (4 msg × v2.5) falls under xsd_export_community
    // (always allowed), everything else requires xsd_export_full (Pro).
    let feature = match tier_for(v, &message_code) {
        "free" => "xsd_export_community",
        _ => "xsd_export_full",
    };
    feature_gate::require(feature)?;

    let s = schema::load(v);
    schema::xsd::generate_xsd(&s, &message_code)
}

fn parse_version(key: &str) -> Result<Hl7Version, String> {
    match key {
        "V2_5" => Ok(Hl7Version::V2_5),
        other => Err(format!("Unsupported HL7 version: {}", other)),
    }
}

fn tier_for(version: Hl7Version, message_code: &str) -> &'static str {
    match version {
        Hl7Version::V2_5 if FREE_MESSAGE_WHITELIST_V2_5.contains(&message_code) => "free",
        _ => "pro",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v25_whitelist_is_free() {
        for code in FREE_MESSAGE_WHITELIST_V2_5 {
            assert_eq!(tier_for(Hl7Version::V2_5, code), "free");
        }
    }

    #[test]
    fn unknown_message_is_pro() {
        assert_eq!(tier_for(Hl7Version::V2_5, "SIU_S12"), "pro");
    }
}
