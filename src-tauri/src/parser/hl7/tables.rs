use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

/// HL7 field definition from the standard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    /// Field position (1-based, HL7 convention)
    pub position: usize,
    /// Field name (e.g., "Patient Name")
    pub name: String,
    /// Data type (e.g., "XPN", "ST", "CX")
    pub data_type: String,
    /// Maximum length
    pub max_length: Option<usize>,
    /// Whether the field is required
    pub required: bool,
    /// Whether the field can repeat
    pub repeating: bool,
    /// Description (short)
    pub description: String,
}

/// Segment definition from the HL7 standard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentDef {
    /// Segment type code (e.g., "MSH", "PID")
    pub code: String,
    /// Full name (e.g., "Message Header")
    pub name: String,
    /// Description
    pub description: String,
    /// Field definitions for this segment
    pub fields: Vec<FieldDef>,
}

/// HL7 standard table for a specific version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hl7Table {
    pub version: String,
    pub segments: HashMap<String, SegmentDef>,
}

/// Segment info returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct SegmentInfo {
    pub code: String,
    pub name: String,
    pub description: String,
    pub fields: Vec<FieldDef>,
}

/// Field info for a specific field position.
#[derive(Debug, Clone, Serialize)]
pub struct FieldInfo {
    pub segment_code: String,
    pub position: usize,
    pub name: String,
    pub data_type: String,
    pub max_length: Option<usize>,
    pub required: bool,
    pub repeating: bool,
    pub description: String,
}

// Global table cache
static TABLES: OnceLock<HashMap<String, Hl7Table>> = OnceLock::new();

/// Initialize all built-in HL7 tables.
fn init_tables() -> HashMap<String, Hl7Table> {
    let mut tables = HashMap::new();

    // Build tables for common versions
    for version in &["2.3", "2.3.1", "2.4", "2.5", "2.5.1", "2.6", "2.7", "2.8"] {
        let table = build_table(version);
        tables.insert(version.to_string(), table);
    }

    tables
}

/// Get the tables cache, initializing if needed.
fn get_tables() -> &'static HashMap<String, Hl7Table> {
    TABLES.get_or_init(init_tables)
}

/// Look up segment info by segment code and version.
pub fn get_segment_info(segment_type: &str, version: &str) -> Option<SegmentInfo> {
    let tables = get_tables();
    // Try exact version, then fall back to major.minor
    let table = tables.get(version).or_else(|| {
        let major_minor = version.split('.').take(2).collect::<Vec<_>>().join(".");
        tables.get(&major_minor)
    })?;

    let seg_def = table.segments.get(segment_type)?;
    Some(SegmentInfo {
        code: seg_def.code.clone(),
        name: seg_def.name.clone(),
        description: seg_def.description.clone(),
        fields: seg_def.fields.clone(),
    })
}

/// Look up a specific field info.
pub fn get_field_info(segment_type: &str, field_position: usize, version: &str) -> Option<FieldInfo> {
    let tables = get_tables();
    let table = tables.get(version).or_else(|| {
        let major_minor = version.split('.').take(2).collect::<Vec<_>>().join(".");
        tables.get(&major_minor)
    })?;

    let seg_def = table.segments.get(segment_type)?;
    let field_def = seg_def.fields.iter().find(|f| f.position == field_position)?;

    Some(FieldInfo {
        segment_code: segment_type.to_string(),
        position: field_def.position,
        name: field_def.name.clone(),
        data_type: field_def.data_type.clone(),
        max_length: field_def.max_length,
        required: field_def.required,
        repeating: field_def.repeating,
        description: field_def.description.clone(),
    })
}

