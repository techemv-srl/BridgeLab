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

/// Why the builder refuses a rule, a save or a test. The dialog shows it in
/// the user's language from `code` and `params` (`fhirRules.refusal.<code>`
/// in the frontend's strings); `message` is the same in English, for logs
/// and tests. Code `other` is a message only English can give: an error
/// from the operating system, the licence check.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Refusal {
    pub code: &'static str,
    pub params: std::collections::BTreeMap<&'static str, String>,
    pub message: String,
}

impl Refusal {
    fn new(code: &'static str, params: &[(&'static str, &str)], message: String) -> Self {
        Refusal {
            code,
            params: params.iter().map(|(k, v)| (*k, v.to_string())).collect(),
            message,
        }
    }

    /// Whether the English message holds `text`.
    pub fn contains(&self, text: &str) -> bool {
        self.message.contains(text)
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<String> for Refusal {
    fn from(message: String) -> Self {
        Refusal { code: "other", params: Default::default(), message }
    }
}

fn parse_pack(text: &str, path: &std::path::Path) -> Result<PluginPack, Refusal> {
    serde_json::from_str::<PluginPack>(plugins::strip_bom(text)).map_err(|e| {
        let (path, detail) = (path.display().to_string(), e.to_string());
        Refusal::new(
            "invalid_pack",
            &[("path", &path), ("detail", &detail)],
            format!("{} is not a valid rule pack: {}", path, detail),
        )
    })
}

fn could_not(code: &'static str, verb: &str, path: &std::path::Path, e: impl std::fmt::Display) -> Refusal {
    let (path, detail) = (path.display().to_string(), e.to_string());
    let message = format!("Could not {} {}: {}", verb, path, detail);
    Refusal::new(code, &[("path", &path), ("detail", &detail)], message)
}

/// Identifier of the pack the builder owns.
const USER_PACK_ID: &str = "user-fhir-rules";
const USER_PACK_FILE: &str = "user-rules.json";

fn user_pack_path() -> Result<PathBuf, Refusal> {
    let root = plugins::plugins_root().ok_or_else(|| Refusal::from("Could not determine the config directory".to_string()))?;
    let dir = root.join("fhir");
    fs::create_dir_all(&dir).map_err(|e| could_not("could_not_write", "create", &dir, e))?;
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
fn current_stamp(path: &std::path::Path) -> Result<String, Refusal> {
    match fs::read(path) {
        Ok(bytes) => Ok(stamp_of(&bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(could_not("could_not_read", "read", path, e)),
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
///
/// This and `fhir_rules_save` are async, off the main thread: the pack's
/// status waits for the packs to finish loading after startup, and saving
/// reloads them.
#[tauri::command]
pub async fn fhir_rules_list(registry: State<'_, PluginRegistry>) -> Result<FhirRuleSet, Refusal> {
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
fn check_rules(rules: &[FhirRule]) -> Result<(), Refusal> {
    let mut ids = std::collections::HashSet::new();
    for rule in rules {
        check_rule(rule)?;
        let id = rule.rule_id.trim();
        if !ids.insert(id) {
            return Err(Refusal::new(
                "duplicate_id",
                &[("id", id)],
                format!("Two rules share the id '{}'; give each rule its own id", id),
            ));
        }
    }
    Ok(())
}

/// The pack text the builder writes over `existing` (the file as it is,
/// if any): its rules replaced, everything else in it kept.
fn merged_pack(existing: Option<&str>, path: &std::path::Path, rules: &[FhirRule]) -> Result<String, Refusal> {
    // A file that exists but does not parse is someone's work (hand-written
    // rules with a typo, a BOM from a Windows editor): refuse to save rather
    // than replace it with the builder's rules alone.
    let mut pack = match existing {
        Some(text) => parse_pack(text, path).map_err(|e| Refusal {
            code: "invalid_pack_unsaved",
            message: format!("{}. Nothing was saved: fix the file or move it away, then save again.", e.message),
            ..e
        })?,
        None => empty_pack(),
    };
    pack.fhir_rules = rules.to_vec();
    serde_json::to_string_pretty(&pack).map_err(|e| Refusal::from(format!("Could not serialise the rule pack: {}", e)))
}

/// Replace the builder's rules and reload the registry so the next
/// validation picks them up without restarting the app.
///
/// `stamp` is the one [`fhir_rules_list`] returned: when the file changed
/// on disk since (a colleague's rule, a `git pull`), nothing is saved, so
/// rules the builder never showed are not overwritten.
#[tauri::command]
pub async fn fhir_rules_save(
    rules: Vec<FhirRule>,
    stamp: Option<String>,
    registry: State<'_, PluginRegistry>,
) -> Result<FhirRuleSet, Refusal> {
    feature_gate::require("fhir_rules_builder")?;

    // Reject anything that cannot run before it reaches disk, so a saved
    // pack is always one the validator can execute.
    check_rules(&rules)?;

    let path = user_pack_path()?;
    let existing = match fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(could_not("could_not_read", "read", &path, e)),
    };
    let now = existing.as_deref().map(stamp_of).unwrap_or_default();
    if let Some(expected) = stamp {
        if expected != now {
            return Err(changed_on_disk(&path));
        }
    }
    let existing_text = existing.as_deref().map(String::from_utf8_lossy);
    let text = merged_pack(existing_text.as_deref(), &path, &rules)?;
    crate::utils::atomic_write::write_atomic(&path, text.as_bytes())
        .map_err(|e| could_not("could_not_write", "write", &path, e))?;

    registry.reload()?;

    let stamp = current_stamp(&path)?;
    Ok(rule_set(rules, &path, stamp, &registry))
}

/// The refusal of a save over a file someone else changed meanwhile.
fn changed_on_disk(path: &std::path::Path) -> Refusal {
    let shown = path.display().to_string();
    let message = format!(
        "{} changed on disk after the builder opened it. Nothing was saved: \
         close the builder and open it again to see the current rules.",
        shown
    );
    Refusal::new("changed_on_disk", &[("path", &shown)], message)
}

/// Validate a single rule without saving it — used to keep the editor's
/// Save button honest.
#[tauri::command]
pub fn fhir_rule_check(rule: FhirRule) -> Result<(), Refusal> {
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
///
/// On a blocking-task thread: run on the window's thread, a slow rule
/// froze the whole app until it finished.
#[tauri::command]
pub async fn fhir_rule_test(
    rule: FhirRule,
    message_id: String,
    store: State<'_, MessageStore>,
) -> Result<FhirRuleTest, Refusal> {
    feature_gate::require("fhir_rules_builder")?;
    check_rule(&rule)?;

    let resource = store.get_fhir(&message_id).ok_or_else(|| {
        Refusal::new("no_resource", &[], format!("No FHIR resource open under id {}", message_id))
    })?;
    Ok(test_rule_off_thread(rule, resource).await?)
}

pub(crate) async fn test_rule_off_thread(
    rule: FhirRule,
    resource: std::sync::Arc<fhir::FhirResource>,
) -> Result<FhirRuleTest, String> {
    if resource.json_value.is_none() {
        return Err("The open resource has no JSON representation".into());
    }
    // The shared resource moves into the worker: no copy of the JSON.
    tauri::async_runtime::spawn_blocking(move || {
        let json = resource.json_value.as_ref().expect("checked above");
        test_rule(&rule, json)
    })
    .await
    .map_err(|e| format!("The rule test failed: {}", e))
}

/// What [`fhir_rule_test`] reports. The rule and the values it selects run
/// under one budget, as a validation's rules do.
fn test_rule(rule: &FhirRule, json: &serde_json::Value) -> FhirRuleTest {
    test_rule_within(rule, json, fhirpath::Budget::for_validation())
}

fn test_rule_within(
    rule: &FhirRule,
    json: &serde_json::Value,
    mut budget: fhirpath::Budget,
) -> FhirRuleTest {
    let mut issues =
        plugins::run_fhir_validations_within(json, std::slice::from_ref(rule), &mut budget);

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
    let mut selected = Vec::new();
    if let (Some(path), true) = (rule.path.as_deref(), applied) {
        let mut references = fhirpath::References::default();
        // Where and why the values stopped being listed, if they did: the
        // Test must not then pass showing only part of them. That holds
        // also when the rule spent the budget to the last step without
        // being stopped, so that its own issues say nothing.
        let mut unlisted: Option<(String, String)> = None;
        for (target, prefix) in plugins::fhir_targets(json) {
            let wanted = rule.resource.as_deref().map_or(true, |r| {
                target.get("resourceType").and_then(|v| v.as_str()) == Some(r)
            });
            if !wanted {
                continue;
            }
            if budget.exhausted() {
                unlisted.get_or_insert_with(|| {
                    (
                        format!("{prefix}{path}"),
                        format!("the test ran out of time or steps, more than {}", budget.limits()),
                    )
                });
                continue;
            }
            let result = fhirpath::evaluate_keeping(path, target, json, &mut budget, &mut references);
            selected.extend(result.results);
            if let Some(e) = result.error {
                unlisted.get_or_insert_with(|| (format!("{prefix}{path}"), e));
            }
        }
        if let Some((at, why)) = unlisted {
            issues.push(fhir::FhirValidationIssue {
                severity: "error".into(),
                message: format!(
                    "Not every value the rule's path selects could be listed ({why}), \
                     so the values shown may be incomplete."
                ),
                path: at,
                rule_id: Some(rule.rule_id.clone()),
            });
        }
    }

    FhirRuleTest {
        passed: issues.is_empty(),
        applied,
        issues,
        selected,
    }
}

/// A rule must have an id, a message, exactly one of the two forms, and
/// FHIRPath that parses.
fn check_rule(rule: &FhirRule) -> Result<(), Refusal> {
    let id = rule.rule_id.as_str();
    let refuse = |code: &'static str, message: String| Err(Refusal::new(code, &[("id", id)], message));
    if id.trim().is_empty() {
        return Err(Refusal::new("no_id", &[], "A rule needs an id".into()));
    }
    if rule.message.trim().is_empty() {
        return refuse("no_message", format!("Rule '{}' needs a message", id));
    }
    if !matches!(rule.severity.as_str(), "error" | "warning" | "info") {
        return refuse("bad_severity", format!("Rule '{}': severity must be error, warning or info", id));
    }

    if let Some(crate::plugins::FhirCheck::Regex { pattern }) = &rule.check {
        if let Err(e) = regex::Regex::new(pattern) {
            // The reason ("unclosed group"), not the error's first line,
            // which only says "regex parse error:".
            let full = e.to_string();
            let detail = plugins::regex_reason(&full);
            return Err(Refusal::new(
                "bad_pattern",
                &[("id", id), ("detail", detail)],
                format!("Rule '{}': the pattern does not compile ({})", id, detail),
            ));
        }
    }

    let fhirpath_ok = |expression: &str| {
        fhirpath::check(expression).map_err(|e| {
            Refusal::new("bad_fhirpath", &[("id", id), ("detail", e.as_str())], format!("Rule '{}': {}", id, e))
        })
    };
    match (&rule.expression, &rule.path) {
        (Some(expression), None) => fhirpath_ok(expression),
        (None, Some(path)) => {
            fhirpath_ok(path)?;
            if rule.check.is_none() {
                return refuse("path_needs_check", format!("Rule '{}': a path needs a check to apply to it", id));
            }
            Ok(())
        }
        (Some(_), Some(_)) => refuse(
            "both_forms",
            format!("Rule '{}': use either an expression or a path with a check, not both", id),
        ),
        (None, None) => refuse("no_form", format!("Rule '{}': needs an expression, or a path with a check", id)),
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

    /// A refusal names its reason (the regex's own, not its "regex parse
    /// error:" header) and carries a code and values the dialog translates.
    #[test]
    fn refusals_carry_their_reason_and_a_code() {
        let bad = FhirRule {
            expression: None,
            path: Some("name.family".into()),
            check: Some(FhirCheck::Regex { pattern: "([a-z".into() }),
            ..rule()
        };
        let e = check_rule(&bad).unwrap_err();
        assert_eq!(e.code, "bad_pattern");
        assert_eq!(e.params["id"], "r1");
        assert!(e.params["detail"].contains("unclosed"), "{:?}", e.params);
        assert!(e.message.contains("unclosed") && !e.message.contains("regex parse error"), "{e}");

        let dup = check_rules(&[rule(), rule()]).unwrap_err();
        assert_eq!((dup.code, dup.params["id"].as_str()), ("duplicate_id", "r1"));
        assert_eq!(check_rule(&FhirRule { rule_id: " ".into(), ..rule() }).unwrap_err().code, "no_id");
        assert_eq!(check_rule(&FhirRule { message: String::new(), ..rule() }).unwrap_err().code, "no_message");
        assert_eq!(check_rule(&FhirRule { severity: "fatal".into(), ..rule() }).unwrap_err().code, "bad_severity");
        assert_eq!(check_rule(&FhirRule { expression: Some("name.(".into()), ..rule() }).unwrap_err().code, "bad_fhirpath");
        assert_eq!(check_rule(&FhirRule { expression: None, path: Some("name".into()), ..rule() }).unwrap_err().code, "path_needs_check");
        assert_eq!(check_rule(&FhirRule { path: Some("name".into()), ..rule() }).unwrap_err().code, "both_forms");
        assert_eq!(check_rule(&FhirRule { expression: None, ..rule() }).unwrap_err().code, "no_form");

        // One space between the sentences, not a run of them.
        let changed = changed_on_disk(std::path::Path::new("/x/user-rules.json"));
        assert_eq!(changed.code, "changed_on_disk");
        assert!(changed.message.contains("Nothing was saved: close the builder"), "{changed}");
        assert!(!changed.message.contains("  "), "{changed}");

        let unsaved = merged_pack(Some("{not json"), std::path::Path::new("u.json"), &[rule()]).unwrap_err();
        assert_eq!(unsaved.code, "invalid_pack_unsaved");
        assert!(unsaved.params.contains_key("detail") && unsaved.message.contains("Nothing was saved"));
        // Anything else is passed on as English, code "other".
        assert_eq!(Refusal::from("UPGRADE_REQUIRED:x:pro:".to_string()).code, "other");
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
        // Empty, the pack checks nothing and takes no slot under the cap;
        // once a save gives it a rule (and reloads), the cap applies.
        assert_eq!(pack_status(&reg, &path, Some(0)), None);
        fs::write(
            &path,
            r#"{"id": "user-fhir-rules", "name": "My FHIR rules", "fhir_rules": [{"rule_id": "R1", "expression": "true", "message": "m"}]}"#,
        )
        .unwrap();
        reg.reload().unwrap();
        assert_eq!(pack_status(&reg, &path, Some(0)).as_deref(), Some("gated"));
        fs::remove_dir_all(&root).ok();
    }

    /// The rules builder's Test runs the rule on a blocking-task thread.
    /// It ran on the window's thread, and a slow rule froze the app for as
    /// long as it took; here one async worker stays free meanwhile.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn a_slow_rule_test_leaves_the_other_commands_free() {
        let names: Vec<serde_json::Value> =
            (0..10).map(|i| serde_json::json!({"family": format!("F{i}")})).collect();
        let patient = serde_json::json!({"resourceType": "Patient", "id": "p", "name": names});
        let resource = std::sync::Arc::new(fhir::parse_fhir_json(&patient.to_string()).unwrap());
        let mut nested = "%resource.name".to_string();
        for _ in 0..5 {
            nested = format!("%resource.name.where({nested}.exists())");
        }
        let slow = FhirRule { expression: Some(format!("{nested}.count() > 100")), ..rule() };

        let started = std::time::Instant::now();
        let run = tokio::spawn(test_rule_off_thread(slow, resource));
        let asked = std::time::Instant::now();
        tokio::spawn(async {}).await.unwrap();
        let answered = asked.elapsed();
        let tested = run.await.unwrap().unwrap();
        assert!(tested.applied && !tested.passed, "{:?}", tested.issues);
        let took = started.elapsed();
        assert!(
            answered < std::time::Duration::from_millis(300) && answered * 3 < took,
            "another command waited {answered:?} while the test took {took:?}"
        );
    }

    /// The Test lists the values the path selects on what the rule left of
    /// the same budget. When that runs out part-way the Test fails saying
    /// so, rather than passing while it shows only some of the values.
    #[test]
    fn a_test_that_cannot_list_every_value_does_not_pass() {
        let entry = |id: &str| {
            serde_json::json!({"resource": {"resourceType": "Patient", "id": id, "name": [{"family": id}]}})
        };
        let bundle = serde_json::json!({
            "resourceType": "Bundle", "type": "collection", "entry": [entry("a"), entry("b")]
        });
        let selector = FhirRule {
            expression: None,
            path: Some("name.family".into()),
            check: Some(FhirCheck::NotEmpty),
            ..rule()
        };
        const PLENTY: u64 = 1_000_000;
        let budget = |steps| fhirpath::Budget::with_limits(steps, std::time::Duration::from_secs(60));

        let full = test_rule_within(&selector, &bundle, budget(PLENTY));
        assert!(full.passed && full.selected.len() == 2, "{full:?}");

        // What the rule costs, then what listing the first entry's values does.
        let mut b = budget(PLENTY);
        assert!(plugins::run_fhir_validations_within(&bundle, std::slice::from_ref(&selector), &mut b).is_empty());
        let rule_cost = PLENTY - b.steps_left();
        let mut references = fhirpath::References::default();
        let first = &bundle["entry"][0]["resource"];
        assert!(fhirpath::evaluate_keeping("name.family", first, &bundle, &mut b, &mut references).error.is_none());
        let one_listing = PLENTY - rule_cost - b.steps_left();

        // Spent by the rule to the last step: it holds, but nothing is listed.
        let none = test_rule_within(&selector, &bundle, budget(rule_cost));
        assert!(none.selected.is_empty(), "{none:?}");
        // Spent after the first entry: the second is never listed.
        let skipped = test_rule_within(&selector, &bundle, budget(rule_cost + one_listing));
        // Spent during the first entry: its listing comes back with an error.
        let cut = test_rule_within(&selector, &bundle, budget(rule_cost + one_listing - 1));
        for t in [&none, &skipped, &cut] {
            assert!(t.applied && !t.passed, "{t:?}");
            assert!(t.selected.len() < 2, "{t:?}");
            let [issue] = t.issues.as_slice() else { panic!("{t:?}") };
            assert!(issue.message.contains("Not every value"), "{}", issue.message);
            assert_eq!(issue.severity, "error");
            assert_eq!(issue.rule_id.as_deref(), Some("r1"));
        }
        assert_eq!(skipped.selected.len(), 1, "{skipped:?}");
        assert!(skipped.issues[0].path.starts_with("entry[1]"), "{}", skipped.issues[0].path);
        assert!(cut.issues[0].message.contains("too long"), "{}", cut.issues[0].message);
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
