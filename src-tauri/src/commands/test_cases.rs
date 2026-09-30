use tauri::State;
use uuid::Uuid;

use crate::database::{Database, TestCase};
use crate::licensing::feature_gate;

#[tauri::command]
pub fn save_test_case(
    id: Option<String>,
    name: String,
    description: String,
    category: String,
    tags: String,
    content: String,
    expected_message_type: String,
    expected_validation_result: String,
    db: State<'_, Database>,
) -> Result<TestCase, String> {
    // Community cap: block only NEW saves beyond the limit. Existing cases
    // (even ones saved over the cap during a trial) stay editable/runnable.
    if let Some(max) = feature_gate::test_case_limit() {
        let existing = db.get_test_cases(None)?;
        let is_update = id
            .as_deref()
            .map(|i| existing.iter().any(|tc| tc.id == i))
            .unwrap_or(false);
        if !is_update && existing.len() >= max {
            feature_gate::require("test_cases_unlimited")?;
        }
    }

    let tc = TestCase {
        id: id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        name, description, category, tags, content,
        expected_message_type, expected_validation_result,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    db.save_test_case(&tc)?;
    Ok(tc)
}

#[tauri::command]
pub fn get_test_cases(
    category: Option<String>,
    db: State<'_, Database>,
) -> Result<Vec<TestCase>, String> {
    db.get_test_cases(category.as_deref())
}

#[tauri::command]
pub fn delete_test_case(id: String, db: State<'_, Database>) -> Result<(), String> {
    db.delete_test_case(&id)
}

// --- Test case packs (export / import) -----------------------------------
//
// Available in every edition. An import adds test cases, so it goes
// through the same Community cap as saving one by one; masking PHI in an
// exported pack uses the Pro anonymization feature.

use std::collections::HashMap;

use crate::commands::anonymization::plugin_phi_rules;
use crate::plugins::PluginRegistry;
use crate::test_packs::{self, ConflictChoice, ImportPlanItem, PhiScan};

fn selected(db: &Database, ids: &[String]) -> Result<Vec<TestCase>, String> {
    let all = db.get_test_cases(None)?;
    Ok(if ids.is_empty() { all } else { all.into_iter().filter(|tc| ids.contains(&tc.id)).collect() })
}

/// Which of the selected test cases carry personal data, for the export
/// warning. Detection only, so every edition gets it.
#[tauri::command]
pub fn scan_test_cases_phi(
    ids: Vec<String>,
    db: State<'_, Database>,
    registry: State<'_, PluginRegistry>,
) -> Result<Vec<PhiScan>, String> {
    let cases = selected(&db, &ids)?;
    Ok(test_packs::scan_phi(&cases, &plugin_phi_rules(&registry)))
}

#[derive(serde::Serialize)]
pub struct PackExportResult {
    pub exported: usize,
    /// HL7 v2 test cases whose PHI was masked in the file.
    pub anonymized: usize,
}

/// Write the selected test cases (all when `ids` is empty) to `path`.
/// With `anonymize`, HL7 v2 content is masked in the file only.
#[tauri::command]
pub fn export_test_cases(
    ids: Vec<String>,
    path: String,
    anonymize: bool,
    app: tauri::AppHandle,
    db: State<'_, Database>,
    registry: State<'_, PluginRegistry>,
) -> Result<PackExportResult, String> {
    if anonymize {
        feature_gate::require("anonymize_mask")?;
    }
    let cases = selected(&db, &ids)?;
    if cases.is_empty() {
        return Err("No test cases to export".into());
    }
    let mut pack = test_packs::build_pack(
        &cases,
        &app.package_info().version.to_string(),
        &chrono::Utc::now().to_rfc3339(),
    );
    let anonymized = if anonymize { test_packs::anonymize_pack(&mut pack, &plugin_phi_rules(&registry)) } else { 0 };
    let json = serde_json::to_string_pretty(&pack).map_err(|e| e.to_string())?;
    crate::utils::atomic_write::write_atomic(&path, json.as_bytes()).map_err(|e| format!("Could not write {}: {}", path, e))?;
    Ok(PackExportResult { exported: pack.test_cases.len(), anonymized })
}

/// Read and parse a pack; also returns the file's fingerprint.
fn read_pack(path: &str) -> Result<(test_packs::TestCasePack, String), String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("Could not read {}: {}", path, e))?;
    if meta.len() > test_packs::MAX_PACK_BYTES {
        return Err(format!("The file is larger than {} MB", test_packs::MAX_PACK_BYTES / (1024 * 1024)));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("Could not read {}: {}", path, e))?;
    let json = String::from_utf8(bytes.clone()).map_err(|_| "The file is not UTF-8 text".to_string())?;
    Ok((test_packs::parse_pack(&json)?, test_packs::fingerprint(&bytes)))
}

#[derive(serde::Serialize)]
pub struct ImportPreview {
    pub app_version: String,
    pub exported_at: String,
    pub items: Vec<ImportPlanItem>,
    /// How many more test cases the Community cap allows; None when unlimited.
    pub room: Option<usize>,
    /// Passed back to `import_test_cases`, which refuses a file that changed
    /// since this preview (the choices are keyed by position in the pack).
    pub fingerprint: String,
}

#[tauri::command]
pub fn preview_test_case_import(path: String, db: State<'_, Database>) -> Result<ImportPreview, String> {
    let (pack, fingerprint) = read_pack(&path)?;
    let existing = db.get_test_cases(None)?;
    let room = feature_gate::test_case_limit().map(|max| max.saturating_sub(existing.len()));
    Ok(ImportPreview {
        app_version: pack.app_version.clone(),
        exported_at: pack.exported_at.clone(),
        items: test_packs::plan_import(&pack, &existing),
        room,
        fingerprint,
    })
}

#[derive(serde::Serialize)]
pub struct PackImportResult {
    pub added: usize,
    pub updated: usize,
    pub skipped: usize,
}

/// Import a pack. `choices` maps the pack position of each conflict to
/// "skip", "overwrite" or "copy"; conflicts without a choice are skipped.
/// `fingerprint` is the one the preview returned: a file changed since
/// then is refused, since the choices refer to positions in the old one.
/// All or nothing: over the Community cap nothing is written.
#[tauri::command]
pub fn import_test_cases(
    path: String,
    fingerprint: String,
    choices: HashMap<usize, ConflictChoice>,
    db: State<'_, Database>,
) -> Result<PackImportResult, String> {
    let (pack, current) = read_pack(&path)?;
    if current != fingerprint {
        return Err("The pack changed after the preview; open it again to review the new content".into());
    }
    let existing = db.get_test_cases(None)?;
    let write = test_packs::resolve_import(
        pack,
        &existing,
        &choices,
        &chrono::Utc::now().to_rfc3339(),
        || Uuid::new_v4().to_string(),
    );
    if let Some(max) = feature_gate::test_case_limit() {
        // Only new cases count against the cap: replacing cases already in
        // the library is editing them, which stays allowed over the cap.
        if !write.added.is_empty() && existing.len() + write.added.len() > max {
            feature_gate::require("test_cases_unlimited")?;
        }
    }
    let mut all = write.added.clone();
    all.extend(write.updated.iter().cloned());
    db.save_test_cases(&all)?;
    Ok(PackImportResult { added: write.added.len(), updated: write.updated.len(), skipped: write.skipped })
}
