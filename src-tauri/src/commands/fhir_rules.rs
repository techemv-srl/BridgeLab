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

fn parse_pack(text: &str, path: &std::path::Path) -> Result<PluginPack, String> {
    serde_json::from_str::<PluginPack>(plugins::strip_bom(text))
        .map_err(|e| format!("{} is not a valid rule pack: {}", path.display(), e))
}

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
        extra: Default::default(),
    }
}

/// The builder's view of the pack.
#[derive(Debug, Serialize)]
pub struct FhirRuleSet {
    pub rules: Vec<FhirRule>,
    /// Absolute path of the file, shown so users can find or share it.
    pub path: String,
    /// The file as read (a hash; empty when there is no file yet). Save
    /// sends it back, and refuses when the file has changed since.
    pub stamp: String,
    /// Why the rules do not run on validation right now: "off" (switched
    /// off in Settings → Plugins), "gated" (beyond the Community limit on
    /// active packs) or "error" (the pack does not load). `None` = they run.
    pub status: Option<String>,
}

fn stamp_of(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

/// The file's current stamp, or "" when there is none.
fn current_stamp(path: &std::path::Path) -> Result<String, String> {
    match fs::read(path) {
        Ok(bytes) => Ok(stamp_of(&bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(format!("Could not read {}: {}", path.display(), e)),
    }
}

/// Whether the registry runs the builder's pack (see [`FhirRuleSet::status`]).
fn pack_status(registry: &PluginRegistry, path: &std::path::Path, limit: Option<usize>) -> Option<String> {
    let shown = path.display().to_string();
    let info = registry.list(limit).into_iter().find(|p| p.kind == "fhir" && p.path == shown)?;
    if info.error.is_some() {
        Some("error".into())
    } else if info.gated {
        Some("gated".into())
    } else if !info.enabled {
        Some("off".into())
    } else {
        None
    }
}

fn rule_set(rules: Vec<FhirRule>, path: &std::path::Path, stamp: String, registry: &PluginRegistry) -> FhirRuleSet {
    FhirRuleSet {
        rules,
        path: path.display().to_string(),
        stamp,
        status: pack_status(registry, path, feature_gate::active_plugin_limit()),
    }
}

/// Read the rules the builder owns. Missing file = no rules yet, not an error.
#[tauri::command]
pub fn fhir_rules_list(registry: State<'_, PluginRegistry>) -> Result<FhirRuleSet, String> {
    let path = user_pack_path()?;
    let (rules, stamp) = match fs::read(&path) {
        Err(_) => (vec![], String::new()),
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            (parse_pack(&text, &path)?.fhir_rules, stamp_of(&bytes))
        }
    };
    Ok(rule_set(rules, &path, stamp, &registry))
}

/// Rules the builder can write: each one runnable, and no two sharing an id.
fn check_rules(rules: &[FhirRule]) -> Result<(), String> {
    let mut ids = std::collections::HashSet::new();
    for rule in rules {
        check_rule(rule)?;
        if !ids.insert(rule.rule_id.trim()) {
            return Err(format!(
                "Two rules share the id '{}'; give each rule its own id",
                rule.rule_id.trim()
            ));
        }
    }
    Ok(())
}

/// The pack text the builder writes over `existing` (the file as it is,
/// if any): its rules replaced, everything else in it kept.
fn merged_pack(existing: Option<&str>, path: &std::path::Path, rules: &[FhirRule]) -> Result<String, String> {
    // A file that exists but does not parse is someone's work (hand-written
    // rules with a typo, a BOM from a Windows editor): refuse to save rather
    // than replace it with the builder's rules alone.
    let mut pack = match existing {
        Some(text) => parse_pack(text, path).map_err(|e| {
            format!("{}. Nothing was saved: fix the file or move it away, then save again.", e)
        })?,
        None => empty_pack(),
    };
    pack.fhir_rules = rules.to_vec();
    serde_json::to_string_pretty(&pack).map_err(|e| format!("Could not serialise the rule pack: {}", e))
}

/// Replace the builder's rules and reload the registry so the next
/// validation picks them up without restarting the app.
///
/// `stamp` is the one [`fhir_rules_list`] returned: when the file changed
/// on disk since (a colleague's rule, a `git pull`), nothing is saved, so
/// rules the builder never showed are not overwritten.
#[tauri::command]
pub fn fhir_rules_save(
    rules: Vec<FhirRule>,
    stamp: Option<String>,
    registry: State<'_, PluginRegistry>,
) -> Result<FhirRuleSet, String> {
    feature_gate::require("fhir_rules_builder")?;

    // Reject anything that cannot run before it reaches disk, so a saved
    // pack is always one the validator can execute.
    check_rules(&rules)?;

    let path = user_pack_path()?;
    let existing = match fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(format!("Could not read {}: {}", path.display(), e)),
    };
    let now = existing.as_deref().map(stamp_of).unwrap_or_default();
    if let Some(expected) = stamp {
        if expected != now {
            return Err(format!(
                "{} changed on disk after the builder opened it. Nothing was saved:                  close the builder and open it again to see the current rules.",
                path.display()
            ));
        }
    }
    let existing_text = existing.as_deref().map(String::from_utf8_lossy);
    let text = merged_pack(existing_text.as_deref(), &path, &rules)?;
    crate::utils::atomic_write::write_atomic(&path, text.as_bytes()).map_err(|e| format!("Could not write {}: {}", path.display(), e))?;

    registry.reload()?;

    let stamp = current_stamp(&path)?;
    Ok(rule_set(rules, &path, stamp, &registry))
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
    let json = resource.json_value.as_ref().ok_or("The open resource has no JSON representation")?;

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

    // The values come from the resources the rule actually runs on: in a
    // Bundle those are the entries, not the Bundle itself.
    let selected = match rule.path.as_deref() {
        Some(path) if applied => plugins::fhir_targets(&json)
            .into_iter()
            .filter(|(target, _)| {
                rule.resource.as_deref().map_or(true, |r| {
                    target.get("resourceType").and_then(|v| v.as_str()) == Some(r)
                })
            })
            .flat_map(|(target, _)| fhirpath::evaluate_within(path, target, &json).results)
            .collect(),
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

    if let Some(crate::plugins::FhirCheck::Regex { pattern }) = &rule.check {
        regex::Regex::new(pattern).map_err(|e| {
            let detail = e.to_string();
            let first = detail.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("").to_string();
            format!("Rule '{}': the pattern does not compile ({})", rule.rule_id, first)
        })?;
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
            extra: Default::default(),
        }
    }

    #[test]
    fn rejects_a_pattern_that_does_not_compile_and_duplicate_ids() {
        let bad = FhirRule {
            expression: None,
            path: Some("name.family".into()),
            check: Some(FhirCheck::Regex { pattern: "([a-z".into() }),
            ..rule()
        };
        assert!(check_rule(&bad).unwrap_err().contains("does not compile"));
        let e = check_rules(&[rule(), FhirRule { message: "other".into(), ..rule() }]).unwrap_err();
        assert!(e.contains("share the id"), "{e}");
        assert!(check_rules(&[rule(), FhirRule { rule_id: "r2".into(), ..rule() }]).is_ok());
    }

    #[test]
    fn saving_keeps_keys_the_builder_does_not_know() {
        let path = std::path::Path::new("user-rules.json");
        let hand_written = r#"{"$schema": "https://acme.example/rule-pack.schema.json", "owner": "integration-team@acme",
            "id": "acme", "name": "ACME",
            "fhir_rules": [{"rule_id": "r1", "severity": "error", "resource": "Patient",
                "expression": "identifier.exists()", "message": "m1",
                "comment": "Required by ACME MPI since 2023", "ticket": "INT-1234"}]}"#;
        // The builder round-trips the rules it was given, extras included.
        let mut rules = parse_pack(hand_written, path).unwrap().fhir_rules;
        rules.push(FhirRule { rule_id: "r2".into(), ..rule() });
        let saved: serde_json::Value = serde_json::from_str(&merged_pack(Some(hand_written), path, &rules).unwrap()).unwrap();
        assert_eq!(saved["$schema"], "https://acme.example/rule-pack.schema.json");
        assert_eq!(saved["owner"], "integration-team@acme");
        assert_eq!(saved["id"], "acme");
        assert_eq!(saved["fhir_rules"][0]["comment"], "Required by ACME MPI since 2023");
        assert_eq!(saved["fhir_rules"][0]["ticket"], "INT-1234");
        assert_eq!(saved["fhir_rules"][1]["rule_id"], "r2");
    }

    #[test]
    fn stamps_tell_a_changed_file_apart() {
        assert_eq!(stamp_of(b"{}"), stamp_of(b"{}"));
        assert_ne!(stamp_of(b"{}"), stamp_of(b"{ }"));
        let missing = std::env::temp_dir().join(format!("bl-no-such-{}.json", uuid::Uuid::new_v4()));
        assert_eq!(current_stamp(&missing).unwrap(), "");
    }

    #[test]
    fn the_status_says_when_the_pack_does_not_run() {
        let root = std::env::temp_dir().join(format!("bl-rules-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("fhir")).unwrap();
        let path = root.join("fhir").join("user-rules.json");
        fs::write(&path, r#"{"id": "user-fhir-rules", "name": "My FHIR rules", "fhir_rules": []}"#).unwrap();
        let reg = PluginRegistry::with_root(root.clone());
        reg.reload().unwrap();
        assert_eq!(pack_status(&reg, &path, None), None);
        reg.set_enabled("fhir/user-fhir-rules", false).unwrap();
        assert_eq!(pack_status(&reg, &path, None).as_deref(), Some("off"));
        reg.set_enabled("fhir/user-fhir-rules", true).unwrap();
        assert_eq!(pack_status(&reg, &path, Some(0)).as_deref(), Some("gated"));
        fs::remove_dir_all(&root).ok();
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
