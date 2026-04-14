use serde::Serialize;
use crate::parser::Hl7Message;

#[derive(Debug, Clone, Serialize)]
pub struct ValidationIssue {
    pub severity: String,
    pub rule_id: String,
    pub segment_idx: Option<usize>,
    pub segment_type: Option<String>,
    pub field_position: Option<usize>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub issues: Vec<ValidationIssue>,
}

pub fn validate(msg: &Hl7Message) -> ValidationReport {
    let mut issues = Vec::new();

    if msg.segments.is_empty() {
        issues.push(ValidationIssue {
            severity: "error".into(), rule_id: "STRUCT-001".into(),
            segment_idx: None, segment_type: None, field_position: None,
            message: "Message has no segments".into(),
        });
    } else if msg.segments[0].segment_type != "MSH" {
        issues.push(ValidationIssue {
            severity: "error".into(), rule_id: "STRUCT-002".into(),
            segment_idx: Some(0), segment_type: Some(msg.segments[0].segment_type.clone()),
            field_position: None,
            message: format!("First segment must be MSH, found '{}'", msg.segments[0].segment_type),
        });
    }

    // Validate segment types
    for (i, seg) in msg.segments.iter().enumerate() {
        if seg.segment_type.len() != 3 ||
           !seg.segment_type.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
            issues.push(ValidationIssue {
                severity: "warning".into(), rule_id: "STRUCT-003".into(),
                segment_idx: Some(i), segment_type: Some(seg.segment_type.clone()),
                field_position: None,
                message: format!("Invalid segment type '{}'", seg.segment_type),
            });
        }
    }

    // MSH checks
    if let Some(msh) = msg.segments.iter().find(|s| s.segment_type == "MSH") {
        let has_f9 = msh.fields.iter().any(|f| f.position == 9 && !f.span.as_str(&msg.raw).trim().is_empty());
        if !has_f9 {
            issues.push(ValidationIssue {
                severity: "error".into(), rule_id: "MSH-002".into(),
                segment_idx: Some(0), segment_type: Some("MSH".into()), field_position: Some(9),
                message: "MSH-9 (Message Type) is required".into(),
            });
        }
        let has_f10 = msh.fields.iter().any(|f| f.position == 10 && !f.span.as_str(&msg.raw).trim().is_empty());
        if !has_f10 {
            issues.push(ValidationIssue {
                severity: "warning".into(), rule_id: "MSH-003".into(),
                segment_idx: Some(0), segment_type: Some("MSH".into()), field_position: Some(10),
                message: "MSH-10 (Message Control ID) should be present".into(),
            });
        }
        if msg.version.is_empty() {
            issues.push(ValidationIssue {
                severity: "warning".into(), rule_id: "MSH-004".into(),
                segment_idx: Some(0), segment_type: Some("MSH".into()), field_position: Some(12),
                message: "MSH-12 (Version ID) is missing".into(),
            });
        }
    }

    let error_count = issues.iter().filter(|i| i.severity == "error").count();
    let warning_count = issues.iter().filter(|i| i.severity == "warning").count();
    let info_count = issues.iter().filter(|i| i.severity == "info").count();

    ValidationReport {
        valid: error_count == 0,
        error_count, warning_count, info_count,
        issues,
    }
}
