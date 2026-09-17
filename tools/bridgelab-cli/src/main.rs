//! bridgelab-cli — BridgeLab's validators from the command line.
//!
//! A thin front-end over `bridgelab_lib`: the same HL7 v2 parser and
//! validator, the same FHIR checks and profile engine (the built-in R4
//! core plus any installed package), the same plugin packs and PHI
//! anonymiser the desktop app runs. No licence is read: the CLI behaves as
//! the Community edition, by construction — it is built without the `pro`
//! feature.
//!
//! Exit codes: 0 clean, 1 errors found (or a file failed to parse),
//! 2 usage error.

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, Subcommand};
use serde::Serialize;

use bridgelab_lib::anonymization::{self, ExtraPhiField, PhiSensitivity};
use bridgelab_lib::licensing::feature_gate::COMMUNITY_MAX_ACTIVE_PLUGINS;
use bridgelab_lib::parser::fhir::{self, profile::ProfileRegistry, FhirFormat};
use bridgelab_lib::parser::hl7::lexer::Hl7Lexer;
use bridgelab_lib::parser::hl7::message::Hl7Message;
use bridgelab_lib::plugins::{self, PluginRegistry};
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
        /// Files or glob patterns to validate
        #[arg(required = true)]
        paths: Vec<String>,
        /// Output format: text, json, junit
        #[arg(short, long, default_value = "text")]
        format: String,
        /// Exit with a non-zero code if any errors are found
        #[arg(short, long, default_value_t = true)]
        strict: bool,
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
        /// Files or glob patterns
        #[arg(required = true)]
        paths: Vec<String>,
        /// Output as JSON instead of a table
        #[arg(long)]
        json: bool,
    },
    /// Anonymize an HL7 v2 message (mask PHI fields)
    Anonymize {
        /// Input file
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
        /// Input HL7 file
        input: PathBuf,
        /// Output JSON file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Validate every message in a directory
    Batch {
        /// Directory path
        dir: PathBuf,
        /// File extension to scan (default: hl7)
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
        Commands::Validate { paths, format, strict, fhir_packages, no_plugins } => {
            cmd_validate(&paths, &format, strict, Engines::new(fhir_packages.as_deref(), no_plugins))
        }
        Commands::Info { paths, json } => cmd_info(&paths, json),
        Commands::Anonymize { input, output, no_plugins } => {
            cmd_anonymize(&input, output.as_deref(), no_plugins)
        }
        Commands::ToJson { input, output } => cmd_to_json(&input, output.as_deref()),
        Commands::Batch { dir, extension, json, fhir_packages, no_plugins } => {
            cmd_batch(&dir, &extension, json, Engines::new(fhir_packages.as_deref(), no_plugins))
        }
    };
    process::exit(code);
}

// ---------------------------------------------------------------------------
// The validators, loaded once per run.

struct Engines {
    plugins: Option<PluginRegistry>,
    profiles: ProfileRegistry,
}

impl Engines {
    fn new(fhir_packages: Option<&Path>, no_plugins: bool) -> Self {
        let plugins = if no_plugins {
            None
        } else {
            let registry = PluginRegistry::new();
            let _ = registry.reload();
            Some(registry)
        };
        let profiles = ProfileRegistry::new();
        if let Err(e) = profiles.reload_from(fhir_packages) {
            eprintln!("warning: FHIR packages not loaded: {}", e);
        }
        Self { plugins, profiles }
    }

    /// Community edition: the same cap on active plugin packs the app
    /// applies without a licence.
    fn plugin_limit() -> Option<usize> {
        Some(COMMUNITY_MAX_ACTIVE_PLUGINS)
    }
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
        if let Some(found) = engines.profiles.validate(json) {
            profiles_applied = true;
            issues.extend(found);
        }
    }
    if !profiles_applied {
        issues.push(fhir::FhirValidationIssue {
            severity: "info".into(),
            message: format!(
                "Profile conformance was not checked: no installed package defines the resource type '{}'",
                resource.resource_type
            ),
            path: "resourceType".into(),
        });
    }
    let issues = issues
        .into_iter()
        .map(|i| Issue {
            severity: i.severity,
            message: i.message,
            location: i.path,
            rule_id: String::new(),
        })
        .collect();
    Report::from_issues(
        "fhir",
        resource.resource_type.clone(),
        resource.fhir_version.clone(),
        issues,
    )
}

