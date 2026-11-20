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

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, LazyLock, Mutex, MutexGuard, RwLock, Weak};

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

impl PluginPack {
    /// Rules of every kind the pack holds, whichever folder it sits in.
    pub fn rule_count(&self) -> usize {
        self.validation_rules.len() + self.fhir_rules.len() + self.phi_rules.len()
    }
}

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

/// Where an unreadable state file is kept when new choices are saved.
const BROKEN_STATE_FILE: &str = ".state.json.broken";

/// Why the state file could not be used.
#[derive(Debug, Clone)]
enum StateProblem {
    /// The file is there but is not the JSON BridgeLab writes (a hand edit
    /// gone wrong, a merge conflict): its choices cannot be read.
    Malformed(String),
    /// The file could not be read as text at all (permissions, a folder
    /// there, UTF-16 or other bytes that are not UTF-8).
    Unreadable(String),
}

impl StateProblem {
    fn reason(&self) -> &str {
        match self {
            StateProblem::Malformed(e) | StateProblem::Unreadable(e) => e,
        }
    }
}

/// The saved choices; an empty state when there is no file yet. A file
/// that cannot be read is an error, not an empty state: read as empty, it
/// turned every pack switched off back on, and the next save wrote over
/// it, so the choices were lost for good without a word.
fn read_state(root: &Path) -> Result<RegistryState, StateProblem> {
    let text = match fs::read_to_string(root.join(STATE_FILE)) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(RegistryState::default()),
        Err(e) => return Err(StateProblem::Unreadable(e.to_string())),
    };
    serde_json::from_str(strip_bom(&text)).map_err(|e| StateProblem::Malformed(e.to_string()))
}

/// Keep an unreadable state file under a name no earlier one holds
/// (`.state.json.broken`, then `.state.json.broken.1`, `.2`, …) before new
/// choices are saved over it. With one fixed name, a second recovery made
/// the rename fail on Windows, and every switch with it, and on Unix
/// replaced the first backup. Keeping the file is a courtesy: when it
/// cannot be moved or copied, the user's new choice is saved anyway.
fn set_aside_broken_state(root: &Path) {
    let free = |name: String| fs::symlink_metadata(root.join(&name)).is_err().then(|| root.join(name));
    let Some(backup) = std::iter::once(BROKEN_STATE_FILE.to_string())
        .chain((1..1000).map(|n| format!("{}.{}", BROKEN_STATE_FILE, n)))
        .find_map(free)
    else {
        return;
    };
    let state = root.join(STATE_FILE);
    if fs::rename(&state, &backup).is_err() {
        let _ = fs::copy(&state, &backup);
    }
}

/// When a pack file appeared in the folder: its creation time where the
/// file system records one (a pack copied in with an old modification
/// date is still new here), else its last modification.
fn appeared(path: &Path) -> Option<std::time::SystemTime> {
    let meta = fs::metadata(path).ok()?;
    meta.created().or_else(|_| meta.modified()).ok()
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
    /// `error` and `warnings` as notices Settings words in the user's
    /// language (the two above are the English the CLI prints).
    pub error_notice: Option<PluginNotice>,
    pub notices: Vec<PluginNotice>,
}

/// A problem with a pack, as Settings shows it in the user's language:
/// `code` names the sentence, `args` fill it in, and `rule` or
/// `phi_field` is what it is about. Settings showed the English text in
/// every language; [`PluginNotice::english`] is that text, which the CLI
/// prints and the app falls back to for a code it has no words for.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PluginNotice {
    pub code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phi_field: Option<String>,
    pub args: BTreeMap<&'static str, String>,
}

/// What a pattern that compiles large can be written as instead.
const SMALLER: &str = "a shorter repetition, or an ASCII class such as [A-Za-z0-9_] for \\w, is far smaller";

impl PluginNotice {
    fn new(code: &'static str) -> Self {
        Self { code, rule: None, phi_field: None, args: BTreeMap::new() }
    }

    fn arg(mut self, name: &'static str, value: impl ToString) -> Self {
        self.args.insert(name, value.to_string());
        self
    }

    /// About rule `id`.
    fn on_rule(mut self, id: &str) -> Self {
        self.rule = Some(id.to_string());
        self
    }

    /// About the PHI field `field` ("PID-5").
    fn on_phi_field(mut self, field: String) -> Self {
        self.phi_field = Some(field);
        self
    }

