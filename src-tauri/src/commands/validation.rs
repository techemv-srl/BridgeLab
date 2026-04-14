use serde::Serialize;
use tauri::State;

use crate::message_store::MessageStore;
use crate::parser::fhir;
use crate::validation::{self, ValidationReport};

#[derive(Debug, Serialize)]
pub struct FhirValidationReport {
    pub issues: Vec<fhir::FhirValidationIssue>,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

/// Validate an HL7 message by its store ID.
#[tauri::command]
pub fn validate_message(
    message_id: String,
    store: State<'_, MessageStore>,
) -> Result<ValidationReport, String> {
    let msg = store
        .get(&message_id)
        .ok_or_else(|| format!("Message not found: {}", message_id))?;

    Ok(validation::validate_hl7_message(&msg))
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
