use serde::Serialize;
use tauri::{Manager, State};

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

/// The app state the validations and the package install read: the app's
/// own, or what a test sets up. Moved whole into the blocking-task thread
/// they run on.
pub(crate) trait Registries: Send + Sync + 'static {
    fn store(&self) -> &MessageStore;
    fn plugins(&self) -> &PluginRegistry;
    fn profiles(&self) -> &fhir::profile::ProfileRegistry;
}

impl Registries for tauri::AppHandle {
    fn store(&self) -> &MessageStore {
        self.state::<MessageStore>().inner()
    }
    fn plugins(&self) -> &PluginRegistry {
        self.state::<PluginRegistry>().inner()
    }
    fn profiles(&self) -> &fhir::profile::ProfileRegistry {
        self.state::<fhir::profile::ProfileRegistry>().inner()
    }
}

/// Validate an HL7 message by its store ID.
///
/// Runs built-in validations followed by any active user-defined plugin rules
/// (files in `<config>/BridgeLab/plugins/validation/*.json`).
///
/// On a blocking-task thread: a large message with many plugin rules must
/// not freeze the window while it is checked, nor hold one of the few
/// threads every async command (open, save…) runs on.
#[tauri::command]
pub async fn validate_message(
    message_id: String,
    app: tauri::AppHandle,
    tel: State<'_, crate::licensing::telemetry::UsageCounters>,
) -> Result<ValidationReport, String> {
    tel.bump_mem("validations_hl7");
    validate_message_off_thread(message_id, app).await
}

pub(crate) async fn validate_message_off_thread(
    message_id: String,
    states: impl Registries,
) -> Result<ValidationReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_message_now(&message_id, states.store(), states.plugins())
    })
    .await
    .map_err(|e| format!("Validation failed: {}", e))?
}

fn validate_message_now(
    message_id: &str,
    store: &MessageStore,
    registry: &PluginRegistry,
) -> Result<ValidationReport, String> {
    let msg = store
        .get(message_id)
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
    if let Some(note) = registry.cap_notice(crate::licensing::feature_gate::active_plugin_limit()) {
        report.info_count += 1;
        report.issues.push(note);
    }

    Ok(report)
}

/// Validate a FHIR JSON resource.
///
/// Runs the built-in structural checks followed by any active user-defined
/// FHIR rules (files in `<config>/BridgeLab/plugins/fhir/*.json`).
///
/// On a blocking-task thread, off the main thread and off the async
/// workers: a resource opened at startup waits here for the plugin packs
/// to finish loading, and the rules of a heavy pack run for as long as
/// their budget allows. Run inline, a few presses of Validate held every
/// worker, and open, save and the FHIRPath panel stopped answering.
#[tauri::command]
pub async fn validate_fhir(
    content: String,
    app: tauri::AppHandle,
    tel: State<'_, crate::licensing::telemetry::UsageCounters>,
) -> Result<FhirValidationReport, String> {
    tel.bump_mem("validations_fhir");
    validate_fhir_off_thread(content, app).await
}

pub(crate) async fn validate_fhir_off_thread(
    content: String,
    states: impl Registries,
) -> Result<FhirValidationReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_fhir_now(&content, states.plugins(), states.profiles())
    })
    .await
    .map_err(|e| format!("Validation failed: {}", e))?
}

