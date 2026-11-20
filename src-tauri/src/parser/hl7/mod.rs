pub mod ack;
pub mod charset;
pub mod delimiters;
pub mod export;
pub mod grid;
pub mod interpret;
pub mod lexer;
pub mod message;
pub mod schema;
pub mod tables;
pub mod value_tables;

/// An HL7 v2 message as it goes on the wire: every segment, the last one
/// too, ends with one CR, as the CLI's `send` writes it.
///
/// A message typed or pasted in the editor keeps the line endings of the
/// document (LF, or CR LF on Windows), and a receiver reads the LF as the
/// first character of the next segment's name. LF and CR LF become CR. A
/// blank line is not a segment: the blank lines an editor leaves at the
/// end went out as empty segments, which strict receivers refuse, and
/// text without a final line break went out without the CR that ends the
/// last segment. Blank lines (and a byte-order mark) are dropped wherever
/// they are. Text that is not an HL7 v2 message (FHIR JSON or XML, a
/// custom body) is returned as it is.
pub fn to_wire_segments(text: &str) -> std::borrow::Cow<'_, str> {
    let head = text.trim_start_matches(['\u{FEFF}', '\r', '\n']);
    let is_v2 = matches!(head.get(..3), Some("MSH" | "FHS" | "BHS"))
        && head[3..].chars().next().is_some_and(|c| !c.is_alphanumeric() && !c.is_whitespace());
    if !is_v2 {
        return std::borrow::Cow::Borrowed(text);
    }
    let blank = |segment: &str| segment.trim().is_empty();
    let on_the_wire_already = head.len() == text.len()
        && !text.contains('\n')
        && text.strip_suffix('\r').is_some_and(|body| !body.split('\r').any(blank));
    if on_the_wire_already {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut wire = String::with_capacity(head.len() + 1);
    for segment in head.split(['\r', '\n']).filter(|s| !blank(s)) {
        wire.push_str(segment);
        wire.push('\r');
    }
    std::borrow::Cow::Owned(wire)
}

#[cfg(test)]
mod wire_tests {
    use super::to_wire_segments;

    #[test]
    fn lf_and_crlf_become_cr_for_hl7_v2_only() {
        assert_eq!(to_wire_segments("MSH|^~\\&|A\nPID|1\n"), "MSH|^~\\&|A\rPID|1\r");
        assert_eq!(to_wire_segments("MSH|^~\\&|A\r\nPID|1\r\n"), "MSH|^~\\&|A\rPID|1\r");
        assert_eq!(to_wire_segments("\u{FEFF}\r\nMSH|^~\\&|A\rPID|1"), "MSH|^~\\&|A\rPID|1\r");
        assert!(matches!(to_wire_segments("MSH|^~\\&|A\rPID|1\r"), std::borrow::Cow::Borrowed(_)));
        // FHIR and free text keep their line breaks.
        let json = "{\n  \"resourceType\": \"Patient\"\n}";
        assert_eq!(to_wire_segments(json), json);
        assert_eq!(to_wire_segments("MSHALLOW\nx"), "MSHALLOW\nx");
    }

    /// What the CLI's `send` writes: the last segment ends with CR too, and
    /// blank lines are not sent as empty segments.
    #[test]
    fn the_last_segment_ends_with_cr_and_blank_lines_are_not_segments() {
        assert_eq!(to_wire_segments("MSH|^~\\&|A\nPID|1\n\n\n"), "MSH|^~\\&|A\rPID|1\r");
        assert_eq!(to_wire_segments("MSH|^~\\&|A\rPID|1"), "MSH|^~\\&|A\rPID|1\r");
        assert_eq!(to_wire_segments("MSH|^~\\&|A\rPID|1\r\r\r"), "MSH|^~\\&|A\rPID|1\r");
        assert_eq!(to_wire_segments("MSH|^~\\&|A\r\n\r\nPID|1\r\n  \r\nOBX|1"), "MSH|^~\\&|A\rPID|1\rOBX|1\r");
        assert_eq!(to_wire_segments("MSH|^~\\&|A"), "MSH|^~\\&|A\r");
        // Already as it goes on the wire: not copied.
        assert!(matches!(to_wire_segments("MSH|^~\\&|A\r"), std::borrow::Cow::Borrowed(_)));
        assert!(matches!(to_wire_segments("MSH|^~\\&|A\r \rPID|1\r"), std::borrow::Cow::Owned(_)));
    }
}
