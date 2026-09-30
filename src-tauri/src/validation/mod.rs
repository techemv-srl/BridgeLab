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

    // A missing MSH-10/MSH-12 is already an error from the catalogue's
    // required-field check: the MSH warning would report it a second time.
    let required = |rule: &str| issues.iter().any(|i| i.rule_id == rule);
    let (dup_10, dup_12) = (required("REQ-MSH-10"), required("REQ-MSH-12"));
    issues.retain(|i| !(dup_10 && i.rule_id == "MSH-003" || dup_12 && i.rule_id == "MSH-004"));

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

    // First segment must be MSH; a batch file opens with its FHS/BHS
    // envelope, and then the message header must follow.
    let first = &msg.segments[0].segment_type;
    let batch = first == "FHS" || first == "BHS";
    let has_msh = msg.segments.iter().any(|s| s.segment_type == "MSH");
    if (!batch && first != "MSH") || (batch && !has_msh) {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            message: if batch {
                format!("The batch opens with {} but holds no MSH message header", first)
            } else {
                format!("First segment must be MSH, found '{}'", first)
            },
            segment_idx: Some(0),
            segment_type: Some(first.clone()),
            field_position: None,
            rule_id: "STRUCT-002".into(),
        });
    }

    // A second MSH means several messages in one text: only the first
    // header is the one being validated.
    let first_msh = msg.segments.iter().position(|s| s.segment_type == "MSH").unwrap_or(0);
    for (i, seg) in msg.segments.iter().enumerate().skip(first_msh + 1) {
        if seg.segment_type == "MSH" {
            issues.push(ValidationIssue {
                severity: Severity::Warning,
                message: format!(
                    "Another MSH at segment {}: this text holds more than one message, and only the first is validated as a message",
                    i + 1
                ),
                segment_idx: Some(i),
                segment_type: Some("MSH".into()),
                field_position: None,
                rule_id: "STRUCT-004".into(),
            });
        }
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
    // The first MSH: in a batch file it follows FHS/BHS.
    let (msh_idx, msh) = match msg.segments.iter().enumerate().find(|(_, s)| s.segment_type == "MSH") {
        Some(found) => found,
        None => return,
    };

    // MSH must have at least 12 fields (MSH-1 through MSH-12)
    let max_pos = msh.fields.iter().map(|f| f.position).max().unwrap_or(0);
    if max_pos < 9 {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            message: "MSH segment is missing required fields (needs at least MSH-9 Message Type)".into(),
            segment_idx: Some(msh_idx),
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
                segment_idx: Some(msh_idx),
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
            segment_idx: Some(msh_idx),
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
            segment_idx: Some(msh_idx),
            segment_type: Some("MSH".into()),
            field_position: Some(12),
            rule_id: "MSH-004".into(),
        });
    } else if let Some(used) = tables::fallback_version_for(&msg.version) {
        // An MSH-12 BridgeLab has no catalogue for (a future v2.8, a typo)
        // is checked against another version's: say which one.
        issues.push(ValidationIssue {
            severity: Severity::Info,
            message: format!(
                "MSH-12 declares HL7 version '{}', which BridgeLab does not know: the message was validated against the v{} catalogue",
                msg.version.trim(),
                used
            ),
            segment_idx: Some(msh_idx),
            segment_type: Some("MSH".into()),
            field_position: Some(12),
            rule_id: "MSH-005".into(),
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
                    // HL7 lengths count characters, not UTF-8 bytes, and
                    // apply to each occurrence of a repeating field, not to
                    // all of them joined by '~': the longest one counts.
                    let actual_len = field
                        .repetitions
                        .iter()
                        .map(|r| r.span.as_str(&msg.raw).chars().count())
                        .max()
                        .unwrap_or(0);
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

/// Validate the primitive data types HL7 defines by format: SI and NM
/// (numbers), DT (dates), TS/DTM (timestamps) and TM (times), calendar
/// included. OBX-5 takes its type from OBX-2, as the standard says.
fn validate_data_types(msg: &Hl7Message, issues: &mut Vec<ValidationIssue>) {
    let version = if msg.version.is_empty() { "2.5" } else { &msg.version };

    for (seg_idx, seg) in msg.segments.iter().enumerate() {
        // OBX-5 is VARIES (ST in v2.7+ catalogues); OBX-2 names its real type.
        let obx_value_type = (seg.segment_type == "OBX")
            .then(|| {
                seg.fields
                    .iter()
                    .find(|f| f.position == 2)
                    .map(|f| first_component(f.span.as_str(&msg.raw), msg.delimiters.component).trim().to_ascii_uppercase())
            })
            .flatten();

        for field in &seg.fields {
            if field.span.as_str(&msg.raw).trim().is_empty() {
                continue;
            }
            let Some(field_info) = tables::get_field_info(&seg.segment_type, field.position, version) else {
                continue;
            };
            let data_type = match (&obx_value_type, field.position) {
                (Some(t), 5) => t.clone(),
                _ => field_info.data_type.clone(),
            };
            let Some(kind) = TypeKind::of(&data_type) else { continue };

            // Each repetition is checked on its own. Only a TS is composite
            // (DTM, then the degree of precision): the other types are
            // primitives, so a component separator in them is itself wrong.
            for rep in field.span.as_str(&msg.raw).split(msg.delimiters.repetition as char) {
                let value = if data_type == "TS" { first_component(rep, msg.delimiters.component) } else { rep }.trim();
                if value.is_empty() || value == "\"\"" {
                    continue;
                }
                if let Some(expected) = kind.check(value) {
                    issues.push(ValidationIssue {
                        severity: Severity::Warning,
                        message: format!(
                            "{}-{} ({}) is not a valid {} ({}), found '{}'",
                            seg.segment_type, field.position, field_info.name, data_type, expected, value
                        ),
                        segment_idx: Some(seg_idx),
                        segment_type: Some(seg.segment_type.clone()),
                        field_position: Some(field.position),
                        rule_id: format!("TYPE-{}-{}-{}", kind.rule(), seg.segment_type, field.position),
                    });
                }
            }
        }
    }
}

fn first_component(value: &str, component: u8) -> &str {
    value.split(component as char).next().unwrap_or("")
}

#[derive(Clone, Copy)]
enum TypeKind {
    Si,
    Nm,
    Dt,
    Dtm,
    Tm,
}

impl TypeKind {
    fn of(data_type: &str) -> Option<Self> {
        match data_type {
            "SI" => Some(Self::Si),
            "NM" => Some(Self::Nm),
            "DT" => Some(Self::Dt),
            "TS" | "DTM" => Some(Self::Dtm),
            "TM" => Some(Self::Tm),
            _ => None,
        }
    }

    fn rule(self) -> &'static str {
        match self {
            Self::Si => "SI",
            Self::Nm => "NM",
            Self::Dt => "DT",
            Self::Dtm => "DTM",
            Self::Tm => "TM",
        }
    }

    /// `None` when `value` is valid, else a description of the expected form.
    fn check(self, value: &str) -> Option<&'static str> {
        let ok = match self {
            Self::Si => !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()),
            Self::Nm => is_number(value),
            Self::Dt => is_date(value),
            Self::Dtm => is_timestamp(value),
            Self::Tm => is_time(value),
        };
        (!ok).then_some(match self {
            Self::Si => "a non-negative integer",
            Self::Nm => "a number such as 12, -3.5 or .5",
            Self::Dt => "YYYY[MM[DD]], a real calendar date",
            Self::Dtm => "YYYY[MM[DD[HH[MM[SS[.S]]]]]][+/-ZZZZ], a real date and time",
            Self::Tm => "HH[MM[SS[.S]]][+/-ZZZZ]",
        })
    }
}

