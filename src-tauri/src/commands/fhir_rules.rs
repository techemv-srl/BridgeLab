//! IPC commands backing the FHIR validation rules builder.
//!
//! The builder edits one pack — `<config>/BridgeLab/plugins/fhir/user-rules.json`
//! — through these commands. Packs written by hand in the same directory are
//! loaded by the plugin registry exactly the same way; the builder simply
//! owns this one file so it can rewrite it wholesale.

use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use tauri::State;

use crate::licensing::feature_gate;
use crate::message_store::MessageStore;
use crate::parser::fhir::{self, fhirpath};
use crate::plugins::{self, FhirRule, PluginPack, PluginRegistry};

/// Identifier of the pack the builder owns.
const USER_PACK_ID: &str = "user-fhir-rules";
const USER_PACK_FILE: &str = "user-rules.json";

fn user_pack_path() -> Result<PathBuf, String> {
    let root = plugins::plugins_root().ok_or("Could not determine the config directory")?;
    let dir = root.join("fhir");
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {}", dir.display(), e))?;
    Ok(dir.join(USER_PACK_FILE))
}

fn empty_pack() -> PluginPack {
    PluginPack {
        id: USER_PACK_ID.into(),
        name: "My FHIR rules".into(),
        description: "Rules created with the in-app FHIR rules builder.".into(),
        author: String::new(),
        version: "1.0".into(),
        enabled: true,
        validation_rules: vec![],
        fhir_rules: vec![],
        phi_rules: vec![],
    }
}

/// The builder's view of the pack.
#[derive(Debug, Serialize)]
pub struct FhirRuleSet {
    pub rules: Vec<FhirRule>,
    /// Absolute path of the file, shown so users can find or share it.
    pub path: String,
}

/// Read the rules the builder owns. Missing file = no rules yet, not an error.
#[tauri::command]
pub fn fhir_rules_list() -> Result<FhirRuleSet, String> {
    let path = user_pack_path()?;
    let rules = match fs::read_to_string(&path) {
        Err(_) => vec![],
        Ok(text) => serde_json::from_str::<PluginPack>(&text)
            .map_err(|e| format!("{} is not a valid rule pack: {}", path.display(), e))?
            .fhir_rules,
    };
    Ok(FhirRuleSet {
        rules,
        path: path.display().to_string(),
    })
}

/// Replace the builder's rules and reload the registry so the next
/// validation picks them up without restarting the app.
#[tauri::command]
pub fn fhir_rules_save(
    rules: Vec<FhirRule>,
    registry: State<'_, PluginRegistry>,
) -> Result<FhirRuleSet, String> {
    feature_gate::require("fhir_rules_builder")?;

    // Reject anything that cannot run before it reaches disk, so a saved
    // pack is always one the validator can execute.
    for rule in &rules {
        check_rule(rule)?;
    }

    let path = user_pack_path()?;
    let mut pack = match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<PluginPack>(&text).unwrap_or_else(|_| empty_pack()),
        Err(_) => empty_pack(),
    };
    pack.fhir_rules = rules.clone();

    let text = serde_json::to_string_pretty(&pack)
        .map_err(|e| format!("Could not serialise the rule pack: {}", e))?;
    fs::write(&path, text).map_err(|e| format!("Could not write {}: {}", path.display(), e))?;

    registry.reload()?;

    Ok(FhirRuleSet {
        rules,
        path: path.display().to_string(),
    })
}

/// Validate a single rule without saving it — used to keep the editor's
/// Save button honest.
#[tauri::command]
pub fn fhir_rule_check(rule: FhirRule) -> Result<(), String> {
    check_rule(&rule)
}

/// What a rule does to one resource, so the editor can show it before the
/// rule is saved.
#[derive(Debug, Serialize)]
pub struct FhirRuleTest {
    /// True when the rule is satisfied (or does not apply).
    pub passed: bool,
    /// False when the rule's resource filter excluded every target.
    pub applied: bool,
    pub issues: Vec<fhir::FhirValidationIssue>,
    /// Values the rule's `path` selected, for the selector form.
    pub selected: Vec<serde_json::Value>,
}

