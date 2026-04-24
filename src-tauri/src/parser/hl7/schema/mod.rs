//! HL7 v2.x message schema model (F1 MVP).
//!
//! This layer captures *message structure* — i.e. which segments (and segment
//! groups) compose a given message type for a given HL7 version — and the
//! field layout of each segment. It is a superset of the information in
//! `tables.rs`, which only carries segment/field metadata.
//!
//! F1 scope: four common messages (ADT^A01, ADT^A40, ORM^O01, ORU^R01) in
//! HL7 v2.5, hand-coded. F2 will swap the hand-coded data for auto-imported
//! data across all v2.5 messages and add composite data types + value tables.

pub mod v2_5;
pub mod xsd;

use serde::{Deserialize, Serialize};

/// Supported HL7 versions. Kept as an explicit enum so the UI can offer a
/// fixed, validated list instead of free-form strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hl7Version {
    V2_5,
}

impl Hl7Version {
    pub fn as_str(&self) -> &'static str {
        match self {
            Hl7Version::V2_5 => "2.5",
        }
    }
}

/// Structural element inside a message definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageElement {
    /// A single segment occurrence.
    Segment {
        code: String,
        required: bool,
        repeats: bool,
    },
    /// A named group of elements (used for nested structures like
    /// ORU^R01 PATIENT_RESULT → ORDER_OBSERVATION → OBSERVATION).
    Group {
        name: String,
        required: bool,
        repeats: bool,
        elements: Vec<MessageElement>,
    },
}

/// A message type definition (the "shape" of ADT^A01, ORM^O01, ...).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStructure {
    /// Structure code with the underscore separator used in XSDs, e.g.
    /// `ADT_A01`. Not `ADT^A01` because `^` isn't valid in XML element
    /// names.
    pub code: String,
    /// Message type + trigger event, e.g. `ADT^A01`.
    pub event: String,
    /// Human-readable description.
    pub description: String,
    /// Top-level elements (segments and groups) in order.
    pub elements: Vec<MessageElement>,
}

/// Field inside a segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSpec {
    /// 1-based position per HL7 convention.
    pub position: usize,
    /// Human-readable name.
    pub name: String,
    /// Data type code (e.g. `XPN`, `ST`, `CX`). F1 treats every field as
    /// `xs:string` with `maxLength` — composite expansion lands in F2.
    pub data_type: String,
    /// Max length in bytes, if defined by the standard.
    pub max_length: Option<usize>,
    /// Whether the field is required by the standard.
    pub required: bool,
    /// Whether the field can repeat.
    pub repeats: bool,
}

/// Segment definition (fields ordered by position).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentSpec {
    /// Three-letter segment code, e.g. `PID`.
    pub code: String,
    /// Human-readable name, e.g. `Patient Identification`.
    pub name: String,
    /// Field list, in order.
    pub fields: Vec<FieldSpec>,
}

/// Top-level schema bundle for one HL7 version. Looked up by the generator
/// and the IPC layer.
#[derive(Debug, Clone)]
pub struct Hl7Schema {
    pub version: Hl7Version,
    /// Structure code → message definition (e.g. "ADT_A01" → ...)
    pub messages: Vec<MessageStructure>,
    /// Segment code → segment definition (shared across messages).
    pub segments: Vec<SegmentSpec>,
}

impl Hl7Schema {
    /// Look up a message structure by its XSD-safe code (`ADT_A01`).
    pub fn message(&self, code: &str) -> Option<&MessageStructure> {
        self.messages.iter().find(|m| m.code == code)
    }

    /// Look up a segment definition by its three-letter code.
    pub fn segment(&self, code: &str) -> Option<&SegmentSpec> {
        self.segments.iter().find(|s| s.code == code)
    }

    /// Return the set of segment codes actually referenced by the message
    /// — used by the XSD generator to know which segment complex types
    /// to emit.
    pub fn segments_used_by(&self, message_code: &str) -> Vec<String> {
        let mut codes: Vec<String> = Vec::new();
        if let Some(m) = self.message(message_code) {
            collect_segments(&m.elements, &mut codes);
        }
        codes.sort();
        codes.dedup();
        codes
    }
}

fn collect_segments(elements: &[MessageElement], out: &mut Vec<String>) {
    for e in elements {
        match e {
            MessageElement::Segment { code, .. } => out.push(code.clone()),
            MessageElement::Group { elements, .. } => collect_segments(elements, out),
        }
    }
}

/// Returns the schema bundle for the given version. Errors if the version
/// isn't packaged (F1 only ships v2.5).
pub fn load(version: Hl7Version) -> Hl7Schema {
    match version {
        Hl7Version::V2_5 => v2_5::schema(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v25_has_the_four_f1_messages() {
        let s = load(Hl7Version::V2_5);
        for code in ["ADT_A01", "ADT_A40", "ORM_O01", "ORU_R01"] {
            assert!(s.message(code).is_some(), "missing message {}", code);
        }
    }

    #[test]
    fn every_message_segment_is_defined() {
        let s = load(Hl7Version::V2_5);
        for m in &s.messages {
            for code in s.segments_used_by(&m.code) {
                assert!(
                    s.segment(&code).is_some(),
                    "message {} references segment {} but it is not defined",
                    m.code,
                    code
                );
            }
        }
    }

    #[test]
    fn segments_used_by_deduplicates_and_sorts() {
        let s = load(Hl7Version::V2_5);
        let adt_a01 = s.segments_used_by("ADT_A01");
        // MSH, EVN, PID appear first; full list must be sorted and unique
        assert!(adt_a01.windows(2).all(|w| w[0] < w[1]));
    }
}
