use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

use super::schema::{self, Hl7Version};

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
    /// HL7 value table behind the field's value ("0001" for PID-8; for a
    /// composite, its first component's table), when it is coded.
    #[serde(default)]
    pub table_id: Option<String>,
    /// Data type of the element `table_id` belongs to — the field's own
    /// for a primitive, the first component's for a composite (MSH-9 is
    /// MSG, its table is MSG.1's, an `ID`). It decides whether the table
    /// is closed (`ID`) or a list of suggestions.
    #[serde(default)]
    pub table_data_type: Option<String>,
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
    /// HL7 value table backing this coded field (e.g. "0001" for PID-8),
    /// resolvable via the `get_hl7_table` command. None for free-text fields.
    /// For a composite field this is its first component's table (MSH-9 →
    /// 0076), the one its leading value is checked against.
    pub table_id: Option<String>,
    /// The field's components when its data type is composite, each with
    /// its own table where coded (MSH-9.2 → 0003). Empty for primitives.
    pub components: Vec<ComponentInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComponentInfo {
    pub position: usize,
    pub name: String,
    pub data_type: String,
    pub max_length: Option<usize>,
    pub table_id: Option<String>,
}

// Global table cache
static TABLES: OnceLock<HashMap<String, Hl7Table>> = OnceLock::new();

/// Initialize all built-in HL7 tables.
fn init_tables() -> HashMap<String, Hl7Table> {
    let mut tables = HashMap::new();

    // One entry per shipped version, taken from the schema catalogue rather
    // than a hand-kept list: a version added there but forgotten here would
    // silently lose autocomplete and hover for messages that declare it.
    for version in crate::parser::hl7::schema::Hl7Version::ALL {
        tables.insert(version.as_str().to_string(), build_table(*version));
    }

    tables
}

/// Table used for a version we hold no entry for.
///
/// A message declaring an HL7 version BridgeLab does not ship (a future
/// v2.8, or a typo in MSH-12) still gets field names and hover text from
/// the default catalogue instead of nothing at all.
fn fallback_table(tables: &HashMap<String, Hl7Table>) -> Option<&Hl7Table> {
    tables.get(DEFAULT_VERSION)
}

/// Version whose table stands in for anything unrecognised.
const DEFAULT_VERSION: &str = "2.5";

/// Exact version, then major.minor, then the default table.
///
/// `version` is MSH-12 as the message carries it, which may be a full VID
/// (`2.3^ISO^…`, or `2.3#ISO` under a custom component separator): the
/// version number is its leading part. Now that the tables are genuinely
/// version-specific, a VID left unparsed would fall through to v2.5
/// definitions for a v2.3 message.
fn table_for<'a>(tables: &'a HashMap<String, Hl7Table>, version: &str) -> Option<&'a Hl7Table> {
    let number = schema::version_number(version);
    tables
        .get(number)
        .or_else(|| {
            let major_minor = number.split('.').take(2).collect::<Vec<_>>().join(".");
            tables.get(&major_minor)
        })
        .or_else(|| fallback_table(tables))
}

/// Get the tables cache, initializing if needed.
fn get_tables() -> &'static HashMap<String, Hl7Table> {
    TABLES.get_or_init(init_tables)
}

