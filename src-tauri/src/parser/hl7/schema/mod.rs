//! HL7 v2.x message schema model.
//!
//! Produces XSDs compatible with the standard HL7 v2.xml encoding format
//! (segments → fields with named data types, composite types with
//! components, primitive types as simple-type restrictions on xsd:string).
//!
//! Scope: v2.5, messages ADT^A01, ADT^A40, ORM^O01, ORU^R01, plus every
//! segment, composite and primitive data type those four transitively
//! reference.

pub mod v2_5;
pub mod xsd;

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hl7Version {
    V2_3,
    V2_3_1,
    V2_4,
    V2_5,
    V2_6,
    V2_7,
    V2_7_1,
}

impl Hl7Version {
    pub fn as_str(&self) -> &'static str {
        match self {
            Hl7Version::V2_3 => "2.3",
            Hl7Version::V2_3_1 => "2.3.1",
            Hl7Version::V2_4 => "2.4",
            Hl7Version::V2_5 => "2.5",
            Hl7Version::V2_6 => "2.6",
            Hl7Version::V2_7 => "2.7",
            Hl7Version::V2_7_1 => "2.7.1",
        }
    }

    /// All shipped versions, oldest first.
    pub const ALL: &'static [Hl7Version] = &[
        Hl7Version::V2_3,
        Hl7Version::V2_3_1,
        Hl7Version::V2_4,
        Hl7Version::V2_5,
        Hl7Version::V2_6,
        Hl7Version::V2_7,
        Hl7Version::V2_7_1,
    ];
}