/// Parse and validate one file, whichever family it belongs to.
fn validate_file(path: &Path, engines: &Engines) -> Result<Report, String> {
    let bytes = fs::read(path).map_err(|e| format!("read failed: {}", e))?;
    if let Ok(text) = std::str::from_utf8(&bytes) {
        let text = fhir::strip_bom(text);
        match fhir::detect_fhir(text) {
            Some(FhirFormat::Json) => return fhir::parse_fhir_json(text).map(|r| validate_fhir(&r, engines)),
            Some(FhirFormat::Xml) => return fhir::parse_fhir_xml(text).map(|r| validate_fhir(&r, engines)),
            None => {}
        }
    }
    let msg = Hl7Lexer::new().parse(bytes)?;
    Ok(validate_hl7(&msg, engines))
}

// ---------------------------------------------------------------------------
// Commands

fn expand_paths(patterns: &[String]) -> Vec<PathBuf> {
    let mut all = Vec::new();
    for p in patterns {
        match glob::glob(p) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    if entry.is_file() {
                        all.push(entry);
                    }
                }
            }
            Err(_) => {
                let p = PathBuf::from(p);
                if p.is_file() {
                    all.push(p);
                }
            }
        }
    }
    if all.is_empty() {
        for p in patterns {
            let pb = PathBuf::from(p);
            if pb.exists() {
                all.push(pb);
            }
        }
    }
    all
}

fn cmd_validate(patterns: &[String], format: &str, strict: bool, engines: Engines) -> i32 {
    let files = expand_paths(patterns);
    if files.is_empty() {
        eprintln!("Error: no files matched");
        return 2;
    }

    let mut total_errors = 0;
    let mut results: Vec<(PathBuf, Result<Report, String>)> = Vec::new();
    for file in &files {
        let result = validate_file(file, &engines);
        match &result {
            Ok(r) => total_errors += r.error_count,
            Err(_) => total_errors += 1,
        }
        results.push((file.clone(), result));
    }

    match format {
        "json" => print_validate_json(&results),
        "junit" => print_validate_junit(&results),
        _ => print_validate_text(&results),
    }

    if strict && total_errors > 0 {
        1
    } else {
        0
    }
}

fn print_validate_text(results: &[(PathBuf, Result<Report, String>)]) {
    for (file, result) in results {
        println!("== {} ==", file.display());
        match result {
            Ok(r) => {
                println!(
                    "  {} {} {}",
                    r.kind.to_uppercase(),
                    r.message_type,
                    if r.version.is_empty() { String::new() } else { format!("(v{})", r.version) }
                );
                if r.issues.is_empty() {
                    println!("  OK (no issues)");
                } else {
                    println!(
                        "  Errors: {}, Warnings: {}, Info: {}",
                        r.error_count, r.warning_count, r.info_count
                    );
                    for issue in &r.issues {
                        let rule = if issue.rule_id.is_empty() { String::new() } else { format!(" {}", issue.rule_id) };
                        println!(
                            "  [{:7}] {}{}: {}",
                            issue.severity.to_uppercase(),
                            issue.location,
                            rule,
                            issue.message
                        );
                    }
                }
            }
            Err(e) => println!("  PARSE ERROR: {}", e),
        }
    }
}

fn print_validate_json(results: &[(PathBuf, Result<Report, String>)]) {
    #[derive(Serialize)]
    struct Row<'a> {
        file: String,
        report: Option<&'a Report>,
        parse_error: Option<&'a String>,
    }
    let rows: Vec<Row> = results
        .iter()
        .map(|(f, r)| Row {
            file: f.display().to_string(),
            report: r.as_ref().ok(),
            parse_error: r.as_ref().err(),
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&rows).unwrap_or_default());
}

