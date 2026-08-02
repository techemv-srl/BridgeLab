//! Batch validation: run parse + validation over many files at once and
//! return one summary row per file. This is the GUI counterpart of
//! `bridgelab-cli batch` — the daily regression workflow of integration
//! teams (did yesterday's interface change break any of these 500
//! messages?).

use serde::Serialize;
use tauri::State;

use crate::licensing::feature_gate;
use crate::parser::hl7::lexer::Hl7Lexer;
use crate::plugins::{self, PluginRegistry, ValidationRule};
use crate::validation::{validate_hl7_message, Severity};

/// Safety caps: a directory pick must not OOM the app.
const MAX_FILES: usize = 5000;
const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct BatchFileResult {
    pub path: String,
    pub file_name: String,
    pub message_type: String,
    pub version: String,
    pub segment_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    /// Set when the file could not be read or parsed at all.
    pub parse_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchReport {
    pub results: Vec<BatchFileResult>,
    /// Files skipped because the MAX_FILES cap was hit (0 = none).
    pub skipped: usize,
}

/// Extensions accepted when expanding a directory.
fn is_message_file(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(str::to_lowercase).as_deref(),
        Some("hl7") | Some("txt") | Some("dat")
    )
}

/// Expand the input paths: files pass through, directories contribute
/// their *.hl7/*.txt/*.dat entries (non-recursive — predictable for the
/// user, no surprise walks into node_modules-sized trees).
async fn expand_paths(paths: Vec<String>) -> (Vec<std::path::PathBuf>, usize) {
    let mut files = Vec::new();
    let mut skipped = 0usize;

    for p in paths {
        let pb = std::path::PathBuf::from(&p);
        if pb.is_dir() {
            if let Ok(mut rd) = tokio::fs::read_dir(&pb).await {
                while let Ok(Some(entry)) = rd.next_entry().await {
                    let ep = entry.path();
                    if ep.is_file() && is_message_file(&ep) {
                        if files.len() < MAX_FILES {
                            files.push(ep);
                        } else {
                            skipped += 1;
                        }
                    }
                }
            }
        } else if pb.is_file() {
            if files.len() < MAX_FILES {
                files.push(pb);
            } else {
                skipped += 1;
            }
        }
    }
    files.sort();
    (files, skipped)
}

async fn process_file(path: &std::path::Path, plugin_rules: &[ValidationRule]) -> BatchFileResult {
    let path_str = path.display().to_string();
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path_str.clone());

    let base = BatchFileResult {
        path: path_str,
        file_name,
        message_type: String::new(),
        version: String::new(),
        segment_count: 0,
        error_count: 0,
        warning_count: 0,
        parse_error: None,
    };

    let meta = match tokio::fs::metadata(path).await {
        Ok(m) => m,
        Err(e) => return BatchFileResult { parse_error: Some(format!("read failed: {}", e)), ..base },
    };
    if meta.len() > MAX_FILE_BYTES {
        return BatchFileResult {
            parse_error: Some(format!("file exceeds {} MB cap", MAX_FILE_BYTES / (1024 * 1024))),
            ..base
        };
    }

    let content = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(e) => return BatchFileResult { parse_error: Some(format!("read failed: {}", e)), ..base },
    };
    // Strip UTF-8 BOM like the interactive open path does.
    let content = content.strip_prefix('\u{FEFF}').unwrap_or(&content).to_string();

    let lexer = Hl7Lexer::new().with_truncation_threshold(100);
    let msg = match lexer.parse(content.into_bytes()) {
        Ok(m) => m,
        Err(e) => return BatchFileResult { parse_error: Some(e), ..base },
    };

    let mut report = validate_hl7_message(&msg);
    // Same pipeline as the interactive validate_message command: active
    // plugin rules count toward the totals, or a file failing an
    // organization's custom rules would be reported green here.
    if !plugin_rules.is_empty() {
        for issue in plugins::run_custom_validations(&msg, plugin_rules) {
            match issue.severity {
                Severity::Error => report.error_count += 1,
                Severity::Warning => report.warning_count += 1,
                Severity::Info => report.info_count += 1,
            }
        }
    }
    BatchFileResult {
        message_type: msg.message_type.clone(),
        version: msg.version.clone(),
        segment_count: msg.segments.len(),
        error_count: report.error_count,
        warning_count: report.warning_count,
        parse_error: None,
        ..base
    }
}

