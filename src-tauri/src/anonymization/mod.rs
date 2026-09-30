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

/// How a catalogue field is masked.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    /// Every component, subcomponent and repetition is masked.
    Plain,
    /// An extended identifier (CX): the assigning authority and the
    /// identifier type code (components 4 and 5, e.g. `HOSP` and `MR`) are
    /// not about the patient and stay, so the masked value keeps its
    /// meaning and fits the field's length in every version; every other
    /// component is masked.
    Id,
}

use PhiSensitivity::{High, Medium};
use Shape::{Id, Plain};

/// Known PHI fields by segment type (segment, field position, name,
/// sensitivity, shape), after the HL7 v2.5 field definitions: every field of
/// these segments that names, locates, dates or identifies the patient or
/// a related person (next of kin, guarantor, insured, contacts, employer).
/// OBX-5 is added when the observation is free text (see
/// [`free_text_obx`]).
const PHI_FIELDS: &[(&str, usize, &str, PhiSensitivity, Shape)] = &[
    // PID - Patient Identification
    ("PID", 2, "Patient ID (external)", High, Id),
    ("PID", 3, "Patient Identifier List", High, Id),
    ("PID", 4, "Alternate Patient ID", High, Id),
    ("PID", 5, "Patient Name", High, Plain),
    ("PID", 6, "Mother's Maiden Name", Medium, Plain),
    ("PID", 7, "Date/Time of Birth", High, Plain),
    ("PID", 9, "Patient Alias", Medium, Plain),
    ("PID", 11, "Patient Address", High, Plain),
    ("PID", 13, "Phone Number - Home", High, Plain),
    ("PID", 14, "Phone Number - Business", Medium, Plain),
    ("PID", 18, "Patient Account Number", High, Id),
    ("PID", 19, "SSN Number", High, Plain),
    ("PID", 20, "Driver's License Number", High, Plain),
    ("PID", 21, "Mother's Identifier", High, Id),
    ("PID", 23, "Birth Place", High, Plain),
    ("PID", 29, "Patient Death Date and Time", High, Plain),
    // PV1 - the visit numbers identify the patient's stay.
    ("PV1", 5, "Preadmit Number", High, Id),
    ("PV1", 19, "Visit Number", High, Id),
    ("PV1", 50, "Alternate Visit ID", High, Id),
    // MRG - Merge Patient Information (ADT^A34/A40): the patient's previous
    // identifiers and name.
    ("MRG", 1, "Prior Patient Identifier List", High, Id),
    ("MRG", 2, "Prior Alternate Patient ID", High, Id),
    ("MRG", 3, "Prior Patient Account Number", High, Id),
    ("MRG", 4, "Prior Patient ID", High, Id),
    ("MRG", 5, "Prior Visit Number", High, Id),
    ("MRG", 6, "Prior Alternate Visit ID", High, Id),
    ("MRG", 7, "Prior Patient Name", High, Plain),
    // NTE - free-text comments, where names and phone numbers turn up.
    ("NTE", 3, "Comment", High, Plain),
    // NK1 - Next of Kin / Associated Parties
    ("NK1", 2, "Name", Medium, Plain),
    ("NK1", 4, "Address", Medium, Plain),
    ("NK1", 5, "Phone Number", Medium, Plain),
    ("NK1", 6, "Business Phone Number", Medium, Plain),
    ("NK1", 12, "Employee Number", High, Id),
    ("NK1", 13, "Organization Name", Medium, Plain),
    ("NK1", 16, "Date/Time of Birth", High, Plain),
    ("NK1", 26, "Mother's Maiden Name", Medium, Plain),
    ("NK1", 30, "Contact Person's Name", Medium, Plain),
    ("NK1", 31, "Contact Person's Telephone Number", Medium, Plain),
    ("NK1", 32, "Contact Person's Address", Medium, Plain),
    ("NK1", 33, "Associated Party's Identifiers", High, Id),
    ("NK1", 37, "Contact Person SSN", High, Plain),
    ("NK1", 38, "Birth Place", High, Plain),
    // GT1 - Guarantor
    ("GT1", 2, "Guarantor Number", High, Id),
    ("GT1", 3, "Guarantor Name", Medium, Plain),
    ("GT1", 4, "Guarantor Spouse Name", Medium, Plain),
    ("GT1", 5, "Guarantor Address", Medium, Plain),
    ("GT1", 6, "Guarantor Phone - Home", Medium, Plain),
    ("GT1", 7, "Guarantor Phone - Business", Medium, Plain),
    ("GT1", 8, "Guarantor Date/Time of Birth", High, Plain),
    ("GT1", 12, "Guarantor SSN", High, Plain),
    ("GT1", 16, "Guarantor Employer Name", Medium, Plain),
    ("GT1", 17, "Guarantor Employer Address", Medium, Plain),
    ("GT1", 18, "Guarantor Employer Phone", Medium, Plain),
    ("GT1", 19, "Guarantor Employee ID", High, Id),
    ("GT1", 21, "Guarantor Organization Name", Medium, Plain),
    ("GT1", 24, "Guarantor Death Date and Time", High, Plain),
    ("GT1", 29, "Guarantor Employer ID Number", High, Id),
    ("GT1", 42, "Guarantor Mother's Maiden Name", Medium, Plain),
    ("GT1", 45, "Guarantor Contact Person's Name", Medium, Plain),
    ("GT1", 46, "Guarantor Contact Person's Telephone", Medium, Plain),
    ("GT1", 51, "Guarantor Employer's Organization Name", Medium, Plain),
    ("GT1", 56, "Guarantor Birth Place", High, Plain),
    // IN1 - Insurance
    ("IN1", 10, "Insured's Group Employer ID", High, Id),
    ("IN1", 11, "Insured's Group Employer Name", Medium, Plain),
    ("IN1", 16, "Name of Insured", Medium, Plain),
    ("IN1", 18, "Insured's Date of Birth", High, Plain),
    ("IN1", 19, "Insured's Address", High, Plain),
    ("IN1", 36, "Policy Number", High, Plain),
    ("IN1", 44, "Insured's Employer's Address", Medium, Plain),
    ("IN1", 49, "Insured's ID Number", High, Id),
    ("IN1", 52, "Insured's Birth Place", High, Plain),
    // IN2 - Insurance Additional Information
    ("IN2", 1, "Insured's Employee ID", High, Id),
    ("IN2", 2, "Insured's SSN", High, Plain),
    ("IN2", 3, "Insured's Employer's Name and ID", Medium, Plain),
    ("IN2", 6, "Medicare Health Insurance Card Number", High, Plain),
    ("IN2", 7, "Medicaid Case Name", Medium, Plain),
    ("IN2", 8, "Medicaid Case Number", High, Plain),
    ("IN2", 9, "Military Sponsor Name", Medium, Plain),
    ("IN2", 10, "Military ID Number", High, Plain),
    ("IN2", 26, "Payor Subscriber ID", High, Id),
    ("IN2", 40, "Mother's Maiden Name", Medium, Plain),
    ("IN2", 49, "Employer Contact Person Name", Medium, Plain),
    ("IN2", 50, "Employer Contact Person Phone", Medium, Plain),
    ("IN2", 52, "Insured's Contact Person's Name", Medium, Plain),
    ("IN2", 53, "Insured's Contact Person Phone", Medium, Plain),
    ("IN2", 61, "Patient Member Number", High, Id),
    ("IN2", 63, "Insured's Phone Number - Home", Medium, Plain),
    ("IN2", 64, "Insured's Employer Phone Number", Medium, Plain),
    ("IN2", 69, "Insured Organization Name and ID", Medium, Plain),
    ("IN2", 70, "Insured Employer Organization Name and ID", Medium, Plain),
];

