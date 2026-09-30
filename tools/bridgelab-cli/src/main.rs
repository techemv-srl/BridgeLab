//! bridgelab-cli — BridgeLab's validators from the command line.
//!
//! A thin front-end over `bridgelab_lib`: the same HL7 v2 parser and
//! validator, the same FHIR checks and profile engine (the built-in R4
//! core plus any installed package), the same plugin packs and PHI
//! anonymiser the desktop app runs. No licence is read and none is needed:
//! the CLI is free, including batch validation, PHI masking, FHIRPath and
//! installed FHIR packages. The one exception is the XSD export, which
//! offers the Community set (four common v2.5 messages); the full
//! catalogue is part of BridgeLab Pro. It is built without the BUSL `pro`
//! feature.
//!
//! Every command that reads one message accepts `-` for standard input.
//!
//! Exit codes: 0 clean, 1 errors found (or a file failed to parse or to
//! read, a path matched no file, a test failed, a send was not
//! acknowledged with AA/CA), 2 usage error.

use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process;

use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use serde::Serialize;

/// `println!` that does not panic when standard output goes away (a
/// `| head` that has read enough): the process ends quietly instead, with
/// the status a shell reports for SIGPIPE.
macro_rules! outln {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        if let Err(e) = writeln!(std::io::stdout(), $($arg)*) {
            stdout_failed(e);
        }
    }};
}

/// `print!` counterpart of [`outln!`].
macro_rules! out {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        if let Err(e) = write!(std::io::stdout(), $($arg)*) {
            stdout_failed(e);
        }
    }};
}

/// Text for a terminal: control characters other than tab and newline
/// become visible `\x1B`-style escapes. An escape sequence in a message
/// field or in a peer's ACK could otherwise clear the screen, retitle the
/// window or draw a fake "OK" over the real findings.
fn terminal_safe(s: &str) -> std::borrow::Cow<'_, str> {
    let unsafe_char = |c: char| c.is_control() && c != '\t' && c != '\n';
    if !s.chars().any(unsafe_char) {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        if unsafe_char(c) {
            out.push_str(&format!("\\x{:02X}", c as u32));
        } else {
            out.push(c);
        }
    }
    std::borrow::Cow::Owned(out)
}

/// [`outln!`] for text built from message content: see [`terminal_safe`].
macro_rules! safeln {
    ($($arg:tt)*) => {{
        outln!("{}", terminal_safe(&format!($($arg)*)));
    }};
}

/// `eprintln!` counterpart of [`safeln!`].
macro_rules! esafeln {
    ($($arg:tt)*) => {{
        eprintln!("{}", terminal_safe(&format!($($arg)*)));
    }};
}

fn stdout_failed(e: std::io::Error) -> ! {
    if e.kind() == std::io::ErrorKind::BrokenPipe {
        process::exit(141);
    }
    eprintln!("Error: cannot write to standard output: {}", e);
    process::exit(1);
}

/// Report format of `validate` and `test`.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    Text,
    Json,
    Junit,
}

use bridgelab_lib::anonymization::{self, ExtraPhiField};
use bridgelab_lib::licensing::feature_gate::COMMUNITY_MAX_ACTIVE_PLUGINS;
use bridgelab_lib::parser::fhir::{self, profile::package as fhir_package, profile::ProfileRegistry, FhirFormat};
use bridgelab_lib::parser::hl7::schema::{self as hl7schema, Hl7Version};
use bridgelab_lib::test_packs;
use bridgelab_lib::communication::mllp;
use bridgelab_lib::parser::hl7::charset::{self, Decoded};
use bridgelab_lib::parser::hl7::lexer::Hl7Lexer;
use bridgelab_lib::parser::hl7::message::Hl7Message;
use bridgelab_lib::plugins::{self, PluginRegistry};
use bridgelab_lib::utils::atomic_write::write_atomic;
use bridgelab_lib::validation::{self, Severity};