/// Structural element inside a message definition. Supports both sequences
/// (the common case) and choices (e.g. ORM_O01.ORDER_DETAIL opens with a
/// choice between OBR / RQD / RQ1 / RXO / ODS / ODT).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageElement {
    Segment {
        code: String,
        required: bool,
        repeats: bool,
    },
    Group {
        name: String,
        required: bool,
        repeats: bool,
        elements: Vec<MessageElement>,
    },
    /// A choice block — one of the listed segments may appear.
    /// HL7 v2.xml encodes this as `<xsd:choice minOccurs=... maxOccurs=...>`.
    Choice {
        required: bool,
        repeats: bool,
        segments: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStructure {
    /// XSD-safe code with underscore separator, e.g. `ADT_A01`.
    pub code: String,
    /// HL7 event notation, e.g. `ADT^A01`.
    pub event: String,
    pub description: String,
    pub elements: Vec<MessageElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSpec {
    pub position: usize,
    pub name: String,
    /// Data type reference: either a composite code (`XPN`, `CX`, ...)
    /// or a primitive code (`ST`, `ID`, `NM`, ...).
    pub data_type: String,
    pub required: bool,
    pub repeats: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentSpec {
    pub code: String,
    pub name: String,
    pub fields: Vec<FieldSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSpec {
    pub position: usize,
    pub name: String,
    /// Data type reference (composite or primitive).
    pub data_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeType {
    pub code: String,
    pub components: Vec<ComponentSpec>,
}

/// A primitive type — rendered as `<xsd:simpleType><xsd:restriction base="xsd:string"/></xsd:simpleType>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveType {
    pub code: String,
}

#[derive(Debug, Clone)]
pub struct Hl7Schema {
    pub version: Hl7Version,
    pub messages: Vec<MessageStructure>,
    pub segments: Vec<SegmentSpec>,
    pub composites: Vec<CompositeType>,
    pub primitives: Vec<PrimitiveType>,
}

/// Version-agnostic on-disk payload. Decoupled from the runtime `Hl7Schema`
/// so the JSON files can live in `resources/hl7/<version>.json` without
/// duplicating the version tag in every file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydratedSchema {
    pub messages: Vec<MessageStructure>,
    pub segments: Vec<SegmentSpec>,
    pub composites: Vec<CompositeType>,
    pub primitives: Vec<PrimitiveType>,
}

impl HydratedSchema {
    pub fn into_schema(self, version: Hl7Version) -> Hl7Schema {
        Hl7Schema {
            version,
            messages: self.messages,
            segments: self.segments,
            composites: self.composites,
            primitives: self.primitives,
        }
    }
}

impl Hl7Schema {
    pub fn message(&self, code: &str) -> Option<&MessageStructure> {
        self.messages.iter().find(|m| m.code == code)
    }

    pub fn segment(&self, code: &str) -> Option<&SegmentSpec> {
        self.segments.iter().find(|s| s.code == code)
    }

    pub fn composite(&self, code: &str) -> Option<&CompositeType> {
        self.composites.iter().find(|c| c.code == code)
    }

    pub fn is_primitive(&self, code: &str) -> bool {
        self.primitives.iter().any(|p| p.code == code)
    }

    /// Flatten all segment codes referenced by `message_code` (groups,
    /// choices, nested groups). Deduplicated, sorted.
    pub fn segments_used_by(&self, message_code: &str) -> Vec<String> {
        let mut codes: BTreeSet<String> = BTreeSet::new();
        if let Some(m) = self.message(message_code) {
            collect_segments(&m.elements, &mut codes);
        }
        codes.into_iter().collect()
    }

    /// Transitively collect every composite and primitive data type needed
    /// to render the XSD for `message_code`. Returns (composites, primitives),
    /// each sorted by code.
    pub fn data_types_used_by(&self, message_code: &str) -> (Vec<&CompositeType>, Vec<&PrimitiveType>) {
        let segments = self.segments_used_by(message_code);
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut to_visit: Vec<String> = Vec::new();

        for code in &segments {
            if let Some(seg) = self.segment(code) {
                for f in &seg.fields {
                    if seen.insert(f.data_type.clone()) {
                        to_visit.push(f.data_type.clone());
                    }
                }
            }
        }
        while let Some(code) = to_visit.pop() {
            if let Some(c) = self.composite(&code) {
                for comp in &c.components {
                    if seen.insert(comp.data_type.clone()) {
                        to_visit.push(comp.data_type.clone());
                    }
                }
            }
        }

        let mut composites: Vec<&CompositeType> = self
            .composites
            .iter()
            .filter(|c| seen.contains(&c.code))
            .collect();
        composites.sort_by(|a, b| a.code.cmp(&b.code));

        let mut primitives: Vec<&PrimitiveType> = self
            .primitives
            .iter()
            .filter(|p| seen.contains(&p.code))
            .collect();
        primitives.sort_by(|a, b| a.code.cmp(&b.code));

        (composites, primitives)
    }
}

fn collect_segments(elements: &[MessageElement], out: &mut BTreeSet<String>) {
    for e in elements {
        match e {
            MessageElement::Segment { code, .. } => {
                out.insert(code.clone());
            }
            MessageElement::Group { elements, .. } => collect_segments(elements, out),
            MessageElement::Choice { segments, .. } => {
                for c in segments {
                    out.insert(c.clone());
                }
            }
        }
    }
}

/// Embedded JSON payloads, one per HL7 version.
///
/// These files are bootstrapped from `v2_5::schema()` via the `dump_v25`
/// example binary; the importer tool in `tools/hl7-schema-importer/` will
/// eventually take over production of these files (F2, F3).
const V2_3_JSON: &str = include_str!("../../../../resources/hl7/v2_3.json");
const V2_3_1_JSON: &str = include_str!("../../../../resources/hl7/v2_3_1.json");
const V2_4_JSON: &str = include_str!("../../../../resources/hl7/v2_4.json");
const V2_5_JSON: &str = include_str!("../../../../resources/hl7/v2_5.json");
const V2_6_JSON: &str = include_str!("../../../../resources/hl7/v2_6.json");
const V2_7_JSON: &str = include_str!("../../../../resources/hl7/v2_7.json");
const V2_7_1_JSON: &str = include_str!("../../../../resources/hl7/v2_7_1.json");

pub fn load(version: Hl7Version) -> Hl7Schema {
    let json = match version {
        Hl7Version::V2_3 => V2_3_JSON,
        Hl7Version::V2_3_1 => V2_3_1_JSON,
        Hl7Version::V2_4 => V2_4_JSON,
        Hl7Version::V2_5 => V2_5_JSON,
        Hl7Version::V2_6 => V2_6_JSON,
        Hl7Version::V2_7 => V2_7_JSON,
        Hl7Version::V2_7_1 => V2_7_1_JSON,
    };
    let hydrated: HydratedSchema = serde_json::from_str(json)
        .expect("shipped HL7 schema JSON is malformed — this is a build bug");
    hydrated.into_schema(version)
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

    /// The data-driven loader and the legacy hand-coded schema must produce
    /// identical Hl7Schema payloads. Serves both as a regression guard for
    /// the bootstrap JSON and as a sanity check that the importer tool
    /// (when it later replaces v2_5.rs as the data source) hasn't drifted.
    #[test]
    fn loaded_v2_5_is_superset_of_bootstrap() {
        // The shipped JSON is now the FULL v2.5 catalogue imported from
        // hl7-dictionary; the hand-coded v2_5::schema() remains only as the
        // historical bootstrap. The loaded schema must be a superset: every
        // bootstrap message code still present, and the catalogue at least
        // as large on every axis.
        let loaded = load(Hl7Version::V2_5);
        let direct = v2_5::schema();

        for m in &direct.messages {
            assert!(
                loaded.messages.iter().any(|lm| lm.code == m.code),
                "bootstrap message {} missing from loaded catalogue",
                m.code
            );
        }
        assert!(loaded.messages.len() >= direct.messages.len());
        assert!(loaded.segments.len() >= direct.segments.len());
        assert!(loaded.composites.len() >= direct.composites.len());
        assert!(loaded.primitives.len() >= direct.primitives.len());

        // Full-catalogue sanity: the dictionary import brings the complete
        // v2.5 set, not another bootstrap.
        assert!(loaded.messages.len() >= 200, "expected full catalogue, got {}", loaded.messages.len());
        assert!(loaded.segments.len() >= 100, "expected full segment set, got {}", loaded.segments.len());

        // Every segment referenced by every message must be defined.
        let seg_codes: std::collections::HashSet<_> =
            loaded.segments.iter().map(|s| s.code.clone()).collect();
        for m in &loaded.messages {
            let mut refs = std::collections::BTreeSet::new();
            collect_segments(&m.elements, &mut refs);
            for r in refs {
                assert!(seg_codes.contains(&r), "{} references undefined segment {}", m.code, r);
            }
        }
    }

    #[test]
    fn every_referenced_segment_is_defined() {
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
    fn every_referenced_data_type_is_defined() {
        let s = load(Hl7Version::V2_5);
        for m in &s.messages {
            let (composites, primitives) = s.data_types_used_by(&m.code);
            for seg_code in s.segments_used_by(&m.code) {
                let seg = s.segment(&seg_code).expect("segment missing");
                for f in &seg.fields {
                    let found_composite = composites.iter().any(|c| c.code == f.data_type);
                    let found_primitive = primitives.iter().any(|p| p.code == f.data_type);
                    assert!(
                        found_composite || found_primitive,
                        "segment {} field {} uses data type {} which is not defined",
                        seg_code, f.position, f.data_type
                    );
                }
            }
        }
    }
}