    /// The notice in English: "rule R1: field 0 is the segment name...".
    pub fn english(&self) -> String {
        let a = |name: &str| self.args.get(name).map(String::as_str).unwrap_or("");
        let body = match self.code {
            "duplicate_id" => format!(
                "another pack in {}/ already uses the id \"{}\" ({}); give each pack its own id",
                a("dir"), a("id"), a("file")
            ),
            "not_regular_file" => "not a regular file".to_string(),
            "too_large" => format!("{} KB is too large for a pack (limit {} KB)", a("size_kb"), a("limit_kb")),
            // The system's or the JSON reader's own words.
            "read_error" | "parse_error" => a("reason").to_string(),
            "no_rules" => format!("the pack has no \"{}\": it checks nothing", a("key")),
            "no_rules_unknown_key" => format!(
                "the pack has no \"{}\": it checks nothing (unknown key {}: a misspelling?)",
                a("key"), a("unknown")
            ),
            "empty_id" => "the pack has an empty \"id\": give it one, the pack's on/off switch is saved under it".to_string(),
            "empty_name" => "the pack has an empty \"name\": Settings shows it as the pack's title".to_string(),
            "unknown_severity" => format!("unknown severity \"{}\", reported as a warning", a("value")),
            "field_zero" => "field 0 is the segment name, not a field (fields count from 1); the rule checks an empty value".to_string(),
            "component_zero" => "component 0 does not exist (components count from 1); component 1 is checked".to_string(),
            "fhirpath_invalid" => format!(
                "the FHIRPath does not parse ({}); the rule reports this on every validation",
                a("reason")
            ),
            "unknown_sensitivity" => format!("unknown sensitivity \"{}\", masked as high", a("value")),
            "not_maskable" => "not a maskable field (segment name or delimiters); ignored".to_string(),
            "segment_malformed" => format!(
                "\"{}\" is not a segment name (three letters or digits, such as PID or ZPI); it matches no segment of a well-formed message",
                a("segment")
            ),
            "segment_case" => format!("segment \"{}\" is read as {}; write it as {}", a("segment"), a("key"), a("key")),
            "pattern_too_big" => format!(
                "the pattern is too large to compile (over {} MB, as a long repetition of \\w or \\p{{L}} becomes), so the rule reports this on every validation; {}",
                a("mb"), SMALLER
            ),
            "pattern_invalid" => format!(
                "the pattern does not compile ({}); the rule reports this on every validation",
                a("reason")
            ),
            "pattern_heavy" => format!(
                "the pattern compiles to over {} MB, which slows every load of the pack and takes as much memory; {}",
                a("mb"), SMALLER
            ),
            other => other.to_string(),
        };
        match (&self.rule, &self.phi_field) {
            (Some(rule), _) => format!("rule {}: {}", rule, body),
            (None, Some(field)) => format!("PHI field {}: {}", field, body),
            (None, None) => body,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub pack: PluginPack,
    pub kind: PluginKind,
    pub path: PathBuf,
    pub error: Option<PluginNotice>,
    pub warnings: Vec<PluginNotice>,
    /// The pack's `regex` patterns, compiled when it loaded. While the pack
    /// holds them, the validators find them in [`PATTERNS`] instead of
    /// compiling them again: held, never read.
    #[allow(dead_code)]
    patterns: Vec<Arc<CompiledPattern>>,
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

/// Which rules a report is about, for the cap note it carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleKind {
    /// HL7 v2 validation rules.
    Hl7,
    /// FHIR validation rules.
    Fhir,
    /// PHI fields for detection and anonymization.
    Phi,
}

/// Rule id of the note a report carries when packs are left out by the cap.
pub const CAP_RULE_ID: &str = "PLUGIN-CAP";

/// Global plugin registry (refreshed on `reload`).
pub struct PluginRegistry {
    plugins: RwLock<Vec<LoadedPlugin>>,
    state: RwLock<RegistryState>,
    /// Set when the state file could not be read at the last load: the
    /// packs then follow their own `enabled`, and the file is not written
    /// over until the user saves a new choice.
    state_problem: RwLock<Option<StateProblem>>,
    /// Choices saved before 1.9.0 as `plugin_enabled:<id>` preferences,
    /// keyed by bare id. Used only for packs with no choice in the state
    /// file, and copied into it on first use.
    legacy: RwLock<HashMap<String, bool>>,
    /// Plugins directory; `None` = the standard one under the config dir.
    root: Option<PathBuf>,
    /// Held for the whole of a load, so that two loads (the one at startup
    /// and a Reload in Settings) never interleave.
    load_lock: Mutex<()>,
    /// True from [`begin_background_load`](Self::begin_background_load)
    /// until a load has finished; `loaded` wakes whoever waits for it.
    loading: Mutex<bool>,
    loaded: Condvar,
}

/// A lock whose holder panicked is still usable here: every value behind
/// one is replaced whole, never left half-written.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Marks the end of a load when dropped, however the load ended: an error
/// must leave the registry without packs, not every reader waiting.
struct LoadDone<'a>(&'a PluginRegistry);

impl Drop for LoadDone<'_> {
    fn drop(&mut self) {
        *lock(&self.0.loading) = false;
        self.0.loaded.notify_all();
    }
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
            state_problem: RwLock::new(None),
            legacy: RwLock::new(HashMap::new()),
            root,
            load_lock: Mutex::new(()),
            loading: Mutex::new(false),
            loaded: Condvar::new(),
        }
    }

    fn root(&self) -> Option<PathBuf> {
        self.root.clone().or_else(plugins_root)
    }

    /// Announce a load that another thread is about to run. Until a load
    /// finishes, everything that reads the packs waits for it. The app
    /// opens its window without waiting for the packs (compiling a pack's
    /// patterns can take seconds), but a validation in its first moments
    /// must still run them: no packs would read as no findings.
    pub fn begin_background_load(&self) {
        *lock(&self.loading) = true;
    }

    /// Wait for a load announced by `begin_background_load` to finish.
    fn wait_for_load(&self) {
        let mut loading = lock(&self.loading);
        while *loading {
            loading = self.loaded.wait(loading).unwrap_or_else(|e| e.into_inner());
        }
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
        let _one_at_a_time = lock(&self.load_lock);
        let _done = LoadDone(self);
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
                        lp.error = Some(
                            PluginNotice::new("duplicate_id")
                                .arg("dir", sub)
                                .arg("id", &lp.pack.id)
                                .arg("file", first.path.file_name().and_then(|n| n.to_str()).unwrap_or("?")),
                        );
                    }
                }
                loaded.push(lp);
            }
        }

        // Remember new packs after the known ones, in the order their files
        // appeared (then by id), so the Community cap keeps the packs in
        // use. A read-only load (the CLI) records nothing, so without the
        // app every pack is new on every run: ordered by id, a file added
        // later took the place of a pack in use.
        let (mut state, problem) = match read_state(&root) {
            Ok(s) => (s, None),
            Err(p) => (RegistryState::default(), Some(p)),
        };
        let mut fresh: Vec<(bool, Option<std::time::SystemTime>, String, String)> = loaded
            .iter()
            .filter(|l| l.error.is_none() && !state.seen.contains(&l.key()))
            .map(|l| {
                let at = appeared(&l.path);
                (at.is_none(), at, l.pack.id.clone(), l.key())
            })
            .collect();
        fresh.sort();
        if !fresh.is_empty() {
            state.seen.extend(fresh.into_iter().map(|(_, _, _, k)| k));
            // An unreadable file stays as it is: it may hold choices the
            // user can still recover.
            if write && problem.is_none() {
                let _ = write_state(&root, &state);
            }
        }

        let count = loaded.len();
        *self.plugins.write().map_err(|e| e.to_string())? = loaded;
        *self.state.write().map_err(|e| e.to_string())? = state;
        *self.state_problem.write().map_err(|e| e.to_string())? = problem;
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
            if changed && self.state_problem().is_none() {
                if let Some(root) = self.root() {
                    let _ = write_state(&root, &st);
                }
            }
        }
    }

    /// Switch the pack with this key (`<kind>/<id>`) on or off and save the
    /// choice where both the app and the CLI read it.
    pub fn set_enabled(&self, key: &str, enabled: bool) -> Result<(), String> {
        // The load would overwrite the choice with the state it read.
        self.wait_for_load();
        let root = self.root().ok_or("Could not determine config directory")?;
        let mut st = self.state.write().map_err(|e| e.to_string())?;
        let mut problem = self.state_problem.write().map_err(|e| e.to_string())?;
        // Saved first, applied after: a choice that could not be saved must
        // not take effect here while the CLI and the next launch miss it.
        let mut next = st.clone();
        next.enabled.insert(key.to_string(), enabled);
        // A file that could not be read is kept beside the new one, not
        // written over: the user saw the warning and chose to start again,
        // but the old choices can still be read back by hand. That holds
        // for one that is not text BridgeLab reads (saved as UTF-16 by an
        // editor) as much as for one with a JSON error.
        if problem.is_some() {
            set_aside_broken_state(&root);
        }
        write_state(&root, &next)?;
        *st = next;
        *problem = None;
        Ok(())
    }

    /// Why the saved on/off choices could not be read at the last load, if
    /// they could not: until the file is fixed (or a new choice is saved)
    /// each pack follows its own `enabled`, so a pack switched off may run.
    /// The app shows it in Settings → Plugins, the CLI on standard error.
    pub fn state_problem(&self) -> Option<String> {
        self.state_problem.read().ok()?.as_ref().map(|p| p.reason().to_string())
    }

    /// The file the on/off choices are saved in.
    pub fn state_file_path(&self) -> Option<PathBuf> {
        self.root().map(|r| r.join(STATE_FILE))
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
    /// by id), the first `limit` of them that hold rules. `None` = no cap.
    /// A pack that holds no rules (a misspelt key, an empty draft) checks
    /// nothing, so it takes no slot: counted, it left a working pack
    /// inactive in its place.
    fn active_keys(&self, limit: Option<usize>) -> Vec<String> {
        // Every reader of the packs comes through here (the rules, the
        // list, the counts), so this is where they wait for a load.
        self.wait_for_load();
        let guard = match self.plugins.read() { Ok(g) => g, Err(_) => return vec![] };
        let seen = self.state.read().map(|s| s.seen.clone()).unwrap_or_default();
        let rank = |k: &str| seen.iter().position(|s| s == k).unwrap_or(usize::MAX);
        let mut packs: Vec<(usize, String, String, bool)> = guard
            .iter()
            .filter(|lp| lp.error.is_none() && self.is_enabled(lp))
            .map(|lp| (rank(&lp.key()), lp.pack.id.clone(), lp.key(), lp.pack.rule_count() > 0))
            .collect();
        packs.sort();
        let mut slots = 0;
        packs
            .into_iter()
            .filter(|(_, _, _, holds_rules)| {
                if !holds_rules {
                    return true;
                }
                slots += 1;
                limit.is_none_or(|max| slots <= max)
            })
            .map(|(_, _, k, _)| k)
            .collect()
    }

    /// How many packs the user switched on hold rules of `kind` but are
    /// left out by the cap.
    pub fn gated_count(&self, limit: Option<usize>, kind: RuleKind) -> usize {
        let active = self.active_keys(limit);
        let Ok(guard) = self.plugins.read() else { return 0 };
        guard
            .iter()
            .filter(|lp| lp.error.is_none() && self.is_enabled(lp) && !active.contains(&lp.key()))
            .filter(|lp| match kind {
                RuleKind::Hl7 => !lp.pack.validation_rules.is_empty(),
                RuleKind::Fhir => !lp.pack.fhir_rules.is_empty(),
                RuleKind::Phi => !lp.pack.phi_rules.is_empty(),
            })
            .count()
    }

    /// The note a report carries when packs with rules of its `kind` are
    /// left out by the cap: fewer findings (or fewer masked fields) must
    /// not look like a cleaner message. Each report counts only the packs
    /// that would have added to it: an anonymization pack left out does
    /// not change an HL7 v2 validation.
    pub fn cap_message(&self, limit: Option<usize>, kind: RuleKind) -> Option<String> {
        let gated = self.gated_count(limit, kind);
        (gated > 0).then(|| {
            format!(
                "{} plugin pack{} not applied: the Community edition runs {} packs at a time (Settings → Plugins)",
                gated,
                if gated == 1 { "" } else { "s" },
                limit.unwrap_or(0),
            )
        })
    }

    /// [`cap_message`](Self::cap_message) for an HL7 v2 validation report.
    pub fn cap_notice(&self, limit: Option<usize>) -> Option<ValidationIssue> {
        self.cap_message(limit, RuleKind::Hl7).map(|message| ValidationIssue {
            severity: Severity::Info,
            message,
            segment_idx: None,
            segment_type: None,
            field_position: None,
            rule_id: CAP_RULE_ID.into(),
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
                let rule_count = lp.pack.rule_count();
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
                    error: lp.error.as_ref().map(PluginNotice::english),
                    warnings: lp.warnings.iter().map(PluginNotice::english).collect(),
                    error_notice: lp.error.clone(),
                    notices: lp.warnings.clone(),
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
    let failed = |label: &str, error: PluginNotice| LoadedPlugin {
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
        patterns: vec![],
    };
    // metadata() follows symlinks: a link to a pack is fine, a link to a
    // device or a pipe is not.
    let read_error = |e: std::io::Error| PluginNotice::new("read_error").arg("reason", e);
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => return failed("(read error)", read_error(e)),
    };
    if !meta.is_file() {
        return failed("(read error)", PluginNotice::new("not_regular_file"));
    }
    if meta.len() > MAX_PACK_BYTES {
        return failed(
            "(read error)",
            PluginNotice::new("too_large").arg("size_kb", meta.len() / 1024).arg("limit_kb", MAX_PACK_BYTES / 1024),
        );
    }
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => return failed("(read error)", read_error(e)),
    };
    match serde_json::from_str::<PluginPack>(strip_bom(&text)) {
        Ok(pack) => {
            // Compiled here, once for all the rules that spell a pattern,
            // and kept with the pack for every validation that follows.
            let patterns = compile_patterns(&pack);
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
                        PluginNotice::new("no_rules").arg("key", key)
                    } else {
                        PluginNotice::new("no_rules_unknown_key").arg("key", key).arg("unknown", unknown.join(", "))
                    },
                );
            }
            LoadedPlugin { warnings, pack, kind, path: path.to_path_buf(), error: None, patterns }
        }
        Err(e) => failed("(parse error)", PluginNotice::new("parse_error").arg("reason", e)),
    }
}