#[derive(Parser)]
#[command(
    name = "bridgelab-cli",
    version,
    about = "HL7 v2 and FHIR validation, anonymization and conversion for CI/CD — the BridgeLab validators, headless"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate HL7 v2 or FHIR files (format detected per file)
    Validate {
        /// Files or glob patterns to validate (- for standard input)
        #[arg(required = true)]
        paths: Vec<String>,
        /// Output format
        #[arg(short, long, value_enum, ignore_case = true, default_value = "text")]
        format: Format,
        /// Exit 1 when a file has errors or fails to parse (the default;
        /// --strict=false or --no-strict to exit 0 anyway)
        #[arg(
            short,
            long,
            action = ArgAction::Set,
            num_args = 0..=1,
            require_equals = true,
            default_missing_value = "true",
            default_value_t = true
        )]
        strict: bool,
        /// Exit 0 even when files have errors or fail to parse (a path that
        /// matches no file still exits 1)
        #[arg(long)]
        no_strict: bool,
        /// Directory of distilled FHIR packages to use instead of the app's
        /// config directory (the built-in R4 core is always included)
        #[arg(long, value_name = "DIR")]
        fhir_packages: Option<PathBuf>,
        /// Ignore plugin packs from the app's config directory
        #[arg(long)]
        no_plugins: bool,
    },
    /// Show info about messages (type, version, segment count)
    Info {
        /// Files or glob patterns (- for standard input)
        #[arg(required = true)]
        paths: Vec<String>,
        /// Output as JSON instead of a table
        #[arg(long)]
        json: bool,
    },
    /// Anonymize an HL7 v2 message (mask PHI fields)
    Anonymize {
        /// Input file, or - for standard input
        input: PathBuf,
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Ignore plugin packs from the app's config directory
        #[arg(long)]
        no_plugins: bool,
    },
    /// Convert an HL7 v2 message to a JSON representation
    ToJson {
        /// Input HL7 file, or - for standard input
        input: PathBuf,
        /// Output JSON file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Evaluate a FHIRPath expression against a FHIR resource (JSON or XML)
    Fhirpath {
        /// The expression, e.g. "Patient.name.family"
        expression: String,
        /// Resource file, or - for standard input
        #[arg(default_value = "-")]
        input: PathBuf,
        /// Print the full result (values, count, trace) as JSON
        #[arg(long)]
        json: bool,
    },
    /// Export the XSD of an HL7 v2 message structure (Community set: ADT_A01,
    /// ADT_A40, ORM_O01, ORU_R01 in v2.5; the full catalogue is in BridgeLab Pro)
    Xsd {
        /// Message structure, e.g. ADT_A01 (ADT^A01 is accepted too)
        message: String,
        /// HL7 version
        #[arg(long = "hl7-version", default_value = "2.5")]
        hl7_version: String,
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Send an HL7 v2 message over MLLP and print the ACK
    Send {
        /// Message file, or - for standard input
        input: PathBuf,
        /// Receiving host
        #[arg(long)]
        host: String,
        /// Receiving port
        #[arg(long)]
        port: u16,
        /// Timeout in seconds (1 to 86400) for connecting, for each write
        /// while sending (it restarts whenever the peer accepts data) and
        /// for the ACK
        #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u64).range(1..=86_400))]
        timeout: u64,
        /// Character encoding for the wire, e.g. ISO-8859-1 (default: the
        /// input's own, UTF-8 for UTF-8 files; a UTF-16 file goes out in the
        /// charset its MSH-18 declares, else UTF-8). A message with
        /// characters the encoding cannot hold is not sent
        #[arg(long)]
        encoding: Option<String>,
        /// Print the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Run test case packs (.bltests.json, exported from the Test Case Library)
    Test {
        /// Pack files or glob patterns
        #[arg(required = true)]
        packs: Vec<String>,
        /// Output format
        #[arg(short, long, value_enum, ignore_case = true, default_value = "text")]
        format: Format,
        /// Directory of distilled FHIR packages (see `validate`)
        #[arg(long, value_name = "DIR")]
        fhir_packages: Option<PathBuf>,
        /// Ignore plugin packs from the app's config directory
        #[arg(long)]
        no_plugins: bool,
    },
    /// Validate every message in a directory and its subdirectories
    Batch {
        /// Directory path
        dir: PathBuf,
        /// File extensions to scan, comma-separated; any case, with or
        /// without the dot (e.g. hl7,txt,dat)
        #[arg(short, long, default_value = "hl7")]
        extension: String,
        /// Output the summary as JSON
        #[arg(long)]
        json: bool,
        /// Directory of distilled FHIR packages (see `validate`)
        #[arg(long, value_name = "DIR")]
        fhir_packages: Option<PathBuf>,
        /// Ignore plugin packs from the app's config directory
        #[arg(long)]
        no_plugins: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        Commands::Validate { paths, format, strict, no_strict, fhir_packages, no_plugins } => {
            cmd_validate(&paths, format, strict && !no_strict, fhir_packages.as_deref(), no_plugins)
        }
        Commands::Info { paths, json } => cmd_info(&paths, json),
        Commands::Anonymize { input, output, no_plugins } => {
            cmd_anonymize(&input, output.as_deref(), no_plugins)
        }
        Commands::ToJson { input, output } => cmd_to_json(&input, output.as_deref()),
        Commands::Fhirpath { expression, input, json } => cmd_fhirpath(&expression, &input, json),
        Commands::Xsd { message, hl7_version, output } => cmd_xsd(&message, &hl7_version, output.as_deref()),
        Commands::Send { input, host, port, timeout, encoding, json } => {
            cmd_send(&input, &host, port, timeout, encoding.as_deref().unwrap_or(""), json)
        }
        Commands::Test { packs, format, fhir_packages, no_plugins } => {
            cmd_test(&packs, format, fhir_packages.as_deref(), no_plugins)
        }
        Commands::Batch { dir, extension, json, fhir_packages, no_plugins } => {
            cmd_batch(&dir, &extension, json, fhir_packages.as_deref(), no_plugins)
        }
    };
    process::exit(code);
}

// ---------------------------------------------------------------------------
// The validators, loaded once per run.

struct Engines {
    plugins: Option<PluginRegistry>,
    /// The note the app adds to an HL7 v2 report when packs are over the
    /// Community cap.
    cap_note: Option<Issue>,
    profiles: ProfileRegistry,
    /// Set when `--fhir-packages` gave nothing usable: added to every FHIR
    /// report, so a mistyped path never reads as a clean run.
    packages_problem: Option<String>,
}

impl Engines {
    fn new(fhir_packages: Option<&Path>, no_plugins: bool) -> Self {
        let plugins = if no_plugins { None } else { Some(load_plugins()) };
        let cap_note = plugins.as_ref().and_then(|r| r.cap_notice(Self::plugin_limit())).map(|n| Issue {
            severity: severity_name(&n.severity).into(),
            message: n.message.replace("(Settings → Plugins)", "(like the Community edition; switch packs off in the app's Settings → Plugins)"),
            location: "-".into(),
            rule_id: n.rule_id,
        });
        let profiles = ProfileRegistry::new();
        let packages_problem = fhir_packages.and_then(check_packages_dir);
        if let Some(problem) = &packages_problem {
            eprintln!("warning: {}", problem);
        }
        if let Err(e) = profiles.reload_from(fhir_packages) {
            eprintln!("warning: FHIR packages not loaded: {}", e);
        }
        Self { plugins, cap_note, profiles, packages_problem }
    }

    /// Community edition: the same cap on active plugin packs the app
    /// applies without a licence.
    fn plugin_limit() -> Option<usize> {
        Some(COMMUNITY_MAX_ACTIVE_PLUGINS)
    }
}

/// Look at a `--fhir-packages` directory before it is used: each file that
/// is not a distilled package is named on stderr, and the returned problem
/// (a directory that cannot be read, or holds no package) is carried into
/// the results. The built-in R4 core applies either way.
fn check_packages_dir(dir: &Path) -> Option<String> {
    match fhir_package::scan_installed_from(dir) {
        Err(e) => Some(format!(
            "--fhir-packages {}: cannot read the directory ({}); FHIR resources are checked against the built-in R4 core only",
            dir.display(),
            e
        )),
        Ok(scan) => {
            for (path, why) in &scan.skipped {
                eprintln!("warning: FHIR package {} not loaded: {}", path.display(), why);
            }
            scan.packages.is_empty().then(|| {
                format!(
                    "--fhir-packages {}: no readable distilled package in the directory; FHIR resources are checked against the built-in R4 core only",
                    dir.display()
                )
            })
        }
    }
}

/// The plugin packs, as the app has them: on/off choices made in the app
/// apply here too. Packs that did not load, and packs over the Community
/// cap, are named on stderr — a pack silently missing from a CI run looks
/// exactly like a pack whose rules all passed. Read only: the CLI creates
/// no folder and records nothing in the app's plugin state.
fn load_plugins() -> PluginRegistry {
    let registry = PluginRegistry::new();
    if let Err(e) = registry.reload_read_only() {
        eprintln!("warning: plugin packs not loaded: {}", e);
        return registry;
    }
    for p in registry.list(Engines::plugin_limit()) {
        if let Some(e) = &p.error {
            eprintln!("warning: plugin pack {} not loaded: {}", p.path, e);
        } else if p.gated {
            eprintln!(
                "warning: plugin pack {} ({}) inactive: the CLI runs at most {} packs, like the Community edition",
                p.name, p.path, COMMUNITY_MAX_ACTIVE_PLUGINS
            );
        }
        for w in &p.warnings {
            eprintln!("warning: plugin pack {}: {}", p.path, w);
        }
    }
    registry
}

// ---------------------------------------------------------------------------
// One report shape for both families of message.

#[derive(Serialize, Clone)]
struct Issue {
    severity: String,
    message: String,
    /// `PID-5` for HL7 v2, a JSON path for FHIR
    location: String,
    /// The rule id for HL7 v2 findings, empty for FHIR
    rule_id: String,
}

#[derive(Serialize, Clone)]
struct Report {
    /// "hl7" or "fhir"
    kind: String,
    /// `ADT^A01` for HL7 v2, the resource type for FHIR
    message_type: String,
    version: String,
    valid: bool,
    error_count: usize,
    warning_count: usize,
    info_count: usize,
    issues: Vec<Issue>,
}

impl Report {
    fn from_issues(kind: &str, message_type: String, version: String, issues: Vec<Issue>) -> Self {
        let count = |s: &str| issues.iter().filter(|i| i.severity == s).count();
        let (e, w, i) = (count("error"), count("warning"), count("info"));
        Self {
            kind: kind.into(),
            message_type,
            version,
            valid: e == 0,
            error_count: e,
            warning_count: w,
            info_count: i,
            issues,
        }
    }
}

fn severity_name(s: &Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn validate_hl7(msg: &Hl7Message, engines: &Engines) -> Report {
    let mut report = validation::validate_hl7_message(msg);
    if let Some(registry) = &engines.plugins {
        let rules = registry.active_validation_rules(Engines::plugin_limit());
        if !rules.is_empty() {
            report.issues.extend(plugins::run_custom_validations(msg, &rules));
        }
    }
    let issues = report
        .issues
        .iter()
        .map(|i| Issue {
            severity: severity_name(&i.severity).into(),
            message: i.message.clone(),
            location: match (&i.segment_type, i.field_position) {
                (Some(s), Some(f)) => format!("{}-{}", s, f),
                (Some(s), None) => s.clone(),
                _ => "-".into(),
            },
            rule_id: i.rule_id.clone(),
        })
        .chain(engines.cap_note.clone())
        .collect();
    Report::from_issues("hl7", msg.message_type.clone(), msg.version.clone(), issues)
}

/// Mirrors the app's `validate_fhir` command: built-in checks, plugin
/// rules, then conformance against the installed definitions — and a note
/// when nothing defined the resource type, so "no findings" is never
/// mistaken for "checked and clean".
fn validate_fhir(resource: &fhir::FhirResource, engines: &Engines) -> Report {
    let mut issues = fhir::validate_fhir_json(resource);
    let mut profiles_applied = false;
    if let Some(json) = &resource.json_value {
        if let Some(registry) = &engines.plugins {
            let rules = registry.active_fhir_rules(Engines::plugin_limit());
            if !rules.is_empty() {
                issues.extend(plugins::run_fhir_validations(json, &rules));
            }
        }
        let (found, applied) = engines.profiles.validate_with_status(json);
        profiles_applied = applied;
        issues.extend(found);
    }
    let mut issues: Vec<Issue> = issues
        .into_iter()
        .map(|i| {
            // Profiles declared but not applied are named in JUnit as
            // "not checked", so they get a rule id to be found by.
            // Nested resources (Bundle entries, contained) carry their own
            // prefix: `Bundle.entry[0].resource.meta.profile`.
            let rule_id = if (i.path == "meta.profile" || i.path.ends_with(".meta.profile"))
                && (i.message.contains("is declared but not installed") || i.message.contains("was skipped"))
            {
                RULE_PROFILE_NOT_INSTALLED.into()
            } else {
                // A custom rule's own id, so CI can filter by it.
                i.rule_id.clone().unwrap_or_default()
            };
            Issue { severity: i.severity, message: i.message, location: i.path, rule_id }
        })
        .collect();
    if !profiles_applied {
        issues.push(Issue {
            severity: "info".into(),
            message: format!(
                "Profile conformance was not checked: no installed package defines the resource type '{}'",
                resource.resource_type
            ),
            location: "resourceType".into(),
            rule_id: RULE_NOT_CHECKED.into(),
        });
    }
    if let Some(problem) = &engines.packages_problem {
        issues.push(Issue {
            severity: "warning".into(),
            message: problem.clone(),
            location: "-".into(),
            rule_id: RULE_PACKAGES.into(),
        });
    }
    Report::from_issues(
        "fhir",
        resource.resource_type.clone(),
        resource.fhir_version.clone(),
        issues,
    )
}

/// Rule ids of the FHIR notes that mean "not (fully) checked": JUnit shows
/// a resource carrying one as skipped, never as a silent pass.
const RULE_NOT_CHECKED: &str = "PROFILE-NOT-CHECKED";
const RULE_PROFILE_NOT_INSTALLED: &str = "PROFILE-NOT-INSTALLED";
const RULE_PACKAGES: &str = "FHIR-PACKAGES";

fn not_checked_notes(report: &Report) -> Vec<&Issue> {
    report
        .issues
        .iter()
        .filter(|i| matches!(i.rule_id.as_str(), RULE_NOT_CHECKED | RULE_PROFILE_NOT_INSTALLED | RULE_PACKAGES))
        .collect()
}

/// Write an output file without ever leaving it half-written: a failed
/// write (a full disk) keeps the previous file, which matters most when
/// the output is the input itself.
fn write_output(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_atomic(path, bytes).map_err(|e| format!("cannot write {}: {}", path.display(), e))
}

/// Read a file, or standard input when the path is `-`.
fn read_input(path: &Path) -> Result<Vec<u8>, String> {
    if path.as_os_str() == "-" {
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut std::io::stdin(), &mut buf)
            .map_err(|e| format!("read failed (stdin): {}", e))?;
        Ok(buf)
    } else {
        fs::read(path).map_err(|e| format!("read failed: {}", e))
    }
}

/// Read a file or standard input as text: UTF-8 as it is, anything else
/// decoded with the charset MSH-18 declares (Windows-1252 when it declares
/// none). See `charset::decode_input`.
fn read_text(path: &Path) -> Result<Decoded, String> {
    read_input(path).map(|b| charset::decode_input(&b))
}

/// [`read_text`] for a command that works on the message's fields: a
/// charset BridgeLab cannot decode is refused (a byte of a character could
/// read as a delimiter and shift every later field), and a message that
/// declares UTF-8 without being UTF-8 is noted on stderr.
fn read_message_text(path: &Path) -> Result<Decoded, String> {
    let decoded = read_text(path)?;
    match &decoded.warning {
        Some(w) if w.is_unsupported() => Err(w.to_string()),
        Some(w) => {
            eprintln!("warning: {}: {}", path.display(), w);
            Ok(decoded)
        }
        None => Ok(decoded),
    }
}

/// Like `read_input`, refusing anything larger than `max` bytes before
/// reading it all (standard input is read up to `max` + 1 bytes).
fn read_input_limited(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let too_big = || format!("larger than {} MB", max / (1024 * 1024));
    if path.as_os_str() == "-" {
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut std::io::Read::take(std::io::stdin(), max + 1), &mut buf)
            .map_err(|e| format!("read failed (stdin): {}", e))?;
        if buf.len() as u64 > max {
            return Err(too_big());
        }
        Ok(buf)
    } else {
        let meta = fs::metadata(path).map_err(|e| format!("read failed: {}", e))?;
        if meta.len() > max {
            return Err(too_big());
        }
        fs::read(path).map_err(|e| format!("read failed: {}", e))
    }
}

/// Parse text that is not FHIR as an HL7 v2 message. Other XML (a CDA
/// document, HL7 v2 XML) gets a reason that says what it is, instead of
/// "does not start with MSH".
fn parse_hl7(text: String) -> Result<Hl7Message, String> {
    if text.trim_start_matches('\u{FEFF}').trim_start().starts_with('<') {
        return Err("an XML document that is not a FHIR resource: BridgeLab reads HL7 v2 (ER7) messages and FHIR resources, not CDA or HL7 v2 XML".into());
    }
    Hl7Lexer::new().parse(text.into_bytes())
}

/// Parse and validate one file (or stdin), whichever family it belongs to.
fn validate_file(path: &Path, engines: &Engines) -> Result<Report, String> {
    validate_bytes(read_input(path)?, engines)
}

fn validate_bytes(bytes: Vec<u8>, engines: &Engines) -> Result<Report, String> {
    let Decoded { text, warning, .. } = charset::decode_input(&bytes);
    match fhir::detect_fhir(&text) {
        Some(FhirFormat::Json) => return fhir::parse_fhir_json(&text).map(|r| validate_fhir(&r, engines)),
        Some(FhirFormat::Xml) => return fhir::parse_fhir_xml(&text).map(|r| validate_fhir(&r, engines)),
        None => {}
    }
    // Findings on wrongly split fields would be wrong findings.
    if let Some(w) = warning.as_ref().filter(|w| w.is_unsupported()) {
        return Err(w.to_string());
    }
    let msg = parse_hl7(text)?;
    let mut report = validate_hl7(&msg, engines);
    if let Some(w) = warning {
        report.issues.insert(0, Issue {
            severity: "warning".into(),
            message: w.to_string(),
            location: "MSH-18".into(),
            rule_id: "CHARSET".into(),
        });
        report = Report::from_issues("hl7", report.message_type, report.version, report.issues);
    }
    Ok(report)
}


// ---------------------------------------------------------------------------
// Commands

/// Message for an argument that names no file.
const NO_MATCH: &str = "no file matches this path or pattern";

/// Patterns match names in any case, on every OS: `*.hl7` finds `B.HL7`
/// and `c.Hl7`, as the app and `batch --extension` do (Windows and macOS
/// file systems ignore case, and a skipped file would go unnoticed).
const GLOB_OPTIONS: glob::MatchOptions = glob::MatchOptions {
    case_sensitive: false,
    require_literal_separator: false,
    require_literal_leading_dot: false,
};

/// The command-line paths and patterns, resolved.
struct Inputs {
    /// Every file to read, each once, in the order given.
    files: Vec<PathBuf>,
    /// Arguments that gave no file, and folders a pattern could not read,
    /// each with the reason: each is reported, never skipped.
    unmatched: Vec<(String, String)>,
}

/// Resolve paths and glob patterns. A path that exists is taken literally
/// (so `msg[1].hl7` is not read as a pattern); anything else is expanded
/// as a glob. An argument matching nothing is kept in `unmatched`, and a
/// file matched twice (overlapping patterns) is read once. `Err` for a
/// usage error: `-` given more than once, since standard input can be read
/// only once.
fn expand_paths(patterns: &[String]) -> Result<Inputs, String> {
    let mut files = Vec::new();
    let mut unmatched = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut add = |files: &mut Vec<PathBuf>, p: PathBuf| {
        let key = fs::canonicalize(&p).unwrap_or_else(|_| p.clone());
        if seen.insert(key) {
            files.push(p);
        }
    };
    if patterns.iter().filter(|p| p.as_str() == "-").count() > 1 {
        return Err("standard input (-) can be given only once".into());
    }
    for p in patterns {
        if p == "-" {
            files.push(PathBuf::from("-"));
            continue;
        }
        let literal = PathBuf::from(p);
        if literal.is_file() {
            add(&mut files, literal);
            continue;
        }
        if literal.is_dir() {
            unmatched.push((p.clone(), format!("a folder, not a file: `bridgelab-cli batch {}` checks every file in it", p)));
            continue;
        }
        let mut matched = false;
        match glob::glob_with(p, GLOB_OPTIONS) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(e) if e.is_file() => {
                            matched = true;
                            add(&mut files, e);
                        }
                        Ok(_) => {}
                        // A folder on the pattern's way that cannot be
                        // read: its files would be missing without a word.
                        Err(e) => unmatched.push((e.path().display().to_string(), format!("cannot read: {}", e.error()))),
                    }
                }
            }
            Err(e) => {
                unmatched.push((p.clone(), format!("not a valid pattern: {}", e)));
                continue;
            }
        }
        for f in non_utf8_matches(p) {
            matched = true;
            add(&mut files, f);
        }
        if !matched {
            unmatched.push((p.clone(), NO_MATCH.into()));
        }
    }
    Ok(Inputs { files, unmatched })
}

