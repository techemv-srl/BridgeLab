//! Test case packs: the file format used to share test cases between
//! machines, colleagues or a Git repository.
//!
//! A pack is one JSON document:
//!
//! ```json
//! {
//!   "format": "bridgelab-test-cases",
//!   "format_version": 1,
//!   "app_version": "1.8.1",
//!   "exported_at": "2026-09-29T10:00:00Z",
//!   "test_cases": [ { "id": "…", "name": "…", "content": "MSH|…", … } ]
//! }
//! ```
//!
//! Test cases keep their id, so importing a pack again recognises the
//! cases it already brought in: identical ones are skipped, changed ones
//! are reported as conflicts for the user to resolve. Every field but
//! `name` and `content` is optional on the way in, so a hand-written pack
//! stays short.
//!
//! This module is part of the headless core (no Tauri), so the CLI can
//! read the same packs.

use serde::{Deserialize, Serialize};

use crate::anonymization::{self, ExtraPhiField};
use crate::database::TestCase;
use crate::parser::hl7::lexer::Hl7Lexer;

pub const PACK_FORMAT: &str = "bridgelab-test-cases";
pub const PACK_FORMAT_VERSION: u32 = 1;
/// Suffix of a pack file, before `.json`, as proposed by the save dialog.
pub const PACK_FILE_SUFFIX: &str = ".bltests.json";