fn is_number(v: &str) -> bool {
    let v = v.strip_prefix(['+', '-']).unwrap_or(v);
    let (int, frac) = v.split_once('.').unwrap_or((v, ""));
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    !(int.is_empty() && frac.is_empty()) && digits(int) && digits(frac) && !v.is_empty()
}

fn num(s: &str) -> u32 {
    s.parse().unwrap_or(u32::MAX)
}

fn is_date(v: &str) -> bool {
    if !matches!(v.len(), 4 | 6 | 8) || !v.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let year = num(&v[..4]);
    if v.len() >= 6 {
        let month = num(&v[4..6]);
        if !(1..=12).contains(&month) {
            return false;
        }
        if v.len() == 8 {
            let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
            let days = match month {
                2 if leap => 29,
                2 => 28,
                4 | 6 | 9 | 11 => 30,
                _ => 31,
            };
            if !(1..=days).contains(&num(&v[6..8])) {
                return false;
            }
        }
    }
    true
}

/// Split a trailing +/-ZZZZ offset off a DTM or TM value.
fn split_offset(v: &str) -> Option<(&str, Option<&str>)> {
    match v.rfind(['+', '-']) {
        Some(0) => None,
        Some(i) => {
            let tz = &v[i + 1..];
            let ok = tz.len() == 4 && tz.bytes().all(|b| b.is_ascii_digit()) && num(&tz[..2]) <= 14 && num(&tz[2..]) <= 59;
            ok.then_some((&v[..i], Some(tz)))
        }
        None => Some((v, None)),
    }
}

