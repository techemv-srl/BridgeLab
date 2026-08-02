//! XSD generator for HL7 v2.xml message schemas.
//!
//! Output format matches the reference XSDs used by Astraia and compatible
//! tooling: `xmlns:xsd="http://www.w3.org/2001/XMLSchema"`, the message as
//! an element with an inline complex type, each segment / composite as a
//! named complex type, each primitive as an `xsd:string`-based simple type.
//! Groups are represented as inline complex types with a single-level
//! `MESSAGE.GROUP` name.

use super::{CompositeType, Hl7Schema, MessageElement, MessageStructure, PrimitiveType, SegmentSpec};
use std::fmt::Write;

pub fn generate_xsd(schema: &Hl7Schema, message_code: &str) -> Result<String, String> {
    let message = schema.message(message_code).ok_or_else(|| {
        format!(
            "Message '{}' not found in HL7 v{}",
            message_code,
            schema.version.as_str()
        )
    })?;

    let mut out = String::with_capacity(32 * 1024);
    writeln!(out, r#"<xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema">"#).unwrap();

    emit_message(&mut out, message);

    let segments = schema.segments_used_by(&message.code);
    for code in &segments {
        if let Some(seg) = schema.segment(code) {
            emit_segment(&mut out, seg);
        }
    }

    let (composites, primitives) = schema.data_types_used_by(&message.code);
    for c in composites {
        emit_composite(&mut out, c);
    }
    for p in primitives {
        emit_primitive(&mut out, p);
    }

    writeln!(out, "</xsd:schema>").unwrap();
    Ok(out)
}

// ---------- message + groups ------------------------------------------------

/// Make a name a valid XSD `NCName`: replace every character outside
/// `[A-Za-z0-9_.-]` with `_` (hl7-dictionary group names can contain `/`,
/// e.g. v2.7 "Observation/Result_Group"), and guard the first character.
fn ncname(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-' { c } else { '_' })
        .collect();
    if !out.chars().next().map(|c| c.is_ascii_alphabetic() || c == '_').unwrap_or(false) {
        out.insert(0, '_');
    }
    out
}

/// One particle of a (possibly rewritten) sequence content model.
enum Particle<'a> {
    Plain(&'a MessageElement),
    /// A span of particles whose original layout violates XSD's Unique
    /// Particle Attribution rule (same-named particle reachable twice through
    /// an all-optional window, e.g. v2.3 ADT_A06 `[DRG] [OBX*] [AL1*] [DG1*] [DRG]`).
    /// Emitted as an unordered, repeating choice — a strict superset of the
    /// original model — so the schema compiles under standard processors.
    Relaxed(Vec<&'a MessageElement>),
}

/// Element names a particle can start with / match (used for UPA analysis).
fn particle_names(e: &MessageElement, msg_code: &str) -> Vec<String> {
    match e {
        MessageElement::Segment { code, .. } => vec![code.clone()],
        MessageElement::Group { name, .. } => vec![format!("{}.{}", msg_code, ncname(name))],
        MessageElement::Choice { segments, .. } => segments.clone(),
    }
}

fn is_nullable(e: &MessageElement) -> bool {
    match e {
        MessageElement::Segment { required, .. }
        | MessageElement::Group { required, .. }
        | MessageElement::Choice { required, .. } => !required,
    }
}

fn is_repeating(e: &MessageElement) -> bool {
    match e {
        MessageElement::Segment { repeats, .. }
        | MessageElement::Group { repeats, .. }
        | MessageElement::Choice { repeats, .. } => *repeats,
    }
}

/// Rewrite a sequence so it satisfies Unique Particle Attribution: any pair of
/// particles that can match the same element name, where the first is optional
/// or repeating and everything between them is optional, is merged (inclusive)
/// into one `Particle::Relaxed` span. Iterates to a fixpoint so overlapping
/// spans (e.g. NMR_N01 with three ambiguous NTE positions) collapse into one.
fn relax_upa<'a>(elements: &'a [MessageElement], msg_code: &str) -> Vec<Particle<'a>> {
    struct P<'a> {
        names: Vec<String>,
        nullable: bool,
        conflicts: bool, // optional or repeating → can clash with a later twin
        inner: Vec<&'a MessageElement>,
    }
    let mut parts: Vec<P<'a>> = elements
        .iter()
        .map(|e| P {
            names: particle_names(e, msg_code),
            nullable: is_nullable(e),
            conflicts: is_nullable(e) || is_repeating(e),
            inner: vec![e],
        })
        .collect();

    loop {
        let mut merge: Option<(usize, usize)> = None;
        'search: for i in 0..parts.len() {
            if !parts[i].conflicts {
                continue;
            }
            for j in (i + 1)..parts.len() {
                let clash = parts[i].names.iter().any(|n| parts[j].names.contains(n));
                let window_nullable = (i + 1..j).all(|k| parts[k].nullable);
                if clash && window_nullable {
                    merge = Some((i, j));
                    break 'search;
                }
                if !window_nullable && j > i + 1 {
                    break; // a required particle closes the ambiguity window
                }
            }
        }
        let Some((i, j)) = merge else { break };
        let merged: Vec<P<'a>> = parts.drain(i..=j).collect();
        let mut names = Vec::new();
        let mut inner = Vec::new();
        for p in merged {
            for n in p.names {
                if !names.contains(&n) {
                    names.push(n);
                }
            }
            inner.extend(p.inner);
        }
        parts.insert(i, P { names, nullable: true, conflicts: true, inner });
    }

    parts
        .into_iter()
        .map(|p| {
            if p.inner.len() == 1 {
                Particle::Plain(p.inner[0])
            } else {
                Particle::Relaxed(p.inner)
            }
        })
        .collect()
}