/// Run one rule against a message already open in the app.
#[tauri::command]
pub fn fhir_rule_test(
    rule: FhirRule,
    message_id: String,
    store: State<'_, MessageStore>,
) -> Result<FhirRuleTest, String> {
    feature_gate::require("fhir_rules_builder")?;
    check_rule(&rule)?;

    let resource = store
        .get_fhir(&message_id)
        .ok_or_else(|| format!("No FHIR resource open under id {}", message_id))?;
    let json = resource
        .json_value
        .ok_or("The open resource has no JSON representation")?;

    let issues = plugins::run_fhir_validations(&json, std::slice::from_ref(&rule));

    // Whether the rule matched anything at all is as useful as whether it
    // passed: a rule scoped to the wrong resource type silently passes.
    let applied = match &rule.resource {
        None => true,
        Some(required) => {
            json.get("resourceType").and_then(|v| v.as_str()) == Some(required.as_str())
                || json
                    .get("entry")
                    .and_then(|e| e.as_array())
                    .is_some_and(|entries| {
                        entries.iter().any(|entry| {
                            entry
                                .get("resource")
                                .and_then(|r| r.get("resourceType"))
                                .and_then(|v| v.as_str())
                                == Some(required.as_str())
                        })
                    })
        }
    };

    let selected = match rule.path.as_deref() {
        Some(path) if applied => fhirpath::evaluate(path, &json).results,
        _ => vec![],
    };

    Ok(FhirRuleTest {
        passed: issues.is_empty(),
        applied,
        issues,
        selected,
    })
}

/// A rule must have an id, a message, exactly one of the two forms, and
/// FHIRPath that parses.
fn check_rule(rule: &FhirRule) -> Result<(), String> {
    if rule.rule_id.trim().is_empty() {
        return Err("A rule needs an id".into());
    }
    if rule.message.trim().is_empty() {
        return Err(format!("Rule '{}' needs a message", rule.rule_id));
    }
    if !matches!(rule.severity.as_str(), "error" | "warning" | "info") {
        return Err(format!(
            "Rule '{}': severity must be error, warning or info",
            rule.rule_id
        ));
    }

    match (&rule.expression, &rule.path) {
        (Some(expression), None) => fhirpath::check(expression)
            .map_err(|e| format!("Rule '{}': {}", rule.rule_id, e)),
        (None, Some(path)) => {
            fhirpath::check(path).map_err(|e| format!("Rule '{}': {}", rule.rule_id, e))?;
            if rule.check.is_none() {
                return Err(format!(
                    "Rule '{}': a path needs a check to apply to it",
                    rule.rule_id
                ));
            }
            Ok(())
        }
        (Some(_), Some(_)) => Err(format!(
            "Rule '{}': use either an expression or a path with a check, not both",
            rule.rule_id
        )),
        (None, None) => Err(format!(
            "Rule '{}': needs an expression, or a path with a check",
            rule.rule_id
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::FhirCheck;

    fn rule() -> FhirRule {
        FhirRule {
            rule_id: "r1".into(),
            severity: "error".into(),
            resource: Some("Patient".into()),
            expression: Some("identifier.exists()".into()),
            path: None,
            check: None,
            message: "Patient needs an identifier".into(),
        }
    }

    #[test]
    fn accepts_a_well_formed_invariant() {
        assert!(check_rule(&rule()).is_ok());
    }

    #[test]
    fn accepts_a_selector_with_a_check() {
        let r = FhirRule {
            expression: None,
            path: Some("telecom.value".into()),
            check: Some(FhirCheck::NotEmpty),
            ..rule()
        };
        assert!(check_rule(&r).is_ok());
    }

    #[test]
    fn rejects_a_selector_without_a_check() {
        let r = FhirRule {
            expression: None,
            path: Some("telecom.value".into()),
            check: None,
            ..rule()
        };
        assert!(check_rule(&r).unwrap_err().contains("needs a check"));
    }

    #[test]
    fn rejects_both_forms_at_once() {
        let r = FhirRule {
            path: Some("telecom.value".into()),
            check: Some(FhirCheck::NotEmpty),
            ..rule()
        };
        assert!(check_rule(&r).unwrap_err().contains("not both"));
    }

    #[test]
    fn rejects_neither_form() {
        let r = FhirRule {
            expression: None,
            ..rule()
        };
        assert!(check_rule(&r).unwrap_err().contains("needs an expression"));
    }

    #[test]
    fn rejects_fhirpath_that_does_not_parse() {
        let r = FhirRule {
            expression: Some("identifier.exists(".into()),
            ..rule()
        };
        assert!(check_rule(&r).is_err());
    }

    #[test]
    fn rejects_missing_metadata() {
        assert!(check_rule(&FhirRule { rule_id: "  ".into(), ..rule() }).is_err());
        assert!(check_rule(&FhirRule { message: String::new(), ..rule() }).is_err());
        assert!(check_rule(&FhirRule { severity: "fatal".into(), ..rule() }).is_err());
    }
}