fn print_validate_junit(results: &[(PathBuf, Result<Report, String>)]) {
    let total = results.len();
    let failed = results
        .iter()
        .filter(|(_, r)| r.as_ref().map(|x| x.error_count > 0).unwrap_or(true))
        .count();

    println!(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    println!(r#"<testsuites name="bridgelab" tests="{}" failures="{}">"#, total, failed);
    println!(r#"  <testsuite name="validation" tests="{}" failures="{}">"#, total, failed);
    for (file, result) in results {
        let name = file.file_name().and_then(|s| s.to_str()).unwrap_or("unknown");
        match result {
            Ok(r) if r.valid => {
                println!(r#"    <testcase name="{}" classname="{}"/>"#, xml_escape(name), r.kind.to_uppercase())
            }
            Ok(r) => {
                println!(r#"    <testcase name="{}" classname="{}">"#, xml_escape(name), r.kind.to_uppercase());
                for issue in r.issues.iter().filter(|i| i.severity == "error") {
                    println!(
                        r#"      <failure message="{}">{}</failure>"#,
                        xml_escape(&issue.message),
                        xml_escape(if issue.rule_id.is_empty() { &issue.location } else { &issue.rule_id })
                    );
                }
                println!(r#"    </testcase>"#);
            }
            Err(e) => {
                println!(r#"    <testcase name="{}" classname="PARSE">"#, xml_escape(name));
                println!(r#"      <failure message="{}">parse error</failure>"#, xml_escape(e));
                println!(r#"    </testcase>"#);
            }
        }
    }
    println!(r#"  </testsuite>"#);
    println!(r#"</testsuites>"#);
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[derive(Serialize)]
struct InfoRecord {
    file: String,
    valid: bool,
    kind: String,
    message_type: String,
    version: String,
    segment_count: usize,
    size_bytes: u64,
    error: Option<String>,
}

fn info_for(path: &Path) -> InfoRecord {
    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let blank = |error: String| InfoRecord {
        file: path.display().to_string(),
        valid: false,
        kind: String::new(),
        message_type: String::new(),
        version: String::new(),
        segment_count: 0,
        size_bytes: size,
        error: Some(error),
    };
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => return blank(e.to_string()),
    };
    if let Ok(text) = std::str::from_utf8(&bytes) {
        let text = fhir::strip_bom(text);
        let parsed = match fhir::detect_fhir(text) {
            Some(FhirFormat::Json) => Some(fhir::parse_fhir_json(text)),
            Some(FhirFormat::Xml) => Some(fhir::parse_fhir_xml(text)),
            None => None,
        };
        if let Some(parsed) = parsed {
            return match parsed {
                Ok(r) => InfoRecord {
                    file: path.display().to_string(),
                    valid: true,
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
    match Hl7Lexer::new().parse(bytes) {
        Ok(msg) => InfoRecord {
            file: path.display().to_string(),
            valid: true,
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
    let files = expand_paths(patterns);
    if files.is_empty() {
        eprintln!("Error: no files matched");
        return 2;
    }
    let records: Vec<InfoRecord> = files.iter().map(|f| info_for(f)).collect();

    if as_json {
        println!("{}", serde_json::to_string_pretty(&records).unwrap_or_default());
    } else {
        println!(
            "{:<50} {:<5} {:<20} {:<8} {:<9} {:<10}",
            "FILE", "KIND", "MESSAGE TYPE", "VERSION", "SEGMENTS", "SIZE"
        );
        println!("{:-<106}", "");
        for r in &records {
            match &r.error {
                Some(e) => println!("{:<50} {}", truncate(&r.file, 50), e),
                None => println!(
                    "{:<50} {:<5} {:<20} {:<8} {:<9} {:<10}",
                    truncate(&r.file, 50),
                    r.kind,
                    truncate(&r.message_type, 20),
                    truncate(&r.version, 8),
                    r.segment_count,
                    format_size(r.size_bytes)
                ),
            }
        }
    }
    0
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

fn sensitivity(s: &str) -> PhiSensitivity {
    match s {
        "high" => PhiSensitivity::High,
        "low" => PhiSensitivity::Low,
        _ => PhiSensitivity::Medium,
    }
}

fn cmd_anonymize(input: &Path, output: Option<&Path>, no_plugins: bool) -> i32 {
    let content = match fs::read(input) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Read error: {}", e);
            return 1;
        }
    };
    let msg = match Hl7Lexer::new().parse(content) {
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
        let registry = PluginRegistry::new();
        let _ = registry.reload();
        registry
            .active_phi_rules(Engines::plugin_limit())
            .into_iter()
            .map(|r| ExtraPhiField {
                segment: r.segment,
                field: r.field,
                name: r.name,
                sensitivity: sensitivity(&r.sensitivity),
            })
            .collect()
    };
    let anonymized = anonymization::anonymize_message_with_extra(&msg, &extra);

    match output {
        Some(path) => {
            if let Err(e) = fs::write(path, &anonymized) {
                eprintln!("Write error: {}", e);
                return 1;
            }
            eprintln!("Anonymized written to: {}", path.display());
        }
        None => println!("{}", anonymized),
    }
    0
}

fn cmd_to_json(input: &Path, output: Option<&Path>) -> i32 {
    let content = match fs::read(input) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Read error: {}", e);
            return 1;
        }
    };
    let msg = match Hl7Lexer::new().parse(content) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            return 1;
        }
    };

    let segments: Vec<serde_json::Value> = msg
        .segments
        .iter()
        .map(|seg| {
            let mut fields = serde_json::Map::new();
            for f in &seg.fields {
                fields.insert(
                    format!("{}-{}", seg.segment_type, f.position),
                    serde_json::Value::String(f.span.as_str(&msg.raw).to_string()),
                );
            }
            serde_json::json!({
                "segment_type": seg.segment_type,
                "position": seg.position,
                "fields": fields,
            })
        })
        .collect();
    let root = serde_json::json!({
        "message_type": msg.message_type,
        "version": msg.version,
        "segments": segments,
    });
    let json_str = serde_json::to_string_pretty(&root).unwrap_or_default();

    match output {
        Some(path) => {
            if let Err(e) = fs::write(path, &json_str) {
                eprintln!("Write error: {}", e);
                return 1;
            }
            eprintln!("JSON written to: {}", path.display());
        }
        None => println!("{}", json_str),
    }
    0
}

#[derive(Serialize)]
struct BatchSummary {
    total_files: usize,
    valid_files: usize,
    failed_files: usize,
    total_errors: usize,
    total_warnings: usize,
    results: Vec<InfoRecord>,
}

fn cmd_batch(dir: &Path, extension: &str, as_json: bool, engines: Engines) -> i32 {
    if !dir.is_dir() {
        eprintln!("Error: not a directory: {}", dir.display());
        return 2;
    }

    let mut results = Vec::new();
    let mut total_errors = 0;
    let mut total_warnings = 0;
    let mut valid_files = 0;

    for entry in walkdir::WalkDir::new(dir).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some(extension) {
            continue;
        }
        let mut record = info_for(path);
        match validate_file(path, &engines) {
            Ok(report) => {
                if report.valid {
                    valid_files += 1;
                } else {
                    record.valid = false;
                    record.error = Some(format!("{} error(s)", report.error_count));
                }
                total_errors += report.error_count;
                total_warnings += report.warning_count;
            }
            Err(e) => {
                record.valid = false;
                record.error = Some(e);
                total_errors += 1;
            }
        }
        results.push(record);
    }

    let summary = BatchSummary {
        total_files: results.len(),
        valid_files,
        failed_files: results.len() - valid_files,
        total_errors,
        total_warnings,
        results,
    };

    if as_json {
        println!("{}", serde_json::to_string_pretty(&summary).unwrap_or_default());
    } else {
        println!("Batch validation summary:");
        println!("  Total files:    {}", summary.total_files);
        println!("  Valid:          {}", summary.valid_files);
        println!("  Failed:         {}", summary.failed_files);
        println!("  Total errors:   {}", summary.total_errors);
        println!("  Total warnings: {}", summary.total_warnings);
        for r in summary.results.iter().filter(|r| !r.valid) {
            println!("  ✗ {} — {}", r.file, r.error.as_deref().unwrap_or("invalid"));
        }
    }

    if summary.total_errors > 0 {
        1
    } else {
        0
    }
}