/// Number of fields in the built-in catalogue (free-text OBX-5 aside).
pub fn builtin_field_count() -> usize {
    PHI_FIELDS.len()
}

/// OBX value types that are free text, where report writers put names,
/// dates and phone numbers.
const FREE_TEXT_TYPES: &[&str] = &["TX", "FT"];

/// Whether this OBX carries a free-text value (OBX-2 is TX or FT), whose
/// OBX-5 is then masked like a comment. Coded and numeric results stay.
fn free_text_obx(seg: &crate::parser::hl7::message::SegmentIndex, raw: &[u8]) -> bool {
    seg.segment_type == "OBX"
        && seg.fields.iter().find(|f| f.position == 2).is_some_and(|f| {
            let v = f.span.as_str(raw).trim();
            FREE_TEXT_TYPES.iter().any(|t| v.eq_ignore_ascii_case(t))
        })
}

/// The built-in catalogue entries that apply to one segment, OBX-5 included
/// when it is free text.
fn builtin_targets(
    seg: &crate::parser::hl7::message::SegmentIndex,
    raw: &[u8],
    extra: &[ExtraPhiField],
) -> Vec<(usize, &'static str, PhiSensitivity, Shape)> {
    let mut out: Vec<_> = PHI_FIELDS
        .iter()
        .filter(|f| f.0 == seg.segment_type)
        .map(|f| (f.1, f.2, f.3.clone(), f.4))
        .collect();
    // A plugin rule on OBX-5 covers every OBX; the built-in one would mask
    // the field twice.
    if free_text_obx(seg, raw) && !extra.iter().any(|r| r.segment == "OBX" && r.field == 5) {
        out.push((5, "Observation Value (free text)", High, Plain));
    }
    out
}

