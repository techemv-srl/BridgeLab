//! BridgeLab CLI - HL7 validation and tooling for CI/CD pipelines.

mod parser;
mod validation;
mod anonymize;

use clap::{Parser, Subcommand};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Parser)]
#[command(name = "bridgelab-cli", version, about = "HL7 validation, anonymization and conversion for CI/CD")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate one or more HL7 message files
    Validate {
        /// Files or glob patterns to validate
        #[arg(required = true)]
        paths: Vec<String>,
        /// Output format: text, json, junit
        #[arg(short, long, default_value = "text")]
        format: String,
        /// Exit with non-zero code if any errors found
        #[arg(short, long, default_value_t = true)]
        strict: bool,
    },
    /// Show info about HL7 messages (type, version, segment count)
    Info {
        /// Files or glob patterns
        #[arg(required = true)]
        paths: Vec<String>,
        /// Output as JSON instead of table
        #[arg(long)]
        json: bool,
    },
    /// Anonymize HL7 messages (mask PHI fields)
    Anonymize {
        /// Input file
        input: PathBuf,
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Convert HL7 message to JSON representation
    ToJson {
        /// Input HL7 file
        input: PathBuf,
        /// Output JSON file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Batch validate all messages in a directory
    Batch {
        /// Directory path
        dir: PathBuf,
        /// File extension to scan (default: hl7)
        #[arg(short, long, default_value = "hl7")]
        extension: String,
        /// Output summary as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Serialize)]
struct InfoRecord {
    file: String,
    valid: bool,
    message_type: String,
    version: String,
    segment_count: usize,
    size_bytes: u64,
    error: Option<String>,
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

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Validate { paths, format, strict } => cmd_validate(&paths, &format, strict),
        Commands::Info { paths, json } => cmd_info(&paths, json),
        Commands::Anonymize { input, output } => cmd_anonymize(&input, output.as_deref()),
        Commands::ToJson { input, output } => cmd_to_json(&input, output.as_deref()),
        Commands::Batch { dir, extension, json } => cmd_batch(&dir, &extension, json),
    };

    process::exit(exit_code);
}

fn expand_paths(patterns: &[String]) -> Vec<PathBuf> {
    let mut all = Vec::new();
    for p in patterns {
        match glob::glob(p) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    if entry.is_file() { all.push(entry); }
                }
            }
            Err(_) => {
                let p = PathBuf::from(p);
                if p.is_file() { all.push(p); }
            }
        }
    }
    if all.is_empty() {
        // If no glob match, try literal paths
        for p in patterns {
            let pb = PathBuf::from(p);
            if pb.exists() { all.push(pb); }
        }
    }
    all
}

fn cmd_validate(patterns: &[String], format: &str, strict: bool) -> i32 {
    let files = expand_paths(patterns);
    if files.is_empty() {
        eprintln!("Error: no files matched");
        return 2;
    }

    let mut total_errors = 0;
    let mut results = Vec::new();

    for file in &files {
        let content = match fs::read(file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to read {}: {}", file.display(), e);
                total_errors += 1;
                continue;
            }
        };

        match parser::Hl7Message::parse(content) {
            Ok(msg) => {
                let report = validation::validate(&msg);
                total_errors += report.error_count;
                results.push((file.clone(), Ok(report)));
            }
            Err(e) => {
                total_errors += 1;
                results.push((file.clone(), Err(e)));
            }
        }
    }

    match format {
        "json" => print_validate_json(&results),
        "junit" => print_validate_junit(&results),
        _ => print_validate_text(&results),
    }

    if strict && total_errors > 0 { 1 } else { 0 }
}

fn print_validate_text(results: &[(PathBuf, Result<validation::ValidationReport, String>)]) {
    for (file, result) in results {
        println!("== {} ==", file.display());
        match result {
            Ok(r) => {
                if r.valid && r.warning_count == 0 {
                    println!("  OK (no issues)");
                } else {
                    println!("  Errors: {}, Warnings: {}, Info: {}", r.error_count, r.warning_count, r.info_count);
                    for issue in &r.issues {
                        let loc = match (&issue.segment_type, issue.field_position) {
                            (Some(s), Some(f)) => format!("{}-{}", s, f),
                            (Some(s), None) => s.clone(),
                            _ => String::from("-"),
                        };
                        println!("  [{:7}] {:12} {}: {}", issue.severity.to_uppercase(), loc, issue.rule_id, issue.message);
                    }
                }
            }
            Err(e) => println!("  PARSE ERROR: {}", e),
        }
    }
}

