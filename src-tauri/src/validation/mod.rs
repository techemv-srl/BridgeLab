use serde::Serialize;

use crate::parser::hl7::message::Hl7Message;
use crate::parser::hl7::tables;

/// Severity of a validation issue.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A single validation issue found in a message.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub message: String,
    /// Segment index (0-based) where the issue was found
    pub segment_idx: Option<usize>,
    /// Segment type (e.g., "PID")
    pub segment_type: Option<String>,
    /// Field position (1-based, HL7 convention)
    pub field_position: Option<usize>,
    /// Rule ID for grouping
    pub rule_id: String,
}

/// Full validation report.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

/// Validate an HL7 message and return a report.
pub fn validate_hl7_message(msg: &Hl7Message) -> ValidationReport {
    let mut issues = Vec::new();

    validate_structure(msg, &mut issues);
    validate_msh(msg, &mut issues);
    validate_required_fields(msg, &mut issues);
    validate_field_lengths(msg, &mut issues);
    validate_data_types(msg, &mut issues);

    let error_count = issues.iter().filter(|i| i.severity == Severity::Error).count();
    let warning_count = issues.iter().filter(|i| i.severity == Severity::Warning).count();
    let info_count = issues.iter().filter(|i| i.severity == Severity::Info).count();

    ValidationReport {
        issues,
        error_count,
        warning_count,
        info_count,
    }
}

/// Structural validation: message must start with MSH, segments must have valid types.
fn validate_structure(msg: &Hl7Message, issues: &mut Vec<ValidationIssue>) {
    // Must have at least one segment
    if msg.segments.is_empty() {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            message: "Message has no segments".into(),
            segment_idx: None,
            segment_type: None,
            field_position: None,
            rule_id: "STRUCT-001".into(),
        });
        return;
    }

    // First segment must be MSH
    if msg.segments[0].segment_type != "MSH" {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            message: format!(
                "First segment must be MSH, found '{}'",
                msg.segments[0].segment_type
            ),
            segment_idx: Some(0),
            segment_type: Some(msg.segments[0].segment_type.clone()),
            field_position: None,
            rule_id: "STRUCT-002".into(),
        });
    }

    // Validate segment type format (3 uppercase alphanumeric chars)
    for (i, seg) in msg.segments.iter().enumerate() {
        let st = &seg.segment_type;
        if st.len() != 3 || !st.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
            issues.push(ValidationIssue {
                severity: Severity::Warning,
                message: format!("Invalid segment type '{}' at position {}", st, i),
                segment_idx: Some(i),
                segment_type: Some(st.clone()),
                field_position: None,
                rule_id: "STRUCT-003".into(),
            });
        }
    }
}

/// MSH segment validation.
fn validate_msh(msg: &Hl7Message, issues: &mut Vec<ValidationIssue>) {
    let msh = match msg.segments.first() {
        Some(s) if s.segment_type == "MSH" => s,
        _ => return,
    };

    // MSH must have at least 12 fields (MSH-1 through MSH-12)
    let max_pos = msh.fields.iter().map(|f| f.position).max().unwrap_or(0);
    if max_pos < 9 {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            message: "MSH segment is missing required fields (needs at least MSH-9 Message Type)".into(),
            segment_idx: Some(0),
            segment_type: Some("MSH".into()),
            field_position: None,
            rule_id: "MSH-001".into(),
        });
    }

    // MSH-9 (Message Type) must be present and non-empty
    if let Some(f9) = msh.fields.iter().find(|f| f.position == 9) {
        let value = f9.span.as_str(&msg.raw).trim();
        if value.is_empty() {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                message: "MSH-9 (Message Type) is empty".into(),
                segment_idx: Some(0),
                segment_type: Some("MSH".into()),
                field_position: Some(9),
                rule_id: "MSH-002".into(),
            });
        }
    }

    // MSH-10 (Message Control ID) should be present
    let has_f10 = msh.fields.iter().any(|f| {
        f.position == 10 && !f.span.as_str(&msg.raw).trim().is_empty()
    });
    if !has_f10 {
        issues.push(ValidationIssue {
            severity: Severity::Warning,
            message: "MSH-10 (Message Control ID) is missing or empty".into(),
            segment_idx: Some(0),
            segment_type: Some("MSH".into()),
            field_position: Some(10),
            rule_id: "MSH-003".into(),
        });
    }

    // MSH-12 (Version ID) should be present
    if msg.version.is_empty() {
        issues.push(ValidationIssue {
            severity: Severity::Warning,
            message: "MSH-12 (Version ID) is missing".into(),
            segment_idx: Some(0),
            segment_type: Some("MSH".into()),
            field_position: Some(12),
            rule_id: "MSH-004".into(),
        });
    }
}

/// Validate required fields based on HL7 table definitions.
fn validate_required_fields(msg: &Hl7Message, issues: &mut Vec<ValidationIssue>) {
    let version = if msg.version.is_empty() { "2.5" } else { &msg.version };

    for (seg_idx, seg) in msg.segments.iter().enumerate() {
        if let Some(seg_info) = tables::get_segment_info(&seg.segment_type, version) {
            for field_def in &seg_info.fields {
                if !field_def.required {
                    continue;
                }

                let field_present = seg.fields.iter().any(|f| {
                    f.position == field_def.position
                        && !f.span.as_str(&msg.raw).trim().is_empty()
                });

                if !field_present {
                    issues.push(ValidationIssue {
                        severity: Severity::Error,
                        message: format!(
                            "{}-{} ({}) is required but missing or empty",
                            seg.segment_type, field_def.position, field_def.name
                        ),
                        segment_idx: Some(seg_idx),
                        segment_type: Some(seg.segment_type.clone()),
                        field_position: Some(field_def.position),
                        rule_id: format!("REQ-{}-{}", seg.segment_type, field_def.position),
                    });
                }
            }
        }
    }
}

