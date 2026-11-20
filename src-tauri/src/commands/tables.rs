use crate::parser::hl7::tables::{self, SegmentInfo, FieldInfo};

#[tauri::command]
pub fn get_segment_info(segment_type: String, version: String) -> Result<Option<SegmentInfo>, String> {
    Ok(tables::get_segment_info(&segment_type, &version))
}

#[tauri::command]
pub fn get_field_info(
    segment_type: String,
    field_position: usize,
    version: String,
) -> Result<Option<FieldInfo>, String> {
    Ok(tables::get_field_info(&segment_type, field_position, &version))
}

/// Return the values of an HL7 value table (e.g. "0001" Administrative Sex).
/// `data_type` is that of the element the table is shown for: an `ID`
/// makes the table exhaustive (a value outside it is non-standard), any
/// other type — or none — makes it a list of suggestions.
#[tauri::command]
pub fn get_hl7_table(
    table_id: String,
    data_type: Option<String>,
) -> Option<crate::parser::hl7::value_tables::ValueTable> {
    crate::parser::hl7::value_tables::get_table(&table_id, data_type.as_deref())
}

// --- Schema-catalogue-backed structure info (all shipped HL7 versions) ------
//
// `version` is the message's MSH-12 as written ("2.3.1^ITA", "2.7.2"). It
// selects the catalogue through `schema::for_declared`, the lookup that
// validation, hover and the Field Inspector use, so the schema-aware tree
// shows the structure the message is validated against rather than v2.5's.
// The catalogue is the cached one: these commands run on every expansion.

use serde::Serialize;
use crate::parser::hl7::schema::{self, MessageElement};

/// One expected segment position in a message definition, flattened.
#[derive(Debug, Clone, Serialize)]
pub struct ExpectedSegment {
    pub code: String,
    pub required: bool,
    pub repeats: bool,
    /// Group path (" / "-joined), empty for top-level segments.
    pub group: String,
    /// True when the segment is one alternative of an HL7 choice block.
    pub choice: bool,
}

/// Catalogue key for an MSH-9 value. MSH-9.3 (message structure, e.g. the
/// "ACK" in "ACK^A01^ACK") is authoritative when present; otherwise the key
/// is derived as MSH-9.1_MSH-9.2. `component` is the message's own
/// component separator: with `MSH|@...` the value reads `ADT@A01`.
fn message_code_from_type(message_type: &str, component: char) -> String {
    let parts: Vec<&str> = message_type.split(component).collect();
    match parts.as_slice() {
        [_, _, c, ..] if !c.is_empty() => c.to_string(),
        [a, b, ..] if !b.is_empty() => format!("{}_{}", a, b),
        [a] => a.to_string(),
        _ => message_type.replace(component, "_"),
    }
}

fn flatten_elements(
    elements: &[MessageElement],
    group_path: &str,
    group_required: bool,
    group_repeats: bool,
    out: &mut Vec<ExpectedSegment>,
) {
    for e in elements {
        match e {
            MessageElement::Segment { code, required, repeats } => {
                out.push(ExpectedSegment {
                    code: code.clone(),
                    // A "required" segment inside an optional group is not
                    // required in the message (e.g. IN1 inside ADT_A01's
                    // optional INSURANCE group), and a repeating ancestor
                    // makes the segment effectively repeatable.
                    required: *required && group_required,
                    repeats: *repeats || group_repeats,
                    group: group_path.to_string(),
                    choice: false,
                });
            }
            MessageElement::Group { name, required, repeats, elements: inner } => {
                let path = if group_path.is_empty() {
                    name.clone()
                } else {
                    format!("{} / {}", group_path, name)
                };
                flatten_elements(
                    inner,
                    &path,
                    group_required && *required,
                    group_repeats || *repeats,
                    out,
                );
            }
            MessageElement::Choice { required, repeats, segments } => {
                for code in segments {
                    out.push(ExpectedSegment {
                        code: code.clone(),
                        required: *required && group_required,
                        repeats: *repeats || group_repeats,
                        group: group_path.to_string(),
                        choice: true,
                    });
                }
            }
        }
    }
}