/// Files the last component of `pattern` matches whose names are not valid
/// UTF-8 (possible on Linux): `glob` compares names as UTF-8 text and skips
/// them. Matched on the name with invalid bytes shown as U+FFFD; only for
/// a pattern whose folder part has no wildcard.
fn non_utf8_matches(pattern: &str) -> Vec<PathBuf> {
    let path = Path::new(pattern);
    let (Some(parent), Some(name)) = (path.parent(), path.file_name().and_then(|n| n.to_str())) else {
        return vec![];
    };
    if parent.to_string_lossy().contains(['*', '?', '[']) {
        return vec![];
    }
    let Ok(pat) = glob::Pattern::new(name) else { return vec![] };
    let dir = if parent.as_os_str().is_empty() { Path::new(".") } else { parent };
    let Ok(entries) = fs::read_dir(dir) else { return vec![] };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| e.file_name().to_str().is_none())
        .filter(|e| pat.matches_with(&e.file_name().to_string_lossy(), GLOB_OPTIONS))
        .map(|e| parent.join(e.file_name()))
        .filter(|f| f.is_file())
        .collect();
    out.sort();
    out
}

/// One row of a `validate` run: a file (or an argument that matched none)
/// and what came of it.
struct ValidateRow {
    name: String,
    result: Result<Report, String>,
    /// The argument matched no file.
    missing: bool,
}