/// Validate field lengths against HL7 table max_length.
fn validate_field_lengths(msg: &Hl7Message, issues: &mut Vec<ValidationIssue>) {
    let version = if msg.version.is_empty() { "2.5" } else { &msg.version };

    for (seg_idx, seg) in msg.segments.iter().enumerate() {
        for field in &seg.fields {
            if let Some(field_info) = tables::get_field_info(
                &seg.segment_type,
                field.position,
                version,
            ) {
                if let Some(max_len) = field_info.max_length {
                    let actual_len = field.span.len();
                    if actual_len > max_len {
                        issues.push(ValidationIssue {
                            severity: Severity::Warning,
                            message: format!(
                                "{}-{} ({}) exceeds max length: {} > {}",
                                seg.segment_type,
                                field.position,
                                field_info.name,
                                actual_len,
                                max_len
                            ),
                            segment_idx: Some(seg_idx),
                            segment_type: Some(seg.segment_type.clone()),
                            field_position: Some(field.position),
                            rule_id: format!("LEN-{}-{}", seg.segment_type, field.position),
                        });
                    }
                }
            }
        }
    }
}

/// Validate basic data type patterns.
fn validate_data_types(msg: &Hl7Message, issues: &mut Vec<ValidationIssue>) {
    let version = if msg.version.is_empty() { "2.5" } else { &msg.version };

    for (seg_idx, seg) in msg.segments.iter().enumerate() {
        for field in &seg.fields {
            let value = field.span.as_str(&msg.raw).trim();
            if value.is_empty() {
                continue;
            }

            if let Some(field_info) = tables::get_field_info(
                &seg.segment_type,
                field.position,
                version,
            ) {
                match field_info.data_type.as_str() {
                    "SI" => {
                        // Sequence ID must be numeric
                        if !value.chars().all(|c| c.is_ascii_digit()) {
                            issues.push(ValidationIssue {
                                severity: Severity::Warning,
                                message: format!(
                                    "{}-{} ({}) should be numeric (SI), found '{}'",
                                    seg.segment_type, field.position, field_info.name, value
                                ),
                                segment_idx: Some(seg_idx),
                                segment_type: Some(seg.segment_type.clone()),
                                field_position: Some(field.position),
                                rule_id: format!("TYPE-SI-{}-{}", seg.segment_type, field.position),
                            });
                        }
                    }
                    "DT" => {
                        // Date must be 8 digits (YYYYMMDD)
                        if value.len() != 8 || !value.chars().all(|c| c.is_ascii_digit()) {
                            issues.push(ValidationIssue {
                                severity: Severity::Info,
                                message: format!(
                                    "{}-{} ({}) date format should be YYYYMMDD, found '{}'",
                                    seg.segment_type, field.position, field_info.name, value
                                ),
                                segment_idx: Some(seg_idx),
                                segment_type: Some(seg.segment_type.clone()),
                                field_position: Some(field.position),
                                rule_id: format!("TYPE-DT-{}-{}", seg.segment_type, field.position),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::hl7::lexer::Hl7Lexer;

    fn parse_msg(text: &str) -> Hl7Message {
        let lexer = Hl7Lexer::new();
        lexer.parse(text.as_bytes().to_vec()).unwrap()
    }

    #[test]
    fn test_valid_message_minimal_issues() {
        let msg = parse_msg(
            "MSH|^~\\&|SendApp|SendFac|RecvApp|RecvFac|20230101120000||ADT^A01|MSG001|P|2.5\rPID|||12345||Doe^John||19800101|M"
        );
        let report = validate_hl7_message(&msg);
        assert_eq!(report.error_count, 0);
    }

    #[test]
    fn test_missing_msh9() {
        let msg = parse_msg("MSH|^~\\&|SendApp|SendFac|RecvApp|RecvFac|20230101120000");
        let report = validate_hl7_message(&msg);
        assert!(report.issues.iter().any(|i| i.rule_id == "MSH-001"));
    }

    #[test]
    fn test_missing_required_field() {
        // PID-3 (Patient Identifier List) and PID-5 (Patient Name) are required
        let msg = parse_msg(
            "MSH|^~\\&|A|B|C|D|20230101||ADT^A01|MSG001|P|2.5\rPID|1"
        );
        let report = validate_hl7_message(&msg);
        assert!(report.issues.iter().any(|i| i.rule_id.starts_with("REQ-PID")));
    }

    #[test]
    fn test_structural_first_segment_not_msh() {
        let msg = Hl7Message {
            raw: b"PID|||12345".to_vec(),
            delimiters: crate::parser::hl7::delimiters::Delimiters::default(),
            version: String::new(),
            message_type: String::new(),
            segments: vec![crate::parser::hl7::message::SegmentIndex {
                span: crate::parser::hl7::message::Span::new(0, 11),
                segment_type: "PID".into(),
                position: 0,
                fields: vec![],
            }],
        };
        let report = validate_hl7_message(&msg);
        assert!(report.issues.iter().any(|i| i.rule_id == "STRUCT-002"));
    }

    #[test]
    fn test_report_counts() {
        let msg = parse_msg(
            "MSH|^~\\&|A|B|C|D|20230101||ADT^A01|MSG001|P|2.5\rPID|1"
        );
        let report = validate_hl7_message(&msg);
        assert_eq!(
            report.error_count + report.warning_count + report.info_count,
            report.issues.len()
        );
    }
}
