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
    /// True when at least one StructureDefinition was applied. The UI says
    /// so, because "no profile findings" means something very different
    /// when no profile ran.
    pub profiles_applied: bool,
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
    tel: State<'_, crate::licensing::telemetry::UsageCounters>,
) -> Result<ValidationReport, String> {
    tel.bump_mem("validations_hl7");
    let msg = store
        .get(&message_id)
        .ok_or_else(|| format!("Message not found: {}", message_id))?;

    let mut report = validation::validate_hl7_message(&msg);

    // Append plugin rules (if any are installed + enabled + within the cap)
    let plugin_rules =
        registry.active_validation_rules(crate::licensing::feature_gate::active_plugin_limit());
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
///
/// Runs the built-in structural checks followed by any active user-defined
/// FHIR rules (files in `<config>/BridgeLab/plugins/fhir/*.json`).
#[tauri::command]
pub fn validate_fhir(
    content: String,
    registry: State<'_, PluginRegistry>,
    profiles: State<'_, fhir::profile::ProfileRegistry>,
    tel: State<'_, crate::licensing::telemetry::UsageCounters>,
) -> Result<FhirValidationReport, String> {
    tel.bump_mem("validations_fhir");
    // Route by encoding: XML resources go through the XML->JSON converter,
    // then the same rule set runs on both.
    let trimmed = content.trim_start();
    let resource = if trimmed.starts_with('<') {
        fhir::parse_fhir_xml(&content)?
    } else {
        fhir::parse_fhir_json(&content)?
    };
    let mut issues = fhir::validate_fhir_json(&resource);

    let fhir_rules =
        registry.active_fhir_rules(crate::licensing::feature_gate::active_plugin_limit());
    if !fhir_rules.is_empty() {
        if let Some(json) = &resource.json_value {
            issues.extend(plugins::run_fhir_validations(json, &fhir_rules));
        }
    }

    // Conformance against the installed StructureDefinitions, when any
    // apply to this resource type.
    let mut profiles_applied = false;
    if let Some(json) = &resource.json_value {
        if let Some(found) = profiles.validate(json) {
            profiles_applied = true;
            issues.extend(found);
        }
    }
    // A resource that declares a profile and got no conformance findings
    // looks clean; say plainly when that is because nothing was checked.
    if !profiles_applied
        && resource
            .json_value
            .as_ref()
            .and_then(|j| j.get("meta"))
            .and_then(|m| m.get("profile"))
            .is_some()
    {
        issues.push(fhir::FhirValidationIssue {
            severity: "info".into(),
            message: "Profile conformance was not checked: no FHIR profile package is \
                      installed. Add one under Tools → FHIR profile packages…"
                .into(),
            path: "meta.profile".into(),
        });
    }

    let error_count = issues.iter().filter(|i| i.severity == "error").count();
    let warning_count = issues.iter().filter(|i| i.severity == "warning").count();
    let info_count = issues.iter().filter(|i| i.severity == "info").count();

    Ok(FhirValidationReport {
        issues,
        error_count,
        warning_count,
        info_count,
        profiles_applied,
    })
}
