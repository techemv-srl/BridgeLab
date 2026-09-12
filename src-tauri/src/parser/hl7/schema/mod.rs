//! HL7 v2.x message schema model.
//!
//! Produces XSDs compatible with the standard HL7 v2.xml encoding format
//! (segments → fields with named data types, composite types with
//! components, primitive types as simple-type restrictions on xsd:string).
//!
//! Scope: the full message catalogue for every HL7 v2.x release covered by
//! the schema importer (v2.1 through v2.7.1), including every segment,
//! composite and primitive data type those messages reference.

pub mod v2_5;
pub mod xsd;

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hl7Version {
    V2_1,
    V2_2,
    V2_3,
    V2_3_1,
    V2_4,
    V2_5,
    V2_5_1,
    V2_6,
    V2_7,
    V2_7_1,
}

impl Hl7Version {
    pub fn as_str(&self) -> &'static str {
        match self {
            Hl7Version::V2_1 => "2.1",
            Hl7Version::V2_2 => "2.2",
            Hl7Version::V2_3 => "2.3",
            Hl7Version::V2_3_1 => "2.3.1",
            Hl7Version::V2_4 => "2.4",
            Hl7Version::V2_5 => "2.5",
            Hl7Version::V2_5_1 => "2.5.1",
            Hl7Version::V2_6 => "2.6",
            Hl7Version::V2_7 => "2.7",
            Hl7Version::V2_7_1 => "2.7.1",
        }
    }

    /// True when this version has no definitions of its own and reuses an
    /// earlier release's catalogue — see `aliases()`.
    pub fn is_alias(&self) -> bool {
        self.aliases().is_some()
    }

    /// The release whose definitions this version actually loads, when it
    /// does not carry its own.
    ///
    /// HL7 v2.7.1 is a technical-correction release of v2.7 and the upstream
    /// data source (hl7-dictionary) ships byte-identical definitions for the
    /// two. Rather than embed the same 2 MB payload twice — or silently
    /// pretend we hold real 2.7.1 tables — v2.7.1 is declared an alias of
    /// v2.7 so MSH-12 = 2.7.1 still resolves to the closest correct
    /// catalogue instead of falling back to the default version.
    pub fn aliases(&self) -> Option<Hl7Version> {
        match self {
            Hl7Version::V2_7_1 => Some(Hl7Version::V2_7),
            _ => None,
        }
    }

    /// All shipped versions, oldest first.
    pub const ALL: &'static [Hl7Version] = &[
        Hl7Version::V2_1,
        Hl7Version::V2_2,
        Hl7Version::V2_3,
        Hl7Version::V2_3_1,
        Hl7Version::V2_4,
        Hl7Version::V2_5,
        Hl7Version::V2_5_1,
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
/// Produced by `tools/hl7-schema-importer/` from hl7-dictionary (MIT): the
/// Node converter emits the payload and the Rust importer validates
/// referential integrity before the file ships. Regenerating any of them is
/// reproducible — see that tool's README.
const V2_1_JSON: &str = include_str!("../../../../resources/hl7/v2_1.json");
const V2_2_JSON: &str = include_str!("../../../../resources/hl7/v2_2.json");
const V2_3_JSON: &str = include_str!("../../../../resources/hl7/v2_3.json");
const V2_3_1_JSON: &str = include_str!("../../../../resources/hl7/v2_3_1.json");
const V2_4_JSON: &str = include_str!("../../../../resources/hl7/v2_4.json");
const V2_5_JSON: &str = include_str!("../../../../resources/hl7/v2_5.json");
const V2_5_1_JSON: &str = include_str!("../../../../resources/hl7/v2_5_1.json");
const V2_6_JSON: &str = include_str!("../../../../resources/hl7/v2_6.json");
const V2_7_JSON: &str = include_str!("../../../../resources/hl7/v2_7.json");

/// Raw payload backing a version. Alias versions carry none of their own and
/// resolve to the release they point at.
fn payload(version: Hl7Version) -> &'static str {
    match version.aliases().unwrap_or(version) {
        Hl7Version::V2_1 => V2_1_JSON,
        Hl7Version::V2_2 => V2_2_JSON,
        Hl7Version::V2_3 => V2_3_JSON,
        Hl7Version::V2_3_1 => V2_3_1_JSON,
        Hl7Version::V2_4 => V2_4_JSON,
        Hl7Version::V2_5 => V2_5_JSON,
        Hl7Version::V2_5_1 => V2_5_1_JSON,
        Hl7Version::V2_6 => V2_6_JSON,
        Hl7Version::V2_7 | Hl7Version::V2_7_1 => V2_7_JSON,
    }
}

pub fn load(version: Hl7Version) -> Hl7Schema {
    // The payload may come from an aliased release, but the schema keeps the
    // requested version so callers still see what the message declared.
    let json = payload(version);
    let hydrated: HydratedSchema = serde_json::from_str(json)
        .expect("shipped HL7 schema JSON is malformed — this is a build bug");
    hydrated.into_schema(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every shipped version must load, and every non-alias version must
    /// carry its *own* definitions.
    ///
    /// Regression guard: v2.7.1 once shipped as a byte-identical copy of the
    /// v2.7 payload — the app advertised a version whose tables it did not
    /// hold. Duplicates are now only legal when declared via `aliases()`.
    #[test]
    fn non_alias_versions_have_distinct_catalogues() {
        use std::collections::HashMap;

        // Compare the embedded payloads, not their element counts: v2.5 and
        // v2.5.1 happen to hold the same number of messages, segments and
        // composites while differing in the definitions themselves.
        let mut seen: HashMap<&'static str, Hl7Version> = HashMap::new();
        for v in Hl7Version::ALL {
            let s = load(*v);
            assert_eq!(s.version, *v, "load() must preserve the requested version");
            assert!(!s.messages.is_empty(), "{} has no messages", v.as_str());

            if v.is_alias() {
                continue;
            }
            if let Some(prev) = seen.insert(payload(*v), *v) {
                panic!(
                    "v{} and v{} embed byte-identical payloads — if that is \
                     intentional, declare one an alias of the other",
                    prev.as_str(),
                    v.as_str()
                );
            }
        }
    }

    /// An alias resolves to its source's data while keeping its own identity.
    #[test]
    fn v2_7_1_aliases_v2_7() {
        assert_eq!(Hl7Version::V2_7_1.aliases(), Some(Hl7Version::V2_7));
        assert!(!Hl7Version::V2_7.is_alias());

        let alias = load(Hl7Version::V2_7_1);
        let source = load(Hl7Version::V2_7);
        assert_eq!(alias.version, Hl7Version::V2_7_1);
        assert_eq!(alias.messages.len(), source.messages.len());
        assert_eq!(alias.segments.len(), source.segments.len());
    }

    /// v2.5.1 is the US baseline profile and is a distinct release: it must
    /// not silently resolve to the v2.5 catalogue.
    #[test]
    fn v2_5_1_is_its_own_release() {
        assert!(!Hl7Version::V2_5_1.is_alias());
        assert_ne!(
            payload(Hl7Version::V2_5),
            payload(Hl7Version::V2_5_1),
            "v2.5 and v2.5.1 payloads are identical"
        );
    }

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
