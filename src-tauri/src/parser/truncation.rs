use super::hl7::message::Hl7Message;

/// Default number of characters to show before truncation marker.
const DEFAULT_PREVIEW_LEN: usize = 50;

/// Build a truncated text representation of the message.
/// Fields exceeding the threshold show only first N chars + `{...N bytes}`.
/// This produces a much smaller string suitable for Monaco rendering.
pub fn build_truncated_text(msg: &Hl7Message, preview_len: usize) -> String {
    let mut result = String::with_capacity(msg.raw.len().min(256 * 1024));

    for (i, segment) in msg.segments.iter().enumerate() {
        if i > 0 {
            result.push('\r');
        }
        // Write segment type
        result.push_str(&segment.segment_type);

        for field in &segment.fields {
            // MSH-1 is the field separator itself, don't add another one
            if segment.segment_type == "MSH" && field.position == 1 {
                result.push_str(field.span.as_str(&msg.raw));
                continue;
            }
            // MSH-2 is encoding chars, preceded by field separator already included in MSH-1
            if segment.segment_type == "MSH" && field.position == 2 {
                result.push_str(field.span.as_str(&msg.raw));
                // Add field separator after encoding chars
                result.push(msg.delimiters.field as char);
                continue;
            }

            // For non-MSH segments and MSH fields >= 3
            if !(segment.segment_type == "MSH" && field.position == 3) {
                result.push(msg.delimiters.field as char);
            }

            if field.is_truncated {
                let content = field.span.as_str(&msg.raw);
                let preview: String = content.chars().take(preview_len).collect();
                result.push_str(&preview);
                let total_bytes = field.span.len();
                result.push_str(&format!("{{...{} bytes}}", total_bytes));
            } else {
                result.push_str(field.span.as_str(&msg.raw));
            }
        }
    }

    result
}

/// Build the full text of the message (no truncation) for clipboard copy.
pub fn build_full_text(msg: &Hl7Message) -> String {
    String::from_utf8_lossy(&msg.raw).to_string()
}

/// Build a truncated copy: every field > threshold gets truncated.
/// Suitable for email sharing.
pub fn build_truncated_copy(msg: &Hl7Message, threshold: usize) -> String {
    build_truncated_text(msg, threshold.min(DEFAULT_PREVIEW_LEN))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::hl7::lexer::Hl7Lexer;

    #[test]
    fn test_truncated_text_small_message() {
        let data = b"MSH|^~\\&|SEND|FAC|RECV|FAC|20240101||ADT^A01|1|P|2.5\rPID|||123||Doe^John".to_vec();
        let lexer = Hl7Lexer::new();
        let msg = lexer.parse(data).unwrap();
        let text = build_truncated_text(&msg, 50);
        assert!(text.contains("MSH|"));
        assert!(text.contains("PID|"));
        assert!(text.contains("Doe^John"));
        assert!(!text.contains("{...")); // No truncation for small fields
    }

    #[test]
    fn test_truncated_text_large_field() {
        let mut data = b"MSH|^~\\&|S|F|R|F|20240101||ORU^R01|1|P|2.5\rOBX|1|ED|B64||".to_vec();
        let large_content: Vec<u8> = vec![b'A'; 500];
        data.extend_from_slice(&large_content);

        let lexer = Hl7Lexer::new().with_truncation_threshold(100);
        let msg = lexer.parse(data).unwrap();
        let text = build_truncated_text(&msg, 50);

        assert!(text.contains("{..."));
        assert!(text.contains("bytes}"));
        // The truncated text should be much shorter than the original
        assert!(text.len() < 300);
    }

    #[test]
    fn test_full_text_preserves_content() {
        let original = b"MSH|^~\\&|SEND|FAC|RECV|FAC|20240101||ADT^A01|1|P|2.5\rPID|||123".to_vec();
        let lexer = Hl7Lexer::new();
        let msg = lexer.parse(original.clone()).unwrap();
        let full = build_full_text(&msg);
        assert_eq!(full.as_bytes(), &original[..]);
    }
}
