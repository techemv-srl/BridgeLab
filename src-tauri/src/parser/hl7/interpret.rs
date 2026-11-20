//! The message read as values rather than delimiters, for the
//! interpretation panel: every segment with its populated fields, each
//! repetition split into components with HL7 escapes resolved, and the
//! meaning of every coded value the catalogue knows a table for.
//!
//! The panel turns this into sentences ("Patient DOE, JOHN, Male, born
//! 1980-01-01…") in the user's language; this module only gathers the
//! facts, the same way for every message type, so any segment can at least
//! be read field by field. Positions are kept so each part of a sentence
//! can select its field in the editor.

use serde::Serialize;

use super::delimiters::Delimiters;
use super::message::Hl7Message;
use super::schema;
use super::value_tables::describe_code;

/// Longest text a value keeps for reading; a base64 payload in OBX-5 is not
/// something to read as a sentence (the editor has it whole).
pub const MAX_VALUE_CHARS: usize = 200;

#[derive(Debug, Clone, Serialize)]
pub struct Reading {
    /// MSH-12 as written.
    pub version: String,
    /// MSH-9 as written.
    pub message_type: String,
    pub segments: Vec<ReadSegment>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadSegment {
    /// Index of the segment in the message, as the tree and the grid use it.
    pub segment_idx: usize,
    pub segment_type: String,
    /// Segment name from the catalogue, empty for a Z-segment.
    pub segment_name: String,
    /// The populated fields, in position order.
    pub fields: Vec<ReadField>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadField {
    /// HL7 field number (MSH-1 is the field separator).
    pub position: usize,
    /// Field name from the catalogue, empty when it does not know the field.
    pub name: String,
    pub data_type: String,
    pub repetitions: Vec<ReadValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadValue {
    /// The repetition with escapes resolved, cut to [`MAX_VALUE_CHARS`].
    pub text: String,
    pub truncated: bool,
    /// Its components, escapes resolved, subcomponents joined with a space,
    /// each cut to [`MAX_VALUE_CHARS`] as well.
    pub components: Vec<String>,
    /// Meaning of the value's code (its first component) in the field's
    /// table, when the field is coded and the table lists it.
    pub meaning: Option<String>,
    /// Meaning of each component's code in that component's table, aligned
    /// with `components` (PID-3.5 "MR" → "Medical record number").
    pub component_meanings: Vec<Option<String>>,
}

pub fn interpret(msg: &Hl7Message) -> Reading {
    let catalogue = schema::for_declared(&msg.version);
    let d = &msg.delimiters;
    let segments = msg
        .segments
        .iter()
        .enumerate()
        .map(|(segment_idx, seg)| {
            let st = seg.segment_type.as_str();
            let fields = seg
                .fields
                .iter()
                .filter(|f| !f.span.as_str(&msg.raw).is_empty())
                .map(|f| {
                    let spec = catalogue.field(st, f.position);
                    let field_table = catalogue.table_for_field(st, f.position);
                    let repetitions = if is_delimiter_field(st, f.position) {
                        // MSH-1/MSH-2 are the delimiters themselves: never split.
                        vec![plain(f.span.as_str(&msg.raw))]
                    } else {
                        f.repetitions
                            .iter()
                            .map(|r| {
                                let components: Vec<String> = r
                                    .components
                                    .iter()
                                    .map(|c| {
                                        let parts: Vec<String> = c
                                            .subcomponents
                                            .iter()
                                            .map(|s| unescape(s.span.as_str(&msg.raw), d))
                                            .collect();
                                        let text = if parts.is_empty() {
                                            unescape(c.span.as_str(&msg.raw), d)
                                        } else {
                                            parts.join(" ").trim().to_string()
                                        };
                                        // Same bound as the whole value: a
                                        // base64 payload is not sent whole.
                                        cut(text).0
                                    })
                                    .collect();
                                let component_meanings = components
                                    .iter()
                                    .enumerate()
                                    .map(|(i, text)| {
                                        catalogue
                                            .table_for_component(st, f.position, i + 1)
                                            .and_then(|t| describe_code(t, text))
                                            .map(str::to_string)
                                    })
                                    .collect();
                                let first = components.first().map(String::as_str).unwrap_or("");
                                let meaning = field_table
                                    .and_then(|t| describe_code(t, first))
                                    .map(str::to_string);
                                let (text, truncated) = cut(unescape(r.span.as_str(&msg.raw), d));
                                ReadValue {
                                    text,
                                    truncated,
                                    components,
                                    meaning,
                                    component_meanings,
                                }
                            })
                            .collect()
                    };
                    ReadField {
                        position: f.position,
                        name: spec.map(|s| s.name.clone()).unwrap_or_default(),
                        data_type: spec.map(|s| s.data_type.clone()).unwrap_or_default(),
                        repetitions,
                    }
                })
                .collect();
            ReadSegment {
                segment_idx,
                segment_type: seg.segment_type.clone(),
                segment_name: catalogue
                    .segment(st)
                    .map(|s| s.name.clone())
                    .unwrap_or_default(),
                fields,
            }
        })
        .collect();
    Reading {
        version: msg.version.clone(),
        message_type: msg.message_type.clone(),
        segments,
    }
}

/// MSH-1/MSH-2 (and FHS/BHS-1/2) hold the delimiters, not values to split.
fn is_delimiter_field(segment_type: &str, position: usize) -> bool {
    matches!(segment_type, "MSH" | "FHS" | "BHS") && position <= 2
}

fn plain(text: &str) -> ReadValue {
    ReadValue {
        text: text.to_string(),
        truncated: false,
        components: vec![text.to_string()],
        meaning: None,
        component_meanings: vec![None],
    }
}

fn cut(text: String) -> (String, bool) {
    if text.chars().count() > MAX_VALUE_CHARS {
        (text.chars().take(MAX_VALUE_CHARS).collect(), true)
    } else {
        (text, false)
    }
}

/// Resolve the delimiter escapes (`\F\ \S\ \T\ \R\ \E\`) for reading; other
/// escapes (highlighting, hex, `.br`) are kept as written.
pub(crate) fn unescape(text: &str, d: &Delimiters) -> String {
    let esc = d.escape as char;
    if !text.contains(esc) {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(esc) {
        out.push_str(&rest[..start]);
        let after = &rest[start + esc.len_utf8()..];
        let replacement = match after.find(esc) {
            Some(end) => match &after[..end] {
                "F" => Some(d.field as char),
                "S" => Some(d.component as char),
                "T" => Some(d.subcomponent as char),
                "R" => Some(d.repetition as char),
                "E" => Some(esc),
                _ => None,
            }
            .map(|c| (c, end)),
            None => None,
        };
        match replacement {
            Some((c, end)) => {
                out.push(c);
                rest = &after[end + esc.len_utf8()..];
            }
            None => {
                out.push(esc);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::hl7::lexer::Hl7Lexer;

    const ADT: &str = "MSH|^~\\&|LAB|HOSP|EHR|CLINIC|20260105103000||ADT^A01^ADT_A01|MSG001|P|2.5\r\
        EVN|A01|20260105103000\r\
        PID|1||123456^^^HOSP^MR~987^^^NAT^SS||DOE^JOHN^Q||19800101|M|||1 MAIN ST^^SPRINGFIELD^^12345\r\
        PV1|1|I|W3^12^1^HOSP||||1234^SMITH^ANNA\r\
        NTE|1||Smith\\T\\Jones \\F\\ room\r";

    fn read(s: &str) -> Reading {
        interpret(&Hl7Lexer::new().parse(s.as_bytes().to_vec()).unwrap())
    }

    fn field<'a>(r: &'a Reading, seg: &str, pos: usize) -> &'a ReadField {
        r.segments
            .iter()
            .find(|s| s.segment_type == seg)
            .and_then(|s| s.fields.iter().find(|f| f.position == pos))
            .unwrap_or_else(|| panic!("{seg}-{pos} not read"))
    }

    #[test]
    fn segments_keep_their_index_and_name_and_skip_empty_fields() {
        let r = read(ADT);
        assert_eq!(r.version, "2.5");
        assert_eq!(r.segments.len(), 5);
        assert_eq!(r.segments[2].segment_idx, 2);
        assert_eq!(r.segments[2].segment_type, "PID");
        assert!(r.segments[2]
            .segment_name
            .to_lowercase()
            .contains("patient"));
        // PID-2 is empty: not read.
        assert!(r.segments[2].fields.iter().all(|f| f.position != 2));
        assert_eq!(field(&r, "PID", 5).name, "Patient Name");
    }

    #[test]
    fn coded_values_carry_their_meaning() {
        let r = read(ADT);
        assert_eq!(
            field(&r, "PID", 8).repetitions[0].meaning.as_deref(),
            Some("Male")
        );
        assert_eq!(
            field(&r, "PV1", 2).repetitions[0].meaning.as_deref(),
            Some("Inpatient")
        );
        // MSH-9: the message type, and the event in its own component table.
        let msh9 = &field(&r, "MSH", 9).repetitions[0];
        assert_eq!(msh9.components, vec!["ADT", "A01", "ADT_A01"]);
        assert!(msh9.component_meanings[1]
            .as_deref()
            .is_some_and(|m| m.contains("Admit")));
    }

    /// MSH-11 is explained in every version: an ID on table 0103 in v2.1
    /// and v2.2, a PT whose first component is on 0103 from v2.3 on.
    #[test]
    fn the_processing_id_is_explained_in_every_version() {
        for v in ["2.1", "2.2", "2.3", "2.3.1", "2.5", "2.7"] {
            let r = read(&format!("MSH|^~\\&|A|B|C|D|20260101||ADT^A01|1|P^T|{v}\r"));
            let msh11 = &field(&r, "MSH", 11).repetitions[0];
            assert_eq!(msh11.meaning.as_deref(), Some("Production"), "v{v}");
            if v != "2.1" && v != "2.2" {
                assert!(msh11.component_meanings[1].is_some(), "v{v}: processing mode");
            }
        }
    }

    #[test]
    fn repetitions_and_components_are_split() {
        let r = read(ADT);
        let ids = &field(&r, "PID", 3).repetitions;
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0].components[0], "123456");
        assert_eq!(ids[0].components[4], "MR");
        assert_eq!(
            ids[0].component_meanings[4].as_deref(),
            Some("Medical record number")
        );
        assert_eq!(
            field(&r, "PID", 5).repetitions[0].components,
            vec!["DOE", "JOHN", "Q"]
        );
    }

    #[test]
    fn delimiter_fields_stay_whole_and_escapes_are_resolved() {
        let r = read(ADT);
        assert_eq!(field(&r, "MSH", 2).repetitions[0].text, "^~\\&");
        assert_eq!(
            field(&r, "NTE", 3).repetitions[0].text,
            "Smith&Jones | room"
        );
    }

    #[test]
    fn long_values_are_cut_for_reading() {
        let long = "A".repeat(MAX_VALUE_CHARS + 50);
        let msg = format!(
            "MSH|^~\\&|A|B|C|D|20260101||ORU^R01|1|P|2.5\rOBX|1|ED|X||{}\r",
            long
        );
        let r = read(&msg);
        let v = &field(&r, "OBX", 5).repetitions[0];
        assert!(v.truncated);
        assert_eq!(v.text.chars().count(), MAX_VALUE_CHARS);
    }

    #[test]
    fn long_components_are_cut_too() {
        // An ED value: the payload is the fifth component.
        let payload = "QUJD".repeat(5_000);
        let msg = format!(
            "MSH|^~\\&|A|B|C|D|20260101||ORU^R01|1|P|2.5\rOBX|1|ED|PDF||^application^pdf^Base64^{}\r",
            payload
        );
        let r = read(&msg);
        let v = &field(&r, "OBX", 5).repetitions[0];
        assert!(v.truncated);
        assert_eq!(v.components[1], "application");
        assert_eq!(v.components[4].chars().count(), MAX_VALUE_CHARS);
        assert!(v
            .components
            .iter()
            .all(|c| c.chars().count() <= MAX_VALUE_CHARS));
    }
}
