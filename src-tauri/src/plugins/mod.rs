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

    /// Keys BridgeLab does not use (`$schema`, an owner, a note), kept so
    /// that a pack rewritten by the rules builder still carries them.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
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
    /// Optional component index (1-based, split on the message's component
    /// separator).
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
    /// Field length must be <= max (characters).
    MaxLength { max: usize },
    /// Field length must be >= min (characters).
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
    /// Keys BridgeLab does not use (a comment, a ticket reference), kept
    /// when the rules builder saves the pack.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
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

/// Largest pack file the loader reads. Real packs are a few KB; the cap
/// keeps a stray export from slowing startup.
const MAX_PACK_BYTES: u64 = 4 * 1024 * 1024;

/// File in the plugins root that remembers which packs are switched on or
/// off, and the order packs first appeared. The app and the CLI both read
/// it, so a pack switched off in Settings is off for the CLI as well.
const STATE_FILE: &str = ".state.json";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct RegistryState {
    /// On/off choices by pack key (`<kind>/<id>`).
    #[serde(default)]
    enabled: std::collections::BTreeMap<String, bool>,
    /// Pack keys in the order they first appeared. Under the Community cap
    /// the earliest packs stay active, so dropping in a new file never
    /// switches off one already in use.
    #[serde(default)]
    seen: Vec<String>,
}

fn read_state(root: &Path) -> RegistryState {
    fs::read_to_string(root.join(STATE_FILE))
        .ok()
        .and_then(|t| serde_json::from_str(strip_bom(&t)).ok())
        .unwrap_or_default()
}

fn write_state(root: &Path, state: &RegistryState) -> Result<(), String> {
    let text = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    crate::utils::atomic_write::write_atomic(root.join(STATE_FILE), text.as_bytes())
        .map_err(|e| format!("Could not save the plugin settings: {}", e))
}

/// Text without a leading UTF-8 byte-order mark, which Windows editors
/// like to add and JSON parsers reject.
pub fn strip_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

/// Metadata returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    /// `<kind>/<id>`: what the on/off switch refers to. Packs in different
    /// folders may share an id and are still switched separately.
    pub key: String,
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub enabled: bool,
    /// True when the pack is enabled but inactive because the Community cap
    /// on active packs is exceeded (upgrade required to activate it).
    pub gated: bool,
    pub kind: String, // "validation" | "fhir" | "anonymization"
    pub path: String,
    pub rule_count: usize,
    /// Set if the file failed to load; other fields are best-effort placeholders.
    pub error: Option<String>,
    /// Problems that do not stop the pack loading: a misspelt severity, a
    /// pattern that does not compile, a PHI field that cannot be masked.
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub pack: PluginPack,
    pub kind: PluginKind,
    pub path: PathBuf,
    pub error: Option<String>,
    pub warnings: Vec<String>,
}

impl LoadedPlugin {
    pub fn key(&self) -> String {
        format!("{}/{}", self.kind.as_str(), self.pack.id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PluginKind {
    Validation,
    Fhir,
    Anonymization,
}

impl PluginKind {
    pub fn as_str(&self) -> &'static str {
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
    state: RwLock<RegistryState>,
    /// Choices saved before 1.9.0 as `plugin_enabled:<id>` preferences,
    /// keyed by bare id. Used only for packs with no choice in the state
    /// file, and copied into it on first use.
    legacy: RwLock<HashMap<String, bool>>,
    /// Plugins directory; `None` = the standard one under the config dir.
    root: Option<PathBuf>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::with_root_opt(None)
    }

    /// A registry over another plugins directory (tests, tools).
    pub fn with_root(root: PathBuf) -> Self {
        Self::with_root_opt(Some(root))
    }

    fn with_root_opt(root: Option<PathBuf>) -> Self {
        Self {
            plugins: RwLock::new(Vec::new()),
            state: RwLock::new(RegistryState::default()),
            legacy: RwLock::new(HashMap::new()),
            root,
        }
    }

    fn root(&self) -> Option<PathBuf> {
        self.root.clone().or_else(plugins_root)
    }

    /// Replace the in-memory plugins from disk. Never errors hard -
    /// individual files that fail to load are surfaced in `PluginInfo.error`
    /// instead of breaking the whole registry.
    pub fn reload(&self) -> Result<usize, String> {
        self.load(true)
    }

    /// [`reload`](Self::reload) that writes nothing: no plugins folders are
    /// created and the first-seen order is not recorded (it decides which
    /// packs the Community cap keeps, and belongs to the app). The CLI
    /// loads packs this way.
    pub fn reload_read_only(&self) -> Result<usize, String> {
        self.load(false)
    }

    fn load(&self, write: bool) -> Result<usize, String> {
        let root = self.root().ok_or("Could not determine config directory")?;
        if write {
            let _ = ensure_plugins_dirs(&root);
        }

        let mut loaded: Vec<LoadedPlugin> = Vec::new();
        for (kind, sub) in [
            (PluginKind::Validation, "validation"),
            (PluginKind::Fhir, "fhir"),
            (PluginKind::Anonymization, "anonymization"),
        ] {
            let Ok(entries) = fs::read_dir(root.join(sub)) else { continue };
            // Sorted, so the order rules run in, and which of two clashing
            // packs loads, is the same on every file system.
            let mut paths: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|s| s.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("json")))
                .collect();
            paths.sort();
            for path in paths {
                let mut lp = load_pack(&path, kind);
                if lp.error.is_none() {
                    if let Some(first) = loaded
                        .iter()
                        .find(|o| o.error.is_none() && o.kind == kind && o.pack.id == lp.pack.id)
                    {
                        lp.error = Some(format!(
                            "another pack in {}/ already uses the id \"{}\" ({}); give each pack its own id",
                            sub,
                            lp.pack.id,
                            first.path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                        ));
                    }
                }
                loaded.push(lp);
            }
        }

        // Remember new packs after the known ones (a first run orders them
        // by id, as before), so the Community cap keeps the packs in use.
        let mut state = read_state(&root);
        let mut fresh: Vec<(String, String)> = loaded
            .iter()
            .filter(|l| l.error.is_none())
            .map(|l| (l.pack.id.clone(), l.key()))
            .filter(|(_, k)| !state.seen.contains(k))
            .collect();
        fresh.sort();
        if !fresh.is_empty() {
            state.seen.extend(fresh.into_iter().map(|(_, k)| k));
            if write {
                let _ = write_state(&root, &state);
            }
        }

        let count = loaded.len();
        *self.plugins.write().map_err(|e| e.to_string())? = loaded;
        *self.state.write().map_err(|e| e.to_string())? = state;
        self.migrate_legacy();
        Ok(count)
    }

