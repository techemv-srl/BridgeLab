//! Declarative plugin system - MVP (level 1 + 2 of the plugin roadmap).
//!
//! Users can drop `.json` files in:
//!
//!   <config_dir>/BridgeLab/plugins/validation/   - extra validation rules
//!   <config_dir>/BridgeLab/plugins/anonymization/ - extra PHI fields
//!
//! Each file is a [`PluginPack`]. The loader scans both directories at
//! startup (or on explicit `reload`) and the validator / anonymizer
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
    Anonymization,
}

impl PluginKind {
    fn as_str(&self) -> &'static str {
        match self {
            PluginKind::Validation => "validation",
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

    pub fn list(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().ok();
        let mut out = Vec::new();
        if let Some(p) = plugins {
            for lp in p.iter() {
                let rule_count = lp.pack.validation_rules.len() + lp.pack.phi_rules.len();
                out.push(PluginInfo {
                    id: lp.pack.id.clone(),
                    name: lp.pack.name.clone(),
                    description: lp.pack.description.clone(),
                    author: lp.pack.author.clone(),
                    version: lp.pack.version.clone(),
                    enabled: self.is_enabled(&lp.pack) && lp.error.is_none(),
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

    /// Collect enabled validation rules across all packs.
    pub fn active_validation_rules(&self) -> Vec<ValidationRule> {
        let guard = match self.plugins.read() { Ok(g) => g, Err(_) => return vec![] };
        let mut out = Vec::new();
        for lp in guard.iter() {
            if lp.kind != PluginKind::Validation { continue; }
            if lp.error.is_some() { continue; }
            if !self.is_enabled(&lp.pack) { continue; }
            out.extend(lp.pack.validation_rules.clone());
        }
        out
    }

    /// Collect enabled PHI rules across all packs.
    pub fn active_phi_rules(&self) -> Vec<PhiRule> {
        let guard = match self.plugins.read() { Ok(g) => g, Err(_) => return vec![] };
        let mut out = Vec::new();
        for lp in guard.iter() {
            if lp.kind != PluginKind::Anonymization { continue; }
            if lp.error.is_some() { continue; }
            if !self.is_enabled(&lp.pack) { continue; }
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
            phi_rules: vec![],
        };
        assert!(!reg.is_enabled(&pack));
    }
}