fn print_validate_json(results: &[(PathBuf, Result<validation::ValidationReport, String>)]) {
    #[derive(Serialize)]
    struct Row<'a> {
        file: String,
        report: Option<&'a validation::ValidationReport>,
        parse_error: Option<&'a String>,
    }
    let rows: Vec<Row> = results.iter().map(|(f, r)| Row {
        file: f.display().to_string(),
        report: r.as_ref().ok(),
        parse_error: r.as_ref().err(),
    }).collect();
    println!("{}", serde_json::to_string_pretty(&rows).unwrap_or_default());
}

fn print_validate_junit(results: &[(PathBuf, Result<validation::ValidationReport, String>)]) {
    let total = results.len();
    let failed = results.iter().filter(|(_, r)|
        r.as_ref().map(|x| x.error_count > 0).unwrap_or(true)
    ).count();

    println!(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    println!(r#"<testsuites name="bridgelab" tests="{}" failures="{}">"#, total, failed);
    println!(r#"  <testsuite name="hl7-validation" tests="{}" failures="{}">"#, total, failed);

    for (file, result) in results {
        let name = file.file_name().and_then(|s| s.to_str()).unwrap_or("unknown");
        match result {
            Ok(r) if r.valid => println!(r#"    <testcase name="{}" classname="HL7"/>"#, name),
            Ok(r) => {
                println!(r#"    <testcase name="{}" classname="HL7">"#, name);
                for issue in r.issues.iter().filter(|i| i.severity == "error") {
                    println!(r#"      <failure message="{}">{}</failure>"#,
                        xml_escape(&issue.message), xml_escape(&issue.rule_id));
                }
                println!(r#"    </testcase>"#);
            }
            Err(e) => {
                println!(r#"    <testcase name="{}" classname="HL7">"#, name);
                println!(r#"      <failure message="{}">parse error</failure>"#, xml_escape(e));
                println!(r#"    </testcase>"#);
            }
        }
    }
    println!(r#"  </testsuite>"#);
    println!(r#"</testsuites>"#);
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
     .replace('"', "&quot;").replace('\'', "&apos;")
}

fn cmd_info(patterns: &[String], as_json: bool) -> i32 {
    let files = expand_paths(patterns);
    if files.is_empty() {
        eprintln!("Error: no files matched");
        return 2;
    }

    let mut records = Vec::new();
    for file in &files {
        let size = fs::metadata(file).map(|m| m.len()).unwrap_or(0);
        let content = match fs::read(file) {
            Ok(c) => c,
            Err(e) => {
                records.push(InfoRecord {
                    file: file.display().to_string(), valid: false,
                    message_type: String::new(), version: String::new(),
                    segment_count: 0, size_bytes: size,
                    error: Some(e.to_string()),
                });
                continue;
            }
        };

        match parser::Hl7Message::parse(content) {
            Ok(msg) => records.push(InfoRecord {
                file: file.display().to_string(), valid: true,
                message_type: msg.message_type, version: msg.version,
                segment_count: msg.segments.len(), size_bytes: size,
                error: None,
            }),
            Err(e) => records.push(InfoRecord {
                file: file.display().to_string(), valid: false,
                message_type: String::new(), version: String::new(),
                segment_count: 0, size_bytes: size,
                error: Some(e),
            }),
        }
    }

    if as_json {
        println!("{}", serde_json::to_string_pretty(&records).unwrap_or_default());
    } else {
        println!("{:<50} {:<20} {:<8} {:<8} {:<10}", "FILE", "MESSAGE TYPE", "VERSION", "SEGMENTS", "SIZE");
        println!("{:-<96}", "");
        for r in &records {
            let size = format_size(r.size_bytes);
            if r.error.is_some() {
                println!("{:<50} {}", truncate(&r.file, 50), r.error.as_ref().unwrap());
            } else {
                println!("{:<50} {:<20} {:<8} {:<8} {:<10}",
                    truncate(&r.file, 50), truncate(&r.message_type, 20),
                    truncate(&r.version, 8), r.segment_count, size);
            }
        }
    }
    0
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max - 3]) }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 { format!("{} B", bytes) }
    else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
    else { format!("{:.2} MB", bytes as f64 / 1048576.0) }
}

fn cmd_anonymize(input: &Path, output: Option<&Path>) -> i32 {
    let content = match fs::read(input) {
        Ok(c) => c,
        Err(e) => { eprintln!("Read error: {}", e); return 1; }
    };

    let msg = match parser::Hl7Message::parse(content) {
        Ok(m) => m,
        Err(e) => { eprintln!("Parse error: {}", e); return 1; }
    };

    let anonymized = anonymize::anonymize(&msg);

    match output {
        Some(path) => {
            if let Err(e) = fs::write(path, &anonymized) {
                eprintln!("Write error: {}", e); return 1;
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
        Err(e) => { eprintln!("Read error: {}", e); return 1; }
    };

    let msg = match parser::Hl7Message::parse(content) {
        Ok(m) => m,
        Err(e) => { eprintln!("Parse error: {}", e); return 1; }
    };

    let mut segments = Vec::new();
    for seg in &msg.segments {
        let mut fields = serde_json::Map::new();
        for f in &seg.fields {
            fields.insert(
                format!("{}-{}", seg.segment_type, f.position),
                serde_json::Value::String(f.span.as_str(&msg.raw).to_string()),
            );
        }
        let mut seg_obj = serde_json::Map::new();
        seg_obj.insert("segment_type".into(), serde_json::Value::String(seg.segment_type.clone()));
        seg_obj.insert("position".into(), serde_json::json!(seg.position));
        seg_obj.insert("fields".into(), serde_json::Value::Object(fields));
        segments.push(serde_json::Value::Object(seg_obj));
    }

    let root = serde_json::json!({
        "message_type": msg.message_type,
        "version": msg.version,
        "segments": segments,
    });

    let json_str = serde_json::to_string_pretty(&root).unwrap_or_default();

    match output {
        Some(path) => {
            if let Err(e) = fs::write(path, &json_str) {
                eprintln!("Write error: {}", e); return 1;
            }
            eprintln!("JSON written to: {}", path.display());
        }
        None => println!("{}", json_str),
    }
    0
}

fn cmd_batch(dir: &Path, extension: &str, as_json: bool) -> i32 {
    if !dir.is_dir() {
        eprintln!("Error: not a directory: {}", dir.display());
        return 2;
    }

    let mut results = Vec::new();
    let mut total_errors = 0;
    let mut total_warnings = 0;
    let mut valid_files = 0;

    for entry in walkdir::WalkDir::new(dir).into_iter().flatten() {
        if !entry.file_type().is_file() { continue; }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some(extension) { continue; }

        let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        match fs::read(path).ok().and_then(|c| parser::Hl7Message::parse(c).ok()) {
            Some(msg) => {
                let report = validation::validate(&msg);
                if report.valid { valid_files += 1; }
                total_errors += report.error_count;
                total_warnings += report.warning_count;
                results.push(InfoRecord {
                    file: path.display().to_string(),
                    valid: report.valid,
                    message_type: msg.message_type,
                    version: msg.version,
                    segment_count: msg.segments.len(),
                    size_bytes: size,
                    error: if report.error_count > 0 {
                        Some(format!("{} error(s)", report.error_count))
                    } else { None },
                });
            }
            None => {
                results.push(InfoRecord {
                    file: path.display().to_string(), valid: false,
                    message_type: String::new(), version: String::new(),
                    segment_count: 0, size_bytes: size,
                    error: Some("parse failed".into()),
                });
                total_errors += 1;
            }
        }
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
        if summary.failed_files > 0 {
            println!();
            println!("Failed files:");
            for r in &summary.results {
                if !r.valid {
                    println!("  {} - {}", r.file, r.error.as_deref().unwrap_or(""));
                }
            }
        }
    }

    if total_errors > 0 { 1 } else { 0 }
}