    /// Choices saved by an older version as `plugin_enabled:<id>`
    /// preferences. They apply to every pack with that id that has no
    /// choice of its own yet, and are copied into the state file.
    pub fn apply_legacy_overrides(&self, map: HashMap<String, bool>) {
        if let Ok(mut w) = self.legacy.write() {
            *w = map;
        }
        self.migrate_legacy();
    }

    fn migrate_legacy(&self) {
        let legacy = match self.legacy.read() { Ok(l) if !l.is_empty() => l.clone(), _ => return };
        let keys: Vec<(String, bool)> = match self.plugins.read() {
            Ok(p) => p.iter().filter_map(|lp| legacy.get(&lp.pack.id).map(|v| (lp.key(), *v))).collect(),
            Err(_) => return,
        };
        let mut changed = false;
        if let Ok(mut st) = self.state.write() {
            for (k, v) in keys {
                if !st.enabled.contains_key(&k) {
                    st.enabled.insert(k, v);
                    changed = true;
                }
            }
            if changed {
                if let Some(root) = self.root() {
                    let _ = write_state(&root, &st);
                }
            }
        }
    }

    /// Switch the pack with this key (`<kind>/<id>`) on or off and save the
    /// choice where both the app and the CLI read it.
    pub fn set_enabled(&self, key: &str, enabled: bool) -> Result<(), String> {
        let root = self.root().ok_or("Could not determine config directory")?;
        let mut st = self.state.write().map_err(|e| e.to_string())?;
        // Saved first, applied after: a choice that could not be saved must
        // not take effect here while the CLI and the next launch miss it.
        let mut next = st.clone();
        next.enabled.insert(key.to_string(), enabled);
        write_state(&root, &next)?;
        *st = next;
        Ok(())
    }

    fn is_enabled(&self, lp: &LoadedPlugin) -> bool {
        if let Ok(st) = self.state.read() {
            if let Some(v) = st.enabled.get(&lp.key()) { return *v; }
        }
        if let Ok(l) = self.legacy.read() {
            if let Some(v) = l.get(&lp.pack.id) { return *v; }
        }
        lp.pack.enabled
    }

    /// Keys of the packs that actually contribute rules under the given cap:
    /// the enabled, error-free packs in the order they first appeared (then
    /// by id), cut to `limit`. `None` = no cap.
    fn active_keys(&self, limit: Option<usize>) -> Vec<String> {
        let guard = match self.plugins.read() { Ok(g) => g, Err(_) => return vec![] };
        let seen = self.state.read().map(|s| s.seen.clone()).unwrap_or_default();
        let rank = |k: &str| seen.iter().position(|s| s == k).unwrap_or(usize::MAX);
        let mut packs: Vec<(usize, String, String)> = guard
            .iter()
            .filter(|lp| lp.error.is_none() && self.is_enabled(lp))
            .map(|lp| (rank(&lp.key()), lp.pack.id.clone(), lp.key()))
            .collect();
        packs.sort();
        if let Some(max) = limit {
            packs.truncate(max);
        }
        packs.into_iter().map(|(_, _, k)| k).collect()
    }