/// Look up segment info by segment code and version.
pub fn get_segment_info(segment_type: &str, version: &str) -> Option<SegmentInfo> {
    let table = table_for(get_tables(), version)?;

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
    let table = table_for(get_tables(), version)?;

    let seg_def = table.segments.get(segment_type)?;
    let field_def = seg_def.fields.iter().find(|f| f.position == field_position)?;

    let components = schema::for_declared(&table.version)
        .composite(&field_def.data_type)
        .map(|c| {
            c.components
                .iter()
                .map(|comp| ComponentInfo {
                    position: comp.position,
                    name: comp.name.clone(),
                    data_type: comp.data_type.clone(),
                    max_length: comp.max_length,
                    table_id: comp.table.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    Some(FieldInfo {
        segment_code: segment_type.to_string(),
        position: field_def.position,
        name: field_def.name.clone(),
        data_type: field_def.data_type.clone(),
        max_length: field_def.max_length,
        required: field_def.required,
        repeating: field_def.repeating,
        description: field_def.description.clone(),
        table_id: field_def.table_id.clone(),
        components,
    })
}

/// The field table of one HL7 version: every segment of that version's
/// catalogue, with the curated one-line descriptions layered on top for
/// the segments and fields an integration engineer meets daily. Names,
/// types, cardinality, lengths and value tables come from the catalogue,
/// so they follow the declared version; only the prose is hand-written.
fn build_table(version: Hl7Version) -> Hl7Table {
    let catalogue = schema::cached(version);
    let curated = curated_descriptions();

    let segments = catalogue
        .segments
        .iter()
        .map(|seg| {
            let prose = curated.get(&seg.code);
            let fields = seg
                .fields
                .iter()
                .map(|f| {
                    let field_prose = prose.and_then(|p| p.fields.iter().find(|pf| pf.position == f.position));
                    let table_id = catalogue.table_for_field(&seg.code, f.position).map(str::to_string);
                    let table_data_type = table_id.as_ref().map(|_| {
                        if f.table.is_some() {
                            f.data_type.clone()
                        } else {
                            catalogue
                                .component(&f.data_type, 1)
                                .map(|c| c.data_type.clone())
                                .unwrap_or_else(|| f.data_type.clone())
                        }
                    });
                    FieldDef {
                        position: f.position,
                        name: f.name.clone(),
                        data_type: f.data_type.clone(),
                        max_length: f.max_length,
                        required: f.required,
                        repeating: f.repeats,
                        description: field_prose
                            .map(|p| p.description.clone())
                            .unwrap_or_else(|| f.name.clone()),
                        table_id,
                        table_data_type,
                    }
                })
                .collect();
            let def = SegmentDef {
                code: seg.code.clone(),
                name: seg.name.clone(),
                description: prose.map(|p| p.description.clone()).unwrap_or_else(|| seg.name.clone()),
                fields,
            };
            (seg.code.clone(), def)
        })
        .collect();

    Hl7Table {
        version: version.as_str().to_string(),
        segments,
    }
}

/// Hand-written descriptions for the common segments and fields. Only the
/// `description` strings are used; the structural attributes in here are
/// the historical bootstrap and are superseded by the catalogue.
fn curated_descriptions() -> HashMap<String, SegmentDef> {
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
            field(24, "Multiple Birth Indicator", "ID", Some(1), false, false, "Y if the patient is part of a multiple birth"),
            field(30, "Patient Death Indicator", "ID", Some(1), false, false, "Y if the patient is deceased"),
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

    segments
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
        table_id: None,
        table_data_type: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every version the schema catalogue ships must resolve here too.
    ///
    /// Regression guard: the version list was hand-kept and stopped at 2.3,
    /// so adding v2.1 and v2.2 to the catalogue silently removed field
    /// completion and hover for any message declaring them — the lookup
    /// missed on both the exact and the major.minor key and returned None.
    #[test]
    fn every_shipped_version_resolves() {
        for version in crate::parser::hl7::schema::Hl7Version::ALL {
            assert!(
                get_segment_info("PID", version.as_str()).is_some(),
                "no segment table for HL7 {}",
                version.as_str()
            );
        }
    }

    /// An unrecognised version falls back rather than losing the hints.
    #[test]
    fn an_unknown_version_still_gets_a_table() {
        assert!(get_segment_info("PID", "2.9").is_some());
        assert!(get_segment_info("PID", "").is_some());
        assert!(get_field_info("PID", 5, "nonsense").is_some());
        // An unknown *segment* is still None: there is nothing to fall back to.
        assert!(get_segment_info("ZZZ", "2.5").is_none());
    }

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
        assert_eq!(info.max_length, Some(250));
        assert_eq!(info.table_id, None);
        // XPN.7 (name type code) is coded even though the field is not.
        let name_type = info.components.iter().find(|c| c.position == 7).expect("XPN.7");
        assert_eq!(name_type.table_id.as_deref(), Some("0200"));
    }

    /// The field table is the whole catalogue, not the fifteen curated
    /// segments: any standard segment resolves, in every version, and the
    /// curated prose is layered on where it exists.
    #[test]
    fn every_catalogue_segment_resolves() {
        for version in Hl7Version::ALL {
            let catalogue = schema::cached(*version);
            for seg in &catalogue.segments {
                let info = get_segment_info(&seg.code, version.as_str())
                    .unwrap_or_else(|| panic!("{} missing in {}", seg.code, version.as_str()));
                assert_eq!(info.fields.len(), seg.fields.len(), "{} {}", seg.code, version.as_str());
            }
        }
        // RXA was never hand-coded; it comes from the catalogue.
        let rxa = get_field_info("RXA", 5, "2.5").expect("RXA-5");
        assert_eq!(rxa.name, "Administered Code");
        assert_eq!(rxa.data_type, "CE");
        // Curated prose survives for PID-8; a field without any falls back to its name.
        assert_eq!(get_field_info("PID", 8, "2.5").unwrap().description, "Patient sex (M/F/O/U)");
        assert_eq!(get_field_info("PID", 8, "2.5").unwrap().table_id.as_deref(), Some("0001"));
        assert_eq!(rxa.description, rxa.name);
    }

    /// Structure follows the declared version: PID had 20 fields in v2.1
    /// and 39 in v2.7.
    #[test]
    fn field_tables_are_version_specific() {
        let old = get_segment_info("PID", "2.1").unwrap().fields.len();
        let new = get_segment_info("PID", "2.7").unwrap().fields.len();
        assert!(old < new, "v2.1 PID has {} fields, v2.7 has {}", old, new);
        assert!(get_segment_info("RXA", "2.1").is_none(), "RXA did not exist in v2.1");
    }

    /// Codex review: MSH-12 may be a full VID ("2.3^ISO^…"); it must select
    /// the v2.3 tables, not fall back to v2.5.
    #[test]
    fn a_full_vid_selects_its_own_version() {
        let vid = get_segment_info("PID", "2.1^ISO^HL7").unwrap();
        let bare = get_segment_info("PID", "2.1").unwrap();
        let default = get_segment_info("PID", "2.5").unwrap();
        assert_eq!(vid.fields.len(), bare.fields.len());
        assert_ne!(vid.fields.len(), default.fields.len());
        assert!(get_segment_info("RXA", "2.1^ISO").is_none());
        assert!(get_segment_info("RXA", "2.1#ISO").is_none(), "custom component separator");
    }

    /// The type the table is closed or open by is that of the element
    /// holding the table: MSH-9 is MSG, but its 0076 belongs to MSG.1 (ID).
    #[test]
    fn field_defs_carry_the_coded_element_type() {
        let msh = get_segment_info("MSH", "2.5").unwrap();
        let f9 = msh.fields.iter().find(|f| f.position == 9).unwrap();
        assert_eq!(f9.table_id.as_deref(), Some("0076"));
        assert_eq!(f9.table_data_type.as_deref(), Some("ID"));
        let pid = get_segment_info("PID", "2.5").unwrap();
        let f8 = pid.fields.iter().find(|f| f.position == 8).unwrap();
        assert_eq!(f8.table_data_type.as_deref(), Some("IS"));
        assert_eq!(pid.fields.iter().find(|f| f.position == 5).unwrap().table_data_type, None);
    }

    #[test]
    fn composite_fields_report_their_leading_table() {
        let msh9 = get_field_info("MSH", 9, "2.5").unwrap();
        assert_eq!(msh9.data_type, "MSG");
        assert_eq!(msh9.table_id.as_deref(), Some("0076"));
        assert_eq!(msh9.components.len(), 3);
        assert_eq!(msh9.components[1].table_id.as_deref(), Some("0003"));
        let msa1 = get_field_info("MSA", 1, "2.5").unwrap();
        assert_eq!(msa1.table_id.as_deref(), Some("0008"));
        assert!(msa1.components.is_empty(), "ID is primitive");
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