/// A runtime-defined PHI field (e.g. from a plugin). Built-in fields live in
/// the `PHI_FIELDS` constant, plugins add to this list.
#[derive(Debug, Clone)]
pub struct ExtraPhiField {
    pub segment: String,
    pub field: usize,
    pub name: String,
    pub sensitivity: PhiSensitivity,
}

/// Detect PHI fields in an HL7 message using the built-in rules only.
pub fn detect_phi(msg: &Hl7Message) -> Vec<PhiLocation> {
    detect_phi_with_extra(msg, &[])
}

/// Detect PHI fields, merging the built-in catalogue with extra plugin rules.
pub fn detect_phi_with_extra(msg: &Hl7Message, extra: &[ExtraPhiField]) -> Vec<PhiLocation> {
    let extra = plugin_fields(extra);
    let mut locations = Vec::new();

    // Built-in PHI fields
    for (seg_idx, seg) in msg.segments.iter().enumerate() {
        for (field_pos, field_name, sensitivity, _) in builtin_targets(seg, &msg.raw, &extra) {
            if let Some(field) = seg.fields.iter().find(|f| f.position == field_pos) {
                let value = field.span.as_str(&msg.raw).trim();
                if !value.is_empty() {
                    locations.push(PhiLocation {
                        segment_idx: seg_idx,
                        segment_type: seg.segment_type.clone(),
                        field_position: field_pos,
                        field_name: field_name.to_string(),
                        sensitivity,
                        current_value: value.chars().take(50).collect(),
                    });
                }
            }
        }

        // Plugin-contributed PHI fields
        for rule in &extra {
            if seg.segment_type != rule.segment { continue; }
            if let Some(field) = seg.fields.iter().find(|f| f.position == rule.field) {
                let value = field.span.as_str(&msg.raw).trim();
                if !value.is_empty() {
                    locations.push(PhiLocation {
                        segment_idx: seg_idx,
                        segment_type: rule.segment.clone(),
                        field_position: rule.field,
                        field_name: if rule.name.is_empty() {
                            format!("{}-{}", rule.segment, rule.field)
                        } else {
                            rule.name.clone()
                        },
                        sensitivity: rule.sensitivity.clone(),
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
    anonymize_message_with_extra(msg, &[])
}

/// Anonymize an HL7 message, respecting both the built-in PHI catalogue and
/// any extra plugin-contributed fields.
pub fn anonymize_message_with_extra(msg: &Hl7Message, extra: &[ExtraPhiField]) -> String {
    anonymize_by_replacement(msg, extra)
}

/// Plugin PHI fields the anonymizer can act on, each segment and field
/// once: fields the built-in catalogue already covers, duplicates across
/// packs (the strongest sensitivity wins, whatever order the packs loaded
/// in), and fields that are not data (field 0 is the segment name, MSH-1
/// and MSH-2 the delimiters) are dropped.
fn plugin_fields(extra: &[ExtraPhiField]) -> Vec<ExtraPhiField> {
    let strength = |s: &PhiSensitivity| match s {
        PhiSensitivity::High => 2,
        PhiSensitivity::Medium => 1,
        PhiSensitivity::Low => 0,
    };
    let mut out: Vec<ExtraPhiField> = Vec::new();
    for rule in extra {
        if rule.field == 0 || (rule.segment == "MSH" && rule.field <= 2) { continue; }
        if PHI_FIELDS.iter().any(|f| f.0 == rule.segment.as_str() && f.1 == rule.field) { continue; }
        match out.iter_mut().find(|o| o.segment == rule.segment && o.field == rule.field) {
            Some(o) if strength(&rule.sensitivity) > strength(&o.sensitivity) => *o = rule.clone(),
            Some(_) => {}
            None => out.push(rule.clone()),
        }
    }
    out
}

/// Anonymize by building a new message with PHI fields replaced.
fn anonymize_by_replacement(msg: &Hl7Message, extra: &[ExtraPhiField]) -> String {
    let d = &msg.delimiters;
    let sep = d.field as char;
    let extra = plugin_fields(extra);
    let mut result_segments: Vec<String> = Vec::new();

    for seg in msg.segments.iter() {
        let seg_text = seg.span.as_str(&msg.raw);
        let mut fields: Vec<String> = seg_text.split(sep).map(|s| s.to_string()).collect();
        // Split on the field separator, MSH-n sits at index n - 1 (the
        // separator itself is MSH-1); in every other segment at index n.
        let index = |field: usize| if seg.segment_type == "MSH" { field - 1 } else { field };

        let targets = builtin_targets(seg, &msg.raw, &extra)
            .into_iter()
            .map(|(field, _, sensitivity, shape)| (field, sensitivity, shape))
            .chain(extra.iter().filter(|r| seg.segment_type == r.segment).map(|r| (r.field, r.sensitivity.clone(), Plain)));
        for (field, sensitivity, shape) in targets {
            let idx = index(field);
            if idx < fields.len() && !fields[idx].trim().is_empty() {
                fields[idx] = generate_replacement(&fields[idx], d, &sensitivity, shape);
            }
        }

        result_segments.push(fields.join(&sep.to_string()));
    }

    result_segments.join("\r")
}

/// Generate a replacement value that keeps the field's shape: every
/// repetition, component and subcomponent is masked on its own, with the
/// message's own delimiters, so the output parses the same way.
fn generate_replacement(original: &str, d: &crate::parser::hl7::delimiters::Delimiters, sensitivity: &PhiSensitivity, shape: Shape) -> String {
    let mask_split = |text: &str, sep: u8, inner: &dyn Fn(usize, &str) -> String| -> String {
        text.split(sep as char).enumerate().map(|(i, part)| inner(i, part)).collect::<Vec<_>>().join(&(sep as char).to_string())
    };
    let leaf = |_: usize, v: &str| if v.is_empty() { String::new() } else { mask_value(v, sensitivity) };
    let sub = |_: usize, c: &str| mask_split(c, d.subcomponent, &leaf);
    // CX.4 (assigning authority) and CX.5 (identifier type code) say whose
    // number it is, not whose patient: they stay.
    let comp = |_: usize, r: &str| {
        mask_split(r, d.component, &|i, c| if shape == Id && (i == 3 || i == 4) { c.to_string() } else { sub(i, c) })
    };
    mask_split(original, d.repetition, &comp)
}

/// The masked form of an HL7 date or timestamp, or `None` when `value`
/// is not one. Any value the validator accepts as DT/DTM/TS, from YYYYMM
/// to `YYYYMMDDHHMMSS.SSSS+ZZZZ`, becomes 1900-01-01 00:00:00 with the same
/// shape: as many date/time digits, as many fractional digits, and an
/// offset of +0000 when it had one. So a masked birth or death date still
/// validates (zeros, or REDACTED for a value with a `.` or an offset,
/// failed the DT/TS check). A bare YYYY is left to the numeric rule.
fn masked_timestamp(value: &str) -> Option<String> {
    if !crate::validation::is_timestamp(value) {
        return None;
    }
    let (main, offset) = match value.rfind(['+', '-']) {
        Some(i) if i > 0 => (&value[..i], true),
        _ => (value, false),
    };
    let (digits, frac) = match main.split_once('.') {
        Some((d, f)) => (d, Some(f)),
        None => (main, None),
    };
    if digits.len() < 6 {
        return None;
    }
    let mut out = "19000101000000"[..digits.len()].to_string();
    if let Some(f) = frac {
        out.push('.');
        out.push_str(&"0".repeat(f.len()));
    }
    if offset {
        out.push_str("+0000");
    }
    Some(out)
}

/// Mask a single value based on sensitivity.
fn mask_value(value: &str, sensitivity: &PhiSensitivity) -> String {
    if value.is_empty() {
        return String::new();
    }

    // A date or timestamp becomes a fixed, valid one of the same shape at
    // every level: "1***" or zeros would fail the DT/TS check, and the
    // output must still validate.
    if let Some(masked) = masked_timestamp(value) {
        return masked;
    }

    match sensitivity {
        PhiSensitivity::High => {
            if value.chars().all(|c| c.is_ascii_digit()) {
                // Numeric: replace with zeros of the same length
                "0".repeat(value.len())
            } else {
                // Text: replace with REDACTED
                "REDACTED".to_string()
            }
        }
        PhiSensitivity::Medium => {
            // Partial masking: keep first char, mask rest (characters,
            // not bytes: "Ü" is one character)
            if value.chars().count() <= 1 {
                "*".to_string()
            } else {
                let first: String = value.chars().take(1).collect();
                format!("{}***", first)
            }
        }
        PhiSensitivity::Low => {
            // Keep first 3 chars
            let prefix: String = value.chars().take(3).collect();
            if value.chars().count() > 3 {
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
    fn masked_samples_still_validate_without_type_warnings() {
        // The anonymizer's output must pass the DT/TS checks: a birth date
        // masked to 00000000 used to fail them.
        for s in crate::samples::all() {
            let msg = parse(&s.content);
            let anon = anonymize_message(&msg);
            let report = crate::validation::validate_hl7_message(&parse(&anon));
            let types: Vec<_> = report.issues.iter().filter(|i| i.rule_id.starts_with("TYPE-")).map(|i| i.message.clone()).collect();
            assert!(types.is_empty(), "{}: {:?}", s.id, types);
        }
    }

    #[test]
    fn dates_are_masked_to_a_valid_date_of_the_same_precision() {
        assert_eq!(masked_timestamp("19800515").as_deref(), Some("19000101"));
        assert_eq!(masked_timestamp("202401011230").as_deref(), Some("190001010000"));
        assert_eq!(masked_timestamp("12345678"), None, "month 56: an identifier, not a date");
        assert_eq!(masked_timestamp("123"), None);
        let msg = parse("MSH|^~\\&|A|B|C|D|20230101||ADT^A01|M1|P|2.5\rPID|||12345678||Doe^John||19800515|M");
        let anon = anonymize_message(&msg);
        assert!(anon.contains("||19000101|"), "{anon}");
        assert!(anon.contains("|00000000|"), "{anon}");
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

    fn extra(segment: &str, field: usize, sensitivity: PhiSensitivity) -> ExtraPhiField {
        ExtraPhiField { segment: segment.into(), field, name: String::new(), sensitivity }
    }

    #[test]
    fn a_plugin_field_on_msh_masks_that_field_and_field_zero_is_ignored() {
        let msg = parse("MSH|^~\\&|APP|FACILITY01|RCV|RFAC|20260101120000||ADT^A01|CTRL1|P|2.5\rEVN|A01|20260101120000\rPID|1||123||doe^john");
        let rules = [extra("MSH", 4, PhiSensitivity::High), extra("EVN", 0, PhiSensitivity::High), extra("MSH", 2, PhiSensitivity::High)];
        let out = anonymize_message_with_extra(&msg, &rules);
        assert!(out.starts_with("MSH|^~\\&|APP|REDACTED|RCV|RFAC|"), "{out}");
        assert!(out.contains("\rEVN|A01|"), "{out}");
        let phi = detect_phi_with_extra(&msg, &rules);
        assert_eq!(phi.iter().filter(|p| p.segment_type != "PID").count(), 1);
    }

    #[test]
    fn repetitions_and_the_message_delimiters_survive_masking() {
        let msg = parse("MSH|^~\\&|S|F|R|F|20240101||ORU^R01|MA|P|2.5\rPID|1||111^^^H^MR~222^^^H^PI||DOE^JOHN||19800101|M");
        let out = anonymize_message(&msg);
        assert!(out.contains("|000^^^H^MR~000^^^H^PI|"), "{out}");
        let dollar = parse("MSH|$~\\&|S|F|R|F|20240101||ADT$A01|MC|P|2.5\rPID|1||111$$$H$MR||DOE$JOHN||19800101|M\rNK1|1|SMITH$ANNA|SPO");
        let out = anonymize_message(&dollar);
        assert!(out.contains("|000$$$H$MR||REDACTED$REDACTED|"), "{out}");
        assert!(out.contains("NK1|1|S***$A***|SPO"), "{out}");
    }

    #[test]
    fn a_field_named_by_two_packs_is_masked_once_at_the_stronger_level() {
        let msg = parse("MSH|^~\\&|S|F|R|F|20240101||ADT^A01|M|P|2.5\rZPI|1|X|SECRET");
        for order in [[PhiSensitivity::Low, PhiSensitivity::Medium], [PhiSensitivity::Medium, PhiSensitivity::Low]] {
            let rules: Vec<_> = order.iter().map(|s| extra("ZPI", 3, s.clone())).collect();
            let out = anonymize_message_with_extra(&msg, &rules);
            assert!(out.ends_with("ZPI|1|X|S***"), "{out}");
            assert_eq!(detect_phi_with_extra(&msg, &rules).len(), 1);
        }
    }

    #[test]
    fn long_numbers_keep_their_length() {
        assert_eq!(mask_value("123456789012345678901", &PhiSensitivity::High), "0".repeat(21));
    }

    #[test]
    fn merge_identifiers_and_free_text_comments_are_masked() {
        let msg = parse("MSH|^~\\&|A|B|C|D|20240101||ADT^A40|1|P|2.5\rEVN|A40|20240101\rPID|1|EXT999|123456^^^HOSP^MR||Rossi^Mario||19800101|M\rMRG|654321^^^HOSP^MR||||||Rossi^Mariolino\rNTE|1||Patient Mario Rossi called from 0612345678");
        let out = anonymize_message(&msg);
        assert!(!out.contains("EXT999") && !out.contains("654321") && !out.contains("Mariolino") && !out.contains("0612345678"), "{out}");
        assert!(out.contains("\rMRG|000000^^^HOSP^MR||||||REDACTED^REDACTED"), "{out}");
    }

    /// Every PHI-bearing field of the covered segments, filled in: none of
    /// the values may survive, and the dialog must list each field.
    const COMPACT: &str = "MSH|^~\\&|ADT|HOSP|LAB|HOSP|20260101120000||ADT^A01|C1|P|2.5\r\
PID|1|EXT77|MRN4411^^^HOSP^MR||Nowak^Anna||19801231|F|Nowakowa^Ania||1 Main St^^Salem^OR^97301||(503)555-0101|(503)555-0102||||ACC9001|987-65-4320|OR1234567|MOM881^^^HOSP^MR||Salem General||||||20250101\r\
NK1|1|Kowalczyk^Jan|SPO|9 Oak Ave^^Salem^OR^97302|(541)555-0233|(541)555-0888||||||EMP555|Portland General|||19590202||||||||||Kowalczyk^Marta||||Contact^Carl|(541)555-0999|7 Elm Rd^^Salem^OR|NK1ID^^^HOSP||||111-22-3333\r\
PV1|1|I|W^1^1||||||||||||||||VISIT42\r\
GT1|1|G7788|Guarantor^Gus|Spouse^Sue|3 Pine^^Salem^OR|(541)555-1001|(541)555-1002|19550505|M|||222-33-4444||||Guarantor Employer Inc|4 Birch^^Salem|(541)555-1003|GEMP1\r\
IN1|1|PLAN1|INS1|Acme Insurance||||||||||||Kowalczyk^Bo|SEL|19670423|742 Evergreen Terrace^^Springfield^OR^97403|||||||||||||||||POL123||||||||Evergreen Employer^^Springfield|||||INSID999\r\
IN2|IN2EMP|123-45-6789|||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||(541)555-7777\r\
MRG|OLD1^^^HOSP^MR||OLDACC|||OLDVISIT|Nowak^Anne\r\
OBX|1|TX|NOTE^Note||Seen with daughter Kowalczyk, call 5415550199||||||F\r\
OBX|2|NM|GLU^Glucose||5.4|mmol/L|||||F";

    #[test]
    fn every_phi_value_of_the_covered_segments_is_masked() {
        let msg = parse(COMPACT);
        let out = anonymize_message(&msg);
        for value in [
            "EXT77", "MRN4411", "Nowak", "Anna", "19801231", "Nowakowa", "Main St", "(503)555-0101", "(503)555-0102",
            "ACC9001", "987-65-4320", "OR1234567", "MOM881", "Salem General", "20250101",
            "Kowalczyk", "9 Oak Ave", "(541)555-0233", "(541)555-0888", "EMP555", "Portland General", "19590202",
            "Carl", "(541)555-0999", "7 Elm Rd", "NK1ID", "111-22-3333",
            "VISIT42",
            "G7788", "Guarantor", "Spouse", "3 Pine", "(541)555-1001", "(541)555-1002", "19550505", "222-33-4444",
            "Guarantor Employer", "4 Birch", "(541)555-1003", "GEMP1",
            "19670423", "742 Evergreen", "Springfield", "POL123", "Evergreen Employer", "INSID999",
            "IN2EMP", "123-45-6789", "(541)555-7777",
            "OLD1", "OLDACC", "OLDVISIT", "Anne",
            "5415550199",
        ] {
            assert!(!out.contains(value), "{value} survived:\n{}", out.replace('\r', "\n"));
        }
        // Coded and numeric results, the insurer and the structure stay.
        assert!(out.contains("|NM|GLU^Glucose||5.4|mmol/L|"), "{out}");
        assert!(out.contains("|Acme Insurance|"), "{out}");
        assert_eq!(out.split('\r').count(), COMPACT.split('\r').count());
        let again = parse(&out);
        assert_eq!(again.segments.len(), msg.segments.len());
        // Every masked field is listed in the dialog; a re-scan of the output
        // finds only masked values.
        let listed = detect_phi(&msg);
        for field in ["PID-7", "PID-29", "NK1-6", "NK1-30", "GT1-8", "IN1-18", "IN1-19", "IN2-2", "IN2-63", "PV1-19", "OBX-5"] {
            assert!(listed.iter().any(|p| format!("{}-{}", p.segment_type, p.field_position) == field), "{field} not listed");
        }
        assert_eq!(listed.iter().filter(|p| p.segment_type == "OBX").count(), 1, "only the free-text OBX");
    }

    #[test]
    fn only_uncatalogued_phi_is_never_reported_as_no_phi() {
        // These fields alone used to give "No PHI fields detected".
        let msg = parse("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1\rIN2|X|123-45-6789\rGT1|1||||||(541)555-0888|19670423");
        assert_eq!(detect_phi(&msg).len(), 4);
    }

    #[test]
    fn masking_counts_characters_not_bytes() {
        assert_eq!(mask_value("Ünï", &PhiSensitivity::Low), "Ünï");
        assert_eq!(mask_value("Zoë", &PhiSensitivity::Low), "Zoë");
        assert_eq!(mask_value("Zoëy", &PhiSensitivity::Low), "Zoë...");
        assert_eq!(mask_value("Ü", &PhiSensitivity::Medium), "*");
        assert_eq!(mask_value("Üx", &PhiSensitivity::Medium), "Ü***");
    }

    #[test]
    fn masked_v23_identifiers_fit_the_v23_lengths() {
        // PID-3 is 20 characters in v2.3: the assigning authority and type
        // code stay, so the mask no longer grows the field past it.
        for s in crate::samples::all() {
            let anon = anonymize_message(&parse(&s.content));
            let report = crate::validation::validate_hl7_message(&parse(&anon));
            let lens: Vec<_> = report.issues.iter().filter(|i| i.rule_id.starts_with("LEN-PID")).map(|i| i.message.clone()).collect();
            assert!(lens.is_empty(), "{}: {:?}", s.id, lens);
        }
    }

    #[test]
    fn a_plugin_rule_on_obx5_masks_every_obx_once() {
        let msg = parse("MSH|^~\\&|A|B|C|D|20240101||ORU^R01|1|P|2.5\rOBX|1|TX|N||Free text||||||F\rOBX|2|NM|G||5.4||||||F");
        let rules = [extra("OBX", 5, PhiSensitivity::Medium)];
        let out = anonymize_message_with_extra(&msg, &rules);
        assert!(out.contains("|TX|N||F***|") && out.contains("|NM|G||5***|"), "{out}");
        assert_eq!(detect_phi_with_extra(&msg, &rules).len(), 2);
    }

    #[test]
    fn catalogue_size_matches_the_documentation() {
        // README, the landing page, the CLI README and the manual (5
        // languages) quote this number: update them together.
        assert_eq!(builtin_field_count(), 89);
    }

    #[test]
    fn timestamps_with_fractions_and_offsets_keep_a_valid_shape() {
        assert_eq!(masked_timestamp("20260930123045.12-0500").as_deref(), Some("19000101000000.00+0000"));
        assert_eq!(masked_timestamp("202609301230+0100").as_deref(), Some("190001010000+0000"));
        assert_eq!(masked_timestamp("198005").as_deref(), Some("190001"));
        assert_eq!(masked_timestamp("20260930123045.12345"), None, "5 fraction digits: not a TS");
        assert_eq!(masked_timestamp("1980"), None, "a bare year goes to the numeric rule");
        assert_eq!(mask_value("19800515", &PhiSensitivity::Medium), "19000101");

        // The newly covered date fields, filled with full TS values: masked
        // and still valid.
        let msg = parse("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\r\
PID|1||1^^^H^MR||Doe^Jo||19801231083000.1234+0100|F|||||||||||||||||||||20260930123045.12-0500\r\
NK1|1|Roe^Al|SPO|||||||||||||198002021200-0300\r\
GT1|1||Roe^Al|||||19550505120000.5+0000||||||||||||||||20250101120000.12+0200\r\
IN1|1|P|I|Acme||||||||||||Roe^Al|SEL|197012311159-0800");
        let out = anonymize_message(&msg);
        for v in ["19801231", "20260930", "19800202", "19550505", "20250101", "19701231"] {
            assert!(!out.contains(v), "{v} survived: {out}");
        }
        assert!(out.contains("|19000101000000.0000+0000|"), "{out}");
        let report = crate::validation::validate_hl7_message(&parse(&out));
        let bad: Vec<_> = report.issues.iter().filter(|i| i.rule_id.starts_with("TYPE-")).map(|i| i.message.clone()).collect();
        assert!(bad.is_empty(), "{bad:?}\n{out}");
    }

    #[test]
    fn multibyte_names_and_places_are_masked_without_a_panic() {
        // Byte 8 of each value falls inside a character.
        for v in ["李小龍", "田中太郎", "ヤマダ", "김민준", "Lindström", "Bergström", "Østergård", "Västerås"] {
            let msg = parse(&format!(
                "MSH|^~\\&|A|B|C|D|20240101120000||ADT^A01|MSG1|P|2.5\rPID|1||12345^^^FAC^MR||{v}^Jan||19900515|M|||Via Roma^^{v}\r"
            ));
            let out = anonymize_message(&msg);
            assert!(!out.contains(v), "{v} survived: {out}");
            assert!(!out.contains("19900515"), "{out}");
            for level in [PhiSensitivity::High, PhiSensitivity::Medium, PhiSensitivity::Low] {
                assert!(!mask_value(v, &level).is_empty());
            }
        }
    }

    #[test]
    fn a_big5_message_is_anonymized_birth_date_included() {
        use crate::parser::hl7::charset::{decode_input, encode_output};
        // 四 is A5 7C in Big5: its trail byte is the field separator.
        let text = "MSH|^~\\&|A|B|C|D|20240101120000||ADT^A01|MSG1|P|2.5|||||TWN|BIG-5\rEVN|A01|20240101120000\rPID|1||12345^^^FAC^MR||陳四明^Jan||19900515|M\r";
        let (bytes, _, _) = encoding_rs::BIG5.encode(text);
        let decoded = decode_input(&bytes);
        assert_eq!(decoded.charset.as_deref(), Some("Big5"));
        let out = anonymize_message(&parse(&decoded.text));
        assert!(!out.contains("19900515"), "{out}");
        assert!(!out.contains("陳四明"), "{out}");
        let written = encode_output(&out, decoded.charset.as_deref().unwrap());
        assert!(!written.windows(8).any(|w| w == b"19900515"));
        assert_eq!(decode_input(&written).text, out);
    }
}

