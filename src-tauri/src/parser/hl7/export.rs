//! The structured exports of an HL7 v2 message (Tools → Export JSON / CSV
//! in the app, `to-json` in the CLI): one entry per field, in message order.

use serde::ser::{Serialize, SerializeMap, Serializer};

use super::message::Hl7Message;

/// The fields of one segment as a JSON object whose keys keep the order
/// of the message (MSH-2 before MSH-10): a `serde_json::Map` sorts them as
/// text.
struct Fields<'a>(Vec<(String, &'a str)>);

impl Serialize for Fields<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

#[derive(serde::Serialize)]
struct Segment<'a> {
    segment_type: &'a str,
    position: usize,
    fields: Fields<'a>,
}

#[derive(serde::Serialize)]
struct Root<'a> {
    message_type: &'a str,
    version: &'a str,
    segments: Vec<Segment<'a>>,
}

/// The message as pretty-printed JSON: type, version, then each segment
/// with its fields keyed `SEG-n`, in the order they appear.
pub fn to_json(msg: &Hl7Message) -> Result<String, String> {
    let segments = msg
        .segments
        .iter()
        .map(|seg| Segment {
            segment_type: &seg.segment_type,
            position: seg.position,
            fields: Fields(
                seg.fields
                    .iter()
                    .map(|f| (format!("{}-{}", seg.segment_type, f.position), f.span.as_str(&msg.raw)))
                    .collect(),
            ),
        })
        .collect();
    let root = Root { message_type: &msg.message_type, version: &msg.version, segments };
    serde_json::to_string_pretty(&root).map_err(|e| format!("JSON serialization failed: {}", e))
}

/// The message as CSV, one row per field. UTF-8 with a byte order mark,
/// which Excel needs to read accented letters (`Müller`, not `MÃ¼ller`).
pub fn to_csv(msg: &Hl7Message) -> String {
    let mut csv = String::from("\u{FEFF}Segment,Position,Field,Value\n");
    for seg in &msg.segments {
        for field in &seg.fields {
            csv.push_str(&format!(
                "{},{},{},{}\n",
                csv_cell(&seg.segment_type),
                seg.position,
                csv_cell(&format!("{}-{}", seg.segment_type, field.position)),
                csv_cell(field.span.as_str(&msg.raw)),
            ));
        }
    }
    csv
}

/// One CSV cell, quoted, and safe to open in a spreadsheet: a value
/// starting with = + - @ (or a tab or CR) would be read as a formula, so it
/// gets a leading apostrophe, as the batch export does. A plain number
/// (`-2.3`, `+5`) is not a formula and stays a number.
pub fn csv_cell(value: &str) -> String {
    let safe = if value.starts_with(['=', '+', '-', '@', '\t', '\r']) && !is_plain_number(value) {
        format!("'{}", value)
    } else {
        value.to_string()
    };
    format!("\"{}\"", safe.replace('"', "\"\""))
}

/// A signed decimal number and nothing else: `-2.3`, `+5`, `-.5`.
fn is_plain_number(value: &str) -> bool {
    let d = value.strip_prefix(['+', '-']).unwrap_or(value);
    let (int, frac) = match d.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (d, None),
    };
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    digits(int) && frac.is_none_or(digits) && (!int.is_empty() || frac.is_some_and(|f| !f.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::hl7::lexer::Hl7Lexer;

    fn parse(text: &str) -> Hl7Message {
        Hl7Lexer::new().parse(text.as_bytes().to_vec()).unwrap()
    }

    #[test]
    fn csv_cells_are_quoted_and_formulas_neutralised() {
        assert_eq!(csv_cell("=HYPERLINK(\"x\")"), "\"'=HYPERLINK(\"\"x\"\")\"");
        assert_eq!(csv_cell("@SUM(1,2)"), "\"'@SUM(1,2)\"");
        assert_eq!(csv_cell("A,B"), "\"A,B\"");
        assert_eq!(csv_cell("Rossi^Mario"), "\"Rossi^Mario\"");
        assert_eq!(csv_cell("-2+3"), "\"'-2+3\"");
        assert_eq!(csv_cell("-"), "\"'-\"");
    }

    #[test]
    fn negative_numbers_stay_numbers() {
        assert_eq!(csv_cell("-2.3"), "\"-2.3\"");
        assert_eq!(csv_cell("+5"), "\"+5\"");
        assert_eq!(csv_cell("-.5"), "\"-.5\"");
        assert_eq!(csv_cell("-1e3"), "\"'-1e3\"");
    }

    #[test]
    fn csv_starts_with_a_bom_and_keeps_accents() {
        let csv = to_csv(&parse("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||1||Müller^José"));
        assert!(csv.starts_with("\u{FEFF}Segment,Position,Field,Value\n"));
        assert!(csv.contains("\"Müller^José\""));
    }

    #[test]
    fn json_fields_keep_the_message_order() {
        let json = to_json(&parse("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5")).unwrap();
        let (two, ten) = (json.find("\"MSH-2\"").unwrap(), json.find("\"MSH-10\"").unwrap());
        assert!(two < ten, "{json}");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["segments"][0]["fields"]["MSH-9"], "ADT^A01");
        assert_eq!(v["message_type"], "ADT^A01");
    }
}