/// Values a pack spells in a way the engine does not know, and rules that
/// cannot do anything. The pack still loads; Settings shows the list.
fn lint(pack: &PluginPack) -> Vec<PluginNotice> {
    let mut out = Vec::new();
    // Without an id the pack still loads, but its on/off switch is saved
    // under "validation/", and a second such pack is refused as a clash.
    if pack.id.trim().is_empty() {
        out.push(PluginNotice::new("empty_id"));
    }
    if pack.name.trim().is_empty() {
        out.push(PluginNotice::new("empty_name"));
    }
    let severity = |out: &mut Vec<PluginNotice>, id: &str, s: &str| {
        if !matches!(s.trim().to_ascii_lowercase().as_str(), "error" | "warning" | "info") {
            out.push(PluginNotice::new("unknown_severity").arg("value", s).on_rule(id));
        }
    };
    for r in &pack.validation_rules {
        let rule = |n: PluginNotice| n.on_rule(&r.rule_id);
        severity(&mut out, &r.rule_id, &r.severity);
        out.extend(segment_warning(&r.segment).map(rule));
        if r.field == 0 {
            out.push(rule(PluginNotice::new("field_zero")));
        }
        if r.component == Some(0) {
            out.push(rule(PluginNotice::new("component_zero")));
        }
        if let CheckKind::Regex { pattern } = &r.check {
            out.extend(pattern_warning(pattern).map(rule));
        }
    }
    for r in &pack.fhir_rules {
        severity(&mut out, &r.rule_id, &r.severity);
        for fp in [&r.expression, &r.path].into_iter().flatten() {
            if let Err(e) = crate::parser::fhir::fhirpath::check(fp) {
                out.push(PluginNotice::new("fhirpath_invalid").arg("reason", e).on_rule(&r.rule_id));
            }
        }
        if let Some(FhirCheck::Regex { pattern }) = &r.check {
            out.extend(pattern_warning(pattern).map(|n| n.on_rule(&r.rule_id)));
        }
    }
    for r in &pack.phi_rules {
        let field = |n: PluginNotice| n.on_phi_field(format!("{}-{}", r.segment, r.field));
        out.extend(segment_warning(&r.segment).map(field));
        if !matches!(r.sensitivity.trim().to_ascii_lowercase().as_str(), "high" | "medium" | "low") {
            out.push(field(PluginNotice::new("unknown_sensitivity").arg("value", &r.sensitivity)));
        }
        if r.field == 0 || (segment_key(&r.segment) == "MSH" && r.field <= 2) {
            out.push(field(PluginNotice::new("not_maskable")));
        }
    }
    out
}

/// A segment name as rules and messages are matched by it: trimmed and in
/// capitals. The anonymizer reads names the same way, so a rule on "pid"
/// checks PID, also in a message that indents its lines or writes "pid".
fn segment_key(name: &str) -> String {
    name.trim().to_ascii_uppercase()
}

/// A warning for a segment name not written as HL7 writes one: three
/// capital letters or digits, the first a letter. A name in lower case or
/// with spaces still works, but the author should know it was read so.
fn segment_warning(segment: &str) -> Option<PluginNotice> {
    let key = segment_key(segment);
    let well_formed = key.len() == 3
        && key.starts_with(|c: char| c.is_ascii_uppercase())
        && key.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
    if !well_formed {
        Some(PluginNotice::new("segment_malformed").arg("segment", segment))
    } else if key != segment {
        Some(PluginNotice::new("segment_case").arg("segment", segment).arg("key", key))
    } else {
        None
    }
}

/// The warning for a rule whose pattern does not compile, or compiles to
/// something large enough to slow every load of the pack.
fn pattern_warning(pattern: &str) -> Option<PluginNotice> {
    let compiled = compiled(pattern);
    match &compiled.regex {
        Err(regex::Error::CompiledTooBig(limit)) => Some(PluginNotice::new("pattern_too_big").arg("mb", limit >> 20)),
        Err(e) => Some(PluginNotice::new("pattern_invalid").arg("reason", regex_reason(&e.to_string()))),
        Ok(_) if compiled.heavy => Some(PluginNotice::new("pattern_heavy").arg("mb", HEAVY_PATTERN_BYTES >> 20)),
        Ok(_) => None,
    }
}

/// The reason in a `regex` error. Its message spans several lines (a
/// header, the pattern, a caret, then `error: …`): the reason is the last.
pub(crate) fn regex_reason(s: &str) -> &str {
    let mut lines = s.lines().map(str::trim).filter(|l| !l.is_empty());
    let first = lines.clone().next().unwrap_or(s);
    lines.find_map(|l| l.strip_prefix("error:").map(str::trim)).unwrap_or(first)
}

