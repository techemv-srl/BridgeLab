use serde::Serialize;

use crate::parser::hl7::message::Hl7Message;

/// A detected PHI location in the message.
#[derive(Debug, Clone, Serialize)]
pub struct PhiLocation {
    pub segment_idx: usize,
    pub segment_type: String,
    pub field_position: usize,
    pub field_name: String,
    pub sensitivity: PhiSensitivity,
    pub current_value: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PhiSensitivity {
    High,
    Medium,
    Low,
}

/// Known PHI fields by segment type (field position, name, sensitivity).
const PHI_FIELDS: &[(&str, usize, &str, PhiSensitivity)] = &[
    // PID - Patient Identification
    ("PID", 3, "Patient Identifier List", PhiSensitivity::High),
    ("PID", 4, "Alternate Patient ID", PhiSensitivity::High),
    ("PID", 5, "Patient Name", PhiSensitivity::High),
    ("PID", 6, "Mother's Maiden Name", PhiSensitivity::Medium),
    ("PID", 7, "Date/Time of Birth", PhiSensitivity::High),
    ("PID", 9, "Patient Alias", PhiSensitivity::Medium),
    ("PID", 11, "Patient Address", PhiSensitivity::High),
    ("PID", 13, "Phone Number - Home", PhiSensitivity::High),
    ("PID", 14, "Phone Number - Business", PhiSensitivity::Medium),
    ("PID", 18, "Patient Account Number", PhiSensitivity::High),
    ("PID", 19, "SSN Number", PhiSensitivity::High),
    ("PID", 20, "Driver's License Number", PhiSensitivity::High),
    // NK1 - Next of Kin
    ("NK1", 2, "Name", PhiSensitivity::Medium),
    ("NK1", 4, "Address", PhiSensitivity::Medium),
    ("NK1", 5, "Phone Number", PhiSensitivity::Medium),
    // IN1 - Insurance
    ("IN1", 16, "Name of Insured", PhiSensitivity::Medium),
    ("IN1", 36, "Policy Number", PhiSensitivity::High),
    // GT1 - Guarantor
    ("GT1", 3, "Guarantor Name", PhiSensitivity::Medium),
    ("GT1", 5, "Guarantor Address", PhiSensitivity::Medium),
    ("GT1", 6, "Guarantor Phone - Home", PhiSensitivity::Medium),
    ("GT1", 12, "Guarantor SSN", PhiSensitivity::High),
];

/// Detect PHI fields in an HL7 message.
pub fn detect_phi(msg: &Hl7Message) -> Vec<PhiLocation> {
    let mut locations = Vec::new();

    for (seg_idx, seg) in msg.segments.iter().enumerate() {
        for &(seg_type, field_pos, field_name, ref sensitivity) in PHI_FIELDS {
            if seg.segment_type != seg_type {
                continue;
            }
            if let Some(field) = seg.fields.iter().find(|f| f.position == field_pos) {
                let value = field.span.as_str(&msg.raw).trim();
                if !value.is_empty() {
                    locations.push(PhiLocation {
                        segment_idx: seg_idx,
                        segment_type: seg_type.to_string(),
                        field_position: field_pos,
                        field_name: field_name.to_string(),
                        sensitivity: sensitivity.clone(),
                        current_value: value.chars().take(50).collect(),
                    });
                }
            }
        }
    }

    locations
}

/// Anonymize an HL7 message by replacing PHI fields with masked values.
pub fn anonymize_message(msg: &Hl7Message) -> String {
    let text = String::from_utf8_lossy(&msg.raw).to_string();
    let mut lines: Vec<String> = Vec::new();

    // Split by segment (CR or CRLF)
    for (seg_idx, seg) in msg.segments.iter().enumerate() {
        let seg_text = seg.span.as_str(&msg.raw);
        let mut fields: Vec<&str> = seg_text.split(msg.delimiters.field as char).collect();

        for &(seg_type, field_pos, _name, ref _sensitivity) in PHI_FIELDS {
            if seg.segment_type != seg_type {
                continue;
            }
            // For MSH, field indexing is offset by 1 because MSH-1 is the separator itself
            let field_idx = if seg.segment_type == "MSH" {
                field_pos
            } else {
                field_pos
            };

            if field_idx < fields.len() {
                let original = fields[field_idx];
                if !original.trim().is_empty() {
                    // We can't mutate borrowed slices, so we rebuild
                    // Use a placeholder approach
                }
            }
        }

        lines.push(seg_text.to_string());
    }

    // Rebuild with replacements using a direct approach
    anonymize_by_replacement(msg)
}

/// Anonymize by building a new message with PHI fields replaced.
fn anonymize_by_replacement(msg: &Hl7Message) -> String {
    let sep = msg.delimiters.field as char;
    let mut result_segments: Vec<String> = Vec::new();

    for (seg_idx, seg) in msg.segments.iter().enumerate() {
        let seg_text = seg.span.as_str(&msg.raw);
        let mut fields: Vec<String> = seg_text.split(sep).map(|s| s.to_string()).collect();

        let is_msh = seg.segment_type == "MSH";

        for &(seg_type, field_pos, _name, ref sensitivity) in PHI_FIELDS {
            if seg.segment_type != seg_type {
                continue;
            }

            // In HL7, for non-MSH segments: fields[0] = segment name, fields[1] = field 1
            // For MSH: fields[0] = "MSH", fields[1] = encoding chars, fields[2] = MSH-3
            let idx = if is_msh { field_pos } else { field_pos };

            if idx < fields.len() && !fields[idx].trim().is_empty() {
                fields[idx] = generate_replacement(&fields[idx], field_pos, sensitivity);
            }
        }

        result_segments.push(fields.join(&sep.to_string()));
    }

    result_segments.join("\r")
}

/// Generate a replacement value that preserves format but masks data.
fn generate_replacement(original: &str, field_pos: usize, sensitivity: &PhiSensitivity) -> String {
    // Check if it contains components (^)
    if original.contains('^') {
        // Mask each component separately
        let components: Vec<&str> = original.split('^').collect();
        let masked: Vec<String> = components.iter().enumerate().map(|(i, comp)| {
            if comp.is_empty() {
                String::new()
            } else {
                mask_value(comp, sensitivity)
            }
        }).collect();
        return masked.join("^");
    }

    mask_value(original, sensitivity)
}

/// Mask a single value based on sensitivity.
fn mask_value(value: &str, sensitivity: &PhiSensitivity) -> String {
    if value.is_empty() {
        return String::new();
    }

    match sensitivity {
        PhiSensitivity::High => {
            // Full replacement with asterisks
            if value.chars().all(|c| c.is_ascii_digit()) {
                // Numeric: replace with same-length zeros
                "0".repeat(value.len().min(20))
            } else {
                // Text: replace with REDACTED
                "REDACTED".to_string()
            }
        }
        PhiSensitivity::Medium => {
            // Partial masking: keep first char, mask rest
            if value.len() <= 1 {
                "*".to_string()
            } else {
                let first: String = value.chars().take(1).collect();
                format!("{}***", first)
            }
        }
        PhiSensitivity::Low => {
            // Keep first 3 chars
            let prefix: String = value.chars().take(3).collect();
            if value.len() > 3 {
                format!("{}...", prefix)
            } else {
                prefix
            }
        }
    }
}

/// Build a truncated copy of the message for email sharing.
pub fn build_truncated_copy(msg: &Hl7Message, threshold: usize) -> String {
    let sep = msg.delimiters.field as char;
    let mut result_segments: Vec<String> = Vec::new();

    for seg in &msg.segments {
        let seg_text = seg.span.as_str(&msg.raw);
        let fields: Vec<&str> = seg_text.split(sep).collect();
        let mut out_fields: Vec<String> = Vec::new();

        for field_str in &fields {
            if field_str.len() > threshold {
                let preview: String = field_str.chars().take(threshold / 2).collect();
                out_fields.push(format!("{}{{...{} bytes}}", preview, field_str.len()));
            } else {
                out_fields.push(field_str.to_string());
            }
        }

        result_segments.push(out_fields.join(&sep.to_string()));
    }

    result_segments.join("\r")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::hl7::lexer::Hl7Lexer;

    fn parse(text: &str) -> Hl7Message {
        Hl7Lexer::new().parse(text.as_bytes().to_vec()).unwrap()
    }

    #[test]
    fn test_detect_phi_finds_patient_name() {
        let msg = parse("MSH|^~\\&|A|B|C|D|20230101||ADT^A01|M1|P|2.5\rPID|||123||Doe^John||19800101|M|||123 Main St");
        let phi = detect_phi(&msg);
        assert!(phi.iter().any(|p| p.field_name == "Patient Name"));
        assert!(phi.iter().any(|p| p.field_name == "Patient Identifier List"));
        assert!(phi.iter().any(|p| p.field_name == "Date/Time of Birth"));
    }

    #[test]
    fn test_detect_phi_skips_empty() {
        let msg = parse("MSH|^~\\&|A|B|C|D|20230101||ADT^A01|M1|P|2.5\rPID|1");
        let phi = detect_phi(&msg);
        assert!(phi.is_empty());
    }

    #[test]
    fn test_anonymize_replaces_name() {
        let msg = parse("MSH|^~\\&|A|B|C|D|20230101||ADT^A01|M1|P|2.5\rPID|||123||Doe^John||19800101|M");
        let anon = anonymize_message(&msg);
        assert!(!anon.contains("Doe"));
        assert!(!anon.contains("John"));
        assert!(anon.contains("REDACTED"));
    }

    #[test]
    fn test_anonymize_preserves_structure() {
        let msg = parse("MSH|^~\\&|A|B|C|D|20230101||ADT^A01|M1|P|2.5\rPID|||123||Doe^John||19800101|M");
        let anon = anonymize_message(&msg);
        assert!(anon.starts_with("MSH|"));
        assert!(anon.contains("\rPID|"));
    }

    #[test]
    fn test_truncated_copy() {
        let msg = parse("MSH|^~\\&|A|B|C|D|20230101||ADT^A01|M1|P|2.5\rOBX|1|ED|||AAAAABBBBBCCCCCDDDDD");
        let truncated = build_truncated_copy(&msg, 10);
        assert!(truncated.contains("{..."));
    }

    #[test]
    fn test_mask_value_high() {
        assert_eq!(mask_value("12345", &PhiSensitivity::High), "00000");
        assert_eq!(mask_value("John Doe", &PhiSensitivity::High), "REDACTED");
    }

    #[test]
    fn test_mask_value_medium() {
        assert_eq!(mask_value("Smith", &PhiSensitivity::Medium), "S***");
    }
}
