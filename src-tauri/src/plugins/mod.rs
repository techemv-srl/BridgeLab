//! Declarative plugin system - MVP (level 1 + 2 of the plugin roadmap).
//!
//! Users can drop `.json` files in:
//!
//!   <config_dir>/BridgeLab/plugins/validation/   - extra HL7 v2 rules
//!   <config_dir>/BridgeLab/plugins/fhir/         - extra FHIR rules
//!   <config_dir>/BridgeLab/plugins/anonymization/ - extra PHI fields
//!
//! Each file is a [`PluginPack`]. The loader scans all three directories at
//! startup (or on explicit `reload`) and the validators / anonymizer
//! consume whatever is enabled.
//!
//! No code execution - plugins are pure data. JS / WASM plugins are a
//! separate future layer.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::anonymization::PhiSensitivity;
use crate::parser::hl7::message::Hl7Message;
use crate::validation::{Severity, ValidationIssue};

/// Container shared across both plugin kinds. Each file on disk represents
/// exactly one `PluginPack`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPack {
    /// Stable identifier (also used to scope `rule_id`s).
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Validation rules (only present in validation/*.json).
    #[serde(default)]
    pub validation_rules: Vec<ValidationRule>,

    /// FHIR validation rules (only present in fhir/*.json).
    #[serde(default)]
    pub fhir_rules: Vec<FhirRule>,

    /// PHI entries (only present in anonymization/*.json).
    #[serde(default)]
    pub phi_rules: Vec<PhiRule>,
}

fn default_version() -> String { "1.0".into() }
fn default_enabled() -> bool { true }

/// A single user-defined validation rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    /// Rule identifier, displayed in the validation panel.
    pub rule_id: String,
    /// Severity of the issue produced on failure.
    #[serde(default = "default_severity")]
    pub severity: String, // "error" | "warning" | "info"
    /// HL7 segment type the rule applies to (e.g. "PID").
    pub segment: String,
    /// HL7 field position (1-based).
    pub field: usize,
    /// Optional component index (1-based, ^ separated).
    #[serde(default)]
    pub component: Option<usize>,
    /// The check to apply.
    pub check: CheckKind,
    /// Human-readable message emitted when the rule fires.
    pub message: String,
}

fn default_severity() -> String { "warning".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CheckKind {
    /// Field (or component) must not be empty.
    NotEmpty,
    /// Field must match the given regular expression.
    Regex { pattern: String },
    /// Field length must be <= max (bytes).
    MaxLength { max: usize },
    /// Field length must be >= min (bytes).
    MinLength { min: usize },
    /// Field must be one of the given values.
    OneOf { values: Vec<String> },
    /// Field must contain the given substring.
    Contains { value: String },
}

/// A user-defined FHIR validation rule.
///
/// Two shapes, matching the two ways people actually express these:
///
/// - an **invariant**, a single FHIRPath expression that must be true, the
///   way FHIR's own constraints are written:
///   `{ "expression": "identifier.exists()" }`
/// - a **selector plus check**, which reads better for field-level rules and
///   is what the in-app builder produces:
///   `{ "path": "telecom.where(system = 'phone').value",
///      "check": { "type": "regex", "pattern": "..." } }`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirRule {
    pub rule_id: String,
    #[serde(default = "default_severity")]
    pub severity: String,
    /// Resource type the rule applies to, e.g. "Patient". Omit to apply it
    /// to every resource.
    #[serde(default)]
    pub resource: Option<String>,
    /// Invariant form: a FHIRPath expression that must evaluate to true.
    #[serde(default)]
    pub expression: Option<String>,
    /// Selector form: a FHIRPath expression picking the values to check.
    #[serde(default)]
    pub path: Option<String>,
    /// The check applied to each selected value. Required with `path`.
    #[serde(default)]
    pub check: Option<FhirCheck>,
    pub message: String,
}

/// Checks a [`FhirRule`] can apply to the values its `path` selected.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FhirCheck {
    /// At least one value, and none of them blank.
    NotEmpty,
    /// Every value matches the regular expression.
    Regex { pattern: String },
    /// Every value is at most `max` characters.
    MaxLength { max: usize },
    /// Every value is at least `min` characters.
    MinLength { min: usize },
    /// Every value is one of the listed ones.
    OneOf { values: Vec<String> },
    /// Every value contains the substring.
    Contains { value: String },
    /// The number of selected values falls in the range — cardinality,
    /// without needing a StructureDefinition.
    Cardinality {
        #[serde(default)]
        min: Option<usize>,
        #[serde(default)]
        max: Option<usize>,
    },
}

/// An extra PHI field contributed by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhiRule {
    pub segment: String,
    pub field: usize,
    #[serde(default)]
    pub name: String,
    pub sensitivity: String, // "high" | "medium" | "low"
}

/// Where plugins live on disk.
pub fn plugins_root() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("BridgeLab").join("plugins"))
}

/// Create the plugins directory tree if missing. Called lazily by the loader.
fn ensure_plugins_dirs(root: &Path) -> std::io::Result<()> {
    fs::create_dir_all(root.join("validation"))?;
    fs::create_dir_all(root.join("fhir"))?;
    fs::create_dir_all(root.join("anonymization"))?;
    Ok(())
}

/// Metadata returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub enabled: bool,
    /// True when the pack is enabled but inactive because the Community cap
    /// on active packs is exceeded (upgrade required to activate it).
    pub gated: bool,
    pub kind: String, // "validation" | "anonymization"
    pub path: String,
    pub rule_count: usize,
    /// Set if the file failed to parse; other fields are best-effort placeholders.
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub pack: PluginPack,
    pub kind: PluginKind,
    pub path: PathBuf,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PluginKind {
    Validation,
    Fhir,
    Anonymization,
}

impl PluginKind {
    fn as_str(&self) -> &'static str {
        match self {
            PluginKind::Validation => "validation",
            PluginKind::Fhir => "fhir",
            PluginKind::Anonymization => "anonymization",
        }
    }
}

/// Global plugin registry (refreshed on `reload`).
pub struct PluginRegistry {
    plugins: RwLock<Vec<LoadedPlugin>>,
    /// User-overridden enable flags by plugin id, stored in preferences table
    /// (key: "plugin_enabled:<id>"). Populated eagerly by the frontend.
    overrides: RwLock<HashMap<String, bool>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(Vec::new()),
            overrides: RwLock::new(HashMap::new()),
        }
    }

    /// Replace the in-memory plugins from disk. Never errors hard -
    /// individual files that fail to parse are surfaced in `PluginInfo.error`
    /// instead of breaking the whole registry.
    pub fn reload(&self) -> Result<usize, String> {
        let root = match plugins_root() {
            Some(p) => p,
            None => return Err("Could not determine config directory".into()),
        };
        let _ = ensure_plugins_dirs(&root);

        let mut loaded = Vec::new();
        for (kind, sub) in [
            (PluginKind::Validation, "validation"),
            (PluginKind::Fhir, "fhir"),
            (PluginKind::Anonymization, "anonymization"),
        ] {
            let dir = root.join(sub);
            if !dir.is_dir() { continue; }
            let entries = match fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") { continue; }
                match fs::read_to_string(&path) {
                    Ok(text) => match serde_json::from_str::<PluginPack>(&text) {
                        Ok(pack) => loaded.push(LoadedPlugin {
                            pack, kind, path, error: None,
                        }),
                        Err(e) => loaded.push(LoadedPlugin {
                            pack: PluginPack {
                                id: path.file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("unknown").to_string(),
                                name: "(parse error)".into(),
                                description: String::new(),
                                author: String::new(),
                                version: "0".into(),
                                enabled: false,
                                validation_rules: vec![],
                                fhir_rules: vec![],
                                phi_rules: vec![],
                            },
                            kind,
                            path,
                            error: Some(format!("{}", e)),
                        }),
                    },
                    Err(e) => loaded.push(LoadedPlugin {
                        pack: PluginPack {
                            id: path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown").to_string(),
                            name: "(read error)".into(),
                            description: String::new(),
                            author: String::new(),
                            version: "0".into(),
                            enabled: false,
                            validation_rules: vec![],
                            fhir_rules: vec![],
                            phi_rules: vec![],
                        },
                        kind,
                        path,
                        error: Some(format!("{}", e)),
                    }),
                }
            }
        }

        let count = loaded.len();
        *self.plugins.write().map_err(|e| e.to_string())? = loaded;
        Ok(count)
    }

    pub fn set_overrides(&self, map: HashMap<String, bool>) {
        if let Ok(mut w) = self.overrides.write() { *w = map; }
    }

    pub fn set_override(&self, id: &str, enabled: bool) {
        if let Ok(mut w) = self.overrides.write() {
            w.insert(id.to_string(), enabled);
        }
    }

    fn is_enabled(&self, pack: &PluginPack) -> bool {
        if let Ok(o) = self.overrides.read() {
            if let Some(v) = o.get(&pack.id) { return *v; }
        }
        pack.enabled
    }

    /// Ids of the packs that actually contribute rules under the given cap:
    /// the first `limit` enabled, error-free packs sorted by id (deterministic
    /// regardless of filesystem scan order). `None` = no cap.
    fn active_ids(&self, limit: Option<usize>) -> Vec<String> {
        let guard = match self.plugins.read() { Ok(g) => g, Err(_) => return vec![] };
        let mut ids: Vec<String> = guard
            .iter()
            .filter(|lp| lp.error.is_none() && self.is_enabled(&lp.pack))
            .map(|lp| lp.pack.id.clone())
            .collect();
        ids.sort();
        if let Some(max) = limit {
            ids.truncate(max);
        }
        ids
    }

    /// Count of enabled, error-free packs (regardless of any cap).
    pub fn enabled_count(&self) -> usize {
        self.active_ids(None).len()
    }

    pub fn list(&self, limit: Option<usize>) -> Vec<PluginInfo> {
        let active = self.active_ids(limit);
        let plugins = self.plugins.read().ok();
        let mut out = Vec::new();
        if let Some(p) = plugins {
            for lp in p.iter() {
                let rule_count = lp.pack.validation_rules.len()
                    + lp.pack.fhir_rules.len()
                    + lp.pack.phi_rules.len();
                let enabled = self.is_enabled(&lp.pack) && lp.error.is_none();
                out.push(PluginInfo {
                    id: lp.pack.id.clone(),
                    name: lp.pack.name.clone(),
                    description: lp.pack.description.clone(),
                    author: lp.pack.author.clone(),
                    version: lp.pack.version.clone(),
                    enabled,
                    gated: enabled && !active.contains(&lp.pack.id),
                    kind: lp.kind.as_str().to_string(),
                    path: lp.path.display().to_string(),
                    rule_count,
                    error: lp.error.clone(),
                });
            }
        }
        out
    }

    pub fn plugins_root_path(&self) -> Option<PathBuf> { plugins_root() }

    /// Collect validation rules from the packs active under the given cap.
    pub fn active_validation_rules(&self, limit: Option<usize>) -> Vec<ValidationRule> {
        let active = self.active_ids(limit);
        let guard = match self.plugins.read() { Ok(g) => g, Err(_) => return vec![] };
        let mut out = Vec::new();
        for lp in guard.iter() {
            if lp.kind != PluginKind::Validation { continue; }
            if !active.contains(&lp.pack.id) { continue; }
            out.extend(lp.pack.validation_rules.clone());
        }
        out
    }

    /// Collect FHIR rules from the packs active under the given cap.
    pub fn active_fhir_rules(&self, limit: Option<usize>) -> Vec<FhirRule> {
        let active = self.active_ids(limit);
        let guard = match self.plugins.read() { Ok(g) => g, Err(_) => return vec![] };
        let mut out = Vec::new();
        for lp in guard.iter() {
            if lp.kind != PluginKind::Fhir { continue; }
            if !active.contains(&lp.pack.id) { continue; }
            out.extend(lp.pack.fhir_rules.clone());
        }
        out
    }

    /// Collect PHI rules from the packs active under the given cap.
    pub fn active_phi_rules(&self, limit: Option<usize>) -> Vec<PhiRule> {
        let active = self.active_ids(limit);
        let guard = match self.plugins.read() { Ok(g) => g, Err(_) => return vec![] };
        let mut out = Vec::new();
        for lp in guard.iter() {
            if lp.kind != PluginKind::Anonymization { continue; }
            if !active.contains(&lp.pack.id) { continue; }
            out.extend(lp.pack.phi_rules.clone());
        }
        out
    }
}

impl Default for PluginRegistry {
    fn default() -> Self { Self::new() }
}

/// Run all active plugin validation rules against `msg` and return the
/// emitted issues.
pub fn run_custom_validations(
    msg: &Hl7Message,
    rules: &[ValidationRule],
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    for rule in rules {
        for (seg_idx, seg) in msg.segments.iter().enumerate() {
            if seg.segment_type != rule.segment { continue; }

            let field_opt = seg.fields.iter().find(|f| f.position == rule.field);
            let raw_value = match field_opt {
                Some(f) => f.span.as_str(&msg.raw),
                None => "",
            };

            let value: String = if let Some(c) = rule.component {
                raw_value.split('^').nth(c.saturating_sub(1)).unwrap_or("").to_string()
            } else {
                raw_value.to_string()
            };

            let passed = apply_check(&rule.check, &value);
            if !passed {
                issues.push(ValidationIssue {
                    severity: parse_severity(&rule.severity),
                    message: rule.message.clone(),
                    segment_idx: Some(seg_idx),
                    segment_type: Some(seg.segment_type.clone()),
                    field_position: Some(rule.field),
                    rule_id: rule.rule_id.clone(),
                });
            }
        }
    }
    issues
}

/// Run the active FHIR rules against a parsed resource.
///
/// A Bundle is walked entry by entry as well as checked itself, so a rule
/// scoped to `Patient` fires for the Patients inside a transaction Bundle —
/// which is where they usually live.
pub fn run_fhir_validations(
    root: &serde_json::Value,
    rules: &[FhirRule],
) -> Vec<crate::parser::fhir::FhirValidationIssue> {
    let mut issues = Vec::new();
    for (target, prefix) in fhir_targets(root) {
        for rule in rules {
            if let Some(required) = &rule.resource {
                if target.get("resourceType").and_then(|v| v.as_str()) != Some(required.as_str()) {
                    continue;
                }
            }
            if let Some(issue) = apply_fhir_rule(rule, target, &prefix) {
                issues.push(issue);
            }
        }
    }
    issues
}

/// The resource itself plus, for a Bundle, every entry resource — each with
/// the path prefix to report findings against.
fn fhir_targets(root: &serde_json::Value) -> Vec<(&serde_json::Value, String)> {
    let mut out = vec![(root, String::new())];
    if root.get("resourceType").and_then(|v| v.as_str()) == Some("Bundle") {
        if let Some(entries) = root.get("entry").and_then(|e| e.as_array()) {
            for (i, entry) in entries.iter().enumerate() {
                if let Some(resource) = entry.get("resource") {
                    out.push((resource, format!("entry[{}].resource.", i)));
                }
            }
        }
    }
    out
}

fn apply_fhir_rule(
    rule: &FhirRule,
    target: &serde_json::Value,
    prefix: &str,
) -> Option<crate::parser::fhir::FhirValidationIssue> {
    use crate::parser::fhir::fhirpath;

    let fail = |detail: Option<String>, path: String| {
        Some(crate::parser::fhir::FhirValidationIssue {
            severity: rule.severity.clone(),
            message: match detail {
                Some(d) => format!("{} ({})", rule.message, d),
                None => rule.message.clone(),
            },
            path,
        })
    };

    // Invariant form: the expression must come back true.
    if let Some(expression) = &rule.expression {
        let result = fhirpath::evaluate(expression, target);
        if let Some(e) = result.error {
            // A broken rule is worth reporting: silently passing would hide
            // the fact that the check never ran.
            return fail(
                Some(format!("rule {} failed to evaluate: {}", rule.rule_id, e)),
                format!("{}{}", prefix, expression),
            );
        }
        let holds = matches!(result.results.as_slice(), [serde_json::Value::Bool(true)]);
        return if holds {
            None
        } else {
            fail(None, format!("{}{}", prefix, expression))
        };
    }

    // Selector form: evaluate the path, then apply the check to the result.
    let path = rule.path.as_deref()?;
    let check = rule.check.as_ref()?;
    let result = fhirpath::evaluate(path, target);
    if let Some(e) = result.error {
        return fail(
            Some(format!("rule {} failed to evaluate: {}", rule.rule_id, e)),
            format!("{}{}", prefix, path),
        );
    }

    let values: Vec<String> = result
        .results
        .iter()
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        })
        .collect();

    let failure = apply_fhir_check(check, &values);
    match failure {
        None => None,
        Some(detail) => fail(detail, format!("{}{}", prefix, path)),
    }
}

/// `None` when the check passes; `Some(detail)` when it fails, where the
/// detail names the offending value if there is a single obvious one.
fn apply_fhir_check(check: &FhirCheck, values: &[String]) -> Option<Option<String>> {
    let first_bad = |mut failing: Vec<&String>| -> Option<String> {
        match failing.len() {
            0 => None,
            1 => Some(format!("got '{}'", failing.remove(0))),
            n => Some(format!("{} values do not match", n)),
        }
    };

    match check {
        FhirCheck::Cardinality { min, max } => {
            let n = values.len();
            if min.is_some_and(|m| n < m) || max.is_some_and(|m| n > m) {
                let expected = match (min, max) {
                    (Some(a), Some(b)) if a == b => format!("exactly {}", a),
                    (Some(a), Some(b)) => format!("between {} and {}", a, b),
                    (Some(a), None) => format!("at least {}", a),
                    (None, Some(b)) => format!("at most {}", b),
                    (None, None) => return None,
                };
                return Some(Some(format!("expected {}, found {}", expected, n)));
            }
            None
        }
        FhirCheck::NotEmpty => {
            if values.is_empty() {
                return Some(Some("no value present".into()));
            }
            let bad: Vec<&String> = values.iter().filter(|v| v.trim().is_empty()).collect();
            (!bad.is_empty()).then(|| Some("value is blank".into()))
        }
        // Every other check is vacuously satisfied by nothing to check; a
        // rule that also wants the value present pairs it with cardinality.
        FhirCheck::Regex { pattern } => match regex::Regex::new(pattern) {
            Err(e) => Some(Some(format!("invalid pattern: {}", e))),
            Ok(re) => {
                let bad: Vec<&String> = values.iter().filter(|v| !re.is_match(v)).collect();
                (!bad.is_empty()).then(|| first_bad(bad))
            }
        },
        FhirCheck::MaxLength { max } => {
            let bad: Vec<&String> = values.iter().filter(|v| v.chars().count() > *max).collect();
            (!bad.is_empty()).then(|| first_bad(bad))
        }
        FhirCheck::MinLength { min } => {
            let bad: Vec<&String> = values.iter().filter(|v| v.chars().count() < *min).collect();
            (!bad.is_empty()).then(|| first_bad(bad))
        }
        FhirCheck::OneOf { values: allowed } => {
            let bad: Vec<&String> = values.iter().filter(|v| !allowed.contains(v)).collect();
            (!bad.is_empty()).then(|| first_bad(bad))
        }
        FhirCheck::Contains { value: needle } => {
            let bad: Vec<&String> = values.iter().filter(|v| !v.contains(needle)).collect();
            (!bad.is_empty()).then(|| first_bad(bad))
        }
    }
}

fn apply_check(check: &CheckKind, value: &str) -> bool {
    match check {
        CheckKind::NotEmpty => !value.trim().is_empty(),
        CheckKind::Regex { pattern } => {
            match regex::Regex::new(pattern) {
                Ok(re) => re.is_match(value),
                Err(_) => true, // bad regex = skip rule silently
            }
        }
        CheckKind::MaxLength { max } => value.len() <= *max,
        CheckKind::MinLength { min } => value.len() >= *min,
        CheckKind::OneOf { values } => values.iter().any(|v| v == value),
        CheckKind::Contains { value: needle } => value.contains(needle.as_str()),
    }
}

fn parse_severity(s: &str) -> Severity {
    match s.to_ascii_lowercase().as_str() {
        "error" => Severity::Error,
        "info"  => Severity::Info,
        _       => Severity::Warning,
    }
}

/// Convert a PHI rule's sensitivity string into the enum used by the anonymizer.
pub fn parse_sensitivity(s: &str) -> PhiSensitivity {
    match s.to_ascii_lowercase().as_str() {
        "high"   => PhiSensitivity::High,
        "low"    => PhiSensitivity::Low,
        _        => PhiSensitivity::Medium,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::hl7::lexer::Hl7Lexer;

    fn sample_msg() -> Hl7Message {
        let raw = b"MSH|^~\\&|A|B|C|D|20260415120000||ADT^A01|CTRL1|P|2.5\rPID|1||MRN001||DOE^JOHN\rPV1|1|I\r".to_vec();
        Hl7Lexer::new().parse(raw).unwrap()
    }

    #[test]
    fn not_empty_check_fires_on_missing_field() {
        let msg = sample_msg();
        let rule = ValidationRule {
            rule_id: "TEST-01".into(),
            severity: "error".into(),
            segment: "PID".into(),
            field: 99, // not present
            component: None,
            check: CheckKind::NotEmpty,
            message: "required".into(),
        };
        let issues = run_custom_validations(&msg, &[rule]);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].rule_id, "TEST-01");
        assert_eq!(issues[0].severity, Severity::Error);
    }

    #[test]
    fn not_empty_check_passes_when_populated() {
        let msg = sample_msg();
        let rule = ValidationRule {
            rule_id: "TEST-02".into(),
            severity: "warning".into(),
            segment: "PID".into(),
            field: 3,
            component: None,
            check: CheckKind::NotEmpty,
            message: "x".into(),
        };
        assert!(run_custom_validations(&msg, &[rule]).is_empty());
    }

    #[test]
    fn regex_check_on_component() {
        let msg = sample_msg();
        // PID-5 = "DOE^JOHN" - component 1 = "DOE" (all upper)
        let rule_ok = ValidationRule {
            rule_id: "TEST-REGEX-OK".into(),
            severity: "warning".into(),
            segment: "PID".into(),
            field: 5,
            component: Some(1),
            check: CheckKind::Regex { pattern: "^[A-Z]+$".into() },
            message: "x".into(),
        };
        let rule_fail = ValidationRule {
            rule_id: "TEST-REGEX-FAIL".into(),
            severity: "warning".into(),
            segment: "PID".into(),
            field: 5,
            component: Some(1),
            check: CheckKind::Regex { pattern: "^[0-9]+$".into() },
            message: "x".into(),
        };
        let issues = run_custom_validations(&msg, &[rule_ok, rule_fail]);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].rule_id, "TEST-REGEX-FAIL");
    }

    #[test]
    fn one_of_and_contains() {
        let msg = sample_msg();
        // PV1-2 = "I"
        let one_of = ValidationRule {
            rule_id: "PV1-CLASS".into(),
            severity: "error".into(),
            segment: "PV1".into(),
            field: 2,
            component: None,
            check: CheckKind::OneOf { values: vec!["O".into(), "E".into()] },
            message: "class must be O or E".into(),
        };
        let issues = run_custom_validations(&msg, &[one_of]);
        assert_eq!(issues.len(), 1);

        let contains = ValidationRule {
            rule_id: "PID-CONT".into(),
            severity: "info".into(),
            segment: "PID".into(),
            field: 5,
            component: None,
            check: CheckKind::Contains { value: "DOE".into() },
            message: "x".into(),
        };
        assert!(run_custom_validations(&msg, &[contains]).is_empty());
    }

    // ---- FHIR rules ---------------------------------------------------

    fn fhir_patient() -> serde_json::Value {
        serde_json::json!({
            "resourceType": "Patient",
            "id": "p1",
            "gender": "female",
            "telecom": [
                {"system": "phone", "value": "555-1234"},
                {"system": "email", "value": "jane@example.com"}
            ]
        })
    }

    fn fhir_rule(id: &str) -> FhirRule {
        FhirRule {
            rule_id: id.into(),
            severity: "error".into(),
            resource: Some("Patient".into()),
            expression: None,
            path: None,
            check: None,
            message: format!("{} failed", id),
        }
    }

    #[test]
    fn invariant_rule_passes_and_fails() {
        let holds = FhirRule {
            expression: Some("telecom.exists()".into()),
            ..fhir_rule("has-telecom")
        };
        assert!(run_fhir_validations(&fhir_patient(), &[holds]).is_empty());

        let broken = FhirRule {
            expression: Some("identifier.exists()".into()),
            ..fhir_rule("has-identifier")
        };
        let issues = run_fhir_validations(&fhir_patient(), &[broken]);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, "error");
        assert_eq!(issues[0].path, "identifier.exists()");
    }

    #[test]
    fn a_rule_scoped_to_another_resource_does_not_fire() {
        let rule = FhirRule {
            resource: Some("Observation".into()),
            expression: Some("false".into()),
            ..fhir_rule("never-applies")
        };
        assert!(run_fhir_validations(&fhir_patient(), &[rule]).is_empty());
    }

    #[test]
    fn selector_rules_apply_their_check_to_every_value() {
        let rule = FhirRule {
            path: Some("telecom.where(system = 'phone').value".into()),
            check: Some(FhirCheck::Regex {
                pattern: r"^\d{3}-\d{4}$".into(),
            }),
            ..fhir_rule("phone-format")
        };
        assert!(run_fhir_validations(&fhir_patient(), std::slice::from_ref(&rule)).is_empty());

        let strict = FhirRule {
            check: Some(FhirCheck::Regex {
                pattern: r"^\+\d+$".into(),
            }),
            ..rule
        };
        let issues = run_fhir_validations(&fhir_patient(), &[strict]);
        assert_eq!(issues.len(), 1);
        // The failing value is named, so the message is actionable.
        assert!(issues[0].message.contains("555-1234"), "{}", issues[0].message);
    }

    #[test]
    fn cardinality_counts_the_selected_values() {
        let at_least_three = FhirRule {
            path: Some("telecom".into()),
            check: Some(FhirCheck::Cardinality { min: Some(3), max: None }),
            ..fhir_rule("three-contacts")
        };
        let issues = run_fhir_validations(&fhir_patient(), &[at_least_three]);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("at least 3, found 2"));

        let at_most_five = FhirRule {
            path: Some("telecom".into()),
            check: Some(FhirCheck::Cardinality { min: None, max: Some(5) }),
            ..fhir_rule("five-contacts")
        };
        assert!(run_fhir_validations(&fhir_patient(), &[at_most_five]).is_empty());
    }

    #[test]
    fn rules_reach_the_resources_inside_a_bundle() {
        let bundle = serde_json::json!({
            "resourceType": "Bundle",
            "type": "transaction",
            "entry": [
                {"resource": {"resourceType": "Patient", "id": "ok", "identifier": [{"value": "1"}]}},
                {"resource": {"resourceType": "Patient", "id": "bad"}}
            ]
        });
        let rule = FhirRule {
            expression: Some("identifier.exists()".into()),
            ..fhir_rule("has-identifier")
        };
        let issues = run_fhir_validations(&bundle, &[rule]);
        assert_eq!(issues.len(), 1, "only the entry without an identifier");
        assert!(issues[0].path.starts_with("entry[1].resource."), "{}", issues[0].path);
    }

    #[test]
    fn a_rule_whose_expression_breaks_is_reported_not_silently_passed() {
        let rule = FhirRule {
            expression: Some("nosuchfunction()".into()),
            ..fhir_rule("broken")
        };
        let issues = run_fhir_validations(&fhir_patient(), &[rule]);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("failed to evaluate"), "{}", issues[0].message);
    }

    #[test]
    fn an_invariant_that_yields_no_boolean_counts_as_unsatisfied() {
        // `gender` is a string, not a condition: the rule cannot be said to
        // hold, so it fires rather than passing by accident.
        let rule = FhirRule {
            expression: Some("gender".into()),
            ..fhir_rule("not-a-boolean")
        };
        assert_eq!(run_fhir_validations(&fhir_patient(), &[rule]).len(), 1);
    }

    #[test]
    fn registry_override_disables_pack() {
        let reg = PluginRegistry::new();
        reg.set_override("mine", false);
        let pack = PluginPack {
            id: "mine".into(),
            name: "x".into(),
            description: String::new(),
            author: String::new(),
            version: "1".into(),
            enabled: true,
            validation_rules: vec![],
            fhir_rules: vec![],
            phi_rules: vec![],
        };
        assert!(!reg.is_enabled(&pack));
    }

    fn make_registry(ids: &[&str]) -> PluginRegistry {
        let reg = PluginRegistry::new();
        let loaded: Vec<LoadedPlugin> = ids.iter().map(|id| LoadedPlugin {
            pack: PluginPack {
                id: id.to_string(),
                name: id.to_string(),
                description: String::new(),
                author: String::new(),
                version: "1".into(),
                enabled: true,
                validation_rules: vec![ValidationRule {
                    rule_id: format!("{id}-01"),
                    severity: "error".into(),
                    segment: "PID".into(),
                    field: 3,
                    component: None,
                    check: CheckKind::NotEmpty,
                    message: "x".into(),
                }],
                fhir_rules: vec![],
                phi_rules: vec![],
            },
            kind: PluginKind::Validation,
            path: PathBuf::from(format!("{id}.json")),
            error: None,
        }).collect();
        *reg.plugins.write().unwrap() = loaded;
        reg
    }

    #[test]
    fn plugin_cap_limits_active_rules() {
        let reg = make_registry(&["a", "b", "c", "d", "e"]);
        assert_eq!(reg.active_validation_rules(None).len(), 5);
        assert_eq!(reg.active_validation_rules(Some(3)).len(), 3);
        // Deterministic: first 3 by id win
        let rules = reg.active_validation_rules(Some(3));
        let ids: Vec<&str> = rules.iter().map(|r| &r.rule_id[..1]).collect();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    #[test]
    fn plugin_cap_marks_excess_packs_gated() {
        let reg = make_registry(&["a", "b", "c", "d"]);
        let list = reg.list(Some(3));
        let gated: Vec<&str> = list.iter().filter(|p| p.gated).map(|p| p.id.as_str()).collect();
        assert_eq!(gated, vec!["d"]);
        // All 4 still report enabled (user intent) — only "d" is inactive
        assert!(list.iter().all(|p| p.enabled));
    }

    #[test]
    fn disabling_a_pack_frees_a_cap_slot() {
        let reg = make_registry(&["a", "b", "c", "d"]);
        reg.set_override("a", false);
        let list = reg.list(Some(3));
        assert!(!list.iter().any(|p| p.gated), "d should now fit under the cap");
        assert_eq!(reg.enabled_count(), 3);
    }

    #[test]
    fn no_cap_without_limit() {
        let reg = make_registry(&["a", "b", "c", "d", "e"]);
        let list = reg.list(None);
        assert!(!list.iter().any(|p| p.gated));
    }
}