// --- Patterns ----------------------------------------------------------------
//
// A pattern used to be compiled when its pack loaded (to lint it) and then
// again for every rule on every validation. Everyday patterns compile in
// well under a millisecond, but a Unicode class repeated many times does
// not: `\w{300}` takes about 50 ms just to fail, so a 55 KB pack of 400
// such rules held startup back by 20 seconds and every validation by as
// long again. Each pattern is now compiled once per load.

/// Compiled size over which a pattern is reported as slow to compile.
/// Everyday patterns (`^\d{4}-\d{2}-\d{2}$`, `^[A-Z0-9]{1,20}$`) stay in
/// the tens of KB; `^[\p{L} .-]{1,100}$` takes about 4 MB and 20 ms.
const HEAVY_PATTERN_BYTES: usize = 2 << 20;

/// A `regex` check's pattern, compiled.
#[derive(Debug)]
struct CompiledPattern {
    regex: Result<regex::Regex, regex::Error>,
    /// Compiled to more than [`HEAVY_PATTERN_BYTES`].
    heavy: bool,
}

impl CompiledPattern {
    fn compile(pattern: &str) -> Self {
        // Built within the lower limit first: almost every pattern fits,
        // and that build is the regex used. A pattern that does not is
        // built again under the regex crate's own limit (10 MB).
        match regex::RegexBuilder::new(pattern).size_limit(HEAVY_PATTERN_BYTES).build() {
            Err(regex::Error::CompiledTooBig(_)) => Self { regex: regex::Regex::new(pattern), heavy: true },
            regex => Self { regex, heavy: false },
        }
    }
}

/// Every compiled pattern still in use, by its text. The packs hold theirs
/// ([`LoadedPlugin::patterns`]); the validators, which receive plain rules
/// cloned out of the registry, find them here. A reload finds the patterns
/// of the packs it replaces, so only new or changed ones are compiled, and
/// a pattern no pack uses any more is freed with the last pack holding it.
static PATTERNS: LazyLock<Mutex<HashMap<String, Weak<CompiledPattern>>>> = LazyLock::new(Default::default);

/// `pattern` compiled: the copy in use when there is one, a new one
/// otherwise (a rule being tried in the editor, or one whose pack is gone).
fn compiled(pattern: &str) -> Arc<CompiledPattern> {
    if let Some(p) = lock(&PATTERNS).get(pattern).and_then(Weak::upgrade) {
        return p;
    }
    // Compiled without the lock: a slow pattern must not hold up the
    // lookups of a validation running meanwhile.
    let p = Arc::new(CompiledPattern::compile(pattern));
    let mut patterns = lock(&PATTERNS);
    // Another thread may have compiled it meanwhile: keep the copy already
    // listed, which a pack may hold, rather than list one about to go.
    if let Some(listed) = patterns.get(pattern).and_then(Weak::upgrade) {
        return listed;
    }
    patterns.retain(|_, w| w.strong_count() > 0);
    patterns.insert(pattern.to_string(), Arc::downgrade(&p));
    p
}

/// The pack's `regex` patterns, compiled, each once however many of its
/// rules spell it.
fn compile_patterns(pack: &PluginPack) -> Vec<Arc<CompiledPattern>> {
    let validation = pack.validation_rules.iter().filter_map(|r| match &r.check {
        CheckKind::Regex { pattern } => Some(pattern),
        _ => None,
    });
    let fhir = pack.fhir_rules.iter().filter_map(|r| match &r.check {
        Some(FhirCheck::Regex { pattern }) => Some(pattern),
        _ => None,
    });
    let mut seen = HashSet::new();
    validation.chain(fhir).filter(|p| seen.insert(p.as_str())).map(|p| compiled(p)).collect()
}

/// A rule's check, its pattern compiled.
enum Compiled<'a> {
    Regex(Arc<CompiledPattern>),
    Plain(&'a CheckKind),
}

impl<'a> Compiled<'a> {
    fn new(check: &'a CheckKind) -> Result<Self, String> {
        match check {
            CheckKind::Regex { pattern } => {
                let p = compiled(pattern);
                match &p.regex {
                    Ok(_) => Ok(Compiled::Regex(p)),
                    Err(e) => Err(format!("invalid pattern: {}", regex_reason(&e.to_string()))),
                }
            }
            other => Ok(Compiled::Plain(other)),
        }
    }