fn cmd_validate(patterns: &[String], format: Format, strict: bool, fhir_packages: Option<&Path>, no_plugins: bool) -> i32 {
    let inputs = match expand_paths(patterns) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 2;
        }
    };
    let engines = Engines::new(fhir_packages, no_plugins);

    let mut total_errors = 0;
    let mut rows: Vec<ValidateRow> = Vec::new();
    for file in &inputs.files {
        let result = validate_file(file, &engines);
        match &result {
            Ok(r) => total_errors += r.error_count,
            Err(_) => total_errors += 1,
        }
        rows.push(ValidateRow { name: file.display().to_string(), result, missing: false });
    }
    for (p, why) in &inputs.unmatched {
        esafeln!("Error: {}: {}", p, why);
        rows.push(ValidateRow { name: p.clone(), result: Err(why.clone()), missing: true });
    }

    match format {
        Format::Json => print_validate_json(&rows),
        Format::Junit => print_validate_junit(&rows),
        Format::Text => print_validate_text(&rows),
    }

    if !inputs.unmatched.is_empty() || (strict && total_errors > 0) {
        1
    } else {
        0
    }
}

fn print_validate_text(rows: &[ValidateRow]) {
    for row in rows {
        safeln!("== {} ==", row.name);
        match &row.result {
            Ok(r) => {
                safeln!(
                    "  {} {} {}",
                    r.kind.to_uppercase(),
                    r.message_type,
                    if r.version.is_empty() { String::new() } else { format!("(v{})", r.version) }
                );
                if r.issues.is_empty() {
                    safeln!("  OK (no issues)");
                } else {
                    safeln!(
                        "  Errors: {}, Warnings: {}, Info: {}",
                        r.error_count, r.warning_count, r.info_count
                    );
                    for issue in &r.issues {
                        let rule = if issue.rule_id.is_empty() { String::new() } else { format!(" {}", issue.rule_id) };
                        safeln!(
                            "  [{:7}] {}{}: {}",
                            issue.severity.to_uppercase(),
                            issue.location,
                            rule,
                            issue.message
                        );
                    }
                }
            }
            Err(e) if row.missing && e == NO_MATCH => safeln!("  NOT FOUND: {}", e),
            Err(e) if row.missing => safeln!("  INPUT ERROR: {}", e),
            Err(e) => safeln!("  PARSE ERROR: {}", e),
        }
    }
}

fn print_validate_json(rows: &[ValidateRow]) {
    #[derive(Serialize)]
    struct Row<'a> {
        file: &'a str,
        report: Option<&'a Report>,
        parse_error: Option<&'a String>,
    }
    let rows: Vec<Row> = rows
        .iter()
        .map(|r| Row {
            file: &r.name,
            report: r.result.as_ref().ok(),
            parse_error: r.result.as_ref().err(),
        })
        .collect();
    outln!("{}", serde_json::to_string_pretty(&rows).unwrap_or_default());
}