    /// A note for validation reports when packs the user switched on are
    /// left out by the cap: fewer findings must not look like a cleaner
    /// message.
    pub fn cap_notice(&self, limit: Option<usize>) -> Option<ValidationIssue> {
        let gated = self.list(limit).into_iter().filter(|p| p.gated && p.rule_count > 0).count();
        (gated > 0).then(|| ValidationIssue {
            severity: Severity::Info,
            message: format!(
                "{} plugin pack{} not applied: the Community edition runs {} packs at a time (Settings → Plugins)",
                gated,
                if gated == 1 { "" } else { "s" },
                limit.unwrap_or(0),
            ),
            segment_idx: None,
            segment_type: None,
            field_position: None,
            rule_id: "PLUGIN-CAP".into(),
        })
    }

    /// Count of enabled, error-free packs (regardless of any cap).
    pub fn enabled_count(&self) -> usize {
        self.active_keys(None).len()
    }

    pub fn list(&self, limit: Option<usize>) -> Vec<PluginInfo> {
        let active = self.active_keys(limit);
        let plugins = self.plugins.read().ok();
        let mut out = Vec::new();
        if let Some(p) = plugins {
            for lp in p.iter() {
                let rule_count = lp.pack.validation_rules.len()
                    + lp.pack.fhir_rules.len()
                    + lp.pack.phi_rules.len();
                let enabled = self.is_enabled(lp) && lp.error.is_none();
                let key = lp.key();
                out.push(PluginInfo {
                    gated: enabled && !active.contains(&key),
                    key,
                    id: lp.pack.id.clone(),
                    name: lp.pack.name.clone(),
                    description: lp.pack.description.clone(),
                    author: lp.pack.author.clone(),
                    version: lp.pack.version.clone(),
                    enabled,
                    kind: lp.kind.as_str().to_string(),
                    path: lp.path.display().to_string(),
                    rule_count,
                    error: lp.error.clone(),
                    warnings: lp.warnings.clone(),
                });
            }
        }
        out
    }

    pub fn plugins_root_path(&self) -> Option<PathBuf> { self.root() }

    /// Rules of one kind from every pack active under the given cap. A pack
    /// contributes all the rules it holds, whichever folder it sits in: PHI
    /// fields listed in a validation pack are masked, not silently dropped.
    fn collect<T: Clone>(&self, limit: Option<usize>, rules: impl Fn(&PluginPack) -> &Vec<T>) -> Vec<T> {
        let active = self.active_keys(limit);
        let guard = match self.plugins.read() { Ok(g) => g, Err(_) => return vec![] };
        guard
            .iter()
            // A refused pack shares its key with the pack it clashes with:
            // the key alone would let its rules run too.
            .filter(|lp| lp.error.is_none() && active.contains(&lp.key()))
            .flat_map(|lp| rules(&lp.pack).iter().cloned())
            .collect()
    }

    /// Collect validation rules from the packs active under the given cap.
    pub fn active_validation_rules(&self, limit: Option<usize>) -> Vec<ValidationRule> {
        self.collect(limit, |p| &p.validation_rules)
    }

    /// Collect FHIR rules from the packs active under the given cap.
    pub fn active_fhir_rules(&self, limit: Option<usize>) -> Vec<FhirRule> {
        self.collect(limit, |p| &p.fhir_rules)
    }

    /// Collect PHI rules from the packs active under the given cap.
    pub fn active_phi_rules(&self, limit: Option<usize>) -> Vec<PhiRule> {
        self.collect(limit, |p| &p.phi_rules)
    }

    /// The active PHI rules in the anonymizer's terms: the app and the CLI
    /// read sensitivities the same way.
    pub fn active_phi_fields(&self, limit: Option<usize>) -> Vec<crate::anonymization::ExtraPhiField> {
        self.active_phi_rules(limit)
            .into_iter()
            .map(|r| crate::anonymization::ExtraPhiField {
                sensitivity: parse_sensitivity(&r.sensitivity),
                segment: r.segment,
                field: r.field,
                name: r.name,
            })
            .collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self { Self::new() }
}

/// Read one pack file. Anything that is not a regular file of sensible
/// size (a named pipe would block the read forever, at startup) is
/// reported instead of read.
fn load_pack(path: &Path, kind: PluginKind) -> LoadedPlugin {
    let failed = |label: &str, error: String| LoadedPlugin {
        pack: PluginPack {
            id: path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string(),
            name: label.into(),
            description: String::new(),
            author: String::new(),
            version: "0".into(),
            enabled: false,
            validation_rules: vec![],
            fhir_rules: vec![],
            phi_rules: vec![],
            extra: Default::default(),
        },
        kind,
        path: path.to_path_buf(),
        error: Some(error),
        warnings: vec![],
    };
    // metadata() follows symlinks: a link to a pack is fine, a link to a
    // device or a pipe is not.
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => return failed("(read error)", e.to_string()),
    };
    if !meta.is_file() {
        return failed("(read error)", "not a regular file".into());
    }
    if meta.len() > MAX_PACK_BYTES {
        return failed(
            "(read error)",
            format!("{} KB is too large for a pack (limit {} KB)", meta.len() / 1024, MAX_PACK_BYTES / 1024),
        );
    }
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => return failed("(read error)", e.to_string()),
    };
    match serde_json::from_str::<PluginPack>(strip_bom(&text)) {
        Ok(pack) => {
            let mut warnings = lint(&pack);
            // A misspelt key ("rules") loads as an empty pack: say so, or a
            // CI run with the pack looks like a run where every rule passed.
            let (key, count) = match kind {
                PluginKind::Validation => ("validation_rules", pack.validation_rules.len()),
                PluginKind::Fhir => ("fhir_rules", pack.fhir_rules.len()),
                PluginKind::Anonymization => ("phi_rules", pack.phi_rules.len()),
            };
            if count == 0 {
                let unknown: Vec<String> =
                    pack.extra.keys().filter(|k| !k.starts_with('$')).map(|k| format!("\"{}\"", k)).collect();
                warnings.insert(
                    0,
                    if unknown.is_empty() {
                        format!("the pack has no \"{}\": it checks nothing", key)
                    } else {
                        format!("the pack has no \"{}\": it checks nothing (unknown key {}: a misspelling?)", key, unknown.join(", "))
                    },
                );
            }
            LoadedPlugin { warnings, pack, kind, path: path.to_path_buf(), error: None }
        }
        Err(e) => failed("(parse error)", e.to_string()),
    }
}

