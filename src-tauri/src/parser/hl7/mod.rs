pub mod ack;
pub mod charset;
pub mod delimiters;
pub mod export;
pub mod grid;
pub mod lexer;
pub mod message;
pub mod schema;
pub mod tables;
pub mod value_tables;

/// An HL7 v2 message as it goes on the wire: segments end with CR only.
///
/// A message typed or pasted in the editor keeps the line endings of the
/// document (LF, or CR LF on Windows), and a receiver reads the LF as the
/// first character of the next segment's name. LF and CR LF become CR,
/// and blank lines (and a byte-order mark) before the header are dropped.
/// Text that is not an HL7 v2 message (FHIR JSON or XML, a custom body)
/// is returned as it is.
pub fn to_wire_segments(text: &str) -> std::borrow::Cow<'_, str> {
    let head = text.trim_start_matches(['\u{FEFF}', '\r', '\n']);
    let is_v2 = matches!(head.get(..3), Some("MSH" | "FHS" | "BHS"))
        && head[3..].chars().next().is_some_and(|c| !c.is_alphanumeric() && !c.is_whitespace());
    if !is_v2 || (head.len() == text.len() && !text.contains('\n')) {
        return std::borrow::Cow::Borrowed(text);
    }
    std::borrow::Cow::Owned(head.replace("\r\n", "\r").replace('\n', "\r"))
}

#[cfg(test)]
mod wire_tests {
    use super::to_wire_segments;

    #[test]
    fn lf_and_crlf_become_cr_for_hl7_v2_only() {
        assert_eq!(to_wire_segments("MSH|^~\\&|A\nPID|1\n"), "MSH|^~\\&|A\rPID|1\r");
        assert_eq!(to_wire_segments("MSH|^~\\&|A\r\nPID|1\r\n"), "MSH|^~\\&|A\rPID|1\r");
        assert_eq!(to_wire_segments("\u{FEFF}\r\nMSH|^~\\&|A\rPID|1"), "MSH|^~\\&|A\rPID|1");
        assert!(matches!(to_wire_segments("MSH|^~\\&|A\rPID|1\r"), std::borrow::Cow::Borrowed(_)));
        // FHIR and free text keep their line breaks.
        let json = "{\n  \"resourceType\": \"Patient\"\n}";
        assert_eq!(to_wire_segments(json), json);
        assert_eq!(to_wire_segments("MSHALLOW\nx"), "MSHALLOW\nx");
    }
}