/// One testcase per file, named by its path as given. A file with errors
/// is a failure (the first error in the message, all of them in the body);
/// one that parsed clean but was not fully checked (a resource type no
/// package defines, a declared profile that is not installed, an unusable
/// `--fhir-packages`) is `skipped` with the reason; the rest pass.
fn print_validate_junit(rows: &[ValidateRow]) {
    enum Outcome<'a> {
        Pass,
        Skipped(String),
        Failure(String, String),
        Error(&'static str, &'a str),
    }
    let outcomes: Vec<(&ValidateRow, Outcome)> = rows
        .iter()
        .map(|row| {
            let outcome = match &row.result {
                Ok(r) if r.error_count > 0 => {
                    let errors: Vec<&Issue> = r.issues.iter().filter(|i| i.severity == "error").collect();
                    let first = errors.first().map(|i| i.message.as_str()).unwrap_or("");
                    let body = errors
                        .iter()
                        .map(|i| {
                            let at = if i.rule_id.is_empty() { i.location.clone() } else { format!("{} {}", i.location, i.rule_id) };
                            format!("{}: {}", at, i.message)
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    Outcome::Failure(format!("{} error(s): {}", r.error_count, first), body)
                }
                Ok(r) => {
                    let notes = not_checked_notes(r);
                    if notes.is_empty() {
                        Outcome::Pass
                    } else {
                        Outcome::Skipped(format!(
                            "not checked: {}",
                            notes.iter().map(|i| i.message.as_str()).collect::<Vec<_>>().join("; ")
                        ))
                    }
                }
                Err(e) if row.missing => Outcome::Error("INPUT", e),
                Err(e) => Outcome::Error("PARSE", e),
            };
            (row, outcome)
        })
        .collect();

    let total = outcomes.len();
    let failed = outcomes.iter().filter(|(_, o)| matches!(o, Outcome::Failure(..) | Outcome::Error(..))).count();
    let skipped = outcomes.iter().filter(|(_, o)| matches!(o, Outcome::Skipped(_))).count();

    outln!(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    outln!(r#"<testsuites name="bridgelab" tests="{}" failures="{}" skipped="{}">"#, total, failed, skipped);
    outln!(r#"  <testsuite name="validation" tests="{}" failures="{}" skipped="{}">"#, total, failed, skipped);
    for (row, outcome) in &outcomes {
        let name = xml_escape(&row.name);
        let class = match (&row.result, outcome) {
            (_, Outcome::Error(class, _)) => class.to_string(),
            (Ok(r), _) => r.kind.to_uppercase(),
            _ => String::new(),
        };
        match outcome {
            Outcome::Pass => outln!(r#"    <testcase name="{}" classname="{}" file="{}"/>"#, name, class, name),
            Outcome::Skipped(why) => {
                outln!(r#"    <testcase name="{}" classname="{}" file="{}">"#, name, class, name);
                outln!(r#"      <skipped message="{}"/>"#, xml_escape(why));
                outln!(r#"    </testcase>"#);
            }
            Outcome::Failure(message, body) => {
                outln!(r#"    <testcase name="{}" classname="{}" file="{}">"#, name, class, name);
                outln!(r#"      <failure message="{}">{}</failure>"#, xml_escape(message), xml_escape(body));
                outln!(r#"    </testcase>"#);
            }
            Outcome::Error(_, e) => {
                outln!(r#"    <testcase name="{}" classname="{}" file="{}">"#, name, class, name);
                outln!(r#"      <failure message="{}">{}</failure>"#, xml_escape(e), if row.missing { "not found" } else { "parse error" });
                outln!(r#"    </testcase>"#);
            }
        }
    }
    outln!(r#"  </testsuite>"#);
    outln!(r#"</testsuites>"#);
}

/// Escape text for an XML attribute or element (tab, LF and CR as
/// character references, so attributes keep them). Characters XML 1.0 cannot
/// carry at all (control codes other than tab, CR and LF; U+FFFE/U+FFFF)
/// become U+FFFD, so the report stays well-formed whatever a message holds.
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // As references: in an attribute, a literal tab or line break
            // is read back as a space.
            '\t' => out.push_str("&#9;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            c if (c as u32) < 0x20 || c == '\u{FFFE}' || c == '\u{FFFF}' => out.push('\u{FFFD}'),
            c => out.push(c),
        }
    }
    out
}

#[derive(Serialize)]
struct InfoRecord {
    file: String,
    /// The file was read and parsed as HL7 v2 or FHIR.
    parsed: bool,
    /// Parsed, and no error from the built-in checks (`validate` and
    /// `batch` also apply plugin packs and FHIR profiles).
    valid: bool,
    kind: String,
    message_type: String,
    version: String,
    segment_count: usize,
    size_bytes: u64,
    error: Option<String>,
}

fn blank_record(file: String, size: u64, error: String) -> InfoRecord {
    InfoRecord {
        file,
        parsed: false,
        valid: false,
        kind: String::new(),
        message_type: String::new(),
        version: String::new(),
        segment_count: 0,
        size_bytes: size,
        error: Some(error),
    }
}

fn info_for(path: &Path) -> InfoRecord {
    let bytes = read_input(path);
    let size = bytes.as_ref().map(|b| b.len() as u64).unwrap_or(0);
    let blank = |error: String| blank_record(path.display().to_string(), size, error);
    let bytes = match bytes {
        Ok(b) => b,
        Err(e) => return blank(e),
    };
    let Decoded { text, warning, .. } = charset::decode_input(&bytes);
    {
        let parsed = match fhir::detect_fhir(&text) {
            Some(FhirFormat::Json) => Some(fhir::parse_fhir_json(&text)),
            Some(FhirFormat::Xml) => Some(fhir::parse_fhir_xml(&text)),
            None => None,
        };
        if let Some(parsed) = parsed {
            return match parsed {
                Ok(r) => InfoRecord {
                    file: path.display().to_string(),
                    parsed: true,
                    valid: !fhir::validate_fhir_json(&r).iter().any(|i| i.severity == "error"),
                    kind: "fhir".into(),
                    message_type: r.resource_type,
                    version: r.fhir_version,
                    segment_count: r
                        .json_value
                        .as_ref()
                        .and_then(|j| j.get("entry"))
                        .and_then(|e| e.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0),
                    size_bytes: size,
                    error: None,
                },
                Err(e) => blank(e),
            };
        }
    }
    if let Some(w) = warning.filter(|w| w.is_unsupported()) {
        return blank(w.to_string());
    }
    match parse_hl7(text) {
        Ok(msg) => InfoRecord {
            file: path.display().to_string(),
            parsed: true,
            valid: validation::validate_hl7_message(&msg).error_count == 0,
            kind: "hl7".into(),
            message_type: msg.message_type,
            version: msg.version,
            segment_count: msg.segments.len(),
            size_bytes: size,
            error: None,
        },
        Err(e) => blank(e),
    }
}

fn cmd_info(patterns: &[String], as_json: bool) -> i32 {
    let inputs = match expand_paths(patterns) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 2;
        }
    };
    let mut records: Vec<InfoRecord> = inputs.files.iter().map(|f| info_for(f)).collect();
    for (p, why) in &inputs.unmatched {
        esafeln!("Error: {}: {}", p, why);
        records.push(blank_record(p.clone(), 0, why.clone()));
    }

    if as_json {
        outln!("{}", serde_json::to_string_pretty(&records).unwrap_or_default());
    } else {
        outln!(
            "{:<50} {:<5} {:<20} {:<8} {:<9} {:<10} {}",
            "FILE", "KIND", "MESSAGE TYPE", "VERSION", "SEGMENTS", "SIZE", "CHECKS"
        );
        outln!("{:-<113}", "");
        for r in &records {
            match &r.error {
                Some(e) => safeln!("{:<50} {}", truncate_start(&r.file, 50), e),
                None => safeln!(
                    "{:<50} {:<5} {:<20} {:<8} {:<9} {:<10} {}",
                    truncate_start(&r.file, 50),
                    r.kind,
                    truncate(&r.message_type, 20),
                    truncate(&r.version, 8),
                    r.segment_count,
                    format_size(r.size_bytes),
                    if r.valid { "ok" } else { "errors" }
                ),
            }
        }
    }
    if inputs.unmatched.is_empty() { 0 } else { 1 }
}

/// `s` cut to `max` characters from the start, so a long path keeps its
/// file name: `.../msgs/adt_a01.hl7`.
fn truncate_start(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        s.to_string()
    } else {
        format!("...{}", s.chars().skip(n - (max - 3)).collect::<String>())
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max - 3).collect::<String>())
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / 1048576.0)
    }
}

fn cmd_anonymize(input: &Path, output: Option<&Path>, no_plugins: bool) -> i32 {
    let Decoded { text, charset: source_charset, .. } = match read_message_text(input) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: not anonymized: {}", e);
            return 1;
        }
    };
    let msg = match Hl7Lexer::new().parse(text.into_bytes()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            return 1;
        }
    };

    // Plugin packs may add PHI fields — regional identifiers, Z-segments —
    // exactly as they do in the app.
    let extra: Vec<ExtraPhiField> = if no_plugins {
        vec![]
    } else {
        load_plugins().active_phi_fields(Engines::plugin_limit())
    };
    let mut anonymized = anonymization::anonymize_message_with_extra(&msg, &extra);
    // Every segment, the last one included, ends with the HL7 CR.
    if !anonymized.is_empty() && !anonymized.ends_with('\r') {
        anonymized.push('\r');
    }
    // Written back in the input's own character set, so MSH-18 stays true.
    let bytes = charset::encode_output(&anonymized, source_charset.as_deref().unwrap_or(""));

    match output {
        Some(path) => {
            if let Err(e) = write_output(path, &bytes) {
                eprintln!("Write error: {}", e);
                return 1;
            }
            eprintln!("Anonymized written to: {}", path.display());
        }
        None => {
            use std::io::Write;
            let mut out = std::io::stdout().lock();
            // On a terminal the text is shown, not the file's bytes: UTF-8
            // whatever the source charset (a Windows console refuses bytes
            // that are not UTF-8, a Linux terminal shows them as U+FFFD),
            // one segment per line (CR alone would draw every segment over
            // the previous one) and control characters made visible. Piped
            // or redirected, the bytes are the message as HL7 has it, in
            // the source charset.
            let result = if out.is_terminal() {
                out.write_all(terminal_safe(&anonymized.replace('\r', "\n")).as_bytes())
            } else {
                out.write_all(&bytes)
            };
            if let Err(e) = result.and_then(|_| out.flush()) {
                stdout_failed(e);
            }
        }
    }
    0
}

fn cmd_to_json(input: &Path, output: Option<&Path>) -> i32 {
    let content = match read_message_text(input) {
        Ok(c) => c.text,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };
    let msg = match Hl7Lexer::new().parse(content.into_bytes()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            return 1;
        }
    };

    let json_str = match bridgelab_lib::parser::hl7::export::to_json(&msg) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };

    match output {
        Some(path) => {
            if let Err(e) = write_output(path, json_str.as_bytes()) {
                eprintln!("Write error: {}", e);
                return 1;
            }
            eprintln!("JSON written to: {}", path.display());
        }
        None => outln!("{}", json_str),
    }
    0
}

/// One file of a batch: its metadata, and the findings `validate --format
/// json` would report for it.
#[derive(Serialize)]
struct BatchRecord {
    #[serde(flatten)]
    info: InfoRecord,
    error_count: usize,
    warning_count: usize,
    /// Set when the file had no errors but was not fully checked (a FHIR
    /// resource no installed package defines, a declared profile that is
    /// not installed, an unusable `--fhir-packages`): why.
    #[serde(skip_serializing_if = "Option::is_none")]
    not_checked: Option<String>,
    issues: Vec<Issue>,
}

#[derive(Serialize)]
struct BatchSummary {
    total_files: usize,
    valid_files: usize,
    failed_files: usize,
    /// Valid files that were not fully checked (counted in `valid_files`
    /// too, as `validate` exits 0 for them; JUnit shows them skipped).
    not_checked_files: usize,
    total_errors: usize,
    total_warnings: usize,
    results: Vec<BatchRecord>,
}

/// `hl7, .TXT` → `["hl7", "txt"]`.
fn parse_extensions(list: &str) -> Vec<String> {
    list.split(',')
        .map(|e| e.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|e| !e.is_empty())
        .collect()
}

fn cmd_batch(dir: &Path, extension: &str, as_json: bool, fhir_packages: Option<&Path>, no_plugins: bool) -> i32 {
    if !dir.is_dir() {
        eprintln!("Error: not a directory: {}", dir.display());
        return 2;
    }
    let extensions = parse_extensions(extension);
    if extensions.is_empty() {
        eprintln!("Error: --extension names no extension");
        return 2;
    }
    let engines = Engines::new(fhir_packages, no_plugins);

    let mut results = Vec::new();
    let mut total_errors = 0;
    let mut total_warnings = 0;
    let mut valid_files = 0;

    // Subdirectories included, symbolic links followed, in name order so
    // two runs list the files alike. A file or folder that cannot be read
    // is a failed entry, not a silent gap.
    for entry in walkdir::WalkDir::new(dir).follow_links(true).sort_by_file_name() {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                let file = e.path().map(|p| p.display().to_string()).unwrap_or_else(|| dir.display().to_string());
                total_errors += 1;
                results.push(BatchRecord {
                    info: blank_record(file, 0, format!("cannot read: {}", e)),
                    error_count: 1,
                    warning_count: 0,
                    not_checked: None,
                    issues: vec![],
                });
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase());
        if !ext.map(|e| extensions.contains(&e)).unwrap_or(false) {
            continue;
        }
        let mut record =
            BatchRecord { info: info_for(path), error_count: 0, warning_count: 0, not_checked: None, issues: vec![] };
        match validate_file(path, &engines) {
            Ok(report) => {
                record.info.valid = report.valid;
                if report.valid {
                    valid_files += 1;
                    let notes = not_checked_notes(&report);
                    if !notes.is_empty() {
                        record.not_checked = Some(notes.iter().map(|i| i.message.as_str()).collect::<Vec<_>>().join("; "));
                    }
                } else {
                    record.info.valid = false;
                    record.info.error = Some(format!("{} error(s)", report.error_count));
                }
                total_errors += report.error_count;
                total_warnings += report.warning_count;
                record.error_count = report.error_count;
                record.warning_count = report.warning_count;
                record.issues = report.issues;
            }
            Err(e) => {
                record.info.valid = false;
                record.info.error = Some(e);
                record.error_count = 1;
                total_errors += 1;
            }
        }
        results.push(record);
    }

    let summary = BatchSummary {
        total_files: results.len(),
        valid_files,
        failed_files: results.len() - valid_files,
        not_checked_files: results.iter().filter(|r| r.not_checked.is_some()).count(),
        total_errors,
        total_warnings,
        results,
    };

    if as_json {
        outln!("{}", serde_json::to_string_pretty(&summary).unwrap_or_default());
    } else {
        outln!("Batch validation summary:");
        outln!("  Total files:    {}", summary.total_files);
        outln!("  Valid:          {}", summary.valid_files);
        outln!("  Failed:         {}", summary.failed_files);
        if summary.not_checked_files > 0 {
            outln!("  Not checked:    {} (valid, but not fully checked)", summary.not_checked_files);
        }
        outln!("  Total errors:   {}", summary.total_errors);
        outln!("  Total warnings: {}", summary.total_warnings);
        for r in summary.results.iter().filter(|r| !r.info.valid) {
            safeln!("  ✗ {} — {}", r.info.file, r.info.error.as_deref().unwrap_or("invalid"));
        }
        for r in summary.results.iter() {
            if let Some(why) = &r.not_checked {
                safeln!("  ~ {} — not checked: {}", r.info.file, why);
            }
        }
    }

    if summary.total_files == 0 {
        // Nothing checked is not a pass: a wrong folder or extension in a
        // CI job must not turn green.
        eprintln!(
            "Error: no file with the extension {} in {}",
            extensions.iter().map(|e| format!(".{}", e)).collect::<Vec<_>>().join(", "),
            dir.display()
        );
        return 1;
    }
    if summary.total_errors > 0 {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// fhirpath

fn cmd_fhirpath(expression: &str, input: &Path, as_json: bool) -> i32 {
    let text = match read_text(input) {
        Ok(d) => d.text,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };
    let text = text.as_str();
    let parsed = match fhir::detect_fhir(text) {
        Some(FhirFormat::Json) => fhir::parse_fhir_json(text),
        Some(FhirFormat::Xml) => fhir::parse_fhir_xml(text),
        None => Err("not a FHIR resource (JSON or XML)".to_string()),
    };
    let json = match parsed.map(|r| r.json_value) {
        Ok(Some(j)) => j,
        Ok(None) => {
            eprintln!("Error: the resource could not be read as a JSON tree");
            return 1;
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };
    let result = fhir::fhirpath::evaluate(expression, &json);
    if as_json {
        outln!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
    } else if let Some(e) = &result.error {
        eprintln!("FHIRPath error: {}", e);
    } else {
        for v in &result.results {
            match v {
                serde_json::Value::String(s) => safeln!("{}", s),
                other => safeln!("{}", other),
            }
        }
    }
    if result.error.is_some() { 1 } else { 0 }
}


// ---------------------------------------------------------------------------
// xsd

fn cmd_xsd(message: &str, version: &str, output: Option<&Path>) -> i32 {
    let Some(v) = Hl7Version::parse(version) else {
        eprintln!("Error: unknown HL7 version '{}' (e.g. 2.3, 2.5.1, 2.7)", version);
        return 2;
    };
    let code = message.trim().to_uppercase().replace('^', "_");
    let schema = hl7schema::cached(v);
    if schema.message(&code).is_none() {
        eprintln!("Error: message '{}' not found in HL7 v{}", code, v.as_str());
        return 2;
    }
    if !hl7schema::xsd_in_community(v, &code) {
        eprintln!(
            "Error: the CLI exports the Community set ({} in v2.5). {} v{} is part of the full XSD catalogue in BridgeLab Pro.",
            hl7schema::COMMUNITY_XSD_MESSAGES_V2_5.join(", "),
            code,
            v.as_str()
        );
        return 1;
    }
    let xsd = match hl7schema::xsd::generate_xsd(schema, &code) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };
    match output {
        Some(path) => {
            if let Err(e) = write_output(path, xsd.as_bytes()) {
                eprintln!("Write error: {}", e);
                return 1;
            }
            eprintln!("XSD written to: {}", path.display());
        }
        None => out!("{}", xsd),
    }
    0
}

// ---------------------------------------------------------------------------
// send

#[derive(Serialize, Clone)]
struct SendReport {
    host: String,
    port: u16,
    delivered: bool,
    ack_code: Option<String>,
    response_time_ms: u64,
    response: String,
    error: Option<String>,
    /// MSH-10 of the message sent.
    control_id: String,
    /// With several messages in the file: one entry per message sent (the
    /// fields above are those of the last one, the failing one if any).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    results: Vec<SendReport>,
}

/// Envelope segments of an HL7 batch file: they frame messages in a file,
/// not on an MLLP connection.
const BATCH_ENVELOPE: [&str; 4] = ["FHS", "BHS", "BTS", "FTS"];

/// Split CR-separated text into its messages, one per MSH. Batch envelope
/// segments (FHS/BHS/BTS/FTS) are dropped and counted; anything before the
/// first MSH stays with the first message, for the receiver to judge.
fn split_messages(text: &str) -> (Vec<String>, usize) {
    let mut messages: Vec<Vec<&str>> = Vec::new();
    let mut dropped = 0;
    for seg in text.split('\r').filter(|s| !s.trim().is_empty()) {
        if BATCH_ENVELOPE.iter().any(|e| seg.starts_with(e)) {
            dropped += 1;
            continue;
        }
        if seg.starts_with("MSH") && messages.last().map(|m| m.iter().any(|s| s.starts_with("MSH"))).unwrap_or(true) {
            messages.push(Vec::new());
        }
        if messages.is_empty() {
            messages.push(Vec::new());
        }
        messages.last_mut().unwrap().push(seg);
    }
    (messages.into_iter().map(|m| m.join("\r") + "\r").collect(), dropped)
}

fn cmd_send(input: &Path, host: &str, port: u16, timeout: u64, encoding: &str, as_json: bool) -> i32 {
    if !encoding.is_empty() && !charset::is_known_label(encoding) {
        eprintln!("Error: unknown encoding '{}' (e.g. UTF-8, ISO-8859-1, windows-1252, ASCII)", encoding);
        return 2;
    }
    let Decoded { text, charset: source_charset, .. } = match read_message_text(input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: not sent: {}", e);
            return 1;
        }
    };
    // Messages on disk often use \n or \r\n between segments; HL7 wants \r.
    let text = text.replace("\r\n", "\r").replace('\n', "\r");
    // Without --encoding, a message read in a legacy charset goes out in
    // that same charset: the one its MSH-18 declares to the receiver. A
    // UTF-16 file (Notepad's "Unicode") is converted: to the charset its
    // MSH-18 declares, else UTF-8, as the app sends it.
    let encoding: String = if !encoding.is_empty() {
        encoding.trim().to_string()
    } else {
        match source_charset.as_deref() {
            Some(c) if charset::is_utf16(c) => mllp::resolve_send_encoding(&text, mllp::AUTO, None),
            Some(c) => c.to_string(),
            None => String::new(),
        }
    };
    let encoding = encoding.as_str();
    // MLLP carries one message per frame: a file of several goes out as
    // one frame each, and each must be acknowledged.
    let (messages, dropped) = split_messages(&text);
    if messages.is_empty() {
        eprintln!("Error: the input holds no message");
        return 1;
    }
    if dropped > 0 {
        eprintln!("note: {} batch envelope segment(s) (FHS/BHS/BTS/FTS) not sent: MLLP carries one message per frame", dropped);
    }
    // Refused before anything goes out, as in the app: a patient's name
    // must not reach the receiver with '?' in it.
    let mut lost: Vec<char> = Vec::new();
    for c in messages.iter().flat_map(|m| mllp::unencodable_chars(m, encoding)) {
        if !lost.contains(&c) {
            lost.push(c);
        }
    }
    if !lost.is_empty() {
        let shown = lost.iter().take(8).map(char::to_string).collect::<Vec<_>>().join(" ");
        let error = format!(
            "not sent: {} cannot represent {} character(s) of the message ({}); send without --encoding, or choose one that can (UTF-8)",
            encoding,
            lost.len(),
            shown
        );
        if as_json {
            let report = SendReport {
                host: host.into(),
                port,
                delivered: false,
                ack_code: None,
                response_time_ms: 0,
                response: String::new(),
                error: Some(error),
                control_id: String::new(),
                results: vec![],
            };
            outln!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
        } else {
            esafeln!("Error: {}", error);
        }
        return 1;
    }

    let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };
    let total = messages.len();
    let mut reports: Vec<SendReport> = Vec::new();
    let mut all_accepted = true;
    for (i, message) in messages.iter().enumerate() {
        let control_id = Hl7Lexer::new()
            .parse(message.clone().into_bytes())
            .ok()
            .and_then(|m| {
                m.segments.first().and_then(|msh| msh.fields.iter().find(|f| f.position == 10)).map(|f| f.span.as_str(&m.raw).trim().to_string())
            })
            .unwrap_or_default();
        let result = runtime.block_on(mllp::send(host, port, message, timeout, encoding));
        let ack_code = if result.success { bridgelab_lib::parser::hl7::ack::ack_code_of(&result.response) } else { None };
        let mut error = result.error.clone();
        if result.success && ack_code.is_none() && error.is_none() {
            error = Some("the reply is not an HL7 acknowledgement (no MSA segment with an acknowledgement code)".into());
        }
        let mut accepted = matches!(ack_code.as_deref(), Some("AA") | Some("CA"));
        // An ACK for another message is not an acceptance of this one.
        if accepted {
            if let Some(acked) = bridgelab_lib::parser::hl7::ack::acknowledged_control_id(&result.response) {
                if !control_id.is_empty() && acked != control_id {
                    accepted = false;
                    error = Some(format!(
                        "the ACK acknowledges message control ID {}, not {} (MSA-2 does not match MSH-10)",
                        acked, control_id
                    ));
                }
            }
        }
        let report = SendReport {
            host: host.into(),
            port,
            delivered: result.success,
            ack_code: ack_code.clone(),
            response_time_ms: result.response_time_ms,
            response: result.response.clone(),
            error,
            control_id: control_id.clone(),
            results: vec![],
        };
        if !as_json {
            let which = if total > 1 { format!("message {}/{} ", i + 1, total) } else { String::new() };
            if !result.success {
                esafeln!("Send {}failed: {}", which, report.error.as_deref().unwrap_or("unknown error"));
            } else {
                eprintln!(
                    "Sent {}to {}:{} in {} ms — ACK {}",
                    which,
                    host,
                    port,
                    report.response_time_ms,
                    ack_code.as_deref().unwrap_or("(none)")
                );
                if let Some(e) = &report.error {
                    esafeln!("Not accepted: {}", e);
                }
                safeln!("{}", report.response.replace('\r', "\n").trim_end());
            }
        }
        reports.push(report);
        if !accepted {
            all_accepted = false;
            if total > 1 && i + 1 < total {
                eprintln!("Stopped: {} of {} message(s) not sent", total - i - 1, total);
            }
            break;
        }
    }

    if as_json {
        let mut last = reports.pop().expect("at least one message was attempted");
        if total > 1 {
            reports.push(last.clone());
            last.results = reports;
        }
        outln!("{}", serde_json::to_string_pretty(&last).unwrap_or_default());
    }
    if all_accepted { 0 } else { 1 }
}

// ---------------------------------------------------------------------------
// test

#[derive(Serialize)]
struct CaseResult {
    pack: String,
    name: String,
    category: String,
    passed: bool,
    /// Set when the case met its expectations but its FHIR resource was not
    /// fully checked (see `validate`): why. Such a case is reported as
    /// skipped, not passed.
    #[serde(skip_serializing_if = "Option::is_none")]
    not_checked: Option<String>,
    problems: Vec<String>,
}

fn observe(content: &str, engines: &Engines) -> (test_packs::Observed, Option<Report>) {
    match validate_bytes(content.as_bytes().to_vec(), engines) {
        Ok(r) => (
            test_packs::Observed { message_type: r.message_type.clone(), errors: Ok(r.error_count) },
            Some(r),
        ),
        Err(e) => (test_packs::Observed { message_type: String::new(), errors: Err(e) }, None),
    }
}

/// What a failed case needs to be understood without re-running it: the
/// first errors found, and why a FHIR resource was not fully checked.
fn failure_details(report: &Report) -> Vec<String> {
    let mut details: Vec<String> = report
        .issues
        .iter()
        .filter(|i| i.severity == "error")
        .take(3)
        .map(|i| {
            let at = if i.rule_id.is_empty() { i.location.clone() } else { format!("{} {}", i.location, i.rule_id) };
            format!("{}: {}", at, i.message)
        })
        .collect();
    if report.error_count > 3 {
        details.push(format!("… {} more error(s)", report.error_count - 3));
    }
    details.extend(not_checked_notes(report).into_iter().map(|i| i.message.clone()));
    details
}

fn cmd_test(patterns: &[String], format: Format, fhir_packages: Option<&Path>, no_plugins: bool) -> i32 {
    let inputs = match expand_paths(patterns) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 2;
        }
    };
    let engines = Engines::new(fhir_packages, no_plugins);
    let mut results = Vec::new();
    let mut load_errors: Vec<(String, String)> = inputs.unmatched.clone();
    for file in &inputs.files {
        let pack = read_input_limited(file, test_packs::MAX_PACK_BYTES).and_then(|b| {
            String::from_utf8(b).map_err(|_| "not UTF-8 text".to_string())
        }).and_then(|t| test_packs::parse_pack(&t));
        let pack = match pack {
            Ok(p) if p.test_cases.is_empty() => {
                // An empty pack tests nothing; passing it would hide a
                // wrong export or a truncated file.
                load_errors.push((file.display().to_string(), "the pack has no test cases".into()));
                continue;
            }
            Ok(p) => p,
            Err(e) => {
                load_errors.push((file.display().to_string(), e));
                continue;
            }
        };
        for tc in &pack.test_cases {
            let (observed, report) = observe(&tc.content, &engines);
            let mut problems = test_packs::check_expectations(
                &tc.expected_message_type,
                &tc.expected_validation_result,
                &observed,
            );
            if !problems.is_empty() {
                if let Some(r) = &report {
                    problems.extend(failure_details(r));
                }
            }
            let not_checked = report
                .as_ref()
                .filter(|_| problems.is_empty())
                .map(not_checked_notes)
                .filter(|n| !n.is_empty())
                .map(|n| n.iter().map(|i| i.message.as_str()).collect::<Vec<_>>().join("; "));
            results.push(CaseResult {
                pack: file.display().to_string(),
                name: tc.name.clone(),
                category: tc.category.clone(),
                passed: problems.is_empty() && not_checked.is_none(),
                not_checked,
                problems,
            });
        }
    }
    for (pack, e) in &load_errors {
        esafeln!("Error: {}: {}", pack, e);
    }
    // An unreadable pack counts as one failed test, in every output, so
    // total == passed + failed holds for CI consumers.
    let skipped = results.iter().filter(|r| r.not_checked.is_some()).count();
    let failed = results.iter().filter(|r| !r.passed && r.not_checked.is_none()).count() + load_errors.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let total = passed + failed + skipped;

    match format {
        Format::Json => {
            let errors: Vec<_> = load_errors.iter().map(|(f, e)| serde_json::json!({ "pack": f, "error": e })).collect();
            outln!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "total": total,
                    "passed": passed,
                    "failed": failed,
                    "skipped": skipped,
                    "pack_errors": errors,
                    "results": results,
                }))
                .unwrap_or_default()
            );
        }
        Format::Junit => {
            outln!(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
            outln!(r#"<testsuites name="bridgelab" tests="{}" failures="{}" skipped="{}">"#, total, failed, skipped);
            for (pack, e) in &load_errors {
                outln!(r#"  <testsuite name="{}" tests="1" failures="1">"#, xml_escape(pack));
                outln!(r#"    <testcase name="load" classname="pack"><failure message="{}"/></testcase>"#, xml_escape(e));
                outln!(r#"  </testsuite>"#);
            }
            for file in &inputs.files {
                let pack = file.display().to_string();
                let rows: Vec<&CaseResult> = results.iter().filter(|r| r.pack == pack).collect();
                if rows.is_empty() {
                    continue;
                }
                let s = rows.iter().filter(|r| r.not_checked.is_some()).count();
                let f = rows.iter().filter(|r| !r.passed).count() - s;
                outln!(r#"  <testsuite name="{}" tests="{}" failures="{}" skipped="{}">"#, xml_escape(&pack), rows.len(), f, s);
                for r in rows {
                    if let Some(why) = &r.not_checked {
                        outln!(r#"    <testcase name="{}" classname="{}">"#, xml_escape(&r.name), xml_escape(&r.category));
                        outln!(r#"      <skipped message="{}"/>"#, xml_escape(&format!("not checked: {}", why)));
                        outln!(r#"    </testcase>"#);
                    } else if r.passed {
                        outln!(r#"    <testcase name="{}" classname="{}"/>"#, xml_escape(&r.name), xml_escape(&r.category));
                    } else {
                        outln!(r#"    <testcase name="{}" classname="{}">"#, xml_escape(&r.name), xml_escape(&r.category));
                        outln!(r#"      <failure message="{}"/>"#, xml_escape(&r.problems.join("; ")));
                        outln!(r#"    </testcase>"#);
                    }
                }
                outln!(r#"  </testsuite>"#);
            }
            outln!(r#"</testsuites>"#);
        }
        Format::Text => {
            for (pack, e) in &load_errors {
                safeln!("✗ {}: {}", pack, e);
            }
            let mut current = "";
            for r in &results {
                if r.pack != current {
                    current = &r.pack;
                    safeln!("== {} ==", r.pack);
                }
                if let Some(why) = &r.not_checked {
                    safeln!("~ {} — not checked: {}", r.name, why);
                } else if r.passed {
                    safeln!("✓ {}", r.name);
                } else {
                    safeln!("✗ {} — {}", r.name, r.problems.join("; "));
                }
            }
            if skipped > 0 {
                outln!("{}/{} passed, {} not checked", passed, total, skipped);
            } else {
                outln!("{}/{} passed", passed, total);
            }
        }
    }
    if failed > 0 { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("bl-cli-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn patterns_match_extensions_in_any_case() {
        let d = tmp("glob");
        for n in ["a.hl7", "B.HL7", "c.Hl7", "d.txt"] {
            fs::write(d.join(n), "MSH|^~\\&|").unwrap();
        }
        let inputs = expand_paths(&[format!("{}/*.hl7", d.display())]).unwrap();
        let mut names: Vec<_> = inputs.files.iter().map(|f| f.file_name().unwrap().to_string_lossy().into_owned()).collect();
        names.sort();
        assert_eq!(names, ["B.HL7", "a.hl7", "c.Hl7"]);
        assert!(inputs.unmatched.is_empty());
        let none = expand_paths(&[format!("{}/*.xml", d.display())]).unwrap();
        assert!(none.files.is_empty());
        assert_eq!(none.unmatched.len(), 1, "a pattern matching nothing is reported");
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn output_refuses_a_read_only_file() {
        let d = tmp("ro");
        let p = d.join("golden.json");
        fs::write(&p, "golden").unwrap();
        let mut perms = fs::metadata(&p).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&p, perms).unwrap();
        let err = write_output(&p, b"new").unwrap_err();
        assert!(err.contains("read-only"), "{err}");
        assert_eq!(fs::read_to_string(&p).unwrap(), "golden");
        let mut perms = fs::metadata(&p).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        fs::set_permissions(&p, perms).unwrap();
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn control_characters_are_shown_not_run() {
        assert_eq!(terminal_safe("ok\tline\n"), "ok\tline\n");
        assert_eq!(terminal_safe("\u{1b}[2J\u{1b}]0;pwned\u{7}x\u{9b}"), "\\x1B[2J\\x1B]0;pwned\\x07x\\x9B");
    }

    #[test]
    fn junit_attributes_keep_tabs_and_line_breaks() {
        assert_eq!(xml_escape("a\tb\nc\rd<"), "a&#9;b&#10;c&#13;d&lt;");
    }

    #[test]
    fn a_long_path_keeps_its_file_name() {
        let p = format!("/very/{}/adt_a01.hl7", "x".repeat(80));
        let t = truncate_start(&p, 50);
        assert_eq!(t.chars().count(), 50);
        assert!(t.starts_with("...") && t.ends_with("/adt_a01.hl7"), "{t}");
    }

    #[test]
    fn a_folder_argument_points_to_batch() {
        let d = tmp("dirarg");
        let inputs = expand_paths(&[d.display().to_string()]).unwrap();
        assert!(inputs.files.is_empty());
        assert!(inputs.unmatched[0].1.contains("batch"), "{:?}", inputs.unmatched);
        fs::remove_dir_all(&d).ok();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_file_name_that_is_not_utf8_is_matched() {
        use std::os::unix::ffi::OsStrExt;
        let d = tmp("nonutf8");
        fs::write(d.join("good.hl7"), "MSH|^~\\&|").unwrap();
        let bad = d.join(std::ffi::OsStr::from_bytes(b"bad\xe9name.hl7"));
        if fs::write(&bad, "MSH|^~\\&|").is_err() {
            return; // a file system that refuses such names
        }
        let inputs = expand_paths(&[format!("{}/*.hl7", d.display())]).unwrap();
        assert_eq!(inputs.files.len(), 2, "{:?}", inputs.files);
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn multibyte_names_do_not_stop_validation_or_anonymization() {
        let d = tmp("cjk");
        for name in ["李小龍", "Lindström", "Västerås"] {
            let input = d.join("in.hl7");
            let out = d.join("out.hl7");
            fs::write(&input, format!("MSH|^~\\&|A|B|C|D|20240101120000||ADT^A01|MSG1|P|2.5\rPID|1||12345^^^FAC^MR||{name}^Jan||1990051年|M|||Via Roma^^{name}\r")).unwrap();
            let engines = Engines::new(None, true);
            let report = validate_file(&input, &engines).unwrap();
            assert!(report.issues.iter().any(|i| i.location == "PID-7"), "PID-7 is not a date");
            assert_eq!(cmd_anonymize(&input, Some(&out), true), 0);
            assert!(!fs::read_to_string(&out).unwrap().contains(name));
        }
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn a_big5_message_is_anonymized_in_big5() {
        let d = tmp("big5");
        let input = d.join("big5.hl7");
        let out = d.join("anon.hl7");
        let text = "MSH|^~\\&|A|B|C|D|20240101120000||ADT^A01|MSG1|P|2.5|||||TWN|BIG-5\rEVN|A01|20240101120000\rPID|1||12345^^^FAC^MR||陳四明^Jan||19900515|M\r";
        let bytes = charset::encode_output(text, "Big5");
        assert!(bytes.windows(2).any(|w| w == b"\xa5\x7c"), "四 carries a | byte");
        fs::write(&input, &bytes).unwrap();
        assert_eq!(cmd_anonymize(&input, Some(&out), true), 0);
        let written = fs::read(&out).unwrap();
        assert!(!written.windows(8).any(|w| w == b"19900515"), "{}", String::from_utf8_lossy(&written));
        let back = charset::decode_input(&written);
        assert!(back.text.contains("|BIG-5\r") && !back.text.contains("陳四明"), "{}", back.text);
        // A charset that cannot be decoded is refused, not guessed.
        fs::write(&input, b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||TWN|CNS 11643-1992\rPID|1||1||\xa4\x7c||19900515\r").unwrap();
        assert_eq!(cmd_anonymize(&input, Some(&out), true), 1);
        assert!(validate_file(&input, &Engines::new(None, true)).is_err());
        // ISO IR87 (ISO-2022-JP) is 7-bit, valid UTF-8: still refused.
        // 山田^太郎, with ESC $ B … ESC ( B around the kanji.
        let jp_bytes: &[u8] = b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||JPN|ISO IR87\rPID|1||1||\x1b$B;3ED\x1b(B^\x1b$BB@O:\x1b(B||19900515\r";
        fs::write(&input, jp_bytes).unwrap();
        let _ = fs::remove_file(&out);
        assert_eq!(cmd_anonymize(&input, Some(&out), true), 1);
        assert!(!out.exists());
        assert!(validate_file(&input, &Engines::new(None, true)).is_err());
        assert_eq!(cmd_to_json(&input, None), 1);
        fs::remove_dir_all(&d).ok();
    }

    /// A one-shot MLLP peer: returns its port and a handle yielding the
    /// bytes it received.
    fn peer(ack: &'static [u8]) -> (u16, std::thread::JoinHandle<Vec<u8>>) {
        use std::io::{Read, Write};
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let h = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 4096];
            while !got.ends_with(b"\x1c\r") {
                let n = s.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                got.extend_from_slice(&buf[..n]);
            }
            s.write_all(ack).unwrap();
            got
        });
        (port, h)
    }

    #[test]
    fn send_refuses_characters_the_encoding_cannot_hold() {
        let d = tmp("sendenc");
        let input = d.join("utf.hl7");
        fs::write(&input, "MSH|^~\\&|A|B|C|D|20260101||ADT^A01|U1|P|2.5\rPID|1||1||Müller^Zoë €||19800101|M\r").unwrap();
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.set_nonblocking(true).unwrap();
        let port = l.local_addr().unwrap().port();
        for enc in ["ASCII", "ISO-8859-1"] {
            assert_eq!(cmd_send(&input, "127.0.0.1", port, 2, enc, true), 1, "{enc}");
        }
        assert!(l.accept().is_err(), "nothing was sent");
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn a_utf16_file_is_sent_as_utf8_and_its_ack_read() {
        let d = tmp("send16");
        let input = d.join("u16.hl7");
        let text = "MSH|^~\\&|A|B|C|D|20260101||ADT^A01|U1|P|2.5\rPID|1||1||Müller||19800101|M\r";
        fs::write(&input, charset::encode_output(text, "UTF-16LE")).unwrap();
        let (port, h) = peer(b"\x0bMSH|^~\\&|R||S||20260101||ACK|A1|P|2.5\rMSA|AA|U1\r\x1c\r");
        assert_eq!(cmd_send(&input, "127.0.0.1", port, 5, "", true), 0);
        let got = h.join().unwrap();
        assert!(got.starts_with(b"\x0bMSH|"), "{:?}", &got[..8]);
        assert!(String::from_utf8_lossy(&got).contains("Müller"));
        fs::remove_dir_all(&d).ok();
    }
}

