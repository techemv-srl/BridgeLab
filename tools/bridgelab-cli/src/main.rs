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

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process;

use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use serde::Serialize;

/// `println!` that does not panic when standard output goes away (a
/// `| head` that has read enough): the process ends quietly instead, with
/// the status a shell reports for SIGPIPE. While a report goes to
/// `--output`, the text is collected for that file instead.
macro_rules! outln {
    ($($arg:tt)*) => {{
        write_out(format_args!($($arg)*), true);
    }};
}

/// `print!` counterpart of [`outln!`].
macro_rules! out {
    ($($arg:tt)*) => {{
        write_out(format_args!($($arg)*), false);
    }};
}

thread_local! {
    /// Set while a command's report goes to `--output`: [`out!`] and
    /// [`outln!`] collect it here instead of writing it to standard output.
    static CAPTURE: std::cell::RefCell<Option<Vec<u8>>> = const { std::cell::RefCell::new(None) };
}

/// What [`out!`] and [`outln!`] do: write to standard output, or to the
/// report being collected for `--output`.
fn write_out(args: std::fmt::Arguments, newline: bool) {
    use std::io::Write as _;
    let collected = CAPTURE.with(|c| match c.borrow_mut().as_mut() {
        Some(buf) => {
            let _ = buf.write_fmt(args);
            if newline {
                buf.push(b'\n');
            }
            true
        }
        None => false,
    });
    if collected {
        return;
    }
    let mut stdout = std::io::stdout();
    let result = if newline { writeln!(stdout, "{}", args) } else { write!(stdout, "{}", args) };
    if let Err(e) = result {
        stdout_failed(e);
    }
}

/// Collects what a command prints until [`finish`](Self::finish), for
/// `--output`. Dropped early (a panic), it stops collecting.
struct Collected;

impl Collected {
    fn start() -> Self {
        CAPTURE.with(|c| *c.borrow_mut() = Some(Vec::new()));
        Collected
    }

    fn finish(self) -> Vec<u8> {
        CAPTURE.with(|c| c.borrow_mut().take()).unwrap_or_default()
    }
}

impl Drop for Collected {
    fn drop(&mut self) {
        CAPTURE.with(|c| *c.borrow_mut() = None);
    }
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

/// [`safeln!`] for standard error, where every line of this tool goes:
/// warnings and errors quote pack rule ids, file names, command-line
/// values, a message's MSH-18 and a peer's MSA-1, any of which can hold an
/// escape sequence. A plain `eprintln!` is refused by clippy.toml and by
/// the test `stderr_is_written_only_through_esafeln`. A failed write to
/// standard error is ignored: there is nowhere left to report it.
macro_rules! esafeln {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stderr(), "{}", terminal_safe(&format!($($arg)*)));
    }};
}

/// DEL and the C1 controls, U+007F to U+009F: serde_json writes them as
/// they are, and some terminals act on them (U+009B is CSI to the Linux
/// console, which then clears the screen or moves the cursor).
fn is_del_or_c1(c: char) -> bool {
    ('\u{7F}'..='\u{9F}').contains(&c)
}

