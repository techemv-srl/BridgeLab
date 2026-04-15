use serde::Serialize;
use tauri::State;

use crate::message_store::MessageStore;
use crate::parser::fhir;
use crate::plugins::{self, PluginRegistry};
use crate::validation::{self, Severity, ValidationReport};

#[derive(Debug, Serialize)]
pub struct FhirValidationReport {
    pub issues: Vec<fhir::FhirValidationIssue>,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

/// Validate an HL7 message by its store ID.
///
/// Runs built-in validations followed by any active user-defined plugin rules
/// (files in `<config>/BridgeLab/plugins/validation/*.json`).
#[tauri::command]
pub fn validate_message(
    message_id: String,
    store: State<'_, MessageStore>,
    registry: State<'_, PluginRegistry>,
) -> Result<ValidationReport, String> {
    let msg = store
        .get(&message_id)
        .ok_or_else(|| format!("Message not found: {}", message_id))?;

    let mut report = validation::validate_hl7_message(&msg);

    // Append plugin rules (if any are installed + enabled)
    let plugin_rules = registry.active_validation_rules();
    if !plugin_rules.is_empty() {
        let extra = plugins::run_custom_validations(&msg, &plugin_rules);
        for issue in extra {
            match issue.severity {
                Severity::Error   => report.error_count   += 1,
                Severity::Warning => report.warning_count += 1,
                Severity::Info    => report.info_count    += 1,
            }
            report.issues.push(issue);
        }
    }

    Ok(report)
}

/// Validate a FHIR JSON resource.
#[tauri::command]
pub fn validate_fhir(content: String) -> Result<FhirValidationReport, String> {
    let resource = fhir::parse_fhir_json(&content)?;
    let issues = fhir::validate_fhir_json(&resource);

    let error_count = issues.iter().filter(|i| i.severity == "error").count();
    let warning_count = issues.iter().filter(|i| i.severity == "warning").count();
    let info_count = issues.iter().filter(|i| i.severity == "info").count();

    Ok(FhirValidationReport {
        issues,
        error_count,
        warning_count,
        info_count,
    })
}