/// HH[MM[SS[.S{1,4}]]] against the clock.
fn is_clock(t: &str) -> bool {
    let (hms, frac) = match t.split_once('.') {
        Some((hms, frac)) => (hms, Some(frac)),
        None => (t, None),
    };
    if !matches!(hms.len(), 2 | 4 | 6) || !hms.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    if let Some(f) = frac {
        if hms.len() != 6 || f.is_empty() || f.len() > 4 || !f.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
    }
    num(&hms[..2]) <= 23
        && (hms.len() < 4 || num(&hms[2..4]) <= 59)
        && (hms.len() < 6 || num(&hms[4..6]) <= 59)
}

pub(crate) fn is_timestamp(v: &str) -> bool {
    let Some((main, _)) = split_offset(v) else { return false };
    // A timestamp is ASCII. Anything else (a name such as "Lindström" or
    // "李小龍" in a field checked as a date) is not one, and splitting it
    // at byte 8 could land inside a character.
    if !main.is_ascii() {
        return false;
    }
    let date_len = main.len().min(8);
    let (date, time) = main.split_at(date_len);
    is_date(date) && (time.is_empty() || (date.len() == 8 && is_clock(time)))
}

fn is_time(v: &str) -> bool {
    split_offset(v).is_some_and(|(t, _)| is_clock(t))
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

    fn type_issues(text: &str) -> Vec<String> {
        let msg = Hl7Lexer::new().parse(text.as_bytes().to_vec()).unwrap();
        validate_hl7_message(&msg)
            .issues
            .into_iter()
            .filter(|i| i.rule_id.starts_with("TYPE-") || i.rule_id == "STRUCT-004")
            .map(|i| i.rule_id)
            .collect()
    }

    #[test]
    fn primitive_formats() {
        for ok in ["2024", "202402", "20240229", "20000229"] { assert!(is_date(ok), "{ok}"); }
        for bad in ["19801399", "20230229", "19000229", "1980-01-01", "1980X101", "198001011"] { assert!(!is_date(bad), "{bad}"); }
        for ok in ["2024", "20240101", "202401011230", "20240101123045", "20240101123045.1234", "20240101123045+0100", "20240101123045.12-0500"] {
            assert!(is_timestamp(ok), "{ok}");
        }
        for bad in ["notadate", "2024-01-01 12:00", "20240101 1200", "2024010124", "202401011260", "20240101123045.12345", "202401011230.5", "20240101+01", "+0100", "20241301"] {
            assert!(!is_timestamp(bad), "{bad}");
        }
        for ok in ["12", "-3.5", "+4", ".5", "5.", "0"] { assert!(is_number(ok), "{ok}"); }
        for bad in ["abc", "1,5", "1.2.3", "-", ".", "1e3", ""] { assert!(!is_number(bad), "{bad}"); }
        for ok in ["08", "0830", "083015", "083015.5", "0830+0100"] { assert!(is_time(ok), "{ok}"); }
        for bad in ["24", "0860", "8:30", "0830.5"] { assert!(!is_time(bad), "{bad}"); }
    }

    #[test]
    fn timestamps_numbers_and_dates_are_checked_in_messages() {
        let bad = "MSH|^~\\&|A|B|C|D|2024-01-01 12:00||ADT^A01|1|P|2.5\rEVN|A01|notadate\rPID|1||1^^^H^MR||Rossi^Mario||19801399|F\rPV1|1|I\r";
        let issues = type_issues(bad);
        for rule in ["TYPE-DTM-MSH-7", "TYPE-DTM-EVN-2", "TYPE-DTM-PID-7"] {
            assert!(issues.iter().any(|r| r == rule), "{rule} missing from {issues:?}");
        }
        let good = "MSH|^~\\&|A|B|C|D|20240101120000+0100||ADT^A01|1|P|2.5\rEVN|A01|20240101120000\rPID|1||1^^^H^MR||Rossi^Mario||19800101|F\rPV1|1|I\r";
        assert!(type_issues(good).is_empty(), "{:?}", type_issues(good));
        // v2.7 declares DTM directly.
        let v27 = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.7\rPID|1||1^^^H^MR||Rossi^Mario||1980011|F\r";
        assert!(type_issues(v27).iter().any(|r| r == "TYPE-DTM-PID-7"));
    }

    #[test]
    fn obx5_follows_obx2() {
        let msg = |t: &str, v: &str| format!("MSH|^~\\&|A|B|C|D|20240101||ORU^R01|1|P|2.5\rOBX|1|{t}|X^Y||{v}||||||F\r");
        assert!(type_issues(&msg("NM", "abc")).iter().any(|r| r == "TYPE-NM-OBX-5"));
        assert!(type_issues(&msg("NM", "6.2")).is_empty());
        assert!(type_issues(&msg("NM", "6.2~7")).is_empty(), "repetitions checked one by one");
        assert!(type_issues(&msg("DT", "20241301")).iter().any(|r| r == "TYPE-DT-OBX-5"));
        assert!(type_issues(&msg("ST", "anything")).is_empty());
        assert!(type_issues(&msg("NM", "6.2^junk")).iter().any(|r| r == "TYPE-NM-OBX-5"), "a primitive has no components");
        assert!(type_issues(&msg("TS", "20240101^S")).is_empty(), "TS precision component");
        assert!(type_issues(&msg("NM", "\"\"")).is_empty(), "explicit null");
    }

    #[test]
    fn a_second_msh_is_reported() {
        let two = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||1\rMSH|^~\\&|A|B|C|D|20240101||ADT^A01|2|P|2.5\r";
        assert!(type_issues(two).iter().any(|r| r == "STRUCT-004"));
    }

    fn rule_ids(text: &str) -> Vec<String> {
        let msg = Hl7Lexer::new().parse(text.as_bytes().to_vec()).unwrap();
        validate_hl7_message(&msg).issues.into_iter().map(|i| i.rule_id).collect()
    }

    #[test]
    fn field_length_counts_characters() {
        let name = "À".repeat(130);
        let text = format!("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||1^^^H^MR||{}^M||19800101|M\r", name);
        // 132 characters (262 bytes) against the v2.5 limit of 250.
        assert!(!rule_ids(&text).iter().any(|r| r == "LEN-PID-5"));
        let long = format!("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||1^^^H^MR||{}^M||19800101|M\r", "À".repeat(260));
        assert!(rule_ids(&long).iter().any(|r| r == "LEN-PID-5"));
    }

    #[test]
    fn unknown_version_names_the_fallback() {
        let msg = |v: &str| format!("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|{v}\rPID|1||1^^^H^MR||Rossi^Mario||19800101|M\r");
        let text = msg("2.8");
        let parsed = Hl7Lexer::new().parse(text.as_bytes().to_vec()).unwrap();
        let report = validate_hl7_message(&parsed);
        let note = report.issues.iter().find(|i| i.rule_id == "MSH-005").expect("fallback note");
        assert_eq!(note.severity, Severity::Info);
        assert!(note.message.contains("v2.5"), "{}", note.message);
        assert!(!rule_ids(&msg("2.5")).iter().any(|r| r == "MSH-005"));
        assert!(!rule_ids(&msg("2.5.1")).iter().any(|r| r == "MSH-005"));
        assert!(rule_ids(&msg("9.9")).iter().any(|r| r == "MSH-005"));
    }

    #[test]
    fn missing_msh12_is_reported_once() {
        let ids = rule_ids("MSH|^~\\&|A|B|C|D|20240101||ADT^A01||P|\rPID|1||1^^^H^MR||Rossi^Mario\r");
        let has = |r: &str| ids.iter().any(|x| x == r);
        assert!(has("REQ-MSH-12") != has("MSH-004"), "{ids:?}");
        assert!(has("REQ-MSH-10") != has("MSH-003"), "{ids:?}");
    }

    #[test]
    fn field_length_applies_to_each_repetition() {
        // 20 identifiers of 16 characters: 339 joined, none over 250.
        let reps = (0..20).map(|i| format!("1000{:02}^^^HOSP^MR", i)).collect::<Vec<_>>().join("~");
        let text = format!("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||{}||Rossi^Mario||19800101|M\r", reps);
        assert!(!rule_ids(&text).iter().any(|r| r == "LEN-PID-3"), "{:?}", rule_ids(&text));
        let one_long = format!("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||1~{}||Rossi^Mario||19800101|M\r", "9".repeat(251));
        assert!(rule_ids(&one_long).iter().any(|r| r == "LEN-PID-3"));
    }

    #[test]
    fn longer_segment_ids_are_reported() {
        let ids = rule_ids("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPIDX|1||1^^^H^MR||Doe^J\rOBX1|1|NM|X||5||||||F\r");
        assert_eq!(ids.iter().filter(|r| *r == "STRUCT-003").count(), 2, "{ids:?}");
    }

    #[test]
    fn a_batch_envelope_is_not_a_structure_error() {
        let batch = "FHS|^~\\&|A\rBHS|^~\\&|A\rMSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||1^^^H^MR||Rossi^Mario||19800101|M\rBTS|1\rFTS|1\r";
        let ids = rule_ids(batch);
        assert!(!ids.iter().any(|r| r == "STRUCT-002" || r.starts_with("MSH-")), "{ids:?}");
        let no_msh = rule_ids("FHS|^~\\&|A\rBHS|^~\\&|A\rBTS|0\r");
        assert!(no_msh.iter().any(|r| r == "STRUCT-002"), "{no_msh:?}");
    }

    /// Values whose byte 8 falls inside a multi-byte character: none is a
    /// timestamp, and checking one must not split the character.
    const MULTIBYTE_AT_BYTE_8: [&str; 9] =
        ["李小龍", "田中太郎", "ヤマダ", "김민준", "Lindström", "Bergström", "Østergård", "Västerås", "1990051年"];

    #[test]
    fn timestamps_with_multibyte_characters_are_rejected_without_a_panic() {
        for v in MULTIBYTE_AT_BYTE_8 {
            assert!(!is_timestamp(v), "{v}");
            assert!(!is_timestamp(&format!("{v}+0100")), "{v}");
            assert!(!is_date(v) && !is_time(v), "{v}");
        }
        assert!(is_timestamp("19900515"));
    }

    #[test]
    fn multibyte_values_in_date_fields_are_reported_not_fatal() {
        for v in MULTIBYTE_AT_BYTE_8 {
            let text = format!(
                "MSH|^~\\&|A|B|C|D|20240101120000||ADT^A01|MSG1|P|2.5\rPID|1||12345^^^FAC^MR||{v}^Jan||{v}|M|||Via Roma^^{v}\r"
            );
            let ids = rule_ids(&text);
            assert!(ids.iter().any(|r| r.starts_with("TYPE-")), "{v}: {ids:?}");
        }
    }
}