    fn passes(&self, value: &str) -> bool {
        match self {
            Compiled::Regex(p) => p.regex.as_ref().is_ok_and(|re| re.is_match(value)),
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
    let names: Vec<String> = msg.segments.iter().map(|s| segment_key(&s.segment_type)).collect();
    let mut issues = Vec::new();
    for rule in rules {
        let segment = segment_key(&rule.segment);
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
        // A field that must not be empty is empty when its segment is not
        // there at all: "PID-3 is required" fires on a message with no PID.
        // Every other check has no value to disagree with then, as with a
        // FHIR rule whose path selects nothing.
        if !names.contains(&segment) {
            if matches!(rule.check, CheckKind::NotEmpty) {
                issues.push(ValidationIssue {
                    severity: parse_severity(&rule.severity),
                    message: rule.message.clone(),
                    segment_idx: None,
                    segment_type: Some(segment.clone()),
                    field_position: Some(rule.field),
                    rule_id: rule.rule_id.clone(),
                });
            }
            continue;
        }
        for (seg_idx, seg) in msg.segments.iter().enumerate() {
            if names[seg_idx] != segment { continue; }

            let raw_value = seg
                .fields
                .iter()
                .find(|f| f.position == rule.field)
                .map(|f| f.span.as_str(&msg.raw))
                .unwrap_or("");
            // MSH-1 and MSH-2 are the delimiters themselves.
            let delimiters = segment == "MSH" && rule.field <= 2;
            let repetitions: Vec<&str> = if delimiters { vec![raw_value] } else { raw_value.split(rep).collect() };
            let passed = repetitions.iter().all(|r| {
                let value = match rule.component {
                    Some(c) if !delimiters => r.split(comp).nth(c.saturating_sub(1)).unwrap_or(""),
                    _ => r,
                };
                // The check sees the value as it reads, its escapes
                // resolved: O\T\BRIEN is O&BRIEN, so "contains &" holds
                // and a length counts the "&" once.
                if delimiters {
                    check.passes(value)
                } else {
                    check.passes(&crate::parser::hl7::interpret::unescape(value, &msg.delimiters))
                }
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
    run_fhir_validations_within(root, rules, &mut crate::parser::fhir::fhirpath::Budget::for_validation())
}

/// [`run_fhir_validations`] under `budget`, which all the rules share
/// with each other and with whatever else the caller evaluates under it.
/// Once it runs out, the rules left are not run, and one finding says how
/// many: a validation that stops must not look complete.
pub fn run_fhir_validations_within(
    root: &serde_json::Value,
    rules: &[FhirRule],
    budget: &mut crate::parser::fhir::fhirpath::Budget,
) -> Vec<crate::parser::fhir::FhirValidationIssue> {
    let mut issues = Vec::new();
    let mut not_run = 0usize;
    // Every rule resolves in `root`: its index is built once.
    let mut references = crate::parser::fhir::fhirpath::References::default();
    for (target, prefix) in fhir_targets(root) {
        for rule in rules {
            if let Some(required) = &rule.resource {
                if target.get("resourceType").and_then(|v| v.as_str()) != Some(required.as_str()) {
                    continue;
                }
            }
            if budget.exhausted() {
                not_run += 1;
                continue;
            }
            match apply_fhir_rule(rule, target, root, &prefix, budget, &mut references) {
                RuleOutcome::Holds => {}
                RuleOutcome::Fails(issue) => issues.push(issue),
                RuleOutcome::Stopped => not_run += 1,
            }
        }
    }
    if not_run > 0 {
        issues.push(rules_stopped(not_run, budget));
    }
    issues
}

/// The finding for rules a validation did not run because its budget ran
/// out. An error, so a CI run fails rather than passing unchecked.
fn rules_stopped(
    not_run: usize,
    budget: &crate::parser::fhir::fhirpath::Budget,
) -> crate::parser::fhir::FhirValidationIssue {
    crate::parser::fhir::FhirValidationIssue {
        severity: "error".into(),
        message: format!(
            "The FHIR rules were stopped: on this resource they take too long to evaluate \
             (more than {} in all), so {} rule {} not run. Narrow the slow rules down, \
             or switch their pack off in Settings → Plugins.",
            budget.limits(),
            not_run,
            if not_run == 1 { "check was" } else { "checks were" },
        ),
        path: "resourceType".into(),
        rule_id: None,
    }
}

/// What one rule found on one resource.
enum RuleOutcome {
    Holds,
    Fails(crate::parser::fhir::FhirValidationIssue),
    /// Cut short because the validation's shared budget ran out: the rule
    /// is not to blame, and is counted with the ones not run.
    Stopped,
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

fn apply_fhir_rule<'a>(
    rule: &FhirRule,
    target: &'a serde_json::Value,
    scope: &'a serde_json::Value,
    prefix: &str,
    budget: &mut crate::parser::fhir::fhirpath::Budget,
    references: &mut crate::parser::fhir::fhirpath::References<'a>,
) -> RuleOutcome {
    use crate::parser::fhir::fhirpath;

    let fail = |detail: Option<String>, path: String| {
        RuleOutcome::Fails(crate::parser::fhir::FhirValidationIssue {
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
        let result = fhirpath::evaluate_keeping(expression, target, scope, budget, references);
        if let Some(e) = result.error {
            if budget.exhausted() {
                return RuleOutcome::Stopped;
            }
            // A broken rule is worth reporting: silently passing would hide
            // the fact that the check never ran.
            return fail(
                Some(format!("rule {} failed to evaluate: {}", rule.rule_id, e)),
                format!("{}{}", prefix, expression),
            );
        }
        let holds = matches!(result.results.as_slice(), [serde_json::Value::Bool(true)]);
        return if holds {
            RuleOutcome::Holds
        } else {
            fail(None, format!("{}{}", prefix, expression))
        };
    }

    // Selector form: evaluate the path, then apply the check to the result.
    let (Some(path), Some(check)) = (rule.path.as_deref(), rule.check.as_ref()) else {
        return RuleOutcome::Holds;
    };
    let result = fhirpath::evaluate_keeping(path, target, scope, budget, references);
    if let Some(e) = result.error {
        if budget.exhausted() {
            return RuleOutcome::Stopped;
        }
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
        None => RuleOutcome::Holds,
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
        FhirCheck::Regex { pattern } => match &compiled(pattern).regex {
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
        CheckKind::Regex { pattern } => compiled(pattern).regex.as_ref().is_ok_and(|re| re.is_match(value)),
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

    /// A required field is missing when its segment is: the rule fires once.
    /// Checks on a value have nothing to check then and pass.
    #[test]
    fn a_missing_segment_fails_not_empty_only() {
        let msg = sample_msg();
        let rule = |check: CheckKind| ValidationRule {
            rule_id: "OBX-REQ".into(),
            severity: "error".into(),
            segment: "obx".into(),
            field: 5,
            component: None,
            check,
            message: "OBX-5 is required".into(),
        };
        let issues = run_custom_validations(&msg, &[rule(CheckKind::NotEmpty)]);
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(issues[0].segment_idx, None);
        assert_eq!(issues[0].segment_type.as_deref(), Some("OBX"));
        assert!(run_custom_validations(&msg, &[rule(CheckKind::MaxLength { max: 1 })]).is_empty());
        assert!(run_custom_validations(&msg, &[rule(CheckKind::OneOf { values: vec!["X".into()] })]).is_empty());
    }

    /// Checks read values with their escapes resolved, as the message means
    /// them: O\T\BRIEN is O&BRIEN.
    #[test]
    fn checks_see_escapes_resolved() {
        let msg = Hl7Lexer::new()
            .parse(b"MSH|^~\\&|A|B|C|D|20260415120000||ADT^A01|CTRL1|P|2.5\rPID|1||MRN001||O\\T\\BRIEN^JOHN\r".to_vec())
            .unwrap();
        let rule = |check: CheckKind| ValidationRule {
            rule_id: "NAME".into(),
            severity: "warning".into(),
            segment: "PID".into(),
            field: 5,
            component: Some(1),
            check,
            message: "m".into(),
        };
        // "contains &" holds: the value is O&BRIEN.
        assert!(run_custom_validations(&msg, &[rule(CheckKind::Contains { value: "&".into() })]).is_empty());
        // Seven characters, not the nine written.
        assert!(run_custom_validations(&msg, &[rule(CheckKind::MaxLength { max: 7 })]).is_empty());
        assert_eq!(run_custom_validations(&msg, &[rule(CheckKind::Contains { value: "\\T\\".into() })]).len(), 1);
        // MSH-2 is the delimiters themselves, never unescaped.
        let msh2 = ValidationRule { segment: "MSH".into(), field: 2, component: None, ..rule(CheckKind::OneOf { values: vec!["^~\\&".into()] }) };
        assert!(run_custom_validations(&msg, &[msh2]).is_empty());
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

    /// A rule that is slow on each entry of a large Bundle must not take a
    /// whole evaluation's budget for every one of them: the rules of one
    /// validation share one budget. A rule past its own limit is reported;
    /// once the shared one is spent the rest are not run, and one finding
    /// says how many, so the validation does not look complete.
    #[test]
    fn a_validation_stops_once_its_rules_have_spent_its_budget() {
        use crate::parser::fhir::fhirpath::Budget;
        let names: Vec<serde_json::Value> = (0..10).map(|i| serde_json::json!({"family": format!("F{i}")})).collect();
        let entries: Vec<serde_json::Value> = (0..50)
            .map(|i| serde_json::json!({"resource": {"resourceType": "Patient", "id": format!("p{i}"), "name": names}}))
            .collect();
        let bundle = serde_json::json!({"resourceType": "Bundle", "type": "collection", "entry": entries});
        // About 15 000 steps on each Patient.
        let mut nested = "%resource.name".to_string();
        for _ in 0..3 {
            nested = format!("%resource.name.where({nested}.exists())");
        }
        let slow = FhirRule {
            expression: Some(format!("{nested}.count() > 5")),
            ..fhir_rule("slow")
        };
        let hour = std::time::Duration::from_secs(3600);

        // Each evaluation may take 10 000 steps, the validation 25 000: two
        // entries break their own limit, the third the shared one.
        let mut budget = Budget::with_limits(25_000, hour).each_at_most(10_000, hour);
        let issues = run_fhir_validations_within(&bundle, std::slice::from_ref(&slow), &mut budget);
        assert_eq!(issues.len(), 3, "{issues:#?}");
        for (i, issue) in issues[..2].iter().enumerate() {
            assert_eq!(issue.rule_id.as_deref(), Some("slow"));
            assert!(issue.path.starts_with(&format!("entry[{i}].resource.")), "{}", issue.path);
            assert!(issue.message.contains("failed to evaluate"), "{}", issue.message);
            assert!(issue.message.contains("more than 10000 steps"), "{}", issue.message);
        }
        let stopped = &issues[2];
        assert_eq!(stopped.rule_id, None);
        assert_eq!(stopped.severity, "error");
        assert!(stopped.message.contains("The FHIR rules were stopped"), "{}", stopped.message);
        assert!(stopped.message.contains("more than 25000 steps or 3600 seconds in all"), "{}", stopped.message);
        assert!(stopped.message.contains("48 rule checks were not run"), "{}", stopped.message);

        // With room for all of them, every entry is checked.
        let mut budget = Budget::with_limits(10_000_000, hour);
        let issues = run_fhir_validations_within(&bundle, std::slice::from_ref(&slow), &mut budget);
        assert!(issues.is_empty(), "{issues:#?}");
        // A validation's own budget holds about two evaluations.
        assert_eq!(
            Budget::for_validation().steps_left(),
            2 * crate::parser::fhir::fhirpath::MAX_EVAL_STEPS
        );
    }

    /// A rule that resolves a reference on every entry walks the Bundle
    /// once for the whole validation, not once per entry: on a large Bundle
    /// that was most of the time, and would now be most of the budget.
    #[test]
    fn rules_on_each_entry_share_one_reference_index() {
        use crate::parser::fhir::fhirpath::Budget;
        let observations = |n: usize| {
            let mut entries = vec![serde_json::json!({"fullUrl": "urn:uuid:p",
                "resource": {"resourceType": "Patient", "name": [{"family": "Y"}]}})];
            entries.extend((0..n).map(|i| serde_json::json!({"fullUrl": format!("urn:uuid:o{i}"),
                "resource": {"resourceType": "Observation", "status": "final", "code": {"text": "x"},
                    "subject": {"reference": "urn:uuid:p"}}})));
            serde_json::json!({"resourceType": "Bundle", "type": "collection", "entry": entries})
        };
        let rule = FhirRule {
            resource: Some("Observation".into()),
            expression: Some("subject.resolve().exists()".into()),
            ..fhir_rule("subject-resolves")
        };
        let steps = |n: usize| {
            let mut budget = Budget::with_limits(1_000_000_000, std::time::Duration::from_secs(3600));
            let issues = run_fhir_validations_within(&observations(n), std::slice::from_ref(&rule), &mut budget);
            assert!(issues.is_empty(), "{issues:?}");
            1_000_000_000 - budget.steps_left()
        };
        // Ten times the entries, about ten times the work: built for each
        // entry, the index made it a hundred.
        let (small, large) = (steps(100), steps(1000));
        assert!(large < 15 * small, "{small} steps for 100 entries, {large} for 1000");
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
            patterns: vec![],
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
            write_pack(&root, "validation", &format!("{id}.json"), &pack_with_a_rule(id));
        }
        let reg = PluginRegistry::with_root(root.clone());
        reg.reload().unwrap();
        write_pack(&root, "validation", "aa-new.json", &pack_with_a_rule("aa-new"));
        reg.reload().unwrap();
        let gated: Vec<String> = reg.list(Some(3)).into_iter().filter(|p| p.gated).map(|p| p.id).collect();
        assert_eq!(gated, vec!["aa-new"]);
    }

    /// A pack of one validation rule, `<id>-01` on PID-3.
    fn pack_with_a_rule(id: &str) -> String {
        serde_json::json!({
            "id": id, "name": id,
            "validation_rules": [{"rule_id": format!("{id}-01"), "segment": "PID", "field": 3,
                "check": {"type": "not_empty"}, "message": "m"}]
        })
        .to_string()
    }

    /// The CLI records nothing in the state file, so without the app every
    /// pack was new on every run and the cap kept the first three by id: a
    /// pack added later ("aa-new") switched off one in use. The order the
    /// files appeared in decides, as it does in the app.
    #[test]
    fn without_the_app_a_new_pack_does_not_displace_one_in_use() {
        let root = temp_root();
        for id in ["pack-b", "pack-c", "pack-d"] {
            write_pack(&root, "validation", &format!("{id}.json"), &pack_with_a_rule(id));
        }
        // Creation times one apart, whatever the file system's resolution.
        std::thread::sleep(std::time::Duration::from_millis(50));
        write_pack(&root, "validation", "aa-new.json", &pack_with_a_rule("aa-new"));
        let cli = PluginRegistry::with_root(root.clone());
        cli.reload_read_only().unwrap();
        let gated: Vec<String> = cli.list(Some(3)).into_iter().filter(|p| p.gated).map(|p| p.id).collect();
        assert_eq!(gated, vec!["aa-new"]);
        assert!(!root.join(STATE_FILE).exists(), "still read only");
        // The app, starting on the same folder, keeps the same packs.
        let app = PluginRegistry::with_root(root.clone());
        app.reload().unwrap();
        let gated: Vec<String> = app.list(Some(3)).into_iter().filter(|p| p.gated).map(|p| p.id).collect();
        assert_eq!(gated, vec!["aa-new"]);
        fs::remove_dir_all(&root).ok();
    }

    /// A pack that holds no rules (here a misspelt "rules" key) checks
    /// nothing: it took one of the three slots and left a working pack
    /// inactive.
    #[test]
    fn a_pack_without_rules_takes_no_slot_under_the_cap() {
        let root = temp_root();
        write_pack(&root, "validation", "a-empty.json", r#"{"id":"a-empty","name":"Empty","rules":[{"rule_id":"X"}]}"#);
        for id in ["b", "c", "d"] {
            write_pack(&root, "validation", &format!("{id}.json"), &pack_with_a_rule(id));
        }
        let reg = PluginRegistry::with_root(root.clone());
        reg.reload().unwrap();
        let list = reg.list(Some(3));
        assert!(list.iter().all(|p| !p.gated), "{:?}", list.iter().map(|p| (&p.id, p.gated)).collect::<Vec<_>>());
        assert_eq!(reg.active_validation_rules(Some(3)).len(), 3);
        assert!(reg.cap_notice(Some(3)).is_none());
        // A fourth pack with rules is over the cap.
        write_pack(&root, "validation", "e.json", &pack_with_a_rule("e"));
        reg.reload().unwrap();
        let gated: Vec<String> = reg.list(Some(3)).into_iter().filter(|p| p.gated).map(|p| p.id).collect();
        assert_eq!(gated, vec!["e"]);
        fs::remove_dir_all(&root).ok();
    }

    /// A .state.json that does not parse was read as empty, so every pack
    /// switched off ran again, and the app's next start wrote an empty
    /// state over it. It is now reported, left as it is, and set aside
    /// only when the user saves a new choice.
    #[test]
    fn a_malformed_state_file_is_reported_and_not_written_over() {
        let root = temp_root();
        write_pack(&root, "validation", "a.json", &pack_with_a_rule("a"));
        write_pack(&root, "validation", "b.json", &pack_with_a_rule("b"));
        let broken = "{\"enabled\": {\"validation/a\": false,}}";
        fs::write(root.join(STATE_FILE), broken).unwrap();

        let cli = PluginRegistry::with_root(root.clone());
        cli.reload_read_only().unwrap();
        assert!(cli.state_problem().is_some_and(|e| e.contains("line 1")), "{:?}", cli.state_problem());

        let app = PluginRegistry::with_root(root.clone());
        app.reload().unwrap();
        assert!(app.state_problem().is_some());
        assert_eq!(fs::read_to_string(root.join(STATE_FILE)).unwrap(), broken, "the start did not write over it");

        // A new choice is saved; the unreadable file is kept beside it.
        app.set_enabled("validation/b", false).unwrap();
        assert!(app.state_problem().is_none());
        assert_eq!(fs::read_to_string(root.join(BROKEN_STATE_FILE)).unwrap(), broken);
        let again = PluginRegistry::with_root(root.clone());
        again.reload().unwrap();
        assert!(again.state_problem().is_none());
        let rules: Vec<String> = again.active_validation_rules(None).into_iter().map(|r| r.rule_id).collect();
        assert_eq!(rules, vec!["a-01"]);
        fs::remove_dir_all(&root).ok();
    }

    /// The unreadable file went to one fixed name: a second recovery made
    /// the rename fail on Windows (and so every switch), and on Unix wrote
    /// over the first backup. Each backup now gets a name of its own.
    #[test]
    fn a_second_unreadable_state_file_keeps_the_first_backup() {
        let root = temp_root();
        write_pack(&root, "validation", "a.json", &pack_with_a_rule("a"));
        fs::write(root.join(BROKEN_STATE_FILE), "first backup").unwrap();
        for (round, broken) in ["{not json", "{\"enabled\": oops}"].into_iter().enumerate() {
            fs::write(root.join(STATE_FILE), broken).unwrap();
            let reg = PluginRegistry::with_root(root.clone());
            reg.reload().unwrap();
            assert!(reg.state_problem().is_some());
            reg.set_enabled("validation/a", round == 1).expect("the switch is saved");
            assert!(reg.state_problem().is_none());
            let backup = root.join(format!("{}.{}", BROKEN_STATE_FILE, round + 1));
            assert_eq!(fs::read_to_string(&backup).unwrap(), broken);
        }
        assert_eq!(fs::read_to_string(root.join(BROKEN_STATE_FILE)).unwrap(), "first backup");
        let again = PluginRegistry::with_root(root.clone());
        again.reload().unwrap();
        assert!(again.state_problem().is_none());
        assert_eq!(again.active_validation_rules(None).len(), 1, "the last choice (on) was saved");
        fs::remove_dir_all(&root).ok();
    }

    /// Only a state file with a JSON error was kept; one that is not UTF-8
    /// (an editor that saved it as UTF-16) was written over by the next
    /// switch. Both are kept, under the same numbered names.
    #[test]
    fn a_state_file_that_is_not_utf8_is_kept_too() {
        let root = temp_root();
        write_pack(&root, "validation", "a.json", &pack_with_a_rule("a"));
        let utf16: Vec<u8> = [0xFF, 0xFE]
            .into_iter()
            .chain("{\"enabled\": {\"validation/a\": false}}".encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        let latin1 = b"{\"enabled\": {\"validation/caf\xe9\": false}}".to_vec();
        for (round, bytes) in [utf16, latin1].into_iter().enumerate() {
            fs::write(root.join(STATE_FILE), &bytes).unwrap();
            let reg = PluginRegistry::with_root(root.clone());
            reg.reload().unwrap();
            assert!(reg.state_problem().is_some(), "round {round}");
            reg.set_enabled("validation/a", false).expect("the switch is saved");
            let backup = if round == 0 { root.join(BROKEN_STATE_FILE) } else { root.join(format!("{}.1", BROKEN_STATE_FILE)) };
            assert_eq!(fs::read(&backup).unwrap(), bytes, "round {round}: kept byte for byte");
            let again = PluginRegistry::with_root(root.clone());
            again.reload().unwrap();
            assert!(again.state_problem().is_none());
            assert!(again.active_validation_rules(None).is_empty(), "the new choice (off) was saved");
        }
        fs::remove_dir_all(&root).ok();
    }

    /// Each report counts the packs left out that would have added to it:
    /// FHIR and PHI results had no note at all, and the HL7 v2 note
    /// counted packs of every kind.
    #[test]
    fn each_kind_of_result_counts_its_own_packs_left_out() {
        let root = temp_root();
        for id in ["a", "b", "c"] {
            write_pack(&root, "validation", &format!("{id}.json"), &pack_with_a_rule(id));
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        write_pack(&root, "fhir", "f.json", r#"{"id":"f","name":"F","fhir_rules":[{"rule_id":"F1","expression":"true","message":"m"}]}"#);
        write_pack(&root, "anonymization", "p.json", r#"{"id":"p","name":"P","phi_rules":[{"segment":"ZPI","field":2,"sensitivity":"high"}]}"#);
        let reg = PluginRegistry::with_root(root.clone());
        reg.reload().unwrap();
        assert!(reg.cap_notice(Some(3)).is_none(), "no validation pack is left out");
        assert_eq!(reg.gated_count(Some(3), RuleKind::Fhir), 1);
        assert_eq!(reg.gated_count(Some(3), RuleKind::Phi), 1);
        let fhir = reg.cap_message(Some(3), RuleKind::Fhir).unwrap();
        assert!(fhir.starts_with("1 plugin pack not applied"), "{fhir}");
        assert!(reg.cap_message(None, RuleKind::Phi).is_none());
        fs::remove_dir_all(&root).ok();
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
    fn problems_come_as_notices_the_app_can_translate_and_as_english() {
        let root = temp_root();
        write_pack(&root, "validation", "n.json", r#"{"id":"n","name":"N","phi_rules":[{"segment":"pid","field":0,"sensitivity":"high"}],"validation_rules":[{"rule_id":"F0","severity":"error","segment":"PID","field":0,"check":{"type":"not_empty"},"message":"m"}]}"#);
        write_pack(&root, "validation", "broken.json", "{ not json");
        let reg = PluginRegistry::with_root(root.clone());
        reg.reload().unwrap();
        let list = reg.list(None);
        let n = list.iter().find(|p| p.id == "n").unwrap();
        assert_eq!(n.notices.len(), n.warnings.len());
        for (notice, text) in n.notices.iter().zip(&n.warnings) {
            assert_eq!(&notice.english(), text);
        }
        let field0 = n.notices.iter().find(|x| x.code == "field_zero").expect("field 0 flagged");
        assert_eq!(field0.rule.as_deref(), Some("F0"));
        assert!(n.warnings.contains(&"rule F0: field 0 is the segment name, not a field (fields count from 1); the rule checks an empty value".to_string()));
        let case = n.notices.iter().find(|x| x.code == "segment_case").expect("lower-case segment flagged");
        assert_eq!(case.phi_field.as_deref(), Some("pid-0"));
        assert_eq!(case.args.get("key").map(String::as_str), Some("PID"));
        assert!(n.warnings.iter().any(|w| w == "PHI field pid-0: segment \"pid\" is read as PID; write it as PID"), "{:?}", n.warnings);

        let broken = list.iter().find(|p| p.path.ends_with("broken.json")).unwrap();
        let err = broken.error_notice.as_ref().unwrap();
        assert_eq!(err.code, "parse_error");
        assert_eq!(broken.error.as_deref(), err.args.get("reason").map(String::as_str));
        // What the frontend reads.
        let json = serde_json::to_value(field0).unwrap();
        assert_eq!(json, serde_json::json!({ "code": "field_zero", "rule": "F0", "args": {} }));
        fs::remove_dir_all(&root).ok();
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
    fn numbers_survive_the_flattened_and_tagged_parts_of_a_pack() {
        // JSON numbers keep their text (serde_json's arbitrary_precision),
        // and serde reads `flatten` and `tag` fields through a buffer that
        // sees such a number as a map: the round trip must still hold.
        let text = r#"{"id": "p", "name": "P", "threshold": 6.30, "limits": {"ratio": 1.250},
            "fhir_rules": [{"rule_id": "R1", "path": "name.family", "message": "m",
                "check": {"type": "max_length", "max": 10}, "weight": 0.50}]}"#;
        let pack: PluginPack = serde_json::from_str(text).unwrap();
        assert_eq!(pack.extra["threshold"].to_string(), "6.30");
        assert_eq!(pack.extra["limits"]["ratio"].to_string(), "1.250");
        let rule = &pack.fhir_rules[0];
        assert!(matches!(rule.check, Some(FhirCheck::MaxLength { max: 10 })), "{:?}", rule.check);
        assert_eq!(rule.extra["weight"].to_string(), "0.50");
        let again = serde_json::to_string(&pack).unwrap();
        assert!(again.contains(r#""threshold":6.30"#) && again.contains(r#""weight":0.50"#), "{again}");
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

    /// Each pattern was compiled when its pack loaded and again
    /// for every rule on every validation (400 `\w{300}` rules: 20 s at
    /// startup, 40 s per CLI run). Now once per load, shared by the rules
    /// that spell it, and reused by a reload and by every validation.
    #[test]
    fn a_pattern_is_compiled_once_per_load_and_reused() {
        // A pattern no other test uses, so the shared table is ours alone.
        let pattern = "^M14-ONCE-[0-9]{3}$";
        let root = temp_root();
        let rule = |id: &str| serde_json::json!({
            "rule_id": id, "segment": "PID", "field": 3,
            "check": {"type": "regex", "pattern": pattern}, "message": "m"
        });
        write_pack(&root, "validation", "p.json", &serde_json::json!({
            "id": "p", "name": "P", "validation_rules": [rule("R1"), rule("R2")]
        }).to_string());
        let reg = PluginRegistry::with_root(root.clone());
        reg.reload().unwrap();
        let held = |reg: &PluginRegistry| reg.plugins.read().unwrap()[0].patterns.clone();
        let first = held(&reg);
        assert_eq!(first.len(), 1, "two rules, one pattern, one compile");
        assert!(Arc::ptr_eq(&first[0], &compiled(pattern)), "validation finds the pack's copy");

        // A validation runs on the copy compiled at load.
        let msg = Hl7Lexer::new().parse(b"MSH|^~\\&|A|B|C|D|20260415120000||ADT^A01|C1|P|2.5\rPID|1||M14-ONCE-123\r".to_vec()).unwrap();
        assert!(run_custom_validations(&msg, &reg.active_validation_rules(None)).is_empty());

        reg.reload().unwrap();
        assert!(Arc::ptr_eq(&first[0], &held(&reg)[0]), "a reload compiles only new patterns");

        // Freed with the last pack that uses it.
        drop((first, reg));
        assert!(lock(&PATTERNS).get(pattern).and_then(Weak::upgrade).is_none());
        fs::remove_dir_all(&root).ok();
    }

    /// A pattern that is too large, or large enough to make
    /// every load slow, is named under its pack instead of costing time
    /// without a word.
    #[test]
    fn a_pattern_too_large_or_slow_to_compile_is_listed() {
        let rule = |id: &str, pattern: &str| ValidationRule {
            rule_id: id.into(),
            severity: "error".into(),
            segment: "PID".into(),
            field: 5,
            component: Some(1),
            check: CheckKind::Regex { pattern: pattern.into() },
            message: "m".into(),
        };
        let pack: PluginPack = serde_json::from_value(serde_json::json!({"id": "p", "name": "P"})).unwrap();
        let pack = PluginPack {
            validation_rules: vec![
                rule("BIG", r"\w{300}"),
                rule("HEAVY", r"^[\p{L} .-]{1,100}$"),
                rule("LIGHT", r"^[A-Z][A-Z -]*$"),
            ],
            ..pack
        };
        let w: Vec<String> = lint(&pack).iter().map(PluginNotice::english).collect();
        assert!(w.iter().any(|m| m.starts_with("rule BIG: the pattern is too large to compile (over 10 MB")), "{w:?}");
        assert!(w.iter().any(|m| m.starts_with("rule HEAVY: the pattern compiles to over 2 MB")), "{w:?}");
        assert!(!w.iter().any(|m| m.contains("LIGHT")), "{w:?}");
        assert_eq!(w.len(), 2, "{w:?}");

        // A heavy pattern still works; the one that does not compile says so.
        let msg = sample_msg();
        let issues = run_custom_validations(&msg, &pack.validation_rules);
        assert!(issues.iter().any(|i| i.rule_id == "BIG" && i.message.contains("cannot run")), "{issues:?}");
        assert!(!issues.iter().any(|i| i.rule_id == "HEAVY" || i.rule_id == "LIGHT"), "{issues:?}");
    }

    /// The app loads the packs on another thread so the window
    /// does not wait; a validation that comes first must wait for them, not
    /// run without them.
    #[test]
    fn a_reader_waits_for_a_background_load() {
        let root = temp_root();
        write_pack(&root, "validation", "p.json", r#"{"id":"p","name":"P","validation_rules":[{"rule_id":"R","segment":"PID","field":3,"check":{"type":"not_empty"},"message":"m"}]}"#);
        let reg = PluginRegistry::with_root(root.clone());
        reg.begin_background_load();
        std::thread::scope(|s| {
            s.spawn(|| {
                std::thread::sleep(std::time::Duration::from_millis(200));
                reg.reload().unwrap();
            });
            assert_eq!(reg.active_validation_rules(None).len(), 1, "the rules of a load still running");
        });
        fs::remove_dir_all(&root).ok();
    }

    /// Rules on "pid", on field 0 or on component 0 loaded
    /// without a word and never worked, a PHI rule on "zpi" included; an
    /// empty id loaded as a pack with a blank title. Each is now listed.
    #[test]
    fn rules_written_in_ways_that_never_match_are_listed() {
        let root = temp_root();
        write_pack(&root, "validation", "x.json", &serde_json::json!({
            "id": " ", "name": "", "validation_rules": [
                {"rule_id": "LOWER", "segment": "pid", "field": 3, "check": {"type": "not_empty"}, "message": "m"},
                {"rule_id": "BAD", "segment": "PIDX", "field": 3, "check": {"type": "not_empty"}, "message": "m"},
                {"rule_id": "FIELD0", "segment": "PID", "field": 0, "check": {"type": "not_empty"}, "message": "m"},
                {"rule_id": "COMP0", "segment": "PID", "field": 5, "component": 0, "check": {"type": "not_empty"}, "message": "m"},
                {"rule_id": "FINE", "segment": "PID", "field": 5, "component": 1, "check": {"type": "not_empty"}, "message": "m"}
            ],
            "phi_rules": [
                {"segment": "zpi", "field": 3, "sensitivity": "high"},
                {"segment": "msh", "field": 2, "sensitivity": "high"}
            ]
        }).to_string());
        let reg = PluginRegistry::with_root(root.clone());
        reg.reload().unwrap();
        let w = reg.list(None).remove(0).warnings;
        let has = |needle: &str| w.iter().any(|m| m.contains(needle));
        assert!(has("empty \"id\""), "{w:?}");
        assert!(has("empty \"name\""), "{w:?}");
        assert!(has("rule LOWER: segment \"pid\" is read as PID"), "{w:?}");
        assert!(has("rule BAD: \"PIDX\" is not a segment name"), "{w:?}");
        assert!(has("rule FIELD0: field 0 is the segment name"), "{w:?}");
        assert!(has("rule COMP0: component 0 does not exist"), "{w:?}");
        assert!(has("PHI field zpi-3: segment \"zpi\" is read as ZPI"), "{w:?}");
        assert!(has("PHI field msh-2: not a maskable field"), "{w:?}");
        assert!(!has("FINE"), "{w:?}");
        fs::remove_dir_all(&root).ok();
    }

    /// Segment names match as the anonymizer matches them,
    /// trimmed and in capitals, on both sides.
    #[test]
    fn segment_names_match_in_any_case() {
        let rule = |segment: &str| ValidationRule {
            rule_id: format!("R-{segment}"),
            severity: "error".into(),
            segment: segment.into(),
            field: 3,
            component: None,
            check: CheckKind::NotEmpty,
            message: "PID-3 is required".into(),
        };
        let empty_pid3 = Hl7Lexer::new().parse(b"MSH|^~\\&|A|B|C|D|20260415120000||ADT^A01|C1|P|2.5\rPID|1||\r".to_vec()).unwrap();
        for segment in ["PID", "pid", " Pid "] {
            let issues = run_custom_validations(&empty_pid3, &[rule(segment)]);
            assert_eq!(issues.len(), 1, "a rule on {segment:?}: {issues:?}");
            assert_eq!(issues[0].segment_idx, Some(1));
        }
        let lower_segment = Hl7Lexer::new().parse(b"MSH|^~\\&|A|B|C|D|20260415120000||ADT^A01|C1|P|2.5\rpid|1||\r".to_vec()).unwrap();
        assert_eq!(run_custom_validations(&lower_segment, &[rule("PID")]).len(), 1, "a message that writes \"pid\"");
    }
}