/// A pack larger than this is refused before parsing (a 10 MB message
/// times a few dozen cases is already far beyond a normal pack).
pub const MAX_PACK_BYTES: u64 = 200 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCasePack {
    pub format: String,
    pub format_version: u32,
    #[serde(default)]
    pub app_version: String,
    #[serde(default)]
    pub exported_at: String,
    pub test_cases: Vec<PackedTestCase>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackedTestCase {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub tags: String,
    pub content: String,
    #[serde(default)]
    pub expected_message_type: String,
    #[serde(default = "default_result")]
    pub expected_validation_result: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

fn default_category() -> String {
    "general".into()
}
fn default_result() -> String {
    "valid".into()
}

impl From<&TestCase> for PackedTestCase {
    fn from(tc: &TestCase) -> Self {
        Self {
            id: tc.id.clone(),
            name: tc.name.clone(),
            description: tc.description.clone(),
            category: tc.category.clone(),
            tags: tc.tags.clone(),
            content: tc.content.clone(),
            expected_message_type: tc.expected_message_type.clone(),
            expected_validation_result: tc.expected_validation_result.clone(),
            created_at: tc.created_at.clone(),
            updated_at: tc.updated_at.clone(),
        }
    }
}

impl PackedTestCase {
    pub fn into_test_case(self, id: String, now: &str) -> TestCase {
        TestCase {
            id,
            name: self.name,
            description: self.description,
            category: if self.category.trim().is_empty() { default_category() } else { self.category },
            tags: self.tags,
            content: self.content,
            expected_message_type: self.expected_message_type,
            expected_validation_result: if self.expected_validation_result == "invalid" {
                "invalid".into()
            } else {
                "valid".into()
            },
            created_at: if self.created_at.is_empty() { now.to_string() } else { self.created_at },
            updated_at: now.to_string(),
        }
    }

    /// Same test case as far as the user can tell: timestamps aside.
    fn same_as(&self, tc: &TestCase) -> bool {
        self.name == tc.name
            && self.description == tc.description
            && self.category == tc.category
            && self.tags == tc.tags
            && self.content == tc.content
            && self.expected_message_type == tc.expected_message_type
            && self.expected_validation_result == tc.expected_validation_result
    }
}

pub fn build_pack(cases: &[TestCase], app_version: &str, exported_at: &str) -> TestCasePack {
    TestCasePack {
        format: PACK_FORMAT.into(),
        format_version: PACK_FORMAT_VERSION,
        app_version: app_version.into(),
        exported_at: exported_at.into(),
        test_cases: cases.iter().map(PackedTestCase::from).collect(),
    }
}

/// Fingerprint of a pack file, so an import can check it is applying the
/// user's choices to the very file they previewed.
pub fn fingerprint(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).iter().map(|b| format!("{:02x}", b)).collect()
}

/// Parse and check a pack. The error strings are shown to the user.
pub fn parse_pack(json: &str) -> Result<TestCasePack, String> {
    let json = json.strip_prefix('\u{FEFF}').unwrap_or(json);
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("Not a JSON file: {}", e))?;
    match value.get("format").and_then(|f| f.as_str()) {
        Some(PACK_FORMAT) => {}
        _ => return Err("Not a BridgeLab test case pack (missing \"format\": \"bridgelab-test-cases\")".into()),
    }
    let version = value.get("format_version").and_then(|v| v.as_u64()).unwrap_or(0);
    if version == 0 {
        return Err("The pack has no valid \"format_version\"".into());
    }
    if version > PACK_FORMAT_VERSION as u64 {
        return Err(format!(
            "The pack was made by a newer BridgeLab (format version {}); update BridgeLab to import it",
            version
        ));
    }
    let pack: TestCasePack =
        serde_json::from_value(value).map_err(|e| format!("Malformed test case pack: {}", e))?;
    let mut seen_ids = std::collections::HashSet::new();
    for (i, tc) in pack.test_cases.iter().enumerate() {
        // Two cases with one id would both plan against the library and
        // the second would silently overwrite the first on write.
        if !tc.id.trim().is_empty() && !seen_ids.insert(tc.id.as_str()) {
            return Err(format!("Test case id \"{}\" appears more than once in the pack", tc.id));
        }
        if tc.name.trim().is_empty() {
            return Err(format!("Test case #{} has no name", i + 1));
        }
        if tc.content.trim().is_empty() {
            return Err(format!("Test case \"{}\" has no content", tc.name));
        }
    }
    Ok(pack)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportStatus {
    /// Not in the library: will be added.
    New,
    /// Already in the library, unchanged: skipped.
    Identical,
    /// Same id, different content: the user decides.
    Conflict,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportPlanItem {
    /// Position in the pack, used to address the item in decisions.
    pub index: usize,
    pub id: String,
    pub name: String,
    pub category: String,
    pub status: ImportStatus,
    /// Name of the existing test case a conflict would overwrite.
    pub existing_name: Option<String>,
}

pub fn plan_import(pack: &TestCasePack, existing: &[TestCase]) -> Vec<ImportPlanItem> {
    pack.test_cases
        .iter()
        .enumerate()
        .map(|(index, tc)| {
            let current = if tc.id.is_empty() { None } else { existing.iter().find(|e| e.id == tc.id) };
            let status = match current {
                None => ImportStatus::New,
                Some(e) if tc.same_as(e) => ImportStatus::Identical,
                Some(_) => ImportStatus::Conflict,
            };
            ImportPlanItem {
                index,
                id: tc.id.clone(),
                name: tc.name.clone(),
                category: tc.category.clone(),
                status,
                existing_name: current.map(|e| e.name.clone()),
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictChoice {
    Skip,
    Overwrite,
    /// Keep both: the imported one gets a new id and "(imported)" in its name.
    Copy,
}

/// What an import will write, given the user's choices for the conflicts
/// (a conflict without a choice is skipped).
pub struct ImportWrite {
    /// Test cases that will be added (count against the Community cap).
    pub added: Vec<TestCase>,
    /// Existing test cases that will be replaced.
    pub updated: Vec<TestCase>,
    pub skipped: usize,
}

pub fn resolve_import(
    pack: TestCasePack,
    existing: &[TestCase],
    choices: &std::collections::HashMap<usize, ConflictChoice>,
    now: &str,
    mut new_id: impl FnMut() -> String,
) -> ImportWrite {
    let plan = plan_import(&pack, existing);
    let mut out = ImportWrite { added: vec![], updated: vec![], skipped: 0 };
    for (item, tc) in plan.into_iter().zip(pack.test_cases) {
        match item.status {
            ImportStatus::New => {
                let id = if tc.id.trim().is_empty() { new_id() } else { tc.id.clone() };
                out.added.push(tc.into_test_case(id, now));
            }
            ImportStatus::Identical => out.skipped += 1,
            ImportStatus::Conflict => match choices.get(&item.index).copied().unwrap_or(ConflictChoice::Skip) {
                ConflictChoice::Skip => out.skipped += 1,
                ConflictChoice::Overwrite => {
                    let id = tc.id.clone();
                    let created = existing.iter().find(|e| e.id == id).map(|e| e.created_at.clone());
                    let mut t = tc.into_test_case(id, now);
                    if let Some(c) = created {
                        t.created_at = c;
                    }
                    out.updated.push(t);
                }
                ConflictChoice::Copy => {
                    let mut t = tc.into_test_case(new_id(), now);
                    t.name = format!("{} (imported)", t.name);
                    t.created_at = now.to_string();
                    out.added.push(t);
                }
            },
        }
    }
    out
}

/// True when the content looks like FHIR (JSON or XML) rather than HL7 v2.
pub fn is_fhir(content: &str) -> bool {
    let t = content.trim_start_matches('\u{FEFF}').trim_start();
    t.starts_with('{') || t.starts_with('<')
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PhiScanKind {
    /// HL7 v2 message checked field by field.
    Hl7,
    /// FHIR resource: not checked field by field; treat as possibly
    /// containing personal data.
    Fhir,
    /// Content that did not parse as HL7 v2.
    Unparsed,
}

#[derive(Debug, Clone, Serialize)]
pub struct PhiScan {
    pub id: String,
    pub name: String,
    pub kind: PhiScanKind,
    /// Names of the PHI fields that hold a value (HL7 v2 only).
    pub fields: Vec<String>,
}

/// Which test cases carry personal data, for the export warning.
/// Only cases that need the user's attention are returned: HL7 v2 with
/// populated PHI fields, and every FHIR or unparsable case.
pub fn scan_phi(cases: &[TestCase], extra: &[ExtraPhiField]) -> Vec<PhiScan> {
    let mut out = Vec::new();
    for tc in cases {
        if is_fhir(&tc.content) {
            out.push(PhiScan { id: tc.id.clone(), name: tc.name.clone(), kind: PhiScanKind::Fhir, fields: vec![] });
            continue;
        }
        let content = tc.content.trim_start_matches('\u{FEFF}');
        match Hl7Lexer::new().parse(content.as_bytes().to_vec()) {
            Ok(msg) => {
                let mut fields: Vec<String> = anonymization::detect_phi_with_extra(&msg, extra)
                    .into_iter()
                    .filter(|p| !p.current_value.trim().is_empty())
                    .map(|p| format!("{}-{} {}", p.segment_type, p.field_position, p.field_name))
                    .collect();
                fields.dedup();
                if !fields.is_empty() {
                    out.push(PhiScan { id: tc.id.clone(), name: tc.name.clone(), kind: PhiScanKind::Hl7, fields });
                }
            }
            Err(_) => out.push(PhiScan {
                id: tc.id.clone(),
                name: tc.name.clone(),
                kind: PhiScanKind::Unparsed,
                fields: vec![],
            }),
        }
    }
    out
}

/// Mask the PHI of every HL7 v2 test case in a pack (the library itself is
/// not touched). FHIR and unparsable content is left as it is; the count
/// of masked cases is returned.
pub fn anonymize_pack(pack: &mut TestCasePack, extra: &[ExtraPhiField]) -> usize {
    let mut masked = 0;
    for tc in &mut pack.test_cases {
        if is_fhir(&tc.content) {
            continue;
        }
        let content = tc.content.trim_start_matches('\u{FEFF}');
        if let Ok(msg) = Hl7Lexer::new().parse(content.as_bytes().to_vec()) {
            let anonymized = anonymization::anonymize_message_with_extra(&msg, extra);
            if anonymized != tc.content {
                tc.content = anonymized;
                masked += 1;
            }
        }
    }
    masked
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const ADT: &str = "MSH|^~\\&|APP|FAC|RCV|RFAC|20260101120000||ADT^A01|MSG1|P|2.5\rPID|1||12345^^^HOSP||SMITH^JOHN||19800101|M\r";

    fn tc(id: &str, name: &str, content: &str) -> TestCase {
        TestCase {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            category: "general".into(),
            tags: String::new(),
            content: content.into(),
            expected_message_type: "ADT^A01".into(),
            expected_validation_result: "valid".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn a_pack_round_trips() {
        let cases = vec![tc("a", "Admit", ADT), tc("b", "Patient", "{\"resourceType\":\"Patient\"}")];
        let pack = build_pack(&cases, "1.8.1", "2026-09-29T00:00:00Z");
        let json = serde_json::to_string_pretty(&pack).unwrap();
        let back = parse_pack(&json).unwrap();
        assert_eq!(back.format_version, PACK_FORMAT_VERSION);
        assert_eq!(back.test_cases.len(), 2);
        assert_eq!(back.test_cases[0], PackedTestCase::from(&cases[0]));
    }

    #[test]
    fn a_minimal_hand_written_pack_is_accepted() {
        let json = r#"{"format":"bridgelab-test-cases","format_version":1,
            "test_cases":[{"name":"x","content":"MSH|^~\\&|A"}]}"#;
        let p = parse_pack(json).unwrap();
        let t = &p.test_cases[0];
        assert_eq!((t.category.as_str(), t.expected_validation_result.as_str()), ("general", "valid"));
        assert!(t.id.is_empty());
    }

    #[test]
    fn foreign_newer_or_broken_files_are_refused_with_a_reason() {
        assert!(parse_pack("not json").unwrap_err().contains("Not a JSON"));
        assert!(parse_pack(r#"{"name":"plugin"}"#).unwrap_err().contains("Not a BridgeLab test case pack"));
        let newer = r#"{"format":"bridgelab-test-cases","format_version":2,"test_cases":[]}"#;
        assert!(parse_pack(newer).unwrap_err().contains("newer BridgeLab"));
        let dup = r#"{"format":"bridgelab-test-cases","format_version":1,"test_cases":[
            {"id":"a","name":"x","content":"y"},{"id":"a","name":"z","content":"w"},
            {"name":"no id","content":"1"},{"name":"no id either","content":"2"}]}"#;
        assert!(parse_pack(dup).unwrap_err().contains("appears more than once"));
        let no_ids = r#"{"format":"bridgelab-test-cases","format_version":1,"test_cases":[
            {"name":"no id","content":"1"},{"name":"no id either","content":"2"}]}"#;
        assert!(parse_pack(no_ids).is_ok(), "cases without an id are never duplicates");
        assert_ne!(fingerprint(b"a"), fingerprint(b"b"));
        assert_eq!(fingerprint(b"a").len(), 64);
        let empty = r#"{"format":"bridgelab-test-cases","format_version":1,"test_cases":[{"name":" ","content":"x"}]}"#;
        assert!(parse_pack(empty).unwrap_err().contains("no name"));
        let bom = "\u{FEFF}{\"format\":\"bridgelab-test-cases\",\"format_version\":1,\"test_cases\":[]}";
        assert!(parse_pack(bom).is_ok());
    }

    #[test]
    fn the_plan_tells_new_identical_and_conflicting_cases_apart() {
        let existing = vec![tc("a", "Admit", ADT), tc("b", "Other", ADT)];
        let mut changed = tc("b", "Other v2", ADT);
        changed.updated_at = "2027-01-01T00:00:00Z".into();
        let mut same_but_newer = tc("a", "Admit", ADT);
        same_but_newer.updated_at = "2027-01-01T00:00:00Z".into();
        let pack = build_pack(&[same_but_newer, changed, tc("c", "New one", ADT)], "", "");
        let plan = plan_import(&pack, &existing);
        let statuses: Vec<_> = plan.iter().map(|p| p.status).collect();
        assert_eq!(statuses, vec![ImportStatus::Identical, ImportStatus::Conflict, ImportStatus::New]);
        assert_eq!(plan[1].existing_name.as_deref(), Some("Other"));
    }

    #[test]
    fn conflicts_follow_the_users_choice_and_default_to_skip() {
        let existing = vec![tc("a", "A", ADT), tc("b", "B", ADT), tc("c", "C", ADT)];
        let pack = build_pack(&[tc("a", "A2", ADT), tc("b", "B2", ADT), tc("c", "C2", ADT), tc("", "Fresh", ADT)], "", "");
        let choices = HashMap::from([(0, ConflictChoice::Overwrite), (1, ConflictChoice::Copy)]);
        let mut n = 0;
        let w = resolve_import(pack, &existing, &choices, "NOW", || { n += 1; format!("new{}", n) });
        assert_eq!(w.updated.len(), 1);
        assert_eq!((w.updated[0].id.as_str(), w.updated[0].name.as_str()), ("a", "A2"));
        assert_eq!(w.updated[0].created_at, "2026-01-01T00:00:00Z", "an overwrite keeps the original creation date");
        let added: Vec<_> = w.added.iter().map(|t| (t.id.as_str(), t.name.as_str())).collect();
        assert_eq!(added, vec![("new1", "B2 (imported)"), ("new2", "Fresh")]);
        assert_eq!(w.skipped, 1, "the conflict without a choice (c) is skipped");
    }

    #[test]
    fn the_phi_scan_flags_hl7_fields_and_every_fhir_case() {
        let clean = "MSH|^~\\&|APP|FAC|RCV|RFAC|20260101||ADT^A01|MSG1|P|2.5\rEVN|A01|20260101\r";
        let cases = vec![tc("a", "With PHI", ADT), tc("b", "Clean", clean), tc("c", "Fhir", "{\"resourceType\":\"Patient\"}")];
        let scan = scan_phi(&cases, &[]);
        let names: Vec<_> = scan.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["With PHI", "Fhir"]);
        assert!(scan[0].fields.iter().any(|f| f.starts_with("PID-5")), "{:?}", scan[0].fields);
    }

    #[test]
    fn anonymizing_a_pack_masks_hl7_and_leaves_fhir_alone() {
        let fhir = "{\"resourceType\":\"Patient\",\"name\":[{\"family\":\"Smith\"}]}";
        let mut pack = build_pack(&[tc("a", "A", ADT), tc("b", "B", fhir)], "", "");
        let masked = anonymize_pack(&mut pack, &[]);
        assert_eq!(masked, 1);
        assert!(!pack.test_cases[0].content.contains("SMITH"));
        assert_eq!(pack.test_cases[1].content, fhir);
    }
}