fn validate_fhir_now(
    content: &str,
    registry: &PluginRegistry,
    profiles: &fhir::profile::ProfileRegistry,
) -> Result<FhirValidationReport, String> {
    // Route by encoding: XML resources go through the XML->JSON converter,
    // then the same rule set runs on both.
    let trimmed = content.trim_start();
    let resource = if trimmed.starts_with('<') {
        fhir::parse_fhir_xml(content)?
    } else {
        fhir::parse_fhir_json(content)?
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
        let (found, applied) = profiles.validate_with_status(json, &resource.format);
        profiles_applied = applied;
        issues.extend(found);
    }
    // A resource that got no conformance findings looks clean; say plainly
    // when that is because nothing was checked. With the R4 core built in
    // this now means a resource type no installed package defines — a
    // typo, or a newer FHIR release than any package on this machine.
    if !profiles_applied {
        issues.push(fhir::FhirValidationIssue {
            severity: "info".into(),
            message: format!(
                "Profile conformance was not checked: no installed package defines the \
                 resource type '{}'. Built-in: FHIR R4 core; add others under \
                 Tools → FHIR profile packages…",
                resource.resource_type
            ),
            path: "resourceType".into(),
            rule_id: None,
        });
    }
    // FHIR packs left out by the cap: fewer findings must not read as a
    // cleaner resource, as the HL7 v2 report already says.
    if let Some(message) = registry.cap_message(
        crate::licensing::feature_gate::active_plugin_limit(),
        plugins::RuleKind::Fhir,
    ) {
        issues.push(fhir::FhirValidationIssue {
            severity: "info".into(),
            message,
            path: "-".into(),
            rule_id: Some(plugins::CAP_RULE_ID.into()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    struct TestStates {
        store: MessageStore,
        plugins: PluginRegistry,
        profiles: fhir::profile::ProfileRegistry,
    }

    impl Registries for Arc<TestStates> {
        fn store(&self) -> &MessageStore {
            &self.store
        }
        fn plugins(&self) -> &PluginRegistry {
            &self.plugins
        }
        fn profiles(&self) -> &fhir::profile::ProfileRegistry {
            &self.profiles
        }
    }

    /// A Patient with ten names, and a pack whose one FHIR rule nests
    /// criteria over them five deep (well within its budget, but about a
    /// second of work in a test build), with one HL7 rule beside it.
    fn slow_setup() -> (Arc<TestStates>, String, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("bl-slow-rules-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("fhir")).unwrap();
        let mut nested = "%resource.name".to_string();
        for _ in 0..5 {
            nested = format!("%resource.name.where({nested}.exists())");
        }
        let pack = serde_json::json!({
            "id": "slow", "name": "Slow",
            "fhir_rules": [{"rule_id": "slow-1", "severity": "warning", "resource": "Patient",
                "expression": format!("{nested}.count() > 100"), "message": "never"}],
            "validation_rules": [{"rule_id": "pid-3", "severity": "error", "segment": "PID", "field": 3,
                "check": {"type": "not_empty"}, "message": "PID-3 is required"}]
        });
        std::fs::write(root.join("fhir").join("slow.json"), pack.to_string()).unwrap();
        let plugins = PluginRegistry::with_root(root.clone());
        plugins.reload_read_only().unwrap();
        let names: Vec<serde_json::Value> =
            (0..10).map(|i| serde_json::json!({"family": format!("F{i}")})).collect();
        let patient = serde_json::json!({"resourceType": "Patient", "id": "p", "name": names}).to_string();
        let states = TestStates {
            store: MessageStore::new(),
            plugins,
            profiles: fhir::profile::ProfileRegistry::new(),
        };
        (Arc::new(states), patient, root)
    }

    /// Validations run off the async workers. On one worker (a four-core
    /// machine has four), three slow validations run inline held it, and
    /// every other async command (open, save, the FHIRPath panel) waited
    /// for all of them.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn slow_rules_leave_the_async_workers_free() {
        let (states, patient, root) = slow_setup();
        let started = Instant::now();
        let runs: Vec<_> = (0..3)
            .map(|_| tokio::spawn(validate_fhir_off_thread(patient.clone(), states.clone())))
            .collect();
        // What any other command needs: a turn on the worker.
        let asked = Instant::now();
        tokio::spawn(async {}).await.unwrap();
        let answered = asked.elapsed();
        for run in runs {
            let report = run.await.unwrap().unwrap();
            assert!(
                report.issues.iter().any(|i| i.rule_id.as_deref() == Some("slow-1")),
                "{:?}",
                report.issues
            );
        }
        let validations = started.elapsed();
        assert!(
            answered < Duration::from_millis(300) && answered * 3 < validations,
            "another command waited {answered:?} while the validations took {validations:?}"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// Only HL7 v2 reports said that packs were left out by the cap: a FHIR
    /// report with fewer findings read as a cleaner resource.
    #[test]
    fn a_fhir_report_notes_fhir_packs_left_out_by_the_cap() {
        let root = std::env::temp_dir().join(format!("bl-fhir-cap-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("fhir")).unwrap();
        for id in ["a", "b", "c", "d"] {
            let pack = serde_json::json!({"id": id, "name": id,
                "fhir_rules": [{"rule_id": format!("{id}-1"), "expression": "true", "message": "m"}]});
            std::fs::write(root.join("fhir").join(format!("{id}.json")), pack.to_string()).unwrap();
        }
        let plugins = PluginRegistry::with_root(root.clone());
        plugins.reload_read_only().unwrap();
        let report =
            validate_fhir_now(r#"{"resourceType":"Patient","id":"p"}"#, &plugins, &fhir::profile::ProfileRegistry::new())
                .unwrap();
        let noted = report.issues.iter().any(|i| i.rule_id.as_deref() == Some(plugins::CAP_RULE_ID));
        // Whatever licence the test process holds: a note exactly when a cap applies.
        assert_eq!(noted, crate::licensing::feature_gate::active_plugin_limit().is_some(), "{:?}", report.issues);
        std::fs::remove_dir_all(&root).ok();
    }

    /// The HL7 v2 validation, plugin rules included, runs on the same kind
    /// of thread and reports the same way.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn hl7_validation_runs_its_plugin_rules_off_the_workers() {
        let (states, _, root) = slow_setup();
        let msg = crate::parser::hl7::lexer::Hl7Lexer::new()
            .parse(b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||\r".to_vec())
            .unwrap();
        states.store.insert("m1".into(), msg);
        let report = validate_message_off_thread("m1".into(), states.clone()).await.unwrap();
        assert!(report.issues.iter().any(|i| i.message.contains("PID-3 is required")), "{:?}", report.issues);
        let missing = validate_message_off_thread("no-such-id".into(), states).await.unwrap_err();
        assert!(missing.contains("not found"), "{missing}");
        std::fs::remove_dir_all(&root).ok();
    }
}