/// Values a pack spells in a way the engine does not know, and rules that
/// cannot do anything. The pack still loads; Settings shows the list.
fn lint(pack: &PluginPack) -> Vec<String> {
    let mut out = Vec::new();
    let severity = |out: &mut Vec<String>, id: &str, s: &str| {
        if !matches!(s.trim().to_ascii_lowercase().as_str(), "error" | "warning" | "info") {
            out.push(format!("rule {}: unknown severity \"{}\", reported as a warning", id, s));
        }
    };
    for r in &pack.validation_rules {
        severity(&mut out, &r.rule_id, &r.severity);
        if let CheckKind::Regex { pattern } = &r.check {
            if let Err(e) = regex::Regex::new(pattern) {
                out.push(format!("rule {}: the pattern does not compile ({}); the rule reports this on every validation", r.rule_id, regex_reason(&e.to_string())));
            }
        }
    }
    for r in &pack.fhir_rules {
        severity(&mut out, &r.rule_id, &r.severity);
        for fp in [&r.expression, &r.path].into_iter().flatten() {
            if let Err(e) = crate::parser::fhir::fhirpath::check(fp) {
                out.push(format!("rule {}: the FHIRPath does not parse ({}); the rule reports this on every validation", r.rule_id, e));
            }
        }
        if let Some(FhirCheck::Regex { pattern }) = &r.check {
            if let Err(e) = regex::Regex::new(pattern) {
                out.push(format!("rule {}: the pattern does not compile ({}); the rule reports this on every validation", r.rule_id, regex_reason(&e.to_string())));
            }
        }
    }
    for r in &pack.phi_rules {
        if !matches!(r.sensitivity.trim().to_ascii_lowercase().as_str(), "high" | "medium" | "low") {
            out.push(format!("PHI field {}-{}: unknown sensitivity \"{}\", masked as high", r.segment, r.field, r.sensitivity));
        }
        if r.field == 0 || (r.segment == "MSH" && r.field <= 2) {
            out.push(format!("PHI field {}-{}: not a maskable field (segment name or delimiters); ignored", r.segment, r.field));
        }
    }
    out
}

/// The reason in a `regex` error. Its message spans several lines (a
/// header, the pattern, a caret, then `error: …`): the reason is the last.
fn regex_reason(s: &str) -> &str {
    let mut lines = s.lines().map(str::trim).filter(|l| !l.is_empty());
    let first = lines.clone().next().unwrap_or(s);
    lines.find_map(|l| l.strip_prefix("error:").map(str::trim)).unwrap_or(first)
}

/// A rule's check, with its pattern compiled once per run instead of once
/// per segment.
enum Compiled<'a> {
    Regex(regex::Regex),
    Plain(&'a CheckKind),
}

impl<'a> Compiled<'a> {
    fn new(check: &'a CheckKind) -> Result<Self, String> {
        match check {
            CheckKind::Regex { pattern } => regex::Regex::new(pattern)
                .map(Compiled::Regex)
                .map_err(|e| format!("invalid pattern: {}", regex_reason(&e.to_string()))),
            other => Ok(Compiled::Plain(other)),
        }
    }

    fn passes(&self, value: &str) -> bool {
        match self {
            Compiled::Regex(re) => re.is_match(value),
            Compiled::Plain(check) => apply_check(check, value),
        }
    }
}