/// Build a table for a given version with common segments.
/// These definitions cover the most commonly used segments across HL7 v2.x.
fn build_table(version: &str) -> Hl7Table {
    let mut segments = HashMap::new();

    // MSH - Message Header
    segments.insert("MSH".into(), SegmentDef {
        code: "MSH".into(),
        name: "Message Header".into(),
        description: "Defines the intent, source, destination, and some specifics of the syntax of a message".into(),
        fields: vec![
            field(1, "Field Separator", "ST", Some(1), true, false, "Separator between fields"),
            field(2, "Encoding Characters", "ST", Some(4), true, false, "Component, repetition, escape, subcomponent separators"),
            field(3, "Sending Application", "HD", Some(227), false, false, "Application sending the message"),
            field(4, "Sending Facility", "HD", Some(227), false, false, "Facility sending the message"),
            field(5, "Receiving Application", "HD", Some(227), false, false, "Application receiving the message"),
            field(6, "Receiving Facility", "HD", Some(227), false, false, "Facility receiving the message"),
            field(7, "Date/Time of Message", "TS", Some(26), true, false, "Date/time the message was created"),
            field(8, "Security", "ST", Some(40), false, false, "Security information"),
            field(9, "Message Type", "MSG", Some(15), true, false, "Message type and trigger event"),
            field(10, "Message Control ID", "ST", Some(199), true, false, "Unique message identifier"),
            field(11, "Processing ID", "PT", Some(3), true, false, "Processing ID (P=production, D=debug, T=training)"),
            field(12, "Version ID", "VID", Some(60), true, false, "HL7 version number"),
        ],
    });

    // PID - Patient Identification
    segments.insert("PID".into(), SegmentDef {
        code: "PID".into(),
        name: "Patient Identification".into(),
        description: "Contains patient identification and demographic information".into(),
        fields: vec![
            field(1, "Set ID", "SI", Some(4), false, false, "Sequence number"),
            field(2, "Patient ID (External)", "CX", Some(20), false, false, "External patient ID (deprecated)"),
            field(3, "Patient Identifier List", "CX", Some(250), true, true, "List of patient identifiers"),
            field(4, "Alternate Patient ID", "CX", Some(20), false, true, "Alternate patient ID (deprecated)"),
            field(5, "Patient Name", "XPN", Some(250), true, true, "Legal name of the patient"),
            field(6, "Mother's Maiden Name", "XPN", Some(250), false, true, "Mother's maiden name"),
            field(7, "Date/Time of Birth", "TS", Some(26), false, false, "Patient date of birth"),
            field(8, "Administrative Sex", "IS", Some(1), false, false, "Patient sex (M/F/O/U)"),
            field(9, "Patient Alias", "XPN", Some(250), false, true, "Alias/previous name"),
            field(10, "Race", "CE", Some(250), false, true, "Patient race"),
            field(11, "Patient Address", "XAD", Some(250), false, true, "Patient mailing address"),
            field(12, "County Code", "IS", Some(4), false, false, "County code"),
            field(13, "Phone Number - Home", "XTN", Some(250), false, true, "Home phone number"),
            field(14, "Phone Number - Business", "XTN", Some(250), false, true, "Business phone number"),
            field(15, "Primary Language", "CE", Some(250), false, false, "Patient primary language"),
            field(16, "Marital Status", "CE", Some(250), false, false, "Patient marital status"),
            field(17, "Religion", "CE", Some(250), false, false, "Patient religion"),
            field(18, "Patient Account Number", "CX", Some(250), false, false, "Patient account number"),
            field(19, "SSN Number", "ST", Some(16), false, false, "Social security number"),
            field(20, "Driver's License Number", "DLN", Some(25), false, false, "Driver's license number"),
        ],
    });

    // PV1 - Patient Visit
    segments.insert("PV1".into(), SegmentDef {
        code: "PV1".into(),
        name: "Patient Visit".into(),
        description: "Contains information about the patient visit".into(),
        fields: vec![
            field(1, "Set ID", "SI", Some(4), false, false, "Sequence number"),
            field(2, "Patient Class", "IS", Some(1), true, false, "Patient class (I=inpatient, O=outpatient, E=emergency)"),
            field(3, "Assigned Patient Location", "PL", Some(80), false, false, "Patient location"),
            field(4, "Admission Type", "IS", Some(2), false, false, "Type of admission"),
            field(5, "Preadmit Number", "CX", Some(250), false, false, "Preadmit number"),
            field(6, "Prior Patient Location", "PL", Some(80), false, false, "Previous patient location"),
            field(7, "Attending Doctor", "XCN", Some(250), false, true, "Attending physician"),
            field(8, "Referring Doctor", "XCN", Some(250), false, true, "Referring physician"),
            field(9, "Consulting Doctor", "XCN", Some(250), false, true, "Consulting physician"),
            field(10, "Hospital Service", "IS", Some(3), false, false, "Hospital service"),
            field(14, "Admit Source", "IS", Some(6), false, false, "Source of admission"),
            field(17, "Admitting Doctor", "XCN", Some(250), false, true, "Admitting physician"),
            field(18, "Patient Type", "IS", Some(2), false, false, "Patient type"),
            field(19, "Visit Number", "CX", Some(250), false, false, "Unique visit identifier"),
            field(36, "Discharge Disposition", "IS", Some(3), false, false, "Discharge disposition"),
            field(44, "Admit Date/Time", "TS", Some(26), false, false, "Date/time of admission"),
            field(45, "Discharge Date/Time", "TS", Some(26), false, false, "Date/time of discharge"),
        ],
    });

    // OBR - Observation Request
    segments.insert("OBR".into(), SegmentDef {
        code: "OBR".into(),
        name: "Observation Request".into(),
        description: "Defines the observation request and associated information".into(),
        fields: vec![
            field(1, "Set ID", "SI", Some(4), false, false, "Sequence number"),
            field(2, "Placer Order Number", "EI", Some(75), false, false, "Placer order number"),
            field(3, "Filler Order Number", "EI", Some(75), false, false, "Filler order number"),
            field(4, "Universal Service Identifier", "CE", Some(250), true, false, "Test/service identifier"),
            field(7, "Observation Date/Time", "TS", Some(26), false, false, "Date/time of observation"),
            field(8, "Observation End Date/Time", "TS", Some(26), false, false, "End date/time of observation"),
            field(14, "Specimen Received Date/Time", "TS", Some(26), false, false, "Specimen receipt date/time"),
            field(16, "Ordering Provider", "XCN", Some(250), false, true, "Ordering provider"),
            field(22, "Results Rpt/Status Chng Date/Time", "TS", Some(26), false, false, "Result status change date"),
            field(25, "Result Status", "ID", Some(1), false, false, "Result status (F=Final, P=Preliminary)"),
        ],
    });

    // OBX - Observation/Result
    segments.insert("OBX".into(), SegmentDef {
        code: "OBX".into(),
        name: "Observation/Result".into(),
        description: "Contains observation results including clinical data and encoded documents".into(),
        fields: vec![
            field(1, "Set ID", "SI", Some(4), false, false, "Sequence number"),
            field(2, "Value Type", "ID", Some(3), true, false, "Data type of OBX-5 (ST, NM, CE, TX, ED, etc.)"),
            field(3, "Observation Identifier", "CE", Some(250), true, false, "Observation identifier code"),
            field(4, "Observation Sub-ID", "ST", Some(20), false, false, "Sub-identifier for multiple OBX per observation"),
            field(5, "Observation Value", "varies", None, false, true, "Actual observation value (may contain base64)"),
            field(6, "Units", "CE", Some(250), false, false, "Units of measurement"),
            field(7, "References Range", "ST", Some(60), false, false, "Normal reference range"),
            field(8, "Abnormal Flags", "IS", Some(5), false, true, "Abnormality flags (H=high, L=low, A=abnormal)"),
            field(11, "Observation Result Status", "ID", Some(1), true, false, "Result status (F=Final, P=Preliminary)"),
            field(14, "Date/Time of Observation", "TS", Some(26), false, false, "Date/time of observation"),
        ],
    });

    // NK1 - Next of Kin
    segments.insert("NK1".into(), SegmentDef {
        code: "NK1".into(),
        name: "Next of Kin".into(),
        description: "Contains information about the patient's next of kin or associated parties".into(),
        fields: vec![
            field(1, "Set ID", "SI", Some(4), true, false, "Sequence number"),
            field(2, "Name", "XPN", Some(250), false, true, "Name of next of kin"),
            field(3, "Relationship", "CE", Some(250), false, false, "Relationship to patient"),
            field(4, "Address", "XAD", Some(250), false, true, "Address"),
            field(5, "Phone Number", "XTN", Some(250), false, true, "Phone number"),
            field(7, "Contact Role", "CE", Some(250), false, false, "Contact role"),
        ],
    });

    // IN1 - Insurance
    segments.insert("IN1".into(), SegmentDef {
        code: "IN1".into(),
        name: "Insurance".into(),
        description: "Contains insurance coverage information".into(),
        fields: vec![
            field(1, "Set ID", "SI", Some(4), true, false, "Sequence number"),
            field(2, "Insurance Plan ID", "CE", Some(250), true, false, "Insurance plan identifier"),
            field(3, "Insurance Company ID", "CX", Some(250), true, true, "Insurance company ID"),
            field(4, "Insurance Company Name", "XON", Some(250), false, true, "Insurance company name"),
            field(5, "Insurance Company Address", "XAD", Some(250), false, true, "Insurance company address"),
            field(12, "Plan Effective Date", "DT", Some(8), false, false, "Plan effective date"),
            field(13, "Plan Expiration Date", "DT", Some(8), false, false, "Plan expiration date"),
            field(16, "Name of Insured", "XPN", Some(250), false, true, "Name of insured person"),
            field(36, "Policy Number", "ST", Some(15), false, false, "Policy number"),
        ],
    });

    // AL1 - Allergy Information
    segments.insert("AL1".into(), SegmentDef {
        code: "AL1".into(),
        name: "Patient Allergy Information".into(),
        description: "Contains allergy information about the patient".into(),
        fields: vec![
            field(1, "Set ID", "SI", Some(4), true, false, "Sequence number"),
            field(2, "Allergen Type Code", "CE", Some(250), false, false, "Type of allergen"),
            field(3, "Allergen Code/Description", "CE", Some(250), true, false, "Allergen code or description"),
            field(4, "Allergy Severity Code", "CE", Some(250), false, false, "Severity of allergy"),
            field(5, "Allergy Reaction Code", "ST", Some(15), false, true, "Allergy reaction"),
            field(6, "Identification Date", "DT", Some(8), false, false, "Date allergy was identified"),
        ],
    });

    // DG1 - Diagnosis
    segments.insert("DG1".into(), SegmentDef {
        code: "DG1".into(),
        name: "Diagnosis".into(),
        description: "Contains diagnosis information".into(),
        fields: vec![
            field(1, "Set ID", "SI", Some(4), true, false, "Sequence number"),
            field(2, "Diagnosis Coding Method", "ID", Some(2), false, false, "Coding method"),
            field(3, "Diagnosis Code", "CE", Some(250), false, false, "Diagnosis code"),
            field(4, "Diagnosis Description", "ST", Some(40), false, false, "Diagnosis description"),
            field(5, "Diagnosis Date/Time", "TS", Some(26), false, false, "Diagnosis date/time"),
            field(6, "Diagnosis Type", "IS", Some(2), true, false, "Diagnosis type"),
        ],
    });

    // EVN - Event Type
    segments.insert("EVN".into(), SegmentDef {
        code: "EVN".into(),
        name: "Event Type".into(),
        description: "Contains the event type that triggered the message".into(),
        fields: vec![
            field(1, "Event Type Code", "ID", Some(3), false, false, "Event type code (deprecated in 2.5+)"),
            field(2, "Recorded Date/Time", "TS", Some(26), true, false, "Date/time event was recorded"),
            field(3, "Date/Time Planned Event", "TS", Some(26), false, false, "Planned event date"),
            field(4, "Event Reason Code", "IS", Some(3), false, false, "Reason for event"),
            field(5, "Operator ID", "XCN", Some(250), false, true, "Operator who triggered the event"),
            field(6, "Event Occurred", "TS", Some(26), false, false, "Date/time event occurred"),
        ],
    });

    // ORC - Common Order
    segments.insert("ORC".into(), SegmentDef {
        code: "ORC".into(),
        name: "Common Order".into(),
        description: "Contains order control information common to all orders".into(),
        fields: vec![
            field(1, "Order Control", "ID", Some(2), true, false, "Order control code (NW=New, CA=Cancel, etc.)"),
            field(2, "Placer Order Number", "EI", Some(75), false, false, "Placer order number"),
            field(3, "Filler Order Number", "EI", Some(75), false, false, "Filler order number"),
            field(4, "Placer Group Number", "EI", Some(75), false, false, "Placer group number"),
            field(5, "Order Status", "ID", Some(2), false, false, "Order status"),
            field(9, "Date/Time of Transaction", "TS", Some(26), false, false, "Transaction date/time"),
            field(12, "Ordering Provider", "XCN", Some(250), false, true, "Ordering provider"),
            field(14, "Call Back Phone Number", "XTN", Some(250), false, true, "Callback phone number"),
        ],
    });

    // MSA - Message Acknowledgment
    segments.insert("MSA".into(), SegmentDef {
        code: "MSA".into(),
        name: "Message Acknowledgment".into(),
        description: "Contains acknowledgment information for a received message".into(),
        fields: vec![
            field(1, "Acknowledgment Code", "ID", Some(2), true, false, "AA=Accept, AE=Error, AR=Reject"),
            field(2, "Message Control ID", "ST", Some(199), true, false, "Message control ID of the message being acknowledged"),
            field(3, "Text Message", "ST", Some(80), false, false, "Text message describing result"),
        ],
    });

    // ERR - Error
    segments.insert("ERR".into(), SegmentDef {
        code: "ERR".into(),
        name: "Error".into(),
        description: "Contains error information".into(),
        fields: vec![
            field(1, "Error Code and Location", "ELD", Some(493), false, true, "Error code and location (deprecated in 2.5+)"),
            field(2, "Error Location", "ERL", Some(18), false, true, "Error location"),
            field(3, "HL7 Error Code", "CWE", Some(705), true, false, "HL7 error code"),
            field(4, "Severity", "ID", Some(2), true, false, "Error severity (E=Error, W=Warning, I=Info)"),
        ],
    });

    // TXA - Transcription Document Header
    segments.insert("TXA".into(), SegmentDef {
        code: "TXA".into(),
        name: "Transcription Document Header".into(),
        description: "Contains document header information for transcribed documents".into(),
        fields: vec![
            field(1, "Set ID", "SI", Some(4), true, false, "Sequence number"),
            field(2, "Document Type", "IS", Some(30), true, false, "Document type"),
            field(3, "Document Content Presentation", "ID", Some(2), false, false, "Content format"),
            field(4, "Activity Date/Time", "TS", Some(26), false, false, "Activity date/time"),
            field(12, "Unique Document Number", "EI", Some(75), true, false, "Unique document ID"),
            field(17, "Document Completion Status", "ID", Some(2), true, false, "Completion status"),
        ],
    });

    // SCH - Scheduling Activity Information
    segments.insert("SCH".into(), SegmentDef {
        code: "SCH".into(),
        name: "Scheduling Activity Information".into(),
        description: "Contains scheduling activity information".into(),
        fields: vec![
            field(1, "Placer Appointment ID", "EI", Some(75), false, false, "Placer appointment ID"),
            field(2, "Filler Appointment ID", "EI", Some(75), false, false, "Filler appointment ID"),
            field(6, "Event Reason", "CE", Some(250), true, false, "Reason for event"),
            field(7, "Appointment Reason", "CE", Some(250), false, false, "Appointment reason"),
            field(11, "Appointment Timing Quantity", "TQ", Some(200), false, true, "Timing/quantity"),
            field(16, "Filler Contact Person", "XCN", Some(250), false, true, "Filler contact"),
            field(25, "Filler Status Code", "CE", Some(250), false, false, "Status code"),
        ],
    });

    Hl7Table {
        version: version.to_string(),
        segments,
    }
}

/// Helper to create a FieldDef concisely.
fn field(
    position: usize,
    name: &str,
    data_type: &str,
    max_length: Option<usize>,
    required: bool,
    repeating: bool,
    description: &str,
) -> FieldDef {
    FieldDef {
        position,
        name: name.to_string(),
        data_type: data_type.to_string(),
        max_length,
        required,
        repeating,
        description: description.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_segment_info() {
        let info = get_segment_info("MSH", "2.5.1").unwrap();
        assert_eq!(info.code, "MSH");
        assert_eq!(info.name, "Message Header");
        assert!(!info.fields.is_empty());
    }

    #[test]
    fn test_get_field_info() {
        let info = get_field_info("PID", 5, "2.5").unwrap();
        assert_eq!(info.name, "Patient Name");
        assert_eq!(info.data_type, "XPN");
        assert!(info.required);
    }

    #[test]
    fn test_version_fallback() {
        let info = get_segment_info("PID", "2.5.1").unwrap();
        assert_eq!(info.code, "PID");
    }

    #[test]
    fn test_unknown_segment() {
        assert!(get_segment_info("ZZZ", "2.5").is_none());
    }
}
