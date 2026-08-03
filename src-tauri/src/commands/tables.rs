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
#[tauri::command]
pub fn get_hl7_table(table_id: String) -> Option<crate::parser::hl7::value_tables::ValueTable> {
    crate::parser::hl7::value_tables::get_table(&table_id)
}

// --- Schema-catalogue-backed structure info (all shipped HL7 versions) ------

use serde::Serialize;
use crate::parser::hl7::schema::{self, Hl7Version, MessageElement};

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

fn parse_schema_version(version: &str) -> Hl7Version {
    Hl7Version::ALL
        .iter()
        .copied()
        .find(|v| v.as_str() == version)
        .unwrap_or(Hl7Version::V2_5)
}

/// "ORU^R01" / "ORU^R01^ORU_R01" → "ORU_R01".
fn message_code_from_type(message_type: &str) -> String {
    let parts: Vec<&str> = message_type.split('^').collect();
    match parts.as_slice() {
        [a, b, ..] if !b.is_empty() => format!("{}_{}", a, b),
        [a] => a.to_string(),
        _ => message_type.replace('^', "_"),
    }
}

fn flatten_elements(
    elements: &[MessageElement],
    group_path: &str,
    out: &mut Vec<ExpectedSegment>,
) {
    for e in elements {
        match e {
            MessageElement::Segment { code, required, repeats } => {
                out.push(ExpectedSegment {
                    code: code.clone(),
                    required: *required,
                    repeats: *repeats,
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
                // Every segment of an optional/repeating group inherits that
                // context for display purposes.
                let _ = (required, repeats);
                flatten_elements(inner, &path, out);
            }
            MessageElement::Choice { required, repeats, segments } => {
                for code in segments {
                    out.push(ExpectedSegment {
                        code: code.clone(),
                        required: *required,
                        repeats: *repeats,
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
#[tauri::command]
pub fn get_expected_segments(message_type: String, version: String) -> Vec<ExpectedSegment> {
    let v = parse_schema_version(&version);
    let s = schema::load(v);
    let code = message_code_from_type(&message_type);
    let Some(msg) = s.message(&code) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    flatten_elements(&msg.elements, "", &mut out);
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
    let v = parse_schema_version(&version);
    let s = schema::load(v);
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
    let v = parse_schema_version(&version);
    let s = schema::load(v);
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
        let segs = get_expected_segments("ORU^R01".into(), "2.5".into());
        assert!(!segs.is_empty(), "ORU_R01 must be in the catalogue");
        let obx = segs.iter().find(|s| s.code == "OBX").expect("OBX expected in ORU^R01");
        assert!(!obx.group.is_empty(), "OBX lives inside a group: {:?}", obx);
        assert!(segs.iter().any(|s| s.code == "MSH" && s.required));
    }

    #[test]
    fn unknown_message_type_yields_empty() {
        assert!(get_expected_segments("XXX^Z99".into(), "2.5".into()).is_empty());
    }

    #[test]
    fn unknown_version_falls_back_to_v25() {
        assert!(!get_expected_segments("ADT^A01".into(), "9.9".into()).is_empty());
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
}