/// Validate every message file in `paths` (files and/or directories).
/// Messages are parsed and validated in memory only — nothing is added
/// to the tab store.
#[tauri::command]
pub async fn batch_validate(
    paths: Vec<String>,
    registry: State<'_, PluginRegistry>,
) -> Result<BatchReport, String> {
    feature_gate::require("batch_validate")?;

    let plugin_rules = registry.active_validation_rules(feature_gate::active_plugin_limit());
    let (files, skipped) = expand_paths(paths).await;
    let mut results = Vec::with_capacity(files.len());
    for f in &files {
        results.push(process_file(f, &plugin_rules).await);
    }
    Ok(BatchReport { results, skipped })
}

// --- Batch anonymization ----------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct BatchAnonFileResult {
    pub path: String,
    pub file_name: String,
    /// Where the anonymized copy was written (empty on error).
    pub output_path: String,
    pub phi_fields_masked: usize,
    /// Set when the file could not be read, parsed or written.
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchAnonReport {
    pub results: Vec<BatchAnonFileResult>,
    /// Files skipped because the MAX_FILES cap was hit (0 = none).
    pub skipped: usize,
    pub output_dir: String,
}

async fn anonymize_file(
    path: &std::path::Path,
    out_dir: &std::path::Path,
    out_name: &str,
    extra: &[crate::anonymization::ExtraPhiField],
) -> BatchAnonFileResult {
    let path_str = path.display().to_string();
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path_str.clone());

    let base = BatchAnonFileResult {
        path: path_str,
        file_name,
        output_path: String::new(),
        phi_fields_masked: 0,
        error: None,
    };

    let out_path = out_dir.join(out_name);
    // Never overwrite the source: writing the mask over the only copy of the
    // original defeats the point of "anonymized copies in a separate folder".
    let same_file = match (tokio::fs::canonicalize(path).await, tokio::fs::canonicalize(&out_path).await) {
        (Ok(a), Ok(b)) => a == b,
        _ => path == out_path,
    };
    if same_file {
        return BatchAnonFileResult {
            error: Some("output would overwrite the source file — pick a different output folder".into()),
            ..base
        };
    }

    let meta = match tokio::fs::metadata(path).await {
        Ok(m) => m,
        Err(e) => return BatchAnonFileResult { error: Some(format!("read failed: {}", e)), ..base },
    };
    if meta.len() > MAX_FILE_BYTES {
        return BatchAnonFileResult {
            error: Some(format!("file exceeds {} MB cap", MAX_FILE_BYTES / (1024 * 1024))),
            ..base
        };
    }

    let content = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(e) => return BatchAnonFileResult { error: Some(format!("read failed: {}", e)), ..base },
    };
    let content = content.strip_prefix('\u{FEFF}').unwrap_or(&content).to_string();

    let msg = match Hl7Lexer::new().parse(content.into_bytes()) {
        Ok(m) => m,
        Err(e) => return BatchAnonFileResult { error: Some(e), ..base },
    };

    let masked = crate::anonymization::detect_phi_with_extra(&msg, extra).len();
    let anonymized = crate::anonymization::anonymize_message_with_extra(&msg, extra);

    if let Err(e) = tokio::fs::write(&out_path, anonymized).await {
        return BatchAnonFileResult { error: Some(format!("write failed: {}", e)), ..base };
    }
    BatchAnonFileResult {
        output_path: out_path.display().to_string(),
        phi_fields_masked: masked,
        error: None,
        ..base
    }
}

