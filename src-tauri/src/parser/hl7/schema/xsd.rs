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
    for e in elements {
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
                let group_full = format!("{}.{}", msg_code, name);
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
