//! The structured exports of an HL7 v2 message (Tools → Export JSON / CSV
//! in the app, `to-json` in the CLI): one entry per field, in message order.

use serde::ser::{Serialize, SerializeMap, Serializer};

use super::lexer::Hl7Lexer;
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
///
/// A text that holds several messages (a later MSH line) becomes an array
/// of such documents, one per message, each with the type and version of
/// its own MSH and its segments numbered from 0: merged into one document
/// under the first header, the later messages read as segments of the
/// first. Segments before the first MSH (a batch header) stay with the
/// first message. See [`message_count`].
pub fn to_json(msg: &Hl7Message) -> Result<String, String> {
    let json = match split_messages(msg) {
        None => serde_json::to_string_pretty(&document(msg)),
        Some(messages) => serde_json::to_string_pretty(&messages.iter().map(document).collect::<Vec<_>>()),
    };
    json.map_err(|e| format!("JSON serialization failed: {}", e))
}

/// One message as a JSON document: type, version, segments.
fn document(msg: &Hl7Message) -> Root<'_> {
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
    Root { message_type: &msg.message_type, version: &msg.version, segments }
}

/// The messages of a text that holds more than one, each parsed with its
/// own delimiters; `None` for a single message.
///
/// The text is split on its raw lines, not on the segments `msg` holds:
/// `msg` was indexed with the first header's delimiters, so a later
/// message that declares another field separator (`MSH*^~\&*…` after
/// `MSH|^~\&|…`) is not even an MSH segment there, and was merged into
/// the first message. A later header that does not parse as one stays
/// with the message before it.
fn split_messages(msg: &Hl7Message) -> Option<Vec<Hl7Message>> {
    let raw = &msg.raw;
    let headers = header_lines(raw);
    if headers.len() < 2 {
        return None;
    }
    // The first message keeps whatever comes before its header; each later
    // header opens a message if what follows it parses as one.
    let mut bounds = vec![0];
    for (i, &start) in headers.iter().enumerate().skip(1) {
        let end = headers.get(i + 1).copied().unwrap_or(raw.len());
        if Hl7Lexer::new().parse(raw[start..end].to_vec()).is_ok() {
            bounds.push(start);
        }
    }
    if bounds.len() < 2 {
        return None;
    }
    bounds
        .iter()
        .enumerate()
        .map(|(i, &start)| {
            let end = bounds.get(i + 1).copied().unwrap_or(raw.len());
            Hl7Lexer::new().parse(raw[start..end].to_vec())
        })
        .collect::<Result<Vec<_>, _>>()
        .ok()
}

/// Offsets of the lines that open a message: `MSH` followed by a field
/// separator (any byte that is not a letter, a digit, a blank or a line
/// end), after any blanks or MLLP frame bytes.
fn header_lines(raw: &[u8]) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut line = 0;
    while line < raw.len() {
        let end = memchr::memchr2(b'\r', b'\n', &raw[line..]).map_or(raw.len(), |p| line + p);
        let text = &raw[line..end];
        let content = text.iter().position(|&b| !matches!(b, b' ' | b'\t' | 0x0B | 0x0C | 0x1C)).map_or(&[][..], |i| &text[i..]);
        if content.len() > 3
            && content.starts_with(b"MSH")
            && !content[3].is_ascii_alphanumeric()
            && !content[3].is_ascii_whitespace()
        {
            starts.push(line);
        }
        line = end + 1;
    }
    starts
}

/// How many messages the text holds (one per MSH line that opens a
/// message, at least one): more than one makes [`to_json`] an array.
pub fn message_count(msg: &Hl7Message) -> usize {
    split_messages(msg).map_or(1, |m| m.len())
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

    /// Two messages in one text were merged into one document under the
    /// first header: the second message's segments read as the first's.
    #[test]
    fn several_messages_become_one_document_each() {
        let msg = parse(
            "FHS|^~\\&\rMSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1\rMSH|^~\\&|A|B|C|D|20240101||ORU^R01|2|P|2.3\rOBX|1\rFTS|2",
        );
        assert_eq!(message_count(&msg), 2);
        let v: serde_json::Value = serde_json::from_str(&to_json(&msg).unwrap()).unwrap();
        let docs = v.as_array().expect("an array of documents");
        assert_eq!(docs.len(), 2);
        assert_eq!((docs[0]["message_type"].as_str(), docs[0]["version"].as_str()), (Some("ADT^A01"), Some("2.5")));
        assert_eq!((docs[1]["message_type"].as_str(), docs[1]["version"].as_str()), (Some("ORU^R01"), Some("2.3")));
        let types = |d: &serde_json::Value| -> Vec<String> {
            d["segments"].as_array().unwrap().iter().map(|s| s["segment_type"].as_str().unwrap().to_string()).collect()
        };
        assert_eq!(types(&docs[0]), ["FHS", "MSH", "PID"]);
        assert_eq!(types(&docs[1]), ["MSH", "OBX", "FTS"]);
        // A later header with a field separator of its own was not an MSH
        // segment to the first message's delimiters: the second message
        // was merged into the first. Each message is now read with its own.
        let mixed = parse(
            "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\r\nPID|1|x\r\nMSH*^~\\&*A*B*C*D*20240101**ORU^R01*2*P*2.3\r\nOBX*1*TX*GLU\r\n",
        );
        assert_eq!(message_count(&mixed), 2);
        let v: serde_json::Value = serde_json::from_str(&to_json(&mixed).unwrap()).unwrap();
        let docs = v.as_array().expect("an array of documents");
        assert_eq!((docs[0]["message_type"].as_str(), docs[0]["version"].as_str()), (Some("ADT^A01"), Some("2.5")));
        assert_eq!((docs[1]["message_type"].as_str(), docs[1]["version"].as_str()), (Some("ORU^R01"), Some("2.3")));
        assert_eq!(types(&docs[0]), ["MSH", "PID"]);
        assert_eq!(types(&docs[1]), ["MSH", "OBX"]);
        assert_eq!(docs[1]["segments"][1]["fields"]["OBX-2"], "TX");
        assert_eq!(docs[1]["segments"][1]["position"], 1, "positions count within each message");
        // One message stays one document, as before.
        let one = parse("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1");
        assert_eq!(message_count(&one), 1);
        assert!(to_json(&one).unwrap().starts_with('{'));
        // A later line too short to open a message stays with the one before.
        let stray = parse("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1\rMSH|");
        assert_eq!(message_count(&stray), 1);
        assert!(to_json(&stray).unwrap().starts_with('{'));
    }
}