/// Anonymize every message file in `paths` (files and/or directories),
/// writing the masked copies into `output_dir`. Same PHI pipeline as the
/// interactive Anonymize dialog: built-in catalogue + active plugin rules.
#[tauri::command]
pub async fn batch_anonymize(
    paths: Vec<String>,
    output_dir: String,
    registry: State<'_, PluginRegistry>,
) -> Result<BatchAnonReport, String> {
    feature_gate::require("anonymize_mask")?;

    let out_dir = std::path::PathBuf::from(&output_dir);
    tokio::fs::create_dir_all(&out_dir)
        .await
        .map_err(|e| format!("cannot create output folder: {}", e))?;

    let extra = crate::commands::anonymization::plugin_phi_rules(&registry);
    let (files, skipped) = expand_paths(paths).await;

    // Two inputs with the same file name (different folders) must not clobber
    // each other in the flat output folder — later ones get a numeric suffix.
    let mut used_names: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut results = Vec::with_capacity(files.len());
    for f in &files {
        let original = f
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "message.hl7".into());
        let n = used_names.entry(original.clone()).or_insert(0);
        *n += 1;
        let out_name = if *n == 1 {
            original
        } else {
            match original.rsplit_once('.') {
                Some((stem, ext)) => format!("{}_{}.{}", stem, n, ext),
                None => format!("{}_{}", original, n),
            }
        };
        results.push(anonymize_file(f, &out_dir, &out_name, &extra).await);
    }
    Ok(BatchAnonReport { results, skipped, output_dir })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_tmp(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, content).unwrap();
        p
    }

    #[tokio::test]
    async fn test_expand_dir_filters_extensions() {
        let dir = std::env::temp_dir().join(format!("bl_batch_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        write_tmp(&dir, "a.hl7", "MSH|x");
        write_tmp(&dir, "b.txt", "MSH|x");
        write_tmp(&dir, "c.pdf", "junk");
        let (files, skipped) = expand_paths(vec![dir.display().to_string()]).await;
        assert_eq!(files.len(), 2, "pdf must be filtered out");
        assert_eq!(skipped, 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_process_valid_and_invalid() {
        let dir = std::env::temp_dir().join(format!("bl_batch2_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let good = write_tmp(&dir, "good.hl7",
            "MSH|^~\\&|A|B|C|D|20260731120000||ADT^A01|M1|P|2.5\rPID|1||42^^^H^MR||ROSSI^MARIO||19800101|M");
        let bad = write_tmp(&dir, "bad.hl7", "not an hl7 message at all");

        let g = process_file(&good, &[]).await;
        assert!(g.parse_error.is_none(), "unexpected parse error: {:?}", g.parse_error);
        assert_eq!(g.message_type, "ADT^A01");
        assert_eq!(g.version, "2.5");
        assert!(g.segment_count >= 2);

        let b = process_file(&bad, &[]).await;
        assert!(b.parse_error.is_some(), "garbage must fail parse");
        std::fs::remove_dir_all(&dir).ok();
    }

    const SAMPLE: &str = "MSH|^~\\&|A|B|C|D|20260731120000||ADT^A01|M1|P|2.5\rPID|1||42^^^H^MR||ROSSI^MARIO||19800101|M|||VIA ROMA 1^^MILANO";

    #[tokio::test]
    async fn test_anonymize_file_masks_phi() {
        let dir = std::env::temp_dir().join(format!("bl_anon_{}", std::process::id()));
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let src = write_tmp(&dir, "pat.hl7", SAMPLE);

        let r = anonymize_file(&src, &out, "pat.hl7", &[]).await;
        assert!(r.error.is_none(), "unexpected error: {:?}", r.error);
        assert!(r.phi_fields_masked > 0, "PID demographics must be detected");
        let written = std::fs::read_to_string(&r.output_path).unwrap();
        assert!(!written.contains("ROSSI"), "patient name must be masked");
        assert!(written.starts_with("MSH|"), "output must still be an HL7 message");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_anonymize_file_refuses_overwriting_source() {
        let dir = std::env::temp_dir().join(format!("bl_anon2_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = write_tmp(&dir, "same.hl7", SAMPLE);

        // Output folder == source folder, same name → must refuse
        let r = anonymize_file(&src, &dir, "same.hl7", &[]).await;
        assert!(r.error.as_deref().unwrap_or("").contains("overwrite"));
        let untouched = std::fs::read_to_string(&src).unwrap();
        assert!(untouched.contains("ROSSI"), "source must stay untouched");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_anonymize_file_reports_parse_errors() {
        let dir = std::env::temp_dir().join(format!("bl_anon3_{}", std::process::id()));
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let bad = write_tmp(&dir, "junk.hl7", "definitely not hl7");

        let r = anonymize_file(&bad, &out, "junk.hl7", &[]).await;
        assert!(r.error.is_some(), "junk must fail");
        assert!(!out.join("junk.hl7").exists(), "no output for failed files");
        std::fs::remove_dir_all(&dir).ok();
    }
}