/// Expected segment sequence for a message type, from the shipped schema
/// catalogue. Empty when the message type is unknown to the catalogue.
///
/// `message_id` is the parsed message `message_type` comes from: MSH-9 is
/// split with that message's component separator, which is `^` only when
/// MSH-2 says so.
#[tauri::command]
pub fn get_expected_segments(
    message_type: String,
    version: String,
    message_id: Option<String>,
    store: tauri::State<'_, crate::message_store::MessageStore>,
) -> Vec<ExpectedSegment> {
    let component = message_id
        .and_then(|id| store.get(&id))
        .map_or('^', |msg| msg.delimiters.component as char);
    expected_segments(&message_type, &version, component)
}

fn expected_segments(message_type: &str, version: &str, component: char) -> Vec<ExpectedSegment> {
    let s = schema::for_declared(version);
    let code = message_code_from_type(message_type, component);
    let Some(msg) = s.message(&code) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    flatten_elements(&msg.elements, "", true, false, &mut out);
    out
}

/// Field list of a segment from the schema catalogue, with a flag telling
/// whether each field's data type expands into components.
#[derive(Debug, Clone, Serialize)]
pub struct SchemaFieldInfo {
    pub position: usize,
    pub name: String,
    pub data_type: String,
    pub required: bool,
    pub repeats: bool,
    pub has_components: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SegmentSchemaInfo {
    pub code: String,
    pub name: String,
    pub fields: Vec<SchemaFieldInfo>,
}

#[tauri::command]
pub fn get_segment_schema(segment: String, version: String) -> Option<SegmentSchemaInfo> {
    let s = schema::for_declared(&version);
    let seg = s.segment(&segment)?;
    Some(SegmentSchemaInfo {
        code: seg.code.clone(),
        name: seg.name.clone(),
        fields: seg
            .fields
            .iter()
            .map(|f| SchemaFieldInfo {
                position: f.position,
                name: f.name.clone(),
                data_type: f.data_type.clone(),
                required: f.required,
                repeats: f.repeats,
                has_components: s.composite(&f.data_type).is_some(),
            })
            .collect(),
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct CompositeComponentInfo {
    pub position: usize,
    pub name: String,
    pub data_type: String,
}

/// Components of a composite data type (empty for primitives / unknown).
#[tauri::command]
pub fn get_composite_components(data_type: String, version: String) -> Vec<CompositeComponentInfo> {
    let s = schema::for_declared(&version);
    match s.composite(&data_type) {
        Some(c) => c
            .components
            .iter()
            .map(|comp| CompositeComponentInfo {
                position: comp.position,
                name: comp.name.clone(),
                data_type: comp.data_type.clone(),
            })
            .collect(),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod schema_structure_tests {
    use super::*;

    #[test]
    fn oru_r01_expects_obx_inside_groups() {
        let segs = expected_segments("ORU^R01", "2.5", '^');
        assert!(!segs.is_empty(), "ORU_R01 must be in the catalogue");
        let obx = segs.iter().find(|s| s.code == "OBX").expect("OBX expected in ORU^R01");
        assert!(!obx.group.is_empty(), "OBX lives inside a group: {:?}", obx);
        assert!(segs.iter().any(|s| s.code == "MSH" && s.required));
    }

    #[test]
    fn unknown_message_type_yields_empty() {
        assert!(expected_segments("XXX^Z99", "2.5", '^').is_empty());
    }

    /// MSH-9 is split with the message's own component separator: with
    /// `MSH#@!\$#…` the type reads `ADT@A01`, and it still has a structure.
    #[test]
    fn a_custom_component_separator_finds_the_structure() {
        let custom = expected_segments("ADT@A01", "2.5", '@');
        assert!(!custom.is_empty(), "ADT@A01 must resolve to ADT_A01");
        let codes = |s: Vec<ExpectedSegment>| s.into_iter().map(|e| e.code).collect::<Vec<_>>();
        assert_eq!(codes(custom), codes(expected_segments("ADT^A01", "2.5", '^')));
        assert!(!expected_segments("ACK@A01@ACK", "2.5", '@').is_empty());
        assert_eq!(message_code_from_type("ADT@A01@ADT_A01", '@'), "ADT_A01");
        // With '@' as the separator, a '^' is an ordinary character.
        assert!(expected_segments("ADT^A01", "2.5", '@').is_empty());
    }

    /// Codex review: MSH-9.3 (message structure) is authoritative — an
    /// ACK^A01^ACK must resolve to the "ACK" catalogue entry.
    #[test]
    fn explicit_structure_component_wins() {
        let segs = expected_segments("ACK^A01^ACK", "2.5", '^');
        assert!(!segs.is_empty(), "ACK^A01^ACK must resolve via MSH-9.3");
        assert!(segs.iter().any(|s| s.code == "MSA"));
    }

    /// Codex review: a required segment inside an optional/repeating group
    /// must inherit the group's cardinality (IN1 in ADT_A01's INSURANCE
    /// group is not message-required, and it is effectively repeatable).
    #[test]
    fn group_cardinality_propagates_to_segments() {
        let segs = expected_segments("ADT^A01", "2.5", '^');
        let in1 = segs.iter().find(|s| s.code == "IN1").expect("IN1 expected in ADT_A01");
        assert!(!in1.group.is_empty(), "IN1 lives inside the INSURANCE group");
        assert!(!in1.required, "IN1 must not be message-required: {:?}", in1);
        assert!(in1.repeats, "repeating INSURANCE group makes IN1 repeatable");
        // Top-level cardinality unaffected
        let msh = segs.iter().find(|s| s.code == "MSH").unwrap();
        assert!(msh.required && !msh.repeats);
    }

    #[test]
    fn unknown_version_falls_back_to_v25() {
        assert!(!expected_segments("ADT^A01", "9.9", '^').is_empty());
    }

    #[test]
    fn segment_schema_marks_composite_fields() {
        let obx = get_segment_schema("OBX".into(), "2.5".into()).expect("OBX schema");
        assert!(!obx.fields.is_empty());
        // OBX-16 (Responsible Observer, XCN) expands into components
        let f16 = obx.fields.iter().find(|f| f.position == 16).expect("OBX-16");
        assert!(f16.has_components, "OBX-16 ({}) should be composite", f16.data_type);
    }

    #[test]
    fn composite_components_resolve_and_primitives_are_empty() {
        let xcn = get_composite_components("XCN".into(), "2.5".into());
        assert!(!xcn.is_empty(), "XCN has components");
        assert!(get_composite_components("ST".into(), "2.5".into()).is_empty());
    }

    /// The tree took MSH-12 only when it was a bare known version,
    /// so "2.3.1^ITA" and "2.7.2" showed v2.5's structure (ghost SFT and PDA
    /// rows, 39 PID fields instead of 30) while validation used the declared
    /// release. Both now resolve as validation resolves them.
    #[test]
    fn the_tree_reads_msh12_as_validation_does() {
        let codes = |v: &str| -> Vec<String> {
            expected_segments("ADT^A01", v, '^').into_iter().map(|s| s.code).collect()
        };
        let pid = |v: &str| get_segment_schema("PID".into(), v.into()).expect("PID").fields.len();
        let xpn = |v: &str| get_composite_components("XPN".into(), v.into()).len();

        // A version ID with its components: the v2.3.1 catalogue.
        assert_eq!(codes("2.3.1^ITA"), codes("2.3.1"));
        assert_ne!(codes("2.3.1"), codes("2.5"), "ADT_A01 differs between 2.3.1 and 2.5");
        assert!(!codes("2.3.1^ITA").contains(&"SFT".to_string()), "SFT is new in v2.5");
        assert_eq!(pid("2.3.1^ITA"), 30);
        assert_eq!(pid("2.5"), 39);
        assert_eq!(xpn("2.3.1^ITA"), xpn("2.3.1"));
        assert_ne!(xpn("2.3.1"), xpn("2.5"));

        // An unknown patch release: its major.minor, v2.7.
        assert_eq!(codes("2.7.2"), codes("2.7"));
        assert_ne!(codes("2.7"), codes("2.5"), "ADT_A01 differs between 2.7 and 2.5");
        assert_eq!(pid("2.7.2"), pid("2.7"));
        assert_eq!(xpn("2.7.2"), xpn("2.7"));

        // The same catalogue validation picks.
        let declared = schema::for_declared("2.3.1^ITA");
        assert_eq!(pid("2.3.1^ITA"), declared.segment("PID").unwrap().fields.len());
    }
}