/// JSON text with DEL and the C1 controls as `\u` escapes, which serde_json
/// leaves raw (it escapes only U+0000 to U+001F). Outside strings JSON has
/// no such characters, so each is in a string, where the escape reads back
/// as the same character.
fn json_controls_escaped(json: String) -> String {
    if !json.chars().any(is_del_or_c1) {
        return json;
    }
    let mut out = String::with_capacity(json.len() + 16);
    for c in json.chars() {
        if is_del_or_c1(c) {
            out.push_str(&format!("\\u{:04x}", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}

/// `--json` / `--format json` output: pretty JSON, controls escaped.
fn json_text<T: Serialize + ?Sized>(value: &T) -> String {
    json_controls_escaped(serde_json::to_string_pretty(value).unwrap_or_default())
}

fn stdout_failed(e: std::io::Error) -> ! {
    if e.kind() == std::io::ErrorKind::BrokenPipe {
        process::exit(141);
    }
    esafeln!("Error: cannot write to standard output: {}", e);
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
use bridgelab_lib::plugins::{self, PluginRegistry, RuleKind};
use bridgelab_lib::utils::atomic_write::write_atomic;
use bridgelab_lib::validation::{self, Severity};

/// `--version`: the CLI version and the build code of the validators it
/// was built with, e.g. `2.1.0 (build 20261117.762cc1c)`, so two builds of
/// the same version can be told apart.
fn cli_version() -> &'static str {
    static VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    VERSION.get_or_init(|| {
        format!("{} (build {})", env!("CARGO_PKG_VERSION"), bridgelab_lib::BUILD_ID)
    })
}

// Examples printed at the end of `--help`, so the usual invocations are
// there without opening the README.
// Reports go to files with --output, not with `>`: Windows PowerShell 5.1
// writes a redirection as UTF-16, and a JUnit report that declares UTF-8
// then cannot be read (a test checks that no example redirects).
const EXAMPLES: &str = "\
Examples:
  bridgelab-cli validate \"messages/*.hl7\" bundle.json
  bridgelab-cli validate \"**/*.hl7\" --format junit --output results.xml
  bridgelab-cli test \"packs/*.bltests.json\" --format junit --output tests.xml
  bridgelab-cli anonymize message.hl7 --output message-safe.hl7

Each command has its own examples: bridgelab-cli <command> --help";

const EXAMPLES_VALIDATE: &str = "\
Examples:
  bridgelab-cli validate message.hl7
  bridgelab-cli validate \"messages/*.hl7\" bundle.json
  bridgelab-cli validate message.hl7 --format json
  bridgelab-cli validate \"**/*.hl7\" --format junit --output results.xml
  bridgelab-cli validate \"inbox/*.hl7\" --no-strict";

const EXAMPLES_INFO: &str = "\
Examples:
  bridgelab-cli info \"messages/*.hl7\" bundle.json
  bridgelab-cli info message.hl7 --json";

const EXAMPLES_ANONYMIZE: &str = "\
Examples:
  bridgelab-cli anonymize patient.hl7
  bridgelab-cli anonymize patient.hl7 --output patient-safe.hl7
  cat patient.hl7 | bridgelab-cli anonymize - --output patient-safe.hl7";

const EXAMPLES_TO_JSON: &str = "\
Examples:
  bridgelab-cli to-json message.hl7
  bridgelab-cli to-json message.hl7 --output message.json";

const EXAMPLES_FHIRPATH: &str = "\
Examples:
  bridgelab-cli fhirpath \"Patient.name.family\" patient.json
  cat bundle.json | bridgelab-cli fhirpath \"Bundle.entry.count()\"
  bridgelab-cli fhirpath \"Observation.value.ofType(Quantity)\" obs.xml --json";

const EXAMPLES_XSD: &str = "\
Examples:
  bridgelab-cli xsd ADT_A01 --output ADT_A01.xsd
  bridgelab-cli xsd \"ORU^R01\" --hl7-version 2.5 --output ORU_R01.xsd
  bridgelab-cli xsd \"ADT^A04^ADT_A01\" --output ADT_A01.xsd";

const EXAMPLES_SEND: &str = "\
Examples:
  bridgelab-cli send message.hl7 --host 10.0.0.5 --port 2575
  bridgelab-cli send message.hl7 --host 10.0.0.5 --port 2575 --encoding ISO-8859-1
  cat message.hl7 | bridgelab-cli send - --host 10.0.0.5 --port 2575 --json";

const EXAMPLES_TEST: &str = "\
Examples:
  bridgelab-cli test regression.bltests.json
  bridgelab-cli test \"packs/*.bltests.json\" --format junit --output tests.xml";

const EXAMPLES_BATCH: &str = "\
Examples:
  bridgelab-cli batch ./messages
  bridgelab-cli batch ./messages --extension hl7,txt,dat
  bridgelab-cli batch ./fhir --extension json --json --output batch-report.json";

#[derive(Parser)]
#[command(
    name = "bridgelab-cli",
    version = cli_version(),
    about = "HL7 v2 and FHIR validation, anonymization and conversion for CI/CD — the BridgeLab validators, headless",
    after_help = EXAMPLES
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate HL7 v2 or FHIR files (format detected per file)
    #[command(after_help = EXAMPLES_VALIDATE)]
    Validate {
        /// Files or glob patterns to validate (- for standard input)
        #[arg(required = true)]
        paths: Vec<OsString>,
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
        /// Write the report to this file instead of standard output (UTF-8,
        /// written atomically)
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
    /// Show info about messages (type, version, segment count)
    #[command(after_help = EXAMPLES_INFO)]
    Info {
        /// Files or glob patterns (- for standard input)
        #[arg(required = true)]
        paths: Vec<OsString>,
        /// Output as JSON instead of a table
        #[arg(long)]
        json: bool,
    },
    /// Anonymize an HL7 v2 message (mask PHI fields)
    #[command(after_help = EXAMPLES_ANONYMIZE)]
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
    #[command(after_help = EXAMPLES_TO_JSON)]
    ToJson {
        /// Input HL7 file, or - for standard input
        input: PathBuf,
        /// Output JSON file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Evaluate a FHIRPath expression against a FHIR resource (JSON or XML)
    #[command(after_help = EXAMPLES_FHIRPATH)]
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
    #[command(after_help = EXAMPLES_XSD)]
    Xsd {
        /// Message structure, e.g. ADT_A01 (ADT^A01 and a whole MSH-9 such
        /// as ADT^A04^ADT_A01 are accepted too)
        message: String,
        /// HL7 version
        #[arg(long = "hl7-version", default_value = "2.5")]
        hl7_version: String,
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Send an HL7 v2 message over MLLP and print the ACK
    #[command(after_help = EXAMPLES_SEND)]
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
    #[command(after_help = EXAMPLES_TEST)]
    Test {
        /// Pack files or glob patterns
        #[arg(required = true)]
        packs: Vec<OsString>,
        /// Output format
        #[arg(short, long, value_enum, ignore_case = true, default_value = "text")]
        format: Format,
        /// Directory of distilled FHIR packages (see `validate`)
        #[arg(long, value_name = "DIR")]
        fhir_packages: Option<PathBuf>,
        /// Ignore plugin packs from the app's config directory
        #[arg(long)]
        no_plugins: bool,
        /// Write the report to this file instead of standard output (UTF-8,
        /// written atomically)
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
    /// Validate every message in a directory and its subdirectories
    #[command(after_help = EXAMPLES_BATCH)]
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
        /// Write the summary to this file instead of standard output
        /// (UTF-8, written atomically)
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        Commands::Validate { paths, format, strict, no_strict, fhir_packages, no_plugins, output } => {
            with_output(output.as_deref(), || {
                cmd_validate(&paths, format, strict && !no_strict, fhir_packages.as_deref(), no_plugins, output.as_deref())
            })
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
        Commands::Test { packs, format, fhir_packages, no_plugins, output } => {
            with_output(output.as_deref(), || {
                cmd_test(&packs, format, fhir_packages.as_deref(), no_plugins, output.as_deref())
            })
        }
        Commands::Batch { dir, extension, json, fhir_packages, no_plugins, output } => {
            with_output(output.as_deref(), || {
                cmd_batch(&dir, &extension, json, fhir_packages.as_deref(), no_plugins, output.as_deref())
            })
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
    /// The same note for a FHIR report, counting the packs with FHIR rules.
    fhir_cap_note: Option<Issue>,
    profiles: ProfileRegistry,
    /// Set when `--fhir-packages` gave nothing usable: added to every FHIR
    /// report, so a mistyped path never reads as a clean run.
    packages_problem: Option<String>,
}

impl Engines {
    fn new(fhir_packages: Option<&Path>, no_plugins: bool) -> Self {
        let plugins = if no_plugins { None } else { Some(load_plugins()) };
        let note = |kind: RuleKind| {
            plugins.as_ref().and_then(|r| r.cap_message(Self::plugin_limit(), kind)).map(|message| Issue {
                severity: "info".into(),
                // The note names the Community edition already: the CLI only
                // adds that it has the same cap, and where packs are switched off.
                message: cli_cap_note(&message),
                location: "-".into(),
                rule_id: plugins::CAP_RULE_ID.into(),
            })
        };
        let (cap_note, fhir_cap_note) = (note(RuleKind::Hl7), note(RuleKind::Fhir));
        let profiles = ProfileRegistry::new();
        let packages_problem = fhir_packages.and_then(check_packages_dir);
        if let Some(problem) = &packages_problem {
            esafeln!("warning: {}", problem);
        }
        if let Err(e) = profiles.reload_from(fhir_packages) {
            esafeln!("warning: FHIR packages not loaded: {}", e);
        }
        Self { plugins, cap_note, fhir_cap_note, profiles, packages_problem }
    }

    /// Community edition: the same cap on active plugin packs the app
    /// applies without a licence.
    fn plugin_limit() -> Option<usize> {
        Some(COMMUNITY_MAX_ACTIVE_PLUGINS)
    }
}

/// The app's note on packs over the cap, as the CLI reports it: "the
/// Community edition runs 3 packs at a time (Settings → Plugins)" becomes
/// "the CLI, like the Community edition, runs 3 packs at a time (switch
/// packs off in the app's Settings → Plugins)". It used to name the
/// Community edition twice.
fn cli_cap_note(message: &str) -> String {
    message
        .replace("the Community edition runs", "the CLI, like the Community edition, runs")
        .replace("(Settings → Plugins)", "(switch packs off in the app's Settings → Plugins)")
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
                esafeln!("warning: FHIR package {} not loaded: {}", path.display(), why);
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
        esafeln!("warning: plugin packs not loaded: {}", e);
        return registry;
    }
    // Read as empty, an unreadable state file switched back on every pack
    // the app had switched off, without a word.
    if let Some(problem) = registry.state_problem() {
        let file = registry.state_file_path().map(|p| p.display().to_string()).unwrap_or_default();
        esafeln!(
            "warning: the plugin on/off choices in {} could not be read ({}): each pack runs as its own \"enabled\" says, so a pack switched off in the app may run",
            file, problem
        );
    }
    for p in registry.list(Engines::plugin_limit()) {
        if let Some(e) = &p.error {
            esafeln!("warning: plugin pack {} not loaded: {}", p.path, e);
        } else if p.gated {
            esafeln!(
                "warning: plugin pack {} ({}) inactive: the CLI runs at most {} packs, like the Community edition",
                p.name, p.path, COMMUNITY_MAX_ACTIVE_PLUGINS
            );
        }
        for w in &p.warnings {
            esafeln!("warning: plugin pack {}: {}", p.path, w);
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
        let (found, applied) = engines.profiles.validate_with_status(json, &resource.format);
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
    issues.extend(engines.fhir_cap_note.clone());
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

/// Run a command whose report goes to standard output, or to `output`
/// when one is given. The file is written atomically, like every
/// `--output`, and holds exactly the bytes the command wrote (UTF-8):
/// Windows PowerShell 5.1 turns a `>` redirection into UTF-16, so a JUnit
/// report declaring UTF-8 could not be read. A usage error (exit 2) writes
/// nothing, so it never replaces an earlier report with an empty one.
fn with_output(output: Option<&Path>, report: impl FnOnce() -> i32) -> i32 {
    let Some(path) = output else { return report() };
    let collected = Collected::start();
    let code = report();
    let bytes = collected.finish();
    if code == 2 {
        return code;
    }
    if let Err(e) = write_output(path, &bytes) {
        esafeln!("Write error: {}", e);
        return 1;
    }
    esafeln!("Report written to: {}", path.display());
    code
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
            esafeln!("warning: {}: {}", path.display(), w);
            Ok(decoded)
        }
        None => Ok(decoded),
    }
}

/// Like `read_input`, refusing anything larger than `max` bytes before
/// reading it all. Standard input, a named pipe or a process substitution
/// (`<(zcat pack.json.gz)`), whose size is not known in advance, is read
/// up to `max` + 1 bytes.
fn read_input_limited(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let too_big = || format!("larger than {} MB", max / (1024 * 1024));
    let mut buf = Vec::new();
    if path.as_os_str() == "-" {
        std::io::Read::read_to_end(&mut std::io::Read::take(std::io::stdin(), max + 1), &mut buf)
            .map_err(|e| format!("read failed (stdin): {}", e))?;
    } else {
        let meta = fs::metadata(path).map_err(|e| format!("read failed: {}", e))?;
        if meta.len() > max {
            return Err(too_big());
        }
        let file = fs::File::open(path).map_err(|e| format!("read failed: {}", e))?;
        std::io::Read::read_to_end(&mut std::io::Read::take(file, max + 1), &mut buf)
            .map_err(|e| format!("read failed: {}", e))?;
    }
    if buf.len() as u64 > max {
        return Err(too_big());
    }
    Ok(buf)
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

/// [`parse_hl7`] for a command that works on HL7 v2 only (`to-json`,
/// `anonymize`, `send`): a FHIR resource is named as one, where it was
/// reported as text that "does not start with MSH".
fn parse_hl7_only(text: String, command: &str) -> Result<Hl7Message, String> {
    if fhir::detect_fhir(&text).is_some() {
        return Err(format!("a FHIR resource, not an HL7 v2 message: {} works on HL7 v2 (ER7) messages only", command));
    }
    parse_hl7(text)
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
/// file systems ignore case, and a skipped file would go unnoticed); see
/// also [`any_case_pattern`] for the names without wildcards. Hidden names
/// are left to [`hidden_and_not_asked_for`]: `glob`'s own option for them
/// drops every hidden name, `.*.hl7` included, and stops at the first
/// name that is not UTF-8.
const GLOB_OPTIONS: glob::MatchOptions = glob::MatchOptions {
    case_sensitive: false,
    require_literal_separator: false,
    require_literal_leading_dot: false,
};

/// A name hidden by the dot convention.
fn is_hidden(name: &OsStr) -> bool {
    name.as_encoded_bytes().first() == Some(&b'.')
}

/// Whether `found`, a match of `pattern`, goes through a hidden file or
/// folder the pattern does not spell with its dot. Such a name is left
/// out: `*.hl7` took in the `._adt.hl7` files macOS leaves on shared
/// drives and USB sticks, which are no messages and failed the run, and
/// `**` went into `.git`. A hidden name the pattern spells (`.*.hl7`,
/// `~/.cache/in/*.hl7`) still matches.
///
/// The components are lined up by position: a hidden name passes only
/// where a pattern component starting with a dot matched it. Compared by
/// name alone, `root/.cache/**/*.hl7` also let `root/.cache/sub/.cache/x.hl7`
/// through, the second `.cache` being one `**` went into.
fn hidden_and_not_asked_for(pattern: &OsStr, found: &Path) -> bool {
    use std::path::Component;
    let steps: Vec<HiddenStep> = Path::new(pattern)
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .map(|c| match c {
            Component::Normal(n) if n == "**" => HiddenStep::AnyDepth,
            Component::Normal(n) => {
                let text = n.to_string_lossy();
                match glob::Pattern::new(&text) {
                    Ok(p) => HiddenStep::Name(p, is_hidden(n)),
                    Err(_) => HiddenStep::Other,
                }
            }
            _ => HiddenStep::Other,
        })
        .collect();
    let names: Vec<Component> = found.components().filter(|c| !matches!(c, Component::CurDir)).collect();
    if hidden_names_asked_for(&steps, &names, true) {
        return false;
    }
    // `glob` found the path, so the pattern lines up with it once hidden
    // names are not looked at. If it does not even then (a spelling of
    // the path this reading does not foresee), each hidden name is judged
    // by name, as the dotted components of the pattern spell them.
    if hidden_names_asked_for(&steps, &names, false) {
        return true;
    }
    names.iter().any(|c| match c {
        Component::Normal(n) if is_hidden(n) => {
            let name = n.to_string_lossy();
            !steps.iter().any(|s| matches!(s, HiddenStep::Name(p, true) if p.matches_with(&name, GLOB_OPTIONS)))
        }
        _ => false,
    })
}

/// One component of a pattern, for [`hidden_and_not_asked_for`].
enum HiddenStep {
    /// `**`: any number of folders, none of them hidden.
    AnyDepth,
    /// A name or a wildcard, and whether it starts with a dot.
    Name(glob::Pattern, bool),
    /// The root, a drive, `..`: taken as it is.
    Other,
}

/// Whether `names` line up with `steps` so that every hidden name stands
/// where a pattern component starting with a dot matched it (with
/// `strict` false: so that they line up at all).
fn hidden_names_asked_for(steps: &[HiddenStep], names: &[std::path::Component], strict: bool) -> bool {
    use std::path::Component;
    let hidden = |c: &Component| matches!(c, Component::Normal(n) if is_hidden(n));
    let Some((step, rest)) = steps.split_first() else { return names.is_empty() };
    match step {
        HiddenStep::AnyDepth => {
            hidden_names_asked_for(rest, names, strict)
                || names.split_first().is_some_and(|(first, more)| {
                    (!strict || !hidden(first)) && hidden_names_asked_for(steps, more, strict)
                })
        }
        HiddenStep::Name(pattern, dotted) => names.split_first().is_some_and(|(first, more)| {
            let fits = match first {
                Component::Normal(n) => pattern.matches_with(&n.to_string_lossy(), GLOB_OPTIONS),
                _ => false,
            };
            fits && (!strict || *dotted || !hidden(first)) && hidden_names_asked_for(rest, more, strict)
        }),
        HiddenStep::Other => names
            .split_first()
            .is_some_and(|(first, more)| !hidden(first) && hidden_names_asked_for(rest, more, strict)),
    }
}

/// `pattern` with its names without wildcards matched in any case too.
/// `glob` takes such a name as it is and looks it up, so on a file system
/// that tells case apart (Linux) `g2/sub/*.hl7` missed `G2/Sub/x.HL7`,
/// although `*.hl7` finds `x.HL7`. A name that exists as spelled stays as
/// it is (a lookup, nothing to read); any other becomes a pattern that
/// `glob` matches against the folder's entries, ignoring case. A name
/// after a wildcard cannot be looked up in advance, so it becomes one too.
fn any_case_pattern(pattern: &str) -> String {
    use std::path::Component;
    let mut out = PathBuf::new();
    // Where the names so far lead, while each is plain and exists as spelled.
    let mut literal = Some(PathBuf::new());
    for c in Path::new(pattern).components() {
        let Component::Normal(name) = c else {
            out.push(c.as_os_str());
            if let Some(l) = literal.as_mut() {
                l.push(c.as_os_str());
            }
            continue;
        };
        let name = name.to_string_lossy();
        if name.contains(['*', '?', '[']) {
            out.push(&*name);
            literal = None;
            continue;
        }
        match literal.as_ref().map(|l| l.join(&*name)) {
            Some(next) if fs::symlink_metadata(&next).is_ok() => {
                out.push(&*name);
                literal = Some(next);
            }
            _ => {
                out.push(any_case_name(&name));
                literal = None;
            }
        }
    }
    out.to_string_lossy().into_owned()
}

/// `name` as a pattern for exactly that name in any case: its first ASCII
/// letter in brackets (`G2` → `[G]2`), which `glob` then compares ignoring
/// case. A leading dot stays outside, so a hidden name still matches. A
/// name without a letter has no case and stays as it is.
fn any_case_name(name: &str) -> String {
    match name.char_indices().find(|(_, c)| c.is_ascii_alphabetic()) {
        Some((i, c)) => format!("{}[{}]{}", &name[..i], c, &name[i + 1..]),
        None => name.to_string(),
    }
}

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
///
/// The arguments are taken as the OS gives them, not as UTF-8 text: a file
/// name in Latin-1 that the shell passes on Linux is read like any other,
/// where it used to end the whole run with a usage error.
fn expand_paths<P: AsRef<OsStr>>(patterns: &[P]) -> Result<Inputs, String> {
    expand_paths_except(patterns, None)
}

/// What identifies a file whatever path names it: device and inode on
/// Unix, the canonical path in lower case elsewhere (Windows file systems
/// ignore case).
#[cfg(unix)]
type FileId = (u64, u64);
#[cfg(not(unix))]
type FileId = String;

/// The identity of the file at `path`; `None` when there is none yet.
/// Read from its metadata, without opening it: a named pipe would block.
fn file_id(path: &Path) -> Option<FileId> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        fs::metadata(path).ok().map(|m| (m.dev(), m.ino()))
    }
    #[cfg(not(unix))]
    {
        fs::canonicalize(path).ok().map(|p| p.to_string_lossy().to_lowercase())
    }
}

/// The `--output` file, as input discovery must leave it out: a report
/// written inside the folder or pattern being checked (`validate "*.json"
/// --format json --output report.json`) was read as an input by the next
/// run, which then failed on it. `None` when there is no such file yet.
fn output_file(output: Option<&Path>) -> Option<FileId> {
    output.and_then(file_id)
}

/// [`expand_paths`], leaving out the `--output` file `skip`.
fn expand_paths_except<P: AsRef<OsStr>>(patterns: &[P], skip: Option<&FileId>) -> Result<Inputs, String> {
    let mut files = Vec::new();
    let mut unmatched = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut noted = false;
    // Whether `p` is taken: the --output file is not, nor a file already taken.
    let mut add = |files: &mut Vec<PathBuf>, p: PathBuf| -> bool {
        if skip.is_some() && file_id(&p).as_ref() == skip {
            if !noted {
                esafeln!("note: {} is the --output file: it is not read as an input", p.display());
                noted = true;
            }
            return false;
        }
        let key = fs::canonicalize(&p).unwrap_or_else(|_| p.clone());
        if seen.insert(key) {
            files.push(p);
        }
        true
    };
    let stdin = OsStr::new("-");
    if patterns.iter().filter(|p| p.as_ref() == stdin).count() > 1 {
        return Err("standard input (-) can be given only once".into());
    }
    for p in patterns.iter().map(AsRef::as_ref) {
        if p == stdin {
            files.push(PathBuf::from("-"));
            continue;
        }
        // As reports show it, with the bytes that are not UTF-8 as U+FFFD.
        let shown = p.to_string_lossy().into_owned();
        let literal = PathBuf::from(p);
        let output_named = || (shown.clone(), "the --output file, which is not read as an input".to_string());
        if literal.is_file() {
            if !add(&mut files, literal) {
                unmatched.push(output_named());
            }
            continue;
        }
        if literal.is_dir() {
            let why = format!("a folder, not a file: `bridgelab-cli batch {}` checks every file in it", shown);
            unmatched.push((shown, why));
            continue;
        }
        // Named as it is, a named pipe, a process substitution
        // (`<(zcat msg.hl7.gz)`, a /dev/fd path) or a device is read like a
        // file, as the commands that take one message always did. A
        // pattern never picks one up: a pipe nobody writes to would wait
        // for ever.
        if fs::metadata(&literal).is_ok() {
            if !add(&mut files, literal) {
                unmatched.push(output_named());
            }
            continue;
        }
        let mut matched = false;
        // `glob` takes text only; a pattern that is not UTF-8 is matched
        // by non_utf8_matches alone, which keeps its bytes.
        let any_case = p.to_str().map(any_case_pattern);
        let p = any_case.as_deref().map(OsStr::new).unwrap_or(p);
        if let Some(text) = p.to_str() {
            match glob::glob_with(text, GLOB_OPTIONS) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(e) if hidden_and_not_asked_for(p, &e) => {}
                            Ok(e) if e.is_file() => {
                                matched |= add(&mut files, e);
                            }
                            Ok(_) => {}
                            // A folder on the pattern's way that cannot be
                            // read: its files would be missing without a word.
                            Err(e) => unmatched.push((e.path().display().to_string(), format!("cannot read: {}", e.error()))),
                        }
                    }
                }
                Err(e) => {
                    unmatched.push((shown, format!("not a valid pattern: {}", e)));
                    continue;
                }
            }
        }
        let (odd, unreadable) = non_utf8_matches(p);
        for f in odd.into_iter().filter(|f| !hidden_and_not_asked_for(p, f)) {
            matched |= add(&mut files, f);
        }
        // Under `**`, glob reads folders with such names too and has
        // reported the ones it could not.
        for (path, why) in unreadable {
            if !unmatched.iter().any(|(q, _)| *q == path) {
                unmatched.push((path, why));
            }
        }
        if !matched {
            unmatched.push((shown, NO_MATCH.into()));
        }
    }
    Ok(Inputs { files, unmatched })
}

/// One component of a pattern, for [`non_utf8_matches`].
enum PatternStep {
    /// A name without wildcards, taken as it is (as `glob` does), bytes
    /// that are not UTF-8 included.
    Name(OsString),
    Wild(glob::Pattern),
    /// `**`: any number of folders, none included.
    AnyDepth,
}

/// Files `pattern` matches that `glob` passes over. It compares names as
/// UTF-8 text, so on Linux it never matches a name that is not valid UTF-8:
/// such a file is skipped in every folder, and so is every file below such
/// a folder when a wildcard has to match the folder (`*/x.hl7`). A CI run
/// would pass without checking them. This walks the pattern one component
/// at a time as `glob` does, matching each name with the invalid bytes
/// shown as U+FFFD, and keeps the files with such a name in their path:
/// `glob` found the others. Folders on that way that cannot be read come
/// back with the reason, as `glob` reports its own.
///
/// A pattern that is not UTF-8 itself (`caf\xe9/*.hl7`) never reached
/// `glob`, so every file it matches is kept: its plain names are looked up
/// with their bytes, its wildcards matched like the names, with U+FFFD.
fn non_utf8_matches(pattern: &OsStr) -> (Vec<PathBuf>, Vec<(String, String)>) {
    use std::path::Component;
    let mut base = PathBuf::new();
    let mut steps = Vec::new();
    for c in Path::new(pattern).components() {
        match c {
            Component::Prefix(_) | Component::RootDir => base.push(c.as_os_str()),
            Component::Normal(name) => {
                let s = name.to_string_lossy();
                if s == "**" {
                    // `**/**` is `**`; walking it twice would read every
                    // folder once per way of splitting the depth.
                    if !matches!(steps.last(), Some(PatternStep::AnyDepth)) {
                        steps.push(PatternStep::AnyDepth);
                    }
                } else if s.contains(['*', '?', '[']) {
                    match glob::Pattern::new(&s) {
                        Ok(p) => steps.push(PatternStep::Wild(p)),
                        Err(_) => return (vec![], vec![]),
                    }
                } else {
                    steps.push(PatternStep::Name(name.to_os_string()));
                }
            }
            // `.` and `..`
            other => steps.push(PatternStep::Name(other.as_os_str().to_os_string())),
        }
    }
    let mut walk = NonUtf8Walk { files: vec![], unreadable: vec![], visited: Default::default() };
    walk.step(&base, &steps, pattern.to_str().is_none());
    walk.files.sort();
    (walk.files, walk.unreadable)
}

/// What [`non_utf8_matches`] found so far.
struct NonUtf8Walk {
    files: Vec<PathBuf>,
    unreadable: Vec<(String, String)>,
    /// Folders `**` went through: a symbolic link back up the tree must
    /// not send it round for ever.
    visited: std::collections::HashSet<PathBuf>,
}

impl NonUtf8Walk {
    /// Match `steps` from `dir`. `odd` is set once a name on the way is not
    /// valid UTF-8.
    fn step(&mut self, dir: &Path, steps: &[PatternStep], odd: bool) {
        let Some((step, rest)) = steps.split_first() else {
            if odd && dir.is_file() {
                self.files.push(dir.to_path_buf());
            }
            return;
        };
        match step {
            PatternStep::Name(name) => {
                let next = dir.join(name);
                if rest.is_empty() || next.is_dir() {
                    self.step(&next, rest, odd);
                }
            }
            PatternStep::Wild(pattern) => {
                for (child, utf8, is_dir) in self.children(dir, odd) {
                    let name = child.file_name().unwrap_or_default().to_string_lossy();
                    if pattern.matches_with(&name, GLOB_OPTIONS) && (rest.is_empty() || is_dir) {
                        self.step(&child, rest, odd || !utf8);
                    }
                }
            }
            PatternStep::AnyDepth => {
                self.step(dir, rest, odd);
                if !self.visited.insert(fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf())) {
                    return;
                }
                for (child, utf8, is_dir) in self.children(dir, odd) {
                    if is_dir {
                        self.step(&child, steps, odd || !utf8);
                    }
                }
            }
        }
    }

    /// The entries of `dir`: path, whether the name is valid UTF-8, whether
    /// it is a folder (following a symbolic link, as `glob` does). A folder
    /// that cannot be read is noted when `glob` may not have seen it.
    fn children(&mut self, dir: &Path, odd: bool) -> Vec<(PathBuf, bool, bool)> {
        let readable = if dir.as_os_str().is_empty() { Path::new(".") } else { dir };
        match fs::read_dir(readable) {
            Ok(entries) => entries
                .flatten()
                .map(|e| {
                    let is_dir = match e.file_type() {
                        Ok(t) if !t.is_symlink() => t.is_dir(),
                        _ => e.path().is_dir(),
                    };
                    (dir.join(e.file_name()), e.file_name().to_str().is_some(), is_dir)
                })
                .collect(),
            Err(e) => {
                if odd {
                    self.unreadable.push((dir.display().to_string(), format!("cannot read: {}", e)));
                }
                vec![]
            }
        }
    }
}

/// One row of a `validate` run: a file (or an argument that matched none)
/// and what came of it.
struct ValidateRow {
    name: String,
    result: Result<Report, String>,
    /// The argument matched no file.
    missing: bool,
}

fn cmd_validate(
    patterns: &[OsString],
    format: Format,
    strict: bool,
    fhir_packages: Option<&Path>,
    no_plugins: bool,
    output: Option<&Path>,
) -> i32 {
    let inputs = match expand_paths_except(patterns, output_file(output).as_ref()) {
        Ok(i) => i,
        Err(e) => {
            esafeln!("Error: {}", e);
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
    outln!("{}", json_text(&rows));
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
            // XML allows these, but raw they reach a terminal that shows
            // the report; as references they read back the same.
            c if is_del_or_c1(c) => out.push_str(&format!("&#{};", c as u32)),
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

fn cmd_info(patterns: &[OsString], as_json: bool) -> i32 {
    let inputs = match expand_paths(patterns) {
        Ok(i) => i,
        Err(e) => {
            esafeln!("Error: {}", e);
            return 2;
        }
    };
    let mut records: Vec<InfoRecord> = inputs.files.iter().map(|f| info_for(f)).collect();
    for (p, why) in &inputs.unmatched {
        esafeln!("Error: {}: {}", p, why);
        records.push(blank_record(p.clone(), 0, why.clone()));
    }

    if as_json {
        outln!("{}", json_text(&records));
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
            esafeln!("Error: not anonymized: {}", e);
            return 1;
        }
    };
    let msg = match parse_hl7_only(text, "anonymize") {
        Ok(m) => m,
        Err(e) => {
            esafeln!("Parse error: {}", e);
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
                esafeln!("Write error: {}", e);
                return 1;
            }
            esafeln!("Anonymized written to: {}", path.display());
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
            esafeln!("Error: {}", e);
            return 1;
        }
    };
    let msg = match parse_hl7_only(content, "to-json") {
        Ok(m) => m,
        Err(e) => {
            esafeln!("Parse error: {}", e);
            return 1;
        }
    };
    let messages = bridgelab_lib::parser::hl7::export::message_count(&msg);
    if messages > 1 {
        esafeln!("note: the input holds {} messages: the JSON is an array, one document per message", messages);
    }

    let json_str = match bridgelab_lib::parser::hl7::export::to_json(&msg) {
        Ok(j) => json_controls_escaped(j),
        Err(e) => {
            esafeln!("Error: {}", e);
            return 1;
        }
    };

    match output {
        Some(path) => {
            if let Err(e) = write_output(path, json_str.as_bytes()) {
                esafeln!("Write error: {}", e);
                return 1;
            }
            esafeln!("JSON written to: {}", path.display());
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

fn cmd_batch(
    dir: &Path,
    extension: &str,
    as_json: bool,
    fhir_packages: Option<&Path>,
    no_plugins: bool,
    output: Option<&Path>,
) -> i32 {
    // Taken before the walk: the report of the previous run, written in
    // the folder, is not one of its messages.
    let skip = output_file(output);
    if !dir.is_dir() {
        esafeln!("Error: not a directory: {}", dir.display());
        return 2;
    }
    let extensions = parse_extensions(extension);
    if extensions.is_empty() {
        esafeln!("Error: --extension names no extension");
        return 2;
    }
    let engines = Engines::new(fhir_packages, no_plugins);

    let mut results = Vec::new();
    let mut total_errors = 0;
    let mut total_warnings = 0;
    let mut valid_files = 0;

    // Subdirectories included, symbolic links followed, in name order so
    // two runs list the files alike. A file or folder that cannot be read
    // is a failed entry, not a silent gap. Hidden files and folders (the
    // `._name.hl7` macOS leaves beside each file on other file systems,
    // `.git`) are not messages, as for patterns: below the folder given,
    // they are passed over.
    let walk = walkdir::WalkDir::new(dir).follow_links(true).sort_by_file_name().into_iter();
    for entry in walk.filter_entry(|e| e.depth() == 0 || !is_hidden(e.file_name())) {
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
        if skip.is_some() && file_id(path) == skip {
            esafeln!("note: {} is the --output file: it is not read as an input", path.display());
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
        outln!("{}", json_text(&summary));
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
        esafeln!(
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
            esafeln!("Error: {}", e);
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
            esafeln!("Error: the resource could not be read as a JSON tree");
            return 1;
        }
        Err(e) => {
            esafeln!("Error: {}", e);
            return 1;
        }
    };
    let result = fhir::fhirpath::evaluate(expression, &json);
    if as_json {
        outln!("{}", json_text(&result));
    } else if let Some(e) = &result.error {
        esafeln!("FHIRPath error: {}", e);
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

/// The message structure an `xsd` argument names: `ADT_A01` as it is,
/// `ADT^A01` as `ADT_A01`, and a whole MSH-9 such as `ADT^A04^ADT_A01` by
/// its third component, the structure (A04 uses the A01 structure); it
/// used to become `ADT_A04_ADT_A01`, which no catalogue holds.
fn message_structure(message: &str) -> String {
    let parts: Vec<&str> = message.trim().split('^').map(str::trim).collect();
    let code = match parts.as_slice() {
        [_, _, structure, ..] if !structure.is_empty() => structure.to_string(),
        [code, event, ..] if !event.is_empty() => format!("{}_{}", code, event),
        _ => parts[0].to_string(),
    };
    code.to_uppercase()
}

fn cmd_xsd(message: &str, version: &str, output: Option<&Path>) -> i32 {
    let Some(v) = Hl7Version::parse(version) else {
        esafeln!("Error: unknown HL7 version '{}' (e.g. 2.3, 2.5.1, 2.7)", version);
        return 2;
    };
    let code = message_structure(message);
    let schema = hl7schema::cached(v);
    if schema.message(&code).is_none() {
        esafeln!("Error: message '{}' not found in HL7 v{}", code, v.as_str());
        return 2;
    }
    if !hl7schema::xsd_in_community(v, &code) {
        esafeln!(
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
            esafeln!("Error: {}", e);
            return 1;
        }
    };
    match output {
        Some(path) => {
            if let Err(e) = write_output(path, xsd.as_bytes()) {
                esafeln!("Write error: {}", e);
                return 1;
            }
            esafeln!("XSD written to: {}", path.display());
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
    /// Why nothing was sent, when a check of the messages' charsets refused
    /// the file before anything went out (`control_id` is then the MSH-10
    /// of the message concerned).
    #[serde(skip_serializing_if = "Option::is_none")]
    refused: Option<SendRefusal>,
}

/// A file refused before sending for the charsets its messages declare.
#[derive(Serialize, Clone)]
struct SendRefusal {
    /// `charset_not_written`: a message's MSH-18 names a charset BridgeLab
    /// cannot send in. `mixed_charsets`: sent in the file's one charset, a
    /// message would reach the receiver in other bytes than its MSH-18 says.
    reason: &'static str,
    /// The message concerned, from 1.
    message: usize,
    /// Its MSH-18, null when empty.
    declared: Option<String>,
    /// The charset its MSH-18 means (mixed_charsets only).
    #[serde(skip_serializing_if = "Option::is_none")]
    charset: Option<String>,
    /// The charset the file would be sent in (mixed_charsets only).
    #[serde(skip_serializing_if = "Option::is_none")]
    sent_in: Option<String>,
}

/// A send refused before anything went out: with --json the report on
/// standard output, as every send gives one; otherwise the reason on
/// standard error. Returns the exit code.
fn send_refused(host: &str, port: u16, as_json: bool, error: String, control_id: Option<String>, refused: Option<SendRefusal>) -> i32 {
    if as_json {
        let report = SendReport {
            host: host.into(),
            port,
            delivered: false,
            ack_code: None,
            response_time_ms: 0,
            response: String::new(),
            error: Some(error),
            control_id: control_id.unwrap_or_default(),
            results: vec![],
            refused,
        };
        outln!("{}", json_text(&report));
    } else {
        esafeln!("Error: {}", error);
    }
    1
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
        esafeln!("Error: unknown encoding '{}' (e.g. UTF-8, ISO-8859-1, windows-1252, ASCII)", encoding);
        return 2;
    }
    let Decoded { text, charset: source_charset, .. } = match read_message_text(input) {
        Ok(d) => d,
        Err(e) => {
            esafeln!("Error: not sent: {}", e);
            return 1;
        }
    };
    // Messages on disk often use \n or \r\n between segments; HL7 wants \r.
    let text = text.replace("\r\n", "\r").replace('\n', "\r");
    let encoding_given = !encoding.trim().is_empty();
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
        esafeln!("Error: the input holds no message");
        return 1;
    }
    // Without --encoding the wire charset comes from the messages: one
    // whose MSH-18 names a charset BridgeLab does not write is refused, as
    // in the app, not sent as UTF-8 under a header that says otherwise.
    // Every message is checked before any goes out (reading the file looks
    // at the first header only, and a UTF-16 file not even at that one).
    if !encoding_given {
        for (i, message) in messages.iter().enumerate() {
            if let Some(unwritten) = mllp::msh18_charset_not_written(message) {
                let which = mllp::UnwrittenCharset { message: i + 1, ..unwritten };
                let error = format!("not sent: {}; no message of the file was sent; correct MSH-18, or give --encoding", which);
                let refused = SendRefusal {
                    reason: "charset_not_written",
                    message: which.message,
                    declared: Some(which.charset.clone()),
                    charset: None,
                    sent_in: None,
                };
                return send_refused(host, port, as_json, error, which.control_id, Some(refused));
            }
        }
        // Every message goes out in the one charset chosen for the file: a
        // later one declaring another would reach the receiver in the wrong
        // bytes, unless its text reads the same in both (plain ASCII). An
        // empty MSH-18 means the file's charset.
        let wire = if encoding.is_empty() { "UTF-8" } else { encoding };
        if let Some(mixed) = mllp::mixed_declared_charsets(&messages.concat(), source_charset.as_deref(), wire) {
            let error = format!(
                "not sent: {}; the file is sent in one charset, so no message of it was sent; send the messages in separate files, or give --encoding",
                mixed
            );
            let refused = SendRefusal {
                reason: "mixed_charsets",
                message: mixed.message,
                declared: mixed.declared.clone(),
                charset: Some(mixed.charset.clone()),
                sent_in: Some(mixed.sent_in.clone()),
            };
            return send_refused(host, port, as_json, error, mixed.control_id.clone(), Some(refused));
        }
    }
    if dropped > 0 {
        esafeln!("note: {} batch envelope segment(s) (FHS/BHS/BTS/FTS) not sent: MLLP carries one message per frame", dropped);
    }
    // Each message must be HL7 v2 before anything goes out. Text that is
    // not has no MSH-10, so the ACK check was skipped, and a peer that
    // acknowledges whatever it gets made "hello world" a delivered,
    // accepted message (exit 0).
    let mut control_ids = Vec::with_capacity(messages.len());
    for (i, message) in messages.iter().enumerate() {
        match parse_hl7_only(message.clone(), "send") {
            Ok(m) => control_ids.push(
                m.segments
                    .first()
                    .and_then(|msh| msh.fields.iter().find(|f| f.position == 10))
                    .map(|f| f.span.as_str(&m.raw).trim().to_string())
                    .unwrap_or_default(),
            ),
            Err(e) if messages.len() > 1 => {
                return send_refused(host, port, as_json, format!("not sent: message {} of {} is not an HL7 v2 message ({})", i + 1, messages.len(), e), None, None)
            }
            Err(e) => return send_refused(host, port, as_json, format!("not sent: the input is not an HL7 v2 message ({})", e), None, None),
        }
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
        return send_refused(host, port, as_json, error, None, None);
    }

    let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(r) => r,
        Err(e) => {
            esafeln!("Error: {}", e);
            return 1;
        }
    };
    let total = messages.len();
    let mut reports: Vec<SendReport> = Vec::new();
    let mut all_accepted = true;
    for (i, (message, control_id)) in messages.iter().zip(control_ids).enumerate() {
        let result = runtime.block_on(mllp::send(host, port, message, timeout, encoding));
        let ack_code = if result.success { result.ack_code.clone() } else { None };
        // A reply that is not HL7, or the ACK of another message, accepts
        // nothing sent here: the library checks it for the app and the CLI
        // alike (MSA-1 present, MSA-2 equal to MSH-10).
        let error = result.error.clone().or_else(|| result.reply_problem.as_ref().map(ToString::to_string));
        let accepted = result.reply_problem.is_none() && matches!(ack_code.as_deref(), Some("AA") | Some("CA"));
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
            refused: None,
        };
        if !as_json {
            let which = if total > 1 { format!("message {}/{} ", i + 1, total) } else { String::new() };
            if !result.success {
                esafeln!("Send {}failed: {}", which, report.error.as_deref().unwrap_or("unknown error"));
            } else {
                // MSA-1 is the peer's text, as it sent it.
                esafeln!(
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
                esafeln!("Stopped: {} of {} message(s) not sent", total - i - 1, total);
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
        outln!("{}", json_text(&last));
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

fn cmd_test(
    patterns: &[OsString],
    format: Format,
    fhir_packages: Option<&Path>,
    no_plugins: bool,
    output: Option<&Path>,
) -> i32 {
    let inputs = match expand_paths_except(patterns, output_file(output).as_ref()) {
        Ok(i) => i,
        Err(e) => {
            esafeln!("Error: {}", e);
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
                json_text(&serde_json::json!({
                    "total": total,
                    "passed": passed,
                    "failed": failed,
                    "skipped": skipped,
                    "pack_errors": errors,
                    "results": results,
                }))
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

    #[test]
    fn the_cap_note_names_the_community_edition_once() {
        let note = cli_cap_note("1 plugin pack not applied: the Community edition runs 3 packs at a time (Settings → Plugins)");
        assert_eq!(
            note,
            "1 plugin pack not applied: the CLI, like the Community edition, runs 3 packs at a time (switch packs off in the app's Settings → Plugins)"
        );
        assert_eq!(note.matches("Community edition").count(), 1);
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("bl-cli-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// Splits an example command line into arguments: whitespace-separated,
    /// double quotes group (the shell would strip them).
    fn shell_words(line: &str) -> Vec<String> {
        let (mut words, mut word, mut quoted) = (Vec::new(), String::new(), false);
        for c in line.chars() {
            match c {
                '"' => quoted = !quoted,
                c if c.is_whitespace() && !quoted => {
                    if !word.is_empty() {
                        words.push(std::mem::take(&mut word));
                    }
                }
                c => word.push(c),
            }
        }
        if !word.is_empty() {
            words.push(word);
        }
        words
    }

    #[test]
    fn help_examples_are_valid_command_lines() {
        let all = [
            EXAMPLES, EXAMPLES_VALIDATE, EXAMPLES_INFO, EXAMPLES_ANONYMIZE, EXAMPLES_TO_JSON,
            EXAMPLES_FHIRPATH, EXAMPLES_XSD, EXAMPLES_SEND, EXAMPLES_TEST, EXAMPLES_BATCH,
        ];
        let mut checked = 0;
        for line in all.iter().flat_map(|t| t.lines()) {
            // The command after any `cat ... |`, without the `> file` redirection.
            let Some(cmd) = line.split("bridgelab-cli ").nth(1) else { continue };
            if cmd.contains('<') {
                continue; // the "bridgelab-cli <command> --help" pointer
            }
            let cmd = cmd.split(" > ").next().unwrap();
            let args = std::iter::once("bridgelab-cli".to_string()).chain(shell_words(cmd));
            if let Err(e) = Cli::try_parse_from(args) {
                panic!("help example does not parse: {line}\n{e}");
            }
            checked += 1;
        }
        assert!(checked >= 25, "only {checked} examples checked");
    }

    /// Windows PowerShell 5.1 writes a `>` redirection as UTF-16: a JUnit
    /// report that declares UTF-8 could not be read, so the examples use
    /// --output, which every command that writes a report or a file has.
    #[test]
    fn help_examples_write_files_with_output_not_redirection() {
        let all = [
            EXAMPLES, EXAMPLES_VALIDATE, EXAMPLES_INFO, EXAMPLES_ANONYMIZE, EXAMPLES_TO_JSON,
            EXAMPLES_FHIRPATH, EXAMPLES_XSD, EXAMPLES_SEND, EXAMPLES_TEST, EXAMPLES_BATCH,
        ];
        for line in all.iter().flat_map(|t| t.lines()) {
            assert!(!line.contains(" > "), "redirects: {line}");
        }
        for command in ["validate", "test", "batch", "anonymize", "to-json", "xsd"] {
            let args = ["bridgelab-cli", command, "x", "--output", "out"];
            let parsed = Cli::try_parse_from(args);
            assert!(parsed.is_ok(), "{command} has no --output: {:?}", parsed.err().map(|e| e.to_string()));
        }
    }

    /// --output for a report: the bytes the command printed, in a file
    /// written atomically; a usage error writes nothing.
    #[test]
    fn a_report_goes_to_its_output_file() {
        let d = tmp("report-out");
        let msg = d.join("m.hl7");
        fs::write(&msg, "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||1^^^H^MR||Doe^Jo||19800101|F\r").unwrap();
        let out = d.join("results.xml");
        let code = with_output(Some(&out), || {
            cmd_validate(&[msg.clone().into_os_string()], Format::Junit, true, None, true, Some(&out))
        });
        assert_eq!(code, 0);
        let xml = fs::read_to_string(&out).unwrap();
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuites"), "{xml}");
        assert!(xml.ends_with("</testsuites>\n"), "{xml}");
        // `-` twice is a usage error: the earlier report stays.
        let dash = OsString::from("-");
        let code = with_output(Some(&out), || {
            cmd_validate(&[dash.clone(), dash.clone()], Format::Junit, true, None, true, Some(&out))
        });
        assert_eq!(code, 2);
        assert_eq!(fs::read_to_string(&out).unwrap(), xml);
        // Nothing is left collecting: the next report prints again.
        assert!(CAPTURE.with(|c| c.borrow().is_none()));
        fs::remove_dir_all(&d).ok();
    }

    /// A report written inside the pattern or folder being checked was
    /// read as an input by the next run, which failed on it: a green CI
    /// run turned red the second time. The second run now gives the same
    /// exit code and the same report, whatever path names the report.
    #[test]
    fn a_report_inside_the_inputs_is_not_read_back() {
        let d = tmp("report-inside");
        let adt = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rEVN|A01|20240101\rPID|1||1^^^H^MR||Doe^Jo||19800101|F\r";
        fs::write(d.join("p.json"), "{\"resourceType\": \"Patient\", \"id\": \"p\"}").unwrap();
        fs::write(d.join("a.hl7"), adt).unwrap();
        let pack = serde_json::json!({"format": "bridgelab-test-cases", "format_version": 1,
            "test_cases": [{"name": "admit", "content": adt, "expected_message_type": "ADT^A01"}]});
        let packs_dir = tmp("report-inside-packs");
        fs::write(packs_dir.join("cases.json"), pack.to_string()).unwrap();

        let report = d.join("report.json");
        let twice = |run: &dyn Fn(&Path) -> i32, out: &Path| {
            let _ = fs::remove_file(out);
            // The same file under another spelling, for the last run.
            let folder = out.parent().unwrap();
            fs::create_dir_all(folder.join("sub")).unwrap();
            let respelled = folder.join("sub").join("..").join(out.file_name().unwrap());
            let first = (with_output(Some(out), || run(out)), fs::read(out).unwrap());
            for again in [out, respelled.as_path()] {
                let next = (with_output(Some(again), || run(again)), fs::read(out).unwrap());
                assert_eq!(first, next, "{}", String::from_utf8_lossy(&next.1));
            }
            first.0
        };

        let all_json = OsString::from(format!("{}/*.json", d.display()));
        let validate = |out: &Path| cmd_validate(std::slice::from_ref(&all_json), Format::Json, true, None, true, Some(out));
        assert_eq!(twice(&validate, &report), 0);

        let batch = |out: &Path| cmd_batch(&d, "json", true, None, true, Some(out));
        assert_eq!(twice(&batch, &report), 0);

        let pack_report = packs_dir.join("results.json");
        let packs = OsString::from(format!("{}/*.json", packs_dir.display()));
        let test = |out: &Path| cmd_test(std::slice::from_ref(&packs), Format::Json, None, true, Some(out));
        assert_eq!(twice(&test, &pack_report), 0);

        // Named as it is, the report is not read either: an input error,
        // as on the first run, when it did not exist yet.
        let named = [d.join("p.json").into_os_string(), report.clone().into_os_string()];
        let validate_named = |out: &Path| cmd_validate(&named, Format::Text, true, None, true, Some(out));
        let _ = fs::remove_file(&report);
        assert_eq!(with_output(Some(&report), || validate_named(&report)), 1);
        assert_eq!(with_output(Some(&report), || validate_named(&report)), 1);
        let text = fs::read_to_string(&report).unwrap();
        assert!(text.contains("INPUT ERROR: the --output file"), "{text}");
        fs::remove_dir_all(&d).ok();
        fs::remove_dir_all(&packs_dir).ok();
    }

    /// `*.hl7` took in the `._name.hl7` files macOS leaves on shared
    /// drives, which are no messages and failed the run; `**` went into
    /// `.git`. A hidden name matches only a pattern that spells the dot.
    #[test]
    fn patterns_leave_hidden_files_and_folders_out() {
        let d = tmp("hidden");
        fs::create_dir_all(d.join(".git")).unwrap();
        fs::create_dir_all(d.join("sub")).unwrap();
        for f in ["a.hl7", "._a.hl7", ".git/b.hl7", "sub/c.hl7", "sub/._c.hl7"] {
            fs::write(d.join(f), "MSH|^~\\&|").unwrap();
        }
        let names = |pattern: &str| {
            let inputs = expand_paths(&[format!("{}/{}", d.display(), pattern)]).unwrap();
            let mut n: Vec<String> =
                inputs.files.iter().map(|f| f.strip_prefix(&d).unwrap().display().to_string()).collect();
            n.sort();
            n
        };
        assert_eq!(names("*.hl7"), ["a.hl7"]);
        assert_eq!(names("**/*.hl7"), ["a.hl7", "sub/c.hl7"]);
        assert_eq!(names(".*.hl7"), ["._a.hl7"]);
        assert_eq!(names(".git/*.hl7"), [".git/b.hl7"]);
        // Named as it is, a hidden file is read.
        assert_eq!(expand_paths(&[d.join("._a.hl7")]).unwrap().files.len(), 1);
        fs::remove_dir_all(&d).ok();
    }

    /// A hidden folder the pattern names lets in only itself, where it
    /// stands in the pattern: compared by name alone, the same name further
    /// down, reached through `**`, passed as well.
    #[test]
    fn a_hidden_name_in_a_pattern_counts_only_where_it_stands() {
        let d = tmp("hidden-pos");
        fs::create_dir_all(d.join(".cache/sub/.cache")).unwrap();
        fs::create_dir_all(d.join("sub/.cache")).unwrap();
        fs::create_dir_all(d.join(".git/.cache")).unwrap();
        for f in [
            ".cache/a.hl7",
            ".cache/sub/a.hl7",
            ".cache/sub/.cache/bad.hl7",
            ".cache/sub/._x.hl7",
            "sub/.cache/in.hl7",
            ".git/.cache/x.hl7",
            "sub/.y.hl7",
        ] {
            fs::write(d.join(f), "MSH|^~\\&|").unwrap();
        }
        let names = |pattern: &str| {
            let inputs = expand_paths(&[format!("{}/{}", d.display(), pattern)]).unwrap();
            let mut n: Vec<String> =
                inputs.files.iter().map(|f| f.strip_prefix(&d).unwrap().display().to_string()).collect();
            n.sort();
            n
        };
        assert_eq!(names(".cache/**/*.hl7"), [".cache/a.hl7", ".cache/sub/a.hl7"]);
        // In any case, as folder names are.
        assert_eq!(names(".CACHE/**/*.hl7"), [".cache/a.hl7", ".cache/sub/a.hl7"]);
        assert_eq!(names("**/.cache/*.hl7"), [".cache/a.hl7", "sub/.cache/in.hl7"]);
        // A dot at the file's place matches hidden files there, and only there.
        assert_eq!(names("*/.*.hl7"), ["sub/.y.hl7"]);
        assert_eq!(names(".cache/*/.*.hl7"), [".cache/sub/._x.hl7"]);
        fs::remove_dir_all(&d).ok();
    }

    /// `batch` passes over the same hidden files and folders.
    #[test]
    fn batch_leaves_hidden_files_out() {
        let d = tmp("batch-hidden");
        fs::create_dir_all(d.join(".cache")).unwrap();
        fs::write(d.join("a.hl7"), "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rEVN|A01|20240101\rPID|1||1^^^H^MR||Doe^Jo||19800101|F\r").unwrap();
        fs::write(d.join("._a.hl7"), [0u8, 5, 22, 7, 0, 2]).unwrap();
        fs::write(d.join(".cache").join("x.hl7"), "not a message").unwrap();
        assert_eq!(cmd_batch(&d, "hl7", true, None, true, None), 0);
        fs::remove_dir_all(&d).ok();
    }

    /// Patterns ignore case, folder names included: `glob` looked names
    /// without wildcards up as spelled, so on Linux `g2/sub/*.hl7` missed
    /// G2/Sub/x.HL7.
    #[test]
    fn folder_names_in_a_pattern_ignore_case() {
        let d = tmp("case");
        fs::create_dir_all(d.join("G2").join("Sub")).unwrap();
        fs::write(d.join("G2").join("Sub").join("x.HL7"), "MSH|^~\\&|").unwrap();
        for pattern in ["g2/sub/*.hl7", "G2/Sub/*.hl7", "*/sub/*.hl7", "g2/sub/X.hl7", "**/sub/x.hl7"] {
            let inputs = expand_paths(&[format!("{}/{}", d.display(), pattern)]).unwrap();
            assert_eq!(inputs.files.len(), 1, "{pattern}: {:?}", inputs.unmatched);
        }
        let none = expand_paths(&[format!("{}/g3/sub/*.hl7", d.display())]).unwrap();
        assert_eq!(none.unmatched.len(), 1);
        assert_eq!(any_case_name("G2"), "[G]2");
        assert_eq!(any_case_name(".cache"), ".[c]ache");
        assert_eq!(any_case_name("2024"), "2024");
        fs::remove_dir_all(&d).ok();
    }

    /// Process substitution, named pipes and /dev/fd paths were refused
    /// by validate, info and test as matching no file.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_named_pipe_given_by_name_is_read() {
        let d = tmp("fifo");
        let fifo = d.join("msg.hl7");
        let made = std::process::Command::new("mkfifo").arg(&fifo).status().map(|s| s.success()).unwrap_or(false);
        if !made {
            return; // no mkfifo here
        }
        let inputs = expand_paths(&[fifo.as_os_str()]).unwrap();
        assert_eq!(inputs.files, vec![fifo.clone()]);
        assert!(inputs.unmatched.is_empty());
        // A pattern does not pick a pipe up: one nobody writes to would wait for ever.
        let by_pattern = expand_paths(&[format!("{}/*.hl7", d.display())]).unwrap();
        assert!(by_pattern.files.is_empty());
        let writer = {
            let fifo = fifo.clone();
            std::thread::spawn(move || fs::write(fifo, "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1\r"))
        };
        let record = info_for(&fifo);
        writer.join().unwrap().unwrap();
        assert!(record.parsed, "{:?}", record.error);
        assert_eq!(record.message_type, "ADT^A01");
        fs::remove_dir_all(&d).ok();
    }

    /// The whole MSH-9 names the structure in its third component: it
    /// became ADT_A01_ADT_A01, "not found", exit 2.
    #[test]
    fn xsd_takes_a_whole_msh9() {
        assert_eq!(message_structure("ADT_A01"), "ADT_A01");
        assert_eq!(message_structure("adt^a01"), "ADT_A01");
        assert_eq!(message_structure("ADT^A01^ADT_A01"), "ADT_A01");
        assert_eq!(message_structure("ADT^A04^ADT_A01"), "ADT_A01");
        assert_eq!(message_structure("ORU^R01^"), "ORU_R01");
        let d = tmp("xsd");
        let out = d.join("adt.xsd");
        assert_eq!(cmd_xsd("ADT^A04^ADT_A01", "2.5", Some(&out)), 0);
        assert!(fs::read_to_string(&out).unwrap().contains("ADT_A01"));
        fs::remove_dir_all(&d).ok();
    }

    /// to-json and anonymize said a FHIR resource "does not start with
    /// MSH"; they now name it.
    #[test]
    fn hl7_only_commands_name_a_fhir_resource() {
        let e = parse_hl7_only("{\"resourceType\": \"Patient\", \"id\": \"p\"}".into(), "to-json").unwrap_err();
        assert!(e.starts_with("a FHIR resource") && e.contains("to-json"), "{e}");
        let e = parse_hl7_only("<Patient xmlns=\"http://hl7.org/fhir\"><id value=\"p\"/></Patient>".into(), "anonymize").unwrap_err();
        assert!(e.starts_with("a FHIR resource"), "{e}");
        let d = tmp("fhir-tojson");
        let p = d.join("p.json");
        fs::write(&p, "{\"resourceType\": \"Patient\", \"id\": \"p\"}").unwrap();
        assert_eq!(cmd_to_json(&p, None), 1);
        assert_eq!(cmd_anonymize(&p, None, true), 1);
        fs::remove_dir_all(&d).ok();
    }

    /// Text that is not HL7 v2 has no MSH-10 to match the ACK against: it
    /// was sent, and a peer that acknowledges anything made it exit 0.
    #[test]
    fn send_refuses_text_that_is_not_hl7() {
        let d = tmp("send-text");
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.set_nonblocking(true).unwrap();
        let port = l.local_addr().unwrap().port();
        for (name, text) in [
            ("hello.txt", "hello world\n"),
            ("p.json", "{\"resourceType\": \"Patient\", \"id\": \"p\"}"),
            ("second.hl7", "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1\rMSH\r"),
        ] {
            let input = d.join(name);
            fs::write(&input, text).unwrap();
            assert_eq!(cmd_send(&input, "127.0.0.1", port, 2, "", true), 1, "{name}");
        }
        assert!(l.accept().is_err(), "nothing was sent");
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn version_carries_the_build_code() {
        use clap::CommandFactory;
        let version = Cli::command().render_version();
        assert!(version.starts_with(&format!("bridgelab-cli {} (build ", env!("CARGO_PKG_VERSION"))), "{version}");
        assert!(version.contains(bridgelab_lib::BUILD_ID), "{version}");
        assert!(!bridgelab_lib::BUILD_ID.is_empty());
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

    /// DEL and the C1 controls (U+009B is CSI) are legal in XML but act on
    /// some terminals: references instead. Other C0 controls cannot be in
    /// XML at all, not even as references.
    #[test]
    fn junit_writes_del_and_c1_controls_as_references() {
        assert_eq!(xml_escape("a\u{9b}31mb\u{7f}c\u{80}\u{9f}\u{a0}"), "a&#155;31mb&#127;c&#128;&#159;\u{a0}");
        assert_eq!(xml_escape("x\u{1b}[2J\u{7}"), "x\u{fffd}[2J\u{fffd}");
    }

    /// serde_json escapes only U+0000 to U+001F; DEL and C1 went out raw.
    #[test]
    fn json_output_escapes_del_and_c1_controls() {
        let value = serde_json::json!({"PID-5": "X\u{9b}2J\u{9b}31mFAKE OK\u{9d}0;t\u{9c}\u{7f}", "esc": "\u{1b}[2J"});
        let text = json_text(&value);
        assert!(!text.chars().any(|c| c.is_control() && c != '\n'), "{text:?}");
        assert!(text.contains(r"X\u009b2J\u009b31mFAKE OK\u009d0;t\u009c\u007f"), "{text}");
        assert!(text.contains(r"\u001b[2J"), "{text}");
        // The same JSON, read back.
        assert_eq!(serde_json::from_str::<serde_json::Value>(&text).unwrap(), value);
        // Nothing to escape: the text as serde_json wrote it.
        assert_eq!(json_controls_escaped("{\"a\": \"é\"}".into()), "{\"a\": \"é\"}");
    }

    /// Every line on standard error goes through esafeln!, so none can
    /// pass an escape sequence to the terminal: a plain eprintln! printed
    /// pack rule ids, file names, arguments, MSH-18 and a peer's MSA-1 raw.
    /// clippy.toml refuses them too, for whoever runs clippy.
    #[test]
    fn stderr_is_written_only_through_esafeln() {
        let source = include_str!("main.rs");
        for banned in [concat!("epr", "intln!"), concat!("epr", "int!")] {
            let hits: Vec<usize> = source
                .lines()
                .enumerate()
                .filter(|(_, l)| l.contains(banned) && !l.trim_start().starts_with("//"))
                .map(|(i, _)| i + 1)
                .collect();
            assert!(hits.is_empty(), "{banned} at main.rs lines {hits:?}: use esafeln!");
        }
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

    /// With a wildcard in the folder part ("**/*.hl7", the
    /// README's CI example) such a file was left out without a word and the
    /// run exited 0, a pass for a file never checked.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_file_name_that_is_not_utf8_is_matched_under_folder_wildcards() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let d = tmp("nonutf8-deep");
        let msg = "MSH|^~\\&|";
        fs::create_dir_all(d.join("sub")).unwrap();
        fs::write(d.join("sub").join("ok.hl7"), msg).unwrap();
        if fs::write(d.join(OsStr::from_bytes(b"bad\xe9.hl7")), msg).is_err() {
            return; // a file system that refuses such names
        }
        fs::write(d.join("sub").join(OsStr::from_bytes(b"bad\xe9.hl7")), msg).unwrap();
        let odd_dir = d.join(OsStr::from_bytes(b"caf\xe9"));
        fs::create_dir(&odd_dir).unwrap();
        fs::write(odd_dir.join("in.hl7"), msg).unwrap();

        let found = |pattern: &str| {
            let inputs = expand_paths(&[format!("{}/{}", d.display(), pattern)]).unwrap();
            assert!(inputs.unmatched.is_empty(), "{}: {:?}", pattern, inputs.unmatched);
            inputs.files.len()
        };
        // Every folder, the pattern's own included; café/in.hl7 once,
        // although glob walks that folder too.
        assert_eq!(found("**/*.hl7"), 4);
        assert_eq!(found("**/bad?.hl7"), 2);
        // A wildcard folder matching a name that is not UTF-8.
        assert_eq!(found("*/*.hl7"), 3);
        assert_eq!(found("*/in.hl7"), 1);
        // Nothing to find is still reported.
        let none = expand_paths(&[format!("{}/**/*.xml", d.display())]).unwrap();
        assert!(none.files.is_empty());
        assert_eq!(none.unmatched.len(), 1);
        fs::remove_dir_all(&d).ok();
    }

    /// Such a name given as it is (typed, or expanded by the shell) is read
    /// like any other: the command line used to be refused as a whole
    /// ("invalid UTF-8 was detected", exit 2).
    #[cfg(target_os = "linux")]
    #[test]
    fn a_file_name_that_is_not_utf8_is_matched_when_given_as_it_is() {
        use std::os::unix::ffi::OsStrExt;
        let d = tmp("nonutf8-literal");
        let msg = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|X|P|2.5\rPID|1\r";
        let bad = d.join(OsStr::from_bytes(b"bad\xe9name.hl7"));
        if fs::write(&bad, msg).is_err() {
            return; // a file system that refuses such names
        }
        let odd_dir = d.join(OsStr::from_bytes(b"caf\xe9"));
        fs::create_dir(&odd_dir).unwrap();
        fs::write(odd_dir.join("in.hl7"), msg).unwrap();

        let args = [OsString::from("bridgelab-cli"), OsString::from("info"), bad.clone().into_os_string()];
        assert!(Cli::try_parse_from(args).is_ok(), "the command line is accepted");
        assert_eq!(cmd_info(&[bad.clone().into_os_string()], true), 0);

        let inputs = expand_paths(&[bad.as_os_str()]).unwrap();
        assert_eq!(inputs.files, vec![bad.clone()]);
        assert!(inputs.unmatched.is_empty());

        // A pattern with such a folder name in it, which glob cannot take.
        let inputs = expand_paths(&[odd_dir.join("*.hl7")]).unwrap();
        assert_eq!(inputs.files, vec![odd_dir.join("in.hl7")]);
        assert!(inputs.unmatched.is_empty(), "{:?}", inputs.unmatched);

        // One that does not exist is reported, shown with U+FFFD.
        let inputs = expand_paths(&[d.join(OsStr::from_bytes(b"gone\xe9.hl7"))]).unwrap();
        assert!(inputs.files.is_empty());
        assert_eq!(inputs.unmatched.len(), 1);
        assert!(inputs.unmatched[0].0.ends_with("gone\u{FFFD}.hl7"), "{:?}", inputs.unmatched);
        assert_eq!(inputs.unmatched[0].1, NO_MATCH);
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

    /// A file of several messages whose later one declares a charset
    /// BridgeLab cannot write is refused whole, before any message goes
    /// out: only the first header was checked.
    #[test]
    fn send_checks_every_message_s_declared_charset_first() {
        let d = tmp("sendmulti");
        let input = d.join("multi.hl7");
        fs::write(
            &input,
            "MSH|^~\\&|A|B|C|D|20260101||ADT^A01|M1|P|2.5\rPID|1\rMSH|^~\\&|A|B|C|D|20260101||ADT^A01|M2|P|2.5|||||JPN|ISO IR87\rPID|1\r",
        )
        .unwrap();
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.set_nonblocking(true).unwrap();
        let port = l.local_addr().unwrap().port();
        assert_eq!(cmd_send(&input, "127.0.0.1", port, 2, "", true), 1);
        assert!(l.accept().is_err(), "the first message was not sent either");
        fs::remove_dir_all(&d).ok();
    }

    /// A file whose messages declare different charsets, even ones
    /// BridgeLab writes, is refused whole without --encoding when the one
    /// charset the file goes out in changes a later message's bytes. Plain
    /// ASCII reads the same in both: such a batch is sent as before.
    #[test]
    fn send_refuses_messages_that_declare_different_charsets() {
        use std::io::{Read, Write};
        let d = tmp("sendmixed");
        let input = d.join("mixed.hl7");
        // Latin-1 bytes (ü is 0xFC): the file is read, and sent, in 8859/1.
        let file = |second: &[u8]| {
            let mut bytes = b"MSH|^~\\&|A|B|C|D|20260101||ADT^A01|M1|P|2.5|||||ITA|8859/1\rPID|1||1||M\xfcller\r".to_vec();
            bytes.extend_from_slice(b"MSH|^~\\&|A|B|C|D|20260101||ADT^A01|M2|P|2.5|||||ITA|UNICODE UTF-8\rPID|1||1||");
            bytes.extend_from_slice(second);
            bytes.extend_from_slice(b"\r");
            bytes
        };
        fs::write(&input, file(b"M\xfcller")).unwrap();
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.set_nonblocking(true).unwrap();
        let port = l.local_addr().unwrap().port();
        assert_eq!(cmd_send(&input, "127.0.0.1", port, 2, "", true), 1);
        assert!(l.accept().is_err(), "the first message was not sent either");
        drop(l);

        // The second message is plain ASCII: both go out and are accepted.
        fs::write(&input, file(b"Rossi")).unwrap();
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let h = std::thread::spawn(move || {
            for id in ["M1", "M2"] {
                let (mut s, _) = l.accept().unwrap();
                let (mut got, mut buf) = (Vec::new(), [0u8; 4096]);
                while !got.ends_with(b"\x1c\r") {
                    let n = s.read(&mut buf).unwrap();
                    if n == 0 {
                        break;
                    }
                    got.extend_from_slice(&buf[..n]);
                }
                s.write_all(format!("\x0bMSH|^~\\&|R||S||20260101||ACK|A|P|2.5\rMSA|AA|{id}\r\x1c\r").as_bytes()).unwrap();
            }
        });
        assert_eq!(cmd_send(&input, "127.0.0.1", port, 5, "", true), 0);
        h.join().unwrap();
        fs::remove_dir_all(&d).ok();
    }

    /// A UTF-16 file whose MSH-18 names UTF-16 is not sent as UTF-8 under
    /// that header: BridgeLab does not send UTF-16 frames.
    #[test]
    fn send_refuses_a_declared_charset_it_cannot_send_in() {
        let d = tmp("send16decl");
        let input = d.join("u16.hl7");
        let text = "MSH|^~\\&|A|B|C|D|20260101||ADT^A01|U1|P|2.5|||||ENG|UNICODE UTF-16\rPID|1||1||Müller||19800101|M\r";
        fs::write(&input, charset::encode_output(text, "UTF-16LE")).unwrap();
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.set_nonblocking(true).unwrap();
        let port = l.local_addr().unwrap().port();
        assert_eq!(cmd_send(&input, "127.0.0.1", port, 2, "", true), 1);
        assert!(l.accept().is_err(), "nothing was sent");
        fs::remove_dir_all(&d).ok();
    }

    /// An ACK for another message, or a reply that is not HL7, is not an
    /// acceptance: the same check as the app's.
    #[test]
    fn a_reply_that_does_not_acknowledge_the_message_fails_the_send() {
        let d = tmp("sendack");
        let input = d.join("m.hl7");
        fs::write(&input, "MSH|^~\\&|A|B|C|D|20260101||ADT^A01|U1|P|2.5\rPID|1\r").unwrap();
        for reply in [
            &b"\x0bMSH|^~\\&|R||S||20260101||ACK|A1|P|2.5\rMSA|AA|OTHER\r\x1c\r"[..],
            b"\x0bHTTP/1.1 400 Bad Request\x1c\r",
            // An AA naming no message (MSA-2 empty) cannot be matched.
            b"\x0bMSH|^~\\&|R||S||20260101||ACK|A1|P|2.5\rMSA|AA\r\x1c\r",
        ] {
            let (port, h) = peer(reply);
            assert_eq!(cmd_send(&input, "127.0.0.1", port, 5, "", true), 1);
            h.join().unwrap();
        }
        let (port, h) = peer(b"\x0bMSH|^~\\&|R||S||20260101||ACK|A1|P|2.5\rMSA|AA|U1\r\x1c\r");
        assert_eq!(cmd_send(&input, "127.0.0.1", port, 5, "", true), 0);
        h.join().unwrap();
        // Sent without MSH-10, there is nothing to match: an AA naming no
        // message is taken at its word.
        fs::write(&input, "MSH|^~\\&|A|B|C|D|20260101||ADT^A01||P|2.5\rPID|1\r").unwrap();
        let (port, h) = peer(b"\x0bMSH|^~\\&|R||S||20260101||ACK|A1|P|2.5\rMSA|AA\r\x1c\r");
        assert_eq!(cmd_send(&input, "127.0.0.1", port, 5, "", true), 0);
        h.join().unwrap();
        fs::remove_dir_all(&d).ok();
    }
}

