//! HL7 value tables ("HL7 tables") for coded fields.
//!
//! The tables ship in `resources/hl7/tables.json`, produced by
//! `tools/hl7-schema-importer` from hl7-dictionary (MIT) — 394 tables,
//! about 5,000 codes. Which table a field or component draws from is part
//! of the per-version schema catalogue (`FieldSpec::table`,
//! `ComponentSpec::table`); the table *contents* are one set for every
//! version, because that is how the upstream source ships them. A code
//! added to a table in a later release is therefore accepted for an
//! earlier one — a tolerance, not a false positive.
//!
//! Whether a table is closed or open follows the HL7 data type of the
//! element using it: `ID` fields draw from HL7-defined tables (the standard
//! lists every legal value), `IS` fields from user-defined tables (the
//! standard suggests values, sites add their own). Only the former justify
//! flagging a value that is not listed.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableValue {
    pub code: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueTable {
    pub id: String,
    pub name: String,
    /// True when the standard defines the complete set of legal values
    /// (the element is an `ID`): a value not listed is non-standard. False
    /// for user-defined tables (`IS` and the like), whose listed values are
    /// suggestions — absence is not evidence of anything.
    #[serde(default)]
    pub exhaustive: bool,
    pub values: Vec<TableValue>,
}

#[derive(Deserialize)]
struct TablesFile {
    tables: Vec<ValueTable>,
}

const TABLES_JSON: &str = include_str!("../../../resources/hl7/tables.json");

fn tables() -> &'static HashMap<String, ValueTable> {
    static TABLES: OnceLock<HashMap<String, ValueTable>> = OnceLock::new();
    TABLES.get_or_init(|| {
        let file: TablesFile = serde_json::from_str(TABLES_JSON)
            .expect("shipped HL7 tables JSON is malformed — this is a build bug");
        file.tables.into_iter().map(|t| (t.id.clone(), t)).collect()
    })
}

/// HL7 data types whose value tables are closed (HL7-defined).
pub fn is_closed_table_type(data_type: &str) -> bool {
    data_type == "ID"
}

/// The named table with the standard's own values, or None for an unknown
/// or user-defined-only id. `exhaustive` is decided by the data type of the
/// element the table is being shown for; pass None when it is not known,
/// which never produces a warning.
pub fn get_table(id: &str, data_type: Option<&str>) -> Option<ValueTable> {
    let mut table = tables().get(id)?.clone();
    table.exhaustive = data_type.is_some_and(is_closed_table_type);
    Some(table)
}

/// The description of `code` in table `id`, for showing "M — Male" next to
/// a value. Codes are matched exactly: HL7 codes are case-sensitive.
pub fn describe_code(id: &str, code: &str) -> Option<&'static str> {
    let code = code.trim();
    if code.is_empty() {
        return None;
    }
    tables()
        .get(id)?
        .values
        .iter()
        .find(|v| v.code == code)
        .map(|v| v.description.as_str())
}

/// Number of shipped tables — for the About/diagnostics counters.
pub fn table_count() -> usize {
    tables().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_full_dictionary_set_ships() {
        assert!(table_count() >= 390, "only {} tables", table_count());
        let values: usize = tables().values().map(|t| t.values.len()).sum();
        assert!(values >= 5000, "only {} values", values);
        // Every table has an id of four digits and at least one value.
        for t in tables().values() {
            assert_eq!(t.id.len(), 4, "table id {:?}", t.id);
            assert!(t.id.bytes().all(|b| b.is_ascii_digit()), "table id {:?}", t.id);
            assert!(!t.values.is_empty(), "table {} is empty", t.id);
        }
    }

    #[test]
    fn the_everyday_tables_are_there() {
        for id in [
            "0001", "0002", "0003", "0004", "0007", "0008", "0038", "0052", "0076", "0085",
            "0103", "0119", "0123", "0127", "0136", "0200", "0203", "0301", "0354", "0396",
        ] {
            let t = get_table(id, None).unwrap_or_else(|| panic!("table {} missing", id));
            assert_eq!(t.id, id);
        }
        let sex = get_table("0001", Some("IS")).unwrap();
        assert_eq!(sex.name, "Administrative Sex");
        assert!(sex.values.iter().any(|v| v.code == "F" && v.description == "Female"));
        // 0076 used to be a hand-picked subset; it is now the standard's list.
        assert!(get_table("0076", None).unwrap().values.len() > 100);
    }

    #[test]
    fn exhaustiveness_follows_the_data_type() {
        assert!(get_table("0008", Some("ID")).unwrap().exhaustive, "MSA-1 is an ID");
        assert!(!get_table("0001", Some("IS")).unwrap().exhaustive, "PID-8 is an IS");
        assert!(!get_table("0001", None).unwrap().exhaustive);
        assert!(!get_table("0001", Some("CWE")).unwrap().exhaustive);
    }

    #[test]
    fn codes_describe_exactly() {
        assert_eq!(describe_code("0001", "M"), Some("Male"));
        assert_eq!(describe_code("0001", " M "), Some("Male"));
        assert_eq!(describe_code("0001", "m"), None, "HL7 codes are case-sensitive");
        assert_eq!(describe_code("0001", ""), None);
        assert_eq!(describe_code("0103", "P"), Some("Production"));
        assert_eq!(describe_code("9999", "P"), None);
    }

    #[test]
    fn user_defined_tables_without_standard_values_are_absent() {
        // IN1-2 Insurance Plan ID draws from 0072, a user-defined table the
        // standard gives no values for: the field keeps its table id in the
        // catalogue, but there is nothing to list.
        assert!(get_table("0072", Some("IS")).is_none());
        assert_eq!(describe_code("0072", "X"), None);
    }
}