/// Run all active plugin validation rules against `msg` and return the
/// emitted issues.
///
/// A repeating field is checked one repetition at a time, and components
/// are split with the message's own delimiters (MSH-2), not a fixed `^`.
pub fn run_custom_validations(
    msg: &Hl7Message,
    rules: &[ValidationRule],
) -> Vec<ValidationIssue> {
    let (rep, comp) = (msg.delimiters.repetition as char, msg.delimiters.component as char);
    let mut issues = Vec::new();
    for rule in rules {
        let check = match Compiled::new(&rule.check) {
            Ok(c) => c,
            Err(e) => {
                // A rule that cannot run is reported, like a FHIR rule that
                // fails to evaluate: passing it would hide that nothing was
                // checked.
                issues.push(ValidationIssue {
                    severity: parse_severity(&rule.severity),
                    message: format!("{} (rule {} cannot run: {})", rule.message, rule.rule_id, e),
                    segment_idx: None,
                    segment_type: Some(rule.segment.clone()),
                    field_position: Some(rule.field),
                    rule_id: rule.rule_id.clone(),
                });
                continue;
            }
        };
        for (seg_idx, seg) in msg.segments.iter().enumerate() {
            if seg.segment_type != rule.segment { continue; }

            let raw_value = seg
                .fields
                .iter()
                .find(|f| f.position == rule.field)
                .map(|f| f.span.as_str(&msg.raw))
                .unwrap_or("");
            // MSH-1 and MSH-2 are the delimiters themselves.
            let delimiters = seg.segment_type == "MSH" && rule.field <= 2;
            let repetitions: Vec<&str> = if delimiters { vec![raw_value] } else { raw_value.split(rep).collect() };
            let passed = repetitions.iter().all(|r| {
                let value = match rule.component {
                    Some(c) if !delimiters => r.split(comp).nth(c.saturating_sub(1)).unwrap_or(""),
                    _ => r,
                };
                check.passes(value)
            });
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
            if let Some(issue) = apply_fhir_rule(rule, target, root, &prefix) {
                issues.push(issue);
            }
        }
    }
    issues
}