fn emit_message(out: &mut String, message: &MessageStructure) {
    let indent = "    ";
    writeln!(out, r#"    <xsd:element name="{}">"#, message.code).unwrap();
    writeln!(out, "{i}<xsd:complexType>", i = indent.repeat(2)).unwrap();
    writeln!(out, "{i}<xsd:sequence>", i = indent.repeat(3)).unwrap();
    emit_elements(out, &message.elements, &message.code, 4);
    writeln!(out, "{i}</xsd:sequence>", i = indent.repeat(3)).unwrap();
    writeln!(out, "{i}</xsd:complexType>", i = indent.repeat(2)).unwrap();
    writeln!(out, "{i}</xsd:element>", i = indent.repeat(1)).unwrap();
}

fn emit_elements(out: &mut String, elements: &[MessageElement], msg_code: &str, depth: usize) {
    let indent = "    ".repeat(depth);
    for particle in relax_upa(elements, msg_code) {
        match particle {
            Particle::Relaxed(span) => emit_relaxed_span(out, &span, msg_code, depth),
            Particle::Plain(e) => emit_element(out, e, msg_code, depth, &indent),
        }
    }
}

/// Emit a UPA-ambiguous span as an unordered repeating choice (see
/// [`Particle::Relaxed`]). Each distinct element name appears exactly once.
fn emit_relaxed_span(out: &mut String, span: &[&MessageElement], msg_code: &str, depth: usize) {
    let indent = "    ".repeat(depth);
    writeln!(out, r#"{}<xsd:choice minOccurs="0" maxOccurs="unbounded">"#, indent).unwrap();
    writeln!(out, "{}    <xsd:annotation>", indent).unwrap();
    writeln!(
        out,
        "{}        <xsd:documentation>Relaxed content model: the HL7 definition repeats a segment across an optional-only window, which violates XSD Unique Particle Attribution. This choice accepts every message the original model accepts.</xsd:documentation>",
        indent
    )
    .unwrap();
    writeln!(out, "{}    </xsd:annotation>", indent).unwrap();

    let mut seen: Vec<String> = Vec::new();
    let inner_indent = format!("{}    ", indent);
    for e in span {
        match e {
            MessageElement::Segment { code, .. } => {
                if !seen.contains(code) {
                    seen.push(code.clone());
                    writeln!(out, r#"{}<xsd:element name="{}" type="{}"/>"#, inner_indent, code, code).unwrap();
                }
            }
            MessageElement::Group { name, elements: inner, .. } => {
                let group_full = format!("{}.{}", msg_code, ncname(name));
                if !seen.contains(&group_full) {
                    seen.push(group_full.clone());
                    writeln!(out, r#"{}<xsd:element name="{}">"#, inner_indent, group_full).unwrap();
                    writeln!(out, "{}    <xsd:complexType>", inner_indent).unwrap();
                    writeln!(out, "{}        <xsd:sequence>", inner_indent).unwrap();
                    emit_elements(out, inner, msg_code, depth + 4);
                    writeln!(out, "{}        </xsd:sequence>", inner_indent).unwrap();
                    writeln!(out, "{}    </xsd:complexType>", inner_indent).unwrap();
                    writeln!(out, "{}</xsd:element>", inner_indent).unwrap();
                }
            }
            MessageElement::Choice { segments, .. } => {
                for s in segments {
                    if !seen.contains(s) {
                        seen.push(s.clone());
                        writeln!(out, r#"{}<xsd:element name="{}" type="{}"/>"#, inner_indent, s, s).unwrap();
                    }
                }
            }
        }
    }
    writeln!(out, "{}</xsd:choice>", indent).unwrap();
}

fn emit_element(out: &mut String, e: &MessageElement, msg_code: &str, depth: usize, indent: &str) {
    {
        match e {
            MessageElement::Segment { code, required, repeats } => {
                let min = if *required { 1 } else { 0 };
                let max = if *repeats { "unbounded".to_string() } else { "1".to_string() };
                // HL7 v2.xml convention: "unbounded" is the default for `maxOccurs` only if
                // explicitly set; for "1" we still emit it for clarity where needed,
                // but reference XSDs omit minOccurs="1" maxOccurs="1" when both are default.
                write!(out, r#"{}<xsd:element name="{}" type="{}""#, indent, code, code).unwrap();
                if min == 0 {
                    write!(out, r#" minOccurs="0""#).unwrap();
                }
                if max != "1" {
                    write!(out, r#" maxOccurs="{}""#, max).unwrap();
                }
                writeln!(out, "/>").unwrap();
            }
            MessageElement::Group { name, required, repeats, elements: inner } => {
                let group_full = format!("{}.{}", msg_code, ncname(name));
                let min = if *required { 1 } else { 0 };
                let max = if *repeats { "unbounded".to_string() } else { "1".to_string() };
                write!(out, r#"{}<xsd:element name="{}""#, indent, group_full).unwrap();
                if min == 0 {
                    write!(out, r#" minOccurs="0""#).unwrap();
                }
                if max != "1" {
                    write!(out, r#" maxOccurs="{}""#, max).unwrap();
                }
                writeln!(out, ">").unwrap();
                writeln!(out, "{}    <xsd:complexType>", indent).unwrap();
                writeln!(out, "{}        <xsd:sequence>", indent).unwrap();
                emit_elements(out, inner, msg_code, depth + 3);
                writeln!(out, "{}        </xsd:sequence>", indent).unwrap();
                writeln!(out, "{}    </xsd:complexType>", indent).unwrap();
                writeln!(out, "{}</xsd:element>", indent).unwrap();
            }
            MessageElement::Choice { required, repeats, segments } => {
                let min = if *required { 1 } else { 0 };
                let max = if *repeats { "unbounded".to_string() } else { "1".to_string() };
                write!(out, "{}<xsd:choice", indent).unwrap();
                if min != 1 {
                    write!(out, r#" minOccurs="{}""#, min).unwrap();
                }
                if max != "1" {
                    write!(out, r#" maxOccurs="{}""#, max).unwrap();
                }
                writeln!(out, ">").unwrap();
                for s in segments {
                    writeln!(out, r#"{}    <xsd:element name="{}" type="{}"/>"#, indent, s, s).unwrap();
                }
                writeln!(out, "{}</xsd:choice>", indent).unwrap();
            }
        }
    }
}

// ---------- segments --------------------------------------------------------

fn emit_segment(out: &mut String, seg: &SegmentSpec) {
    writeln!(out, r#"    <xsd:complexType name="{}">"#, seg.code).unwrap();
    writeln!(out, "        <xsd:sequence>").unwrap();
    for field in &seg.fields {
        let min = if field.required { 1 } else { 0 };
        let max = if field.repeats { "unbounded".to_string() } else { "1".to_string() };
        let field_name = format!("{}.{}", seg.code, field.position);
        write!(out, r#"            <xsd:element name="{}" type="{}""#, field_name, field.data_type).unwrap();
        if min == 0 {
            write!(out, r#" minOccurs="0""#).unwrap();
        }
        if max != "1" {
            write!(out, r#" maxOccurs="{}""#, max).unwrap();
        }
        writeln!(out, "/>").unwrap();
    }
    writeln!(out, "        </xsd:sequence>").unwrap();
    writeln!(out, "    </xsd:complexType>").unwrap();
}

// ---------- composites + primitives -----------------------------------------

fn emit_composite(out: &mut String, c: &CompositeType) {
    writeln!(out, r#"    <xsd:complexType name="{}">"#, c.code).unwrap();
    writeln!(out, "        <xsd:sequence>").unwrap();
    for comp in &c.components {
        let min = if comp.required { 1 } else { 0 };
        let field_name = format!("{}.{}", c.code, comp.position);
        write!(out, r#"            <xsd:element name="{}" type="{}""#, field_name, comp.data_type).unwrap();
        if min == 0 {
            write!(out, r#" minOccurs="0""#).unwrap();
        }
        writeln!(out, "/>").unwrap();
    }
    writeln!(out, "        </xsd:sequence>").unwrap();
    writeln!(out, "    </xsd:complexType>").unwrap();
}

fn emit_primitive(out: &mut String, p: &PrimitiveType) {
    writeln!(out, r#"    <xsd:simpleType name="{}">"#, p.code).unwrap();
    writeln!(out, r#"        <xsd:restriction base="xsd:string"/>"#).unwrap();
    writeln!(out, "    </xsd:simpleType>").unwrap();
}

#[cfg(test)]
mod tests {
    use super::super::{load, Hl7Version};
    use super::*;

    fn xsd(code: &str) -> String {
        let s = load(Hl7Version::V2_5);
        generate_xsd(&s, code).expect("generate_xsd")
    }

    #[test]
    fn uses_xsd_prefix_not_xs() {
        let out = xsd("ADT_A40");
        assert!(out.contains(r#"xmlns:xsd="http://www.w3.org/2001/XMLSchema""#));
        assert!(!out.contains("xmlns:xs="));
        assert!(!out.contains("<xs:"));
    }

    #[test]
    fn message_root_uses_inline_complex_type() {
        let out = xsd("ADT_A40");
        assert!(out.contains(r#"<xsd:element name="ADT_A40">"#));
        // No separate ADT_A40.CONTENT complex type (that was the old format).
        assert!(!out.contains("ADT_A40.CONTENT"));
    }

    #[test]
    fn segments_use_named_types_not_ref() {
        let out = xsd("ADT_A40");
        assert!(out.contains(r#"<xsd:element name="MSH" type="MSH""#));
        assert!(out.contains(r#"<xsd:element name="MRG" type="MRG""#));
        assert!(!out.contains(r#"ref="MSH""#));
    }

    #[test]
    fn fields_use_named_types_not_inline_simple_types() {
        let out = xsd("ADT_A40");
        // Example: EVN.1 is ID, EVN.2 is TS in v2.5
        assert!(out.contains(r#"<xsd:element name="EVN.1" type="ID""#));
        assert!(out.contains(r#"<xsd:element name="EVN.2" type="TS""#));
        // No more inline simpleType restrictions with maxLength
        assert!(!out.contains("xsd:maxLength"));
    }

    #[test]
    fn composite_and_primitive_types_are_emitted() {
        let out = xsd("ADT_A40");
        // Composites needed by ADT_A40
        for t in ["MSG", "VID", "HD", "PT", "XPN", "XAD", "XTN", "XCN", "CX", "CE", "CWE", "TS"] {
            assert!(
                out.contains(&format!(r#"<xsd:complexType name="{}">"#, t)),
                "missing composite type {}", t
            );
        }
        // Primitives needed (some used by MSH/EVN etc.)
        for t in ["ST", "ID", "DTM", "NM", "SI"] {
            assert!(
                out.contains(&format!(r#"<xsd:simpleType name="{}">"#, t)),
                "missing primitive type {}", t
            );
        }
    }

    #[test]
    fn adt_a40_has_patient_id_group() {
        let out = xsd("ADT_A40");
        assert!(out.contains(r#"<xsd:element name="ADT_A40.PATIENT_ID""#));
        // The old code used `.CONTENT` suffix; reference format uses inline complexType.
        assert!(!out.contains("ADT_A40.PATIENT_ID.CONTENT"));
    }

    #[test]
    fn orm_o01_has_choice_inside_order_detail() {
        let out = xsd("ORM_O01");
        assert!(out.contains("<xsd:choice"));
        // The six choice members
        for s in ["OBR", "RQD", "RQ1", "RXO", "ODS", "ODT"] {
            assert!(
                out.contains(&format!(r#"<xsd:element name="{}" type="{}"/>"#, s, s)),
                "choice member {} missing", s
            );
        }
    }

    #[test]
    fn orm_o01_has_insurance_inside_patient_group() {
        let out = xsd("ORM_O01");
        assert!(out.contains(r#"<xsd:element name="ORM_O01.PATIENT""#));
        assert!(out.contains(r#"<xsd:element name="ORM_O01.INSURANCE""#));
        assert!(out.contains(r#"<xsd:element name="ORM_O01.ORDER""#));
        assert!(out.contains(r#"<xsd:element name="ORM_O01.ORDER_DETAIL""#));
        assert!(out.contains(r#"<xsd:element name="ORM_O01.OBSERVATION""#));
    }

    #[test]
    fn oru_r01_has_sft_and_timing_qty_and_specimen() {
        let out = xsd("ORU_R01");
        assert!(out.contains(r#"<xsd:element name="SFT" type="SFT""#));
        assert!(out.contains(r#"<xsd:element name="DSC" type="DSC""#));
        assert!(out.contains(r#"<xsd:element name="ORU_R01.TIMING_QTY""#));
        assert!(out.contains(r#"<xsd:element name="ORU_R01.SPECIMEN""#));
        assert!(out.contains(r#"<xsd:element name="ORU_R01.VISIT""#));
        assert!(out.contains(r#"<xsd:element name="ORU_R01.PATIENT_RESULT""#));
    }

    #[test]
    fn unknown_message_returns_error() {
        let s = load(Hl7Version::V2_5);
        assert!(generate_xsd(&s, "FOO_BAR").is_err());
    }

    #[test]
    fn produces_balanced_xml() {
        let out = xsd("ORM_O01");
        assert_eq!(out.matches("<xsd:schema").count(), out.matches("</xsd:schema>").count());
        assert_eq!(out.matches("<xsd:complexType").count(), out.matches("</xsd:complexType>").count());
        assert_eq!(out.matches("<xsd:sequence>").count(), out.matches("</xsd:sequence>").count());
        assert_eq!(out.matches("<xsd:choice").count(), out.matches("</xsd:choice>").count());
    }
}

#[cfg(test)]
mod full_catalogue_tests {
    use super::*;
    use crate::parser::hl7::schema::{load, Hl7Version};

    /// The full hl7-dictionary catalogue must export cleanly for messages
    /// far outside the original 4-message bootstrap — including one with
    /// nested groups (SIU scheduling) and one with a choice block (ORM).
    #[test]
    fn exports_catalogue_only_messages() {
        let schema = load(Hl7Version::V2_5);
        for code in ["SIU_S12", "ORM_O01", "MDM_T02", "VXU_V04", "QBP_Q11"] {
            let out = generate_xsd(&schema, code)
                .unwrap_or_else(|e| panic!("XSD export failed for {}: {}", code, e));
            assert!(out.contains(&format!(r#"<xsd:element name="{}">"#, code)),
                    "{} root element missing", code);
            assert!(out.contains("xsd:schema"), "{} not a schema", code);
        }
    }

    /// Every message in the catalogue must export without errors — the
    /// importer guarantees referential integrity, this guarantees the
    /// generator holds up across all 248 structures.
    #[test]
    fn exports_every_catalogue_message() {
        let schema = load(Hl7Version::V2_5);
        for m in &schema.messages {
            generate_xsd(&schema, &m.code)
                .unwrap_or_else(|e| panic!("XSD export failed for {}: {}", m.code, e));
        }
    }

    /// Same guarantee for every other shipped version: each catalogue loads
    /// and every one of its message structures exports without error — and
    /// every emitted name/type attribute is a valid XSD NCName (v2.7 group
    /// names like "Observation/Result_Group" must be sanitized).
    #[test]
    fn exports_every_message_of_every_version() {
        fn is_ncname(s: &str) -> bool {
            let mut chars = s.chars();
            matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
        }
        for v in Hl7Version::ALL {
            let schema = load(*v);
            assert!(
                schema.messages.len() >= 170,
                "{:?}: suspiciously small catalogue ({} messages)",
                v,
                schema.messages.len()
            );
            assert!(
                schema.message("ADT_A01").is_some(),
                "{:?}: ADT_A01 missing from catalogue",
                v
            );
            for m in &schema.messages {
                let xsd = generate_xsd(&schema, &m.code)
                    .unwrap_or_else(|e| panic!("{:?}: XSD export failed for {}: {}", v, m.code, e));
                // Dev hook: BL_XSD_DUMP=<dir> writes every export to disk so an
                // external schema compiler (e.g. python-lxml) can verify them.
                if let Ok(dir) = std::env::var("BL_XSD_DUMP") {
                    std::fs::write(format!("{}/{}_{}.xsd", dir, v.as_str(), m.code), &xsd).ok();
                }
                for attr in ["name=\"", "type=\""] {
                    for chunk in xsd.split(attr).skip(1) {
                        let val = chunk.split('"').next().unwrap_or("");
                        if val.starts_with("xsd:") {
                            continue;
                        }
                        assert!(
                            is_ncname(val),
                            "{:?} {}: invalid NCName in {}{}\"",
                            v, m.code, attr, val
                        );
                    }
                }
            }
        }
    }

    /// UPA regression (Codex review, PR #85): structures that repeat a segment
    /// across an all-optional window (v2.3 ADT_A06 double DRG, v2.5 DFT_P03
    /// double ROL, v2.6 ADT_A60 double ARV) must emit the relaxed choice, and
    /// the ambiguous element name must appear exactly once as a declaration.
    #[test]
    fn upa_ambiguous_spans_are_relaxed() {
        for (version, msg) in [
            (Hl7Version::V2_3, "ADT_A06"),
            (Hl7Version::V2_3, "ADT_A07"),
            (Hl7Version::V2_5, "DFT_P03"),
            (Hl7Version::V2_6, "ADT_A60"),
            (Hl7Version::V2_7, "OSM_R26"),
        ] {
            let schema = load(version);
            let xsd = generate_xsd(&schema, msg).unwrap();
            assert!(
                xsd.contains("Relaxed content model"),
                "{:?} {}: expected relaxed-choice annotation",
                version, msg
            );
        }
        // ADT_A06 v2.3: DRG appears nowhere else in the message, so after
        // relaxation it must be declared exactly once (was twice, ambiguous).
        let xsd = generate_xsd(&load(Hl7Version::V2_3), "ADT_A06").unwrap();
        assert_eq!(xsd.matches(r#"name="DRG" type="DRG""#).count(), 1);
    }

    /// Sequences without ambiguity keep their strict ordered model.
    #[test]
    fn unambiguous_messages_stay_strict() {
        let schema = load(Hl7Version::V2_5);
        // ADT_A01 has two top-level ROL positions, but a required PV1 sits
        // between them — no ambiguity, both stay declared (plus more ROLs in
        // other scopes, e.g. the PROCEDURE group).
        let xsd = generate_xsd(&schema, "ADT_A01").unwrap();
        assert!(!xsd.contains("Relaxed content model"), "ADT_A01 must not be relaxed");
        assert!(xsd.matches(r#"name="ROL" type="ROL""#).count() >= 2);
    }

    #[test]
    fn ncname_sanitizes_slashes() {
        assert_eq!(ncname("Observation/Result_Group"), "Observation_Result_Group");
        assert_eq!(ncname("PLAIN_NAME"), "PLAIN_NAME");
        assert_eq!(ncname("1BAD"), "_1BAD");
    }
}
