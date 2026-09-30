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

    let content = match tokio::fs::read(path).await {
        Ok(b) => match crate::parser::hl7::charset::decode_input(&b) {
            // Fields may be split wrongly: no result is better than a wrong one.
            crate::parser::hl7::charset::Decoded { warning: Some(w), .. } if w.is_unsupported() => {
                return BatchFileResult { parse_error: Some(w.to_string()), ..base };
            }
            d => d.text,
        },
        Err(e) => return BatchFileResult { parse_error: Some(format!("read failed: {}", e)), ..base },
    };

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
    tel: State<'_, crate::licensing::telemetry::UsageCounters>,
) -> Result<BatchReport, String> {
    tel.bump_mem("batch_runs");
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

/// Case-insensitive path key: Windows and default macOS filesystems treat
/// `REPORT.hl7` and `report.hl7` as the same file, so every collision check
/// compares lowercased canonical paths. On case-sensitive filesystems this is
/// merely more conservative (a would-be REPORT/report pair still gets a
/// suffix / a refusal instead of two entries).
fn path_key(p: &std::path::Path) -> String {
    p.to_string_lossy().to_lowercase()
}

async fn anonymize_file(
    path: &std::path::Path,
    out_dir: &std::path::Path,
    out_name: &str,
    sources: &std::collections::HashSet<String>,
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
    // Never overwrite ANY selected source (not just this one): if another
    // selected file lives inside the output folder, writing over it would
    // destroy an original and let its later row read already-masked content.
    if sources.contains(&path_key(&out_path)) {
        return BatchAnonFileResult {
            error: Some("output would overwrite a selected source file — pick a different output folder".into()),
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

    let decoded = match tokio::fs::read(path).await {
        Ok(b) => crate::parser::hl7::charset::decode_input(&b),
        Err(e) => return BatchAnonFileResult { error: Some(format!("read failed: {}", e)), ..base },
    };
    // In a charset BridgeLab cannot decode, a byte of a character can read
    // as a delimiter and shift the fields: a birth date could stay in clear.
    if let Some(w) = decoded.warning.as_ref().filter(|w| w.is_unsupported()) {
        return BatchAnonFileResult { error: Some(format!("not anonymized: {}", w)), ..base };
    }

    let msg = match Hl7Lexer::new().parse(decoded.text.into_bytes()) {
        Ok(m) => m,
        Err(e) => return BatchAnonFileResult { error: Some(e), ..base },
    };

    let masked = crate::anonymization::detect_phi_with_extra(&msg, extra).len();
    let anonymized = crate::anonymization::anonymize_message_with_extra(&msg, extra);

    // The masked copy keeps the source's character set, so MSH-18 stays true.
    let bytes = crate::parser::hl7::charset::encode_output(&anonymized, decoded.charset.as_deref().unwrap_or(""));
    if let Err(e) = write_new(&out_path, &bytes).await {
        let error = if e.kind() == std::io::ErrorKind::AlreadyExists {
            "a file with this name already exists in the output folder — nothing was overwritten; pick an empty folder".to_string()
        } else {
            format!("write failed: {}", e)
        };
        return BatchAnonFileResult { error: Some(error), ..base };
    }
    BatchAnonFileResult {
        output_path: crate::commands::fileio::without_verbatim_prefix(&out_path.display().to_string()),
        phi_fields_masked: masked,
        error: None,
        ..base
    }
}

/// Write `bytes` to a file that must not exist yet. Nothing already in the
/// output folder is ever replaced: not an unselected file of the same name,
/// and not a symlink or hard link that leads to a source (the exclusive
/// create refuses any existing name without following it). A write that
/// fails half way removes its own partial file.
pub(crate) async fn write_new(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::OpenOptions::new().write(true).create_new(true).open(path).await?;
    let written = async {
        file.write_all(bytes).await?;
        file.sync_all().await
    }
    .await;
    if written.is_err() {
        drop(file);
        let _ = tokio::fs::remove_file(path).await;
    }
    written
}

/// Anonymize every message file in `paths` (files and/or directories),
/// writing the masked copies into `output_dir`. Same PHI pipeline as the
/// interactive Anonymize dialog: built-in catalogue + active plugin rules.
#[tauri::command]
pub async fn batch_anonymize(
    paths: Vec<String>,
    output_dir: String,
    registry: State<'_, PluginRegistry>,
    tel: State<'_, crate::licensing::telemetry::UsageCounters>,
) -> Result<BatchAnonReport, String> {
    tel.bump_mem("batch_runs");
    feature_gate::require("anonymize_mask")?;

    let out_dir = std::path::PathBuf::from(&output_dir);
    tokio::fs::create_dir_all(&out_dir)
        .await
        .map_err(|e| format!("cannot create output folder: {}", e))?;

    let extra = crate::commands::anonymization::plugin_phi_rules(&registry);
    let (files, skipped) = expand_paths(paths).await;

    // Canonical paths of every selected source, compared (in canonical space)
    // against each destination before any write happens.
    let canon_out_dir = tokio::fs::canonicalize(&out_dir)
        .await
        .unwrap_or_else(|_| out_dir.clone());
    let mut sources: std::collections::HashSet<String> =
        std::collections::HashSet::with_capacity(files.len());
    for f in &files {
        let canon = tokio::fs::canonicalize(f).await.unwrap_or_else(|_| f.clone());
        sources.insert(path_key(&canon));
    }

    let out_names = assign_output_names(&files);
    let mut results = Vec::with_capacity(files.len());
    for (f, out_name) in files.iter().zip(out_names.iter()) {
        results.push(anonymize_file(f, &canon_out_dir, out_name, &sources, &extra).await);
    }
    Ok(BatchAnonReport { results, skipped, output_dir })
}

/// Assign one output file name per input. Same-named inputs (from different
/// folders) get numeric suffixes, and every generated candidate is checked
/// against ALL names already reserved — `a/msg.hl7`, `b/msg.hl7`, `c/msg_2.hl7`
/// must yield three distinct names, not two `msg_2.hl7`.
fn assign_output_names(files: &[std::path::PathBuf]) -> Vec<String> {
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
    files
        .iter()
        .map(|f| {
            let original = f
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "message.hl7".into());
            let mut candidate = original.clone();
            let mut n = 1;
            // Reservation is case-insensitive: REPORT.hl7 and report.hl7 are
            // the same file on Windows / default macOS filesystems.
            while used.contains(&candidate.to_lowercase()) {
                n += 1;
                candidate = match original.rsplit_once('.') {
                    Some((stem, ext)) => format!("{}_{}.{}", stem, n, ext),
                    None => format!("{}_{}", original, n),
                };
            }
            used.insert(candidate.to_lowercase());
            candidate
        })
        .collect()
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

        let sources = std::collections::HashSet::from([path_key(&std::fs::canonicalize(&src).unwrap())]);
        let r = anonymize_file(&src, &std::fs::canonicalize(&out).unwrap(), "pat.hl7", &sources, &[]).await;
        assert!(r.error.is_none(), "unexpected error: {:?}", r.error);
        assert!(r.phi_fields_masked > 0, "PID demographics must be detected");
        let written = std::fs::read_to_string(&r.output_path).unwrap();
        assert!(!written.contains("ROSSI"), "patient name must be masked");
        assert!(written.starts_with("MSH|"), "output must still be an HL7 message");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_anonymize_file_charsets() {
        let dir = std::env::temp_dir().join(format!("bl_anon_cs_{}", std::process::id()));
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let out = std::fs::canonicalize(&out).unwrap();
        // ISO IR87 (ISO-2022-JP) is 7-bit and cannot be decoded: refused.
        let jp = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||JPN|ISO IR87\rPID|1||1||山田^太郎||19900515\r";
        let src = dir.join("jp.hl7");
        std::fs::write(&src, encoding_rs::ISO_2022_JP.encode(jp).0).unwrap();
        let sources = std::collections::HashSet::from([path_key(&std::fs::canonicalize(&src).unwrap())]);
        let r = anonymize_file(&src, &out, "jp.hl7", &sources, &[]).await;
        assert!(r.error.as_deref().is_some_and(|e| e.contains("ISO IR87")), "{:?}", r.error);
        assert!(!out.join("jp.hl7").exists());
        let v = process_file(&src, &[]).await;
        assert!(v.parse_error.as_deref().is_some_and(|e| e.contains("ISO IR87")), "{:?}", v.parse_error);
        // MSH-18 ASCII with an 8-bit byte: the copy stays 7-bit.
        let src = dir.join("ascii.hl7");
        std::fs::write(&src, b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||USA|ASCII\rPID|1||1||M\xfcller||19900515\rNTE|1||Caf\xe9\r").unwrap();
        let sources = std::collections::HashSet::from([path_key(&std::fs::canonicalize(&src).unwrap())]);
        let r = anonymize_file(&src, &out, "ascii.hl7", &sources, &[]).await;
        assert!(r.error.is_none(), "{:?}", r.error);
        assert!(std::fs::read(&r.output_path).unwrap().is_ascii());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_anonymize_file_refuses_overwriting_source() {
        let dir = std::env::temp_dir().join(format!("bl_anon2_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = write_tmp(&dir, "same.hl7", SAMPLE);

        // Output folder == source folder, same name → must refuse
        let sources = std::collections::HashSet::from([path_key(&std::fs::canonicalize(&src).unwrap())]);
        let r = anonymize_file(&src, &std::fs::canonicalize(&dir).unwrap(), "same.hl7", &sources, &[]).await;
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

        let sources = std::collections::HashSet::from([path_key(&std::fs::canonicalize(&bad).unwrap())]);
        let r = anonymize_file(&bad, &std::fs::canonicalize(&out).unwrap(), "junk.hl7", &sources, &[]).await;
        assert!(r.error.is_some(), "junk must fail");
        assert!(!out.join("junk.hl7").exists(), "no output for failed files");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Codex P1: generated suffix names must not collide with other inputs'
    /// original names — a/msg.hl7, b/msg.hl7, c/msg_2.hl7 need 3 distinct outputs.
    #[test]
    fn test_output_names_reserved_globally() {
        let files = vec![
            std::path::PathBuf::from("/a/msg.hl7"),
            std::path::PathBuf::from("/b/msg.hl7"),
            std::path::PathBuf::from("/c/msg_2.hl7"),
        ];
        let names = assign_output_names(&files);
        let unique: std::collections::HashSet<&String> = names.iter().collect();
        assert_eq!(unique.len(), 3, "all output names must be distinct: {:?}", names);
        assert_eq!(names[0], "msg.hl7");
        assert_eq!(names[1], "msg_2.hl7");
        assert_ne!(names[2], "msg_2.hl7");
    }

    /// Codex P2 (1.0.0 review): Windows / default macOS filesystems are
    /// case-insensitive — REPORT.hl7 and report.hl7 must not share an output.
    #[test]
    fn test_output_names_reserved_case_insensitively() {
        let files = vec![
            std::path::PathBuf::from("/a/REPORT.hl7"),
            std::path::PathBuf::from("/b/report.hl7"),
        ];
        let names = assign_output_names(&files);
        assert_eq!(names[0], "REPORT.hl7");
        assert_ne!(
            names[1].to_lowercase(),
            "report.hl7",
            "case-colliding input must get a suffix: {:?}",
            names
        );
    }

    /// Same guarantee for the source-protection check: writing REPORT.hl7
    /// over an existing selected source report.hl7 must be refused.
    #[tokio::test]
    async fn test_source_protection_is_case_insensitive() {
        let dir = std::env::temp_dir().join(format!("bl_anon5_{}", std::process::id()));
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let _a = write_tmp(&dir, "REPORT.hl7", SAMPLE);
        let b = write_tmp(&out, "report.hl7", SAMPLE);

        let canon_out = std::fs::canonicalize(&out).unwrap();
        let sources = std::collections::HashSet::from([
            path_key(&std::fs::canonicalize(dir.join("REPORT.hl7")).unwrap()),
            path_key(&std::fs::canonicalize(&b).unwrap()),
        ]);
        let r = anonymize_file(&dir.join("REPORT.hl7"), &canon_out, "REPORT.hl7", &sources, &[]).await;
        assert!(r.error.as_deref().unwrap_or("").contains("overwrite"), "got: {:?}", r.error);
        let untouched = std::fs::read_to_string(&b).unwrap();
        assert!(untouched.contains("ROSSI"), "case-colliding source must stay untouched");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Codex P1: a DIFFERENT selected source living inside the output folder
    /// must not be overwritten by an earlier file's output.
    #[tokio::test]
    async fn test_never_overwrites_any_selected_source() {
        let dir = std::env::temp_dir().join(format!("bl_anon4_{}", std::process::id()));
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let _a = write_tmp(&dir, "msg.hl7", SAMPLE);
        // Second selected source lives IN the output folder with the same name
        let b = write_tmp(&out, "msg.hl7", SAMPLE);

        let canon_out = std::fs::canonicalize(&out).unwrap();
        let sources = std::collections::HashSet::from([
            path_key(&std::fs::canonicalize(dir.join("msg.hl7")).unwrap()),
            path_key(&std::fs::canonicalize(&b).unwrap()),
        ]);
        // Writing a's output as out/msg.hl7 would destroy source b → refuse
        let r = anonymize_file(&dir.join("msg.hl7"), &canon_out, "msg.hl7", &sources, &[]).await;
        assert!(r.error.as_deref().unwrap_or("").contains("overwrite"), "got: {:?}", r.error);
        let untouched = std::fs::read_to_string(&b).unwrap();
        assert!(untouched.contains("ROSSI"), "the other selected source must stay untouched");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// An unselected file of the same name in the output folder is never
    /// replaced: the row reports it and the file keeps its content.
    #[tokio::test]
    async fn an_existing_unselected_file_is_not_overwritten() {
        let dir = std::env::temp_dir().join(format!("bl_anon6_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let src = write_tmp(&dir, "patient.hl7", SAMPLE);
        let other = write_tmp(&out, "patient.hl7", "ORIGINAL^UNSELECTED");

        let sources = std::collections::HashSet::from([path_key(&std::fs::canonicalize(&src).unwrap())]);
        let r = anonymize_file(&src, &std::fs::canonicalize(&out).unwrap(), "patient.hl7", &sources, &[]).await;
        assert!(r.error.as_deref().unwrap_or("").contains("already exists"), "got: {:?}", r.error);
        assert!(r.output_path.is_empty());
        assert_eq!(std::fs::read_to_string(&other).unwrap(), "ORIGINAL^UNSELECTED");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A symlink or a hard link in the output folder that leads to the
    /// selected source must not let the masked copy land on the original.
    #[cfg(unix)]
    #[tokio::test]
    async fn links_to_a_source_in_the_output_folder_are_refused() {
        let dir = std::env::temp_dir().join(format!("bl_anon7_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let src = write_tmp(&dir, "a.hl7", SAMPLE);
        std::os::unix::fs::symlink(&src, out.join("a.hl7")).unwrap();
        let src2 = write_tmp(&dir, "b.hl7", SAMPLE);
        std::fs::hard_link(&src2, out.join("b.hl7")).unwrap();

        let canon_out = std::fs::canonicalize(&out).unwrap();
        let sources = std::collections::HashSet::from([
            path_key(&std::fs::canonicalize(&src).unwrap()),
            path_key(&std::fs::canonicalize(&src2).unwrap()),
        ]);
        for (f, name) in [(&src, "a.hl7"), (&src2, "b.hl7")] {
            let r = anonymize_file(f, &canon_out, name, &sources, &[]).await;
            assert!(r.error.is_some(), "{name}: must be refused");
            assert!(std::fs::read_to_string(f).unwrap().contains("ROSSI"), "{name}: source must stay untouched");
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