/// The resource itself plus, for a Bundle, every entry resource — each with
/// the path prefix to report findings against.
pub fn fhir_targets(root: &serde_json::Value) -> Vec<(&serde_json::Value, String)> {
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
    scope: &serde_json::Value,
    prefix: &str,
) -> Option<crate::parser::fhir::FhirValidationIssue> {
    use crate::parser::fhir::fhirpath;

    let fail = |detail: Option<String>, path: String| {
        Some(crate::parser::fhir::FhirValidationIssue {
            // Normalised as for HL7 rules: "Error" or a typo must still be
            // counted, not dropped from the totals.
            severity: match parse_severity(&rule.severity) {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
            }
            .to_string(),
            message: match detail {
                Some(d) => format!("{} ({})", rule.message, d),
                None => rule.message.clone(),
            },
            path,
            rule_id: Some(rule.rule_id.clone()),
        })
    };

    // Invariant form: the expression must come back true.
    if let Some(expression) = &rule.expression {
        let result = fhirpath::evaluate_within(expression, target, scope);
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
    let result = fhirpath::evaluate_within(path, target, scope);
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

/// Checks other than `regex` (which [`Compiled`] handles). Lengths count
/// characters, as the FHIR checks do: "Müller" is 6, not 7.
fn apply_check(check: &CheckKind, value: &str) -> bool {
    match check {
        CheckKind::NotEmpty => !value.trim().is_empty(),
        CheckKind::Regex { pattern } => regex::Regex::new(pattern).is_ok_and(|re| re.is_match(value)),
        CheckKind::MaxLength { max } => value.chars().count() <= *max,
        CheckKind::MinLength { min } => value.chars().count() >= *min,
        CheckKind::OneOf { values } => values.iter().any(|v| v == value),
        CheckKind::Contains { value: needle } => value.contains(needle.as_str()),
    }
}

fn parse_severity(s: &str) -> Severity {
    match s.trim().to_ascii_lowercase().as_str() {
        "error" => Severity::Error,
        "info"  => Severity::Info,
        _       => Severity::Warning,
    }
}

/// Convert a PHI rule's sensitivity string into the enum used by the
/// anonymizer. A value it does not recognise masks fully: a typo must not
/// leave more of a patient's data visible than the author meant.
pub fn parse_sensitivity(s: &str) -> PhiSensitivity {
    match s.trim().to_ascii_lowercase().as_str() {
        "medium" => PhiSensitivity::Medium,
        "low"    => PhiSensitivity::Low,
        _        => PhiSensitivity::High,
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
            extra: Default::default(),
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
        assert_eq!(issues[0].rule_id.as_deref(), Some("has-identifier"));
        assert!(issues[0].path.starts_with("entry[1].resource."), "{}", issues[0].path);
    }

    #[test]
    fn a_rule_on_an_entry_resolves_references_to_other_entries() {
        let bundle = serde_json::json!({
            "resourceType": "Bundle", "type": "collection",
            "entry": [
                {"fullUrl": "urn:uuid:a", "resource": {"resourceType": "Patient", "name": [{"family": "Y"}]}},
                {"fullUrl": "urn:uuid:b", "resource": {"resourceType": "Observation", "status": "final",
                    "code": {"text": "x"}, "subject": {"reference": "urn:uuid:a"}}},
                {"fullUrl": "urn:uuid:c", "resource": {"resourceType": "Observation", "status": "final",
                    "code": {"text": "x"}, "subject": {"reference": "urn:uuid:missing"}}}
            ]
        });
        let rule = FhirRule {
            resource: Some("Observation".into()),
            expression: Some("subject.resolve().exists()".into()),
            ..fhir_rule("subject-resolves")
        };
        let issues = run_fhir_validations(&bundle, &[rule]);
        assert_eq!(issues.len(), 1, "{:?}", issues);
        assert!(issues[0].path.starts_with("entry[2].resource."), "{}", issues[0].path);
    }

    #[test]
    fn a_fhir_rule_that_cannot_run_is_listed_when_the_pack_loads() {
        let root = temp_root();
        let deep = format!("{}true{}", "(".repeat(12000), ")".repeat(12000));
        write_pack(&root, "fhir", "deep.json", &serde_json::json!({
            "id": "deep", "name": "Deep", "fhir_rules": [
                {"rule_id": "d1", "severity": "error", "resource": "Patient", "expression": deep, "message": "m"},
                {"rule_id": "d2", "severity": "error", "path": "name.family", "check": {"type": "regex", "pattern": "([a-z"}, "message": "m"}
            ]
        }).to_string());
        let reg = PluginRegistry::with_root(root);
        reg.reload().unwrap();
        let info = reg.list(None).into_iter().find(|p| p.id == "deep").unwrap();
        assert!(info.error.is_none());
        assert!(info.warnings.iter().any(|w| w.contains("rule d1") && w.contains("nested")), "{:?}", info.warnings);
        assert!(info.warnings.iter().any(|w| w.contains("rule d2") && w.contains("pattern")), "{:?}", info.warnings);
        // And running it is a finding, not a crash.
        let issues = run_fhir_validations(&fhir_patient(), &reg.active_fhir_rules(None));
        assert!(issues.iter().any(|i| i.message.contains("failed to evaluate")), "{:?}", issues);
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

    fn temp_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("bl-plugins-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_pack(root: &Path, sub: &str, file: &str, json: &str) {
        fs::create_dir_all(root.join(sub)).unwrap();
        fs::write(root.join(sub).join(file), json).unwrap();
    }

    fn make_registry(ids: &[&str]) -> PluginRegistry {
        let reg = PluginRegistry::with_root(temp_root());
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
                extra: Default::default(),
            },
            kind: PluginKind::Validation,
            path: PathBuf::from(format!("{id}.json")),
            error: None,
            warnings: vec![],
        }).collect();
        *reg.plugins.write().unwrap() = loaded;
        reg
    }

    #[test]
    fn registry_override_disables_pack() {
        let reg = make_registry(&["mine"]);
        reg.set_enabled("validation/mine", false).unwrap();
        assert!(reg.active_validation_rules(None).is_empty());
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
        reg.set_enabled("validation/a", false).unwrap();
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

    fn rule(id: &str, field: usize, component: Option<usize>, check: CheckKind) -> ValidationRule {
        ValidationRule {
            rule_id: id.into(), severity: "error".into(), segment: "PID".into(),
            field, component, check, message: id.into(),
        }
    }

    fn parse(raw: &str) -> Hl7Message {
        Hl7Lexer::new().parse(raw.as_bytes().to_vec()).unwrap()
    }

    #[test]
    fn each_repetition_is_checked_on_its_own() {
        let one_of = rule("TYPE", 3, Some(5), CheckKind::OneOf { values: vec!["MR".into(), "PI".into()] });
        let digits = rule("NUM", 3, Some(1), CheckKind::Regex { pattern: "^[0-9]+$".into() });
        let ok = parse("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||111^^^H^MR~222^^^H^PI\r");
        assert!(run_custom_validations(&ok, &[one_of.clone(), digits.clone()]).is_empty());
        let bad = parse("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||111^^^H^MR~ABC^^^H^PI\r");
        let ids: Vec<String> = run_custom_validations(&bad, &[one_of, digits]).into_iter().map(|i| i.rule_id).collect();
        assert_eq!(ids, vec!["NUM"]);
    }

    #[test]
    fn components_follow_the_message_delimiters() {
        let fam = rule("FAM", 5, Some(1), CheckKind::Regex { pattern: "^[A-Z]+$".into() });
        let dollar = parse("MSH|$~\\&|A|B|C|D|20260101||ADT$A01|1|P|2.5\rPID|1||123||DOE$JOHN\r");
        assert!(run_custom_validations(&dollar, &[fam]).is_empty());
    }

    #[test]
    fn a_pattern_that_does_not_compile_is_reported_not_skipped() {
        let msg = sample_msg();
        let broken = rule("BADRE", 5, None, CheckKind::Regex { pattern: "^[A-Z".into() });
        let issues = run_custom_validations(&msg, &[broken]);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("cannot run"), "{}", issues[0].message);
    }

    #[test]
    fn lengths_count_characters() {
        let msg = parse("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||||UNICODE UTF-8\rPID|1||1||Müller^Jörg\r");
        let max6 = rule("MAX6", 5, Some(1), CheckKind::MaxLength { max: 6 });
        assert!(run_custom_validations(&msg, &[max6]).is_empty());
    }

    #[test]
    fn loader_accepts_bom_and_upper_case_extension_and_refuses_clashing_ids() {
        let root = temp_root();
        write_pack(&root, "validation", "bom.json", "\u{feff}{\"id\":\"bom\",\"name\":\"BOM\"}");
        write_pack(&root, "validation", "UPPER.JSON", r#"{"id":"upper","name":"Upper"}"#);
        write_pack(&root, "validation", "a.json", r#"{"id":"same","name":"A"}"#);
        write_pack(&root, "validation", "b.json", r#"{"id":"same","name":"B"}"#);
        let reg = PluginRegistry::with_root(root);
        reg.reload().unwrap();
        let list = reg.list(None);
        let by_name = |n: &str| list.iter().find(|p| p.name == n || p.path.ends_with(n)).unwrap();
        assert!(by_name("BOM").error.is_none());
        assert!(by_name("Upper").error.is_none());
        assert!(by_name("A").error.is_none());
        assert!(by_name("b.json").error.as_deref().unwrap().contains("already uses the id"));
    }

    #[test]
    fn a_pack_refused_for_its_id_contributes_no_rules() {
        let root = temp_root();
        let pack = |name: &str, rule: &str| {
            format!(
                r#"{{"id":"acme-fhir","name":"{name}","version":"1.0","enabled":true,"fhir_rules":[{{"rule_id":"{rule}","severity":"error","resource":"Patient","expression":"identifier.exists()","message":"{rule} fired"}}]}}"#
            )
        };
        write_pack(&root, "fhir", "a-rules.json", &pack("ACME A", "A1"));
        write_pack(&root, "fhir", "b-rules.json", &pack("ACME B", "B1"));
        let reg = PluginRegistry::with_root(root);
        reg.reload().unwrap();
        let ids: Vec<String> = reg.active_fhir_rules(None).into_iter().map(|r| r.rule_id).collect();
        assert_eq!(ids, vec!["A1"]);
        assert_eq!(reg.enabled_count(), 1);
        // Both files are listed, and each row has its own path to key it by.
        let list = reg.list(None);
        assert_eq!(list.len(), 2);
        assert_ne!(list[0].path, list[1].path);
        assert!(!list.iter().find(|p| p.name == "ACME B").unwrap().enabled);
    }

    #[cfg(unix)]
    #[test]
    fn a_named_pipe_is_reported_not_read() {
        let root = temp_root();
        fs::create_dir_all(root.join("validation")).unwrap();
        let fifo = root.join("validation").join("pipe.json");
        assert!(std::process::Command::new("mkfifo").arg(&fifo).status().unwrap().success());
        let reg = PluginRegistry::with_root(root);
        reg.reload().unwrap();
        assert_eq!(reg.list(None)[0].error.as_deref(), Some("not a regular file"));
    }

    #[test]
    fn packs_sharing_an_id_across_folders_switch_separately_and_the_choice_persists() {
        let root = temp_root();
        write_pack(&root, "validation", "acme.json", r#"{"id":"acme","name":"ACME rules","validation_rules":[{"rule_id":"A1","segment":"PID","field":99,"check":{"type":"not_empty"},"message":"m"}]}"#);
        write_pack(&root, "anonymization", "acme.json", r#"{"id":"acme","name":"ACME PHI","phi_rules":[{"segment":"PID","field":25,"sensitivity":"high"}]}"#);
        let reg = PluginRegistry::with_root(root.clone());
        reg.reload().unwrap();
        reg.set_enabled("validation/acme", false).unwrap();
        assert!(reg.active_validation_rules(None).is_empty());
        assert_eq!(reg.active_phi_rules(None).len(), 1, "the PHI pack stays on");
        // A second registry (the CLI) sees the same choice.
        let cli = PluginRegistry::with_root(root);
        cli.reload().unwrap();
        assert!(cli.active_validation_rules(None).is_empty());
        assert_eq!(cli.active_phi_rules(None).len(), 1);
    }

    #[test]
    fn legacy_choices_apply_until_a_pack_has_its_own() {
        let root = temp_root();
        write_pack(&root, "validation", "old.json", r#"{"id":"old","name":"Old","validation_rules":[{"rule_id":"O1","segment":"PID","field":99,"check":{"type":"not_empty"},"message":"m"}]}"#);
        let reg = PluginRegistry::with_root(root.clone());
        reg.reload().unwrap();
        reg.apply_legacy_overrides(HashMap::from([("old".to_string(), false)]));
        assert!(reg.active_validation_rules(None).is_empty());
        let cli = PluginRegistry::with_root(root);
        cli.reload().unwrap();
        assert!(cli.active_validation_rules(None).is_empty(), "migrated into the state file");
    }

    #[test]
    fn a_new_pack_does_not_displace_one_already_active_under_the_cap() {
        let root = temp_root();
        for id in ["pack-b", "pack-c", "pack-d"] {
            write_pack(&root, "validation", &format!("{id}.json"), &format!(r#"{{"id":"{id}","name":"{id}"}}"#));
        }
        let reg = PluginRegistry::with_root(root.clone());
        reg.reload().unwrap();
        write_pack(&root, "validation", "aa-new.json", r#"{"id":"aa-new","name":"aa-new"}"#);
        reg.reload().unwrap();
        let gated: Vec<String> = reg.list(Some(3)).into_iter().filter(|p| p.gated).map(|p| p.id).collect();
        assert_eq!(gated, vec!["aa-new"]);
    }

    #[test]
    fn phi_rules_count_wherever_the_pack_sits_and_typos_are_listed() {
        let root = temp_root();
        write_pack(&root, "validation", "mixed.json", r#"{"id":"mixed","name":"Mixed","phi_rules":[{"segment":"ZPI","field":4,"sensitivity":"hihg"},{"segment":"MSH","field":2,"sensitivity":"high"}],"validation_rules":[{"rule_id":"S1","severity":"critical","segment":"PID","field":1,"check":{"type":"regex","pattern":"^[A-Z"},"message":"m"}]}"#);
        let reg = PluginRegistry::with_root(root);
        reg.reload().unwrap();
        let fields = reg.active_phi_fields(None);
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].sensitivity, PhiSensitivity::High, "unknown sensitivity masks fully");
        let w = &reg.list(None)[0].warnings;
        assert!(w.iter().any(|m| m.contains("unknown sensitivity")), "{w:?}");
        assert!(w.iter().any(|m| m.contains("unknown severity")), "{w:?}");
        assert!(w.iter().any(|m| m.contains("does not compile")), "{w:?}");
        assert!(w.iter().any(|m| m.contains("MSH-2")), "{w:?}");
    }

    #[test]
    fn a_choice_that_cannot_be_saved_does_not_take_effect() {
        let root = temp_root();
        write_pack(&root, "validation", "p.json", r#"{"id":"p","name":"P","validation_rules":[{"rule_id":"R","segment":"PID","field":99,"check":{"type":"not_empty"},"message":"m"}]}"#);
        let reg = PluginRegistry::with_root(root.clone());
        reg.reload().unwrap();
        // A directory where the state file should be makes the write fail.
        let _ = fs::remove_file(root.join(STATE_FILE));
        fs::create_dir_all(root.join(STATE_FILE)).unwrap();
        assert!(reg.set_enabled("validation/p", false).is_err());
        assert_eq!(reg.active_validation_rules(None).len(), 1);
    }

    #[test]
    fn fhir_rule_severities_are_normalised() {
        let rule = FhirRule {
            rule_id: "F".into(), severity: " Error".into(), resource: None,
            expression: Some("false".into()), path: None, check: None, message: "m".into(), extra: Default::default(),
        };
        let issues = run_fhir_validations(&serde_json::json!({"resourceType":"Patient"}), &[rule]);
        assert_eq!(issues[0].severity, "error");
    }

    #[test]
    fn packs_left_out_by_the_cap_are_noted() {
        let reg = make_registry(&["a", "b", "c", "d"]);
        let note = reg.cap_notice(Some(3)).expect("one pack over the cap");
        assert_eq!(note.severity, Severity::Info);
        assert!(note.message.starts_with("1 plugin pack not applied"), "{}", note.message);
        assert!(reg.cap_notice(None).is_none());
    }

    #[test]
    fn a_pack_with_no_rules_under_its_key_is_flagged() {
        let root = temp_root();
        write_pack(&root, "validation", "typo.json", r#"{"id":"typo","name":"Typo","rules":[{"rule_id":"X"}]}"#);
        let reg = PluginRegistry::with_root(root.clone());
        reg.reload().unwrap();
        let info = reg.list(None).into_iter().find(|p| p.name == "Typo").unwrap();
        assert!(info.warnings.iter().any(|w| w.contains("validation_rules") && w.contains("\"rules\"")), "{:?}", info.warnings);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_regex_warning_keeps_its_reason() {
        let e = regex::Regex::new("([").unwrap_err().to_string();
        assert!(regex_reason(&e).contains("unclosed"), "{}", regex_reason(&e));
        assert_eq!(regex_reason("one line"), "one line");
    }

    #[test]
    fn a_read_only_reload_writes_nothing() {
        let root = temp_root();
        write_pack(&root, "validation", "a.json", r#"{"id":"a","name":"A","validation_rules":[]}"#);
        let reg = PluginRegistry::with_root(root.clone());
        assert_eq!(reg.reload_read_only().unwrap(), 1);
        assert!(!root.join(STATE_FILE).exists());
        assert!(!root.join("fhir").exists());
        reg.reload().unwrap();
        assert!(root.join(STATE_FILE).exists());
        fs::remove_dir_all(&root).ok();
    }
}

