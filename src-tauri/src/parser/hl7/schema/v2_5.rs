//! HL7 v2.5 hand-coded schema data. Scope: 4 message types and every
//! segment / composite / primitive they reference.
//!
//! Derived from the HL7 v2.5 public specification. Output format matches
//! the reference XSDs supplied by the user.

use super::{
    ComponentSpec, CompositeType, FieldSpec, Hl7Schema, Hl7Version,
    MessageElement, MessageStructure, PrimitiveType, SegmentSpec,
};

pub fn schema() -> Hl7Schema {
    Hl7Schema {
        version: Hl7Version::V2_5,
        messages: vec![adt_a01(), adt_a40(), orm_o01(), oru_r01()],
        segments: segments(),
        composites: composites(),
        primitives: primitives(),
    }
}

// --------- small helpers to keep the data tables compact --------------------

fn f(position: usize, name: &str, dt: &str, required: bool, repeats: bool) -> FieldSpec {
    FieldSpec {
        position,
        name: name.into(),
        data_type: dt.into(),
        required,
        repeats,
    }
}

fn c(position: usize, name: &str, dt: &str, required: bool) -> ComponentSpec {
    ComponentSpec { position, name: name.into(), data_type: dt.into(), required }
}

fn seg(code: &str, name: &str, fields: Vec<FieldSpec>) -> SegmentSpec {
    SegmentSpec { code: code.into(), name: name.into(), fields }
}

fn comp(code: &str, components: Vec<ComponentSpec>) -> CompositeType {
    CompositeType { code: code.into(), components }
}

fn primitive(code: &str) -> PrimitiveType {
    PrimitiveType { code: code.into() }
}

// --------- message structures (top-level shapes) ----------------------------

fn adt_a01() -> MessageStructure {
    use MessageElement::*;
    MessageStructure {
        code: "ADT_A01".into(),
        event: "ADT^A01".into(),
        description: "Admit / Visit Notification".into(),
        elements: vec![
            Segment { code: "MSH".into(), required: true,  repeats: false },
            Segment { code: "EVN".into(), required: true,  repeats: false },
            Segment { code: "PID".into(), required: true,  repeats: false },
            Segment { code: "PD1".into(), required: false, repeats: false },
            Segment { code: "NK1".into(), required: false, repeats: true  },
            Segment { code: "PV1".into(), required: true,  repeats: false },
            Segment { code: "PV2".into(), required: false, repeats: false },
            Segment { code: "OBX".into(), required: false, repeats: true  },
            Segment { code: "AL1".into(), required: false, repeats: true  },
            Segment { code: "DG1".into(), required: false, repeats: true  },
            Segment { code: "GT1".into(), required: false, repeats: true  },
            Group {
                name: "INSURANCE".into(),
                required: false,
                repeats: true,
                elements: vec![
                    Segment { code: "IN1".into(), required: true,  repeats: false },
                    Segment { code: "IN2".into(), required: false, repeats: false },
                    Segment { code: "IN3".into(), required: false, repeats: false },
                ],
            },
        ],
    }
}

fn adt_a40() -> MessageStructure {
    use MessageElement::*;
    MessageStructure {
        code: "ADT_A40".into(),
        event: "ADT^A40".into(),
        description: "Merge Patient - Patient Identifier List".into(),
        elements: vec![
            Segment { code: "MSH".into(), required: true, repeats: false },
            Segment { code: "EVN".into(), required: true, repeats: false },
            Group {
                name: "PATIENT_ID".into(),
                required: true,
                repeats: true,
                elements: vec![
                    Segment { code: "PID".into(), required: true,  repeats: false },
                    Segment { code: "PD1".into(), required: false, repeats: false },
                    Segment { code: "MRG".into(), required: true,  repeats: false },
                    Segment { code: "PV1".into(), required: false, repeats: false },
                ],
            },
        ],
    }
}

fn orm_o01() -> MessageStructure {
    use MessageElement::*;
    MessageStructure {
        code: "ORM_O01".into(),
        event: "ORM^O01".into(),
        description: "Order Message".into(),
        elements: vec![
            Segment { code: "MSH".into(), required: true,  repeats: false },
            Segment { code: "NTE".into(), required: false, repeats: true  },
            Group {
                name: "PATIENT".into(),
                required: false,
                repeats: false,
                elements: vec![
                    Segment { code: "PID".into(), required: true,  repeats: false },
                    Segment { code: "PD1".into(), required: false, repeats: false },
                    Segment { code: "NTE".into(), required: false, repeats: true  },
                    Group {
                        name: "PATIENT_VISIT".into(),
                        required: false,
                        repeats: false,
                        elements: vec![
                            Segment { code: "PV1".into(), required: true,  repeats: false },
                            Segment { code: "PV2".into(), required: false, repeats: false },
                        ],
                    },
                    Group {
                        name: "INSURANCE".into(),
                        required: false,
                        repeats: true,
                        elements: vec![
                            Segment { code: "IN1".into(), required: true,  repeats: false },
                            Segment { code: "IN2".into(), required: false, repeats: false },
                            Segment { code: "IN3".into(), required: false, repeats: false },
                        ],
                    },
                    Segment { code: "GT1".into(), required: false, repeats: false },
                    Segment { code: "AL1".into(), required: false, repeats: true  },
                ],
            },
            Group {
                name: "ORDER".into(),
                required: true,
                repeats: true,
                elements: vec![
                    Segment { code: "ORC".into(), required: true,  repeats: false },
                    Group {
                        name: "ORDER_DETAIL".into(),
                        required: false,
                        repeats: false,
                        elements: vec![
                            Choice {
                                required: false,
                                repeats: true,
                                segments: vec![
                                    "OBR".into(), "RQD".into(), "RQ1".into(),
                                    "RXO".into(), "ODS".into(), "ODT".into(),
                                ],
                            },
                            Segment { code: "NTE".into(), required: false, repeats: true  },
                            Segment { code: "CTD".into(), required: false, repeats: false },
                            Segment { code: "DG1".into(), required: false, repeats: true  },
                            Group {
                                name: "OBSERVATION".into(),
                                required: false,
                                repeats: true,
                                elements: vec![
                                    Segment { code: "OBX".into(), required: true,  repeats: false },
                                    Segment { code: "NTE".into(), required: false, repeats: true  },
                                ],
                            },
                        ],
                    },
                    Segment { code: "FT1".into(), required: false, repeats: true  },
                    Segment { code: "CTI".into(), required: false, repeats: true  },
                    Segment { code: "BLG".into(), required: false, repeats: false },
                ],
            },
        ],
    }
}

fn oru_r01() -> MessageStructure {
    use MessageElement::*;
    MessageStructure {
        code: "ORU_R01".into(),
        event: "ORU^R01".into(),
        description: "Unsolicited Observation Result".into(),
        elements: vec![
            Segment { code: "MSH".into(), required: true,  repeats: false },
            Segment { code: "SFT".into(), required: false, repeats: true  },
            Group {
                name: "PATIENT_RESULT".into(),
                required: true,
                repeats: true,
                elements: vec![
                    Group {
                        name: "PATIENT".into(),
                        required: false,
                        repeats: false,
                        elements: vec![
                            Segment { code: "PID".into(), required: true,  repeats: false },
                            Segment { code: "PD1".into(), required: false, repeats: false },
                            Segment { code: "NTE".into(), required: false, repeats: true  },
                            Segment { code: "NK1".into(), required: false, repeats: true  },
                            Group {
                                name: "VISIT".into(),
                                required: false,
                                repeats: false,
                                elements: vec![
                                    Segment { code: "PV1".into(), required: true,  repeats: false },
                                    Segment { code: "PV2".into(), required: false, repeats: false },
                                ],
                            },
                        ],
                    },
                    Group {
                        name: "ORDER_OBSERVATION".into(),
                        required: true,
                        repeats: true,
                        elements: vec![
                            Segment { code: "ORC".into(), required: false, repeats: false },
                            Segment { code: "OBR".into(), required: true,  repeats: false },
                            Segment { code: "NTE".into(), required: false, repeats: true  },
                            Group {
                                name: "TIMING_QTY".into(),
                                required: false,
                                repeats: true,
                                elements: vec![
                                    Segment { code: "TQ1".into(), required: true,  repeats: false },
                                    Segment { code: "TQ2".into(), required: false, repeats: true  },
                                ],
                            },
                            Segment { code: "CTD".into(), required: false, repeats: false },
                            Group {
                                name: "OBSERVATION".into(),
                                required: false,
                                repeats: true,
                                elements: vec![
                                    Segment { code: "OBX".into(), required: true,  repeats: false },
                                    Segment { code: "NTE".into(), required: false, repeats: true  },
                                ],
                            },
                            Segment { code: "FT1".into(), required: false, repeats: true  },
                            Segment { code: "CTI".into(), required: false, repeats: true  },
                            Group {
                                name: "SPECIMEN".into(),
                                required: false,
                                repeats: true,
                                elements: vec![
                                    Segment { code: "SPM".into(), required: true,  repeats: false },
                                    Segment { code: "OBX".into(), required: false, repeats: true  },
                                ],
                            },
                        ],
                    },
                ],
            },
            Segment { code: "DSC".into(), required: false, repeats: false },
        ],
    }
}

// --------- segments ---------------------------------------------------------

fn segments() -> Vec<SegmentSpec> {
    vec![
        seg_msh(), seg_evn(), seg_pid(), seg_pd1(), seg_nk1(),
        seg_pv1(), seg_pv2(), seg_obx(), seg_al1(), seg_dg1(),
        seg_gt1(), seg_in1(), seg_in2(), seg_in3(), seg_mrg(),
        seg_nte(), seg_orc(), seg_obr(),
        seg_ctd(), seg_cti(), seg_dsc(), seg_ft1(),
        seg_tq1(), seg_tq2(), seg_spm(), seg_sft(),
        seg_blg(), seg_rqd(), seg_rq1(), seg_rxo(),
        seg_ods(), seg_odt(),
    ]
}

// Placeholder stubs; populated in follow-up chunks.
fn seg_msh() -> SegmentSpec {
    seg("MSH", "Message Header", vec![
        f(1,  "Field Separator",             "ST",  true,  false),
        f(2,  "Encoding Characters",         "ST",  true,  false),
        f(3,  "Sending Application",         "HD",  false, false),
        f(4,  "Sending Facility",            "HD",  false, false),
        f(5,  "Receiving Application",       "HD",  false, false),
        f(6,  "Receiving Facility",          "HD",  false, false),
        f(7,  "Date/Time Of Message",        "TS",  true,  false),
        f(8,  "Security",                    "ST",  false, false),
        f(9,  "Message Type",                "MSG", true,  false),
        f(10, "Message Control ID",          "ST",  true,  false),
        f(11, "Processing ID",               "PT",  true,  false),
        f(12, "Version ID",                  "VID", true,  false),
        f(13, "Sequence Number",             "NM",  false, false),
        f(14, "Continuation Pointer",        "ST",  false, false),
        f(15, "Accept Acknowledgment Type",  "ID",  false, false),
        f(16, "Application Acknowledgment Type","ID", false, false),
        f(17, "Country Code",                "ID",  false, false),
        f(18, "Character Set",               "ID",  false, true),
        f(19, "Principal Language Of Message","CE", false, false),
        f(20, "Alternate Character Set Handling Scheme","ID", false, false),
        f(21, "Message Profile Identifier",  "EI",  false, true),
    ])
}

fn seg_evn() -> SegmentSpec {
    seg("EVN", "Event Type", vec![
        f(1, "Event Type Code",          "ID",  false, false),
        f(2, "Recorded Date/Time",       "TS",  true,  false),
        f(3, "Date/Time Planned Event",  "TS",  false, false),
        f(4, "Event Reason Code",        "IS",  false, false),
        f(5, "Operator ID",              "XCN", false, true),
        f(6, "Event Occurred",           "TS",  false, false),
        f(7, "Event Facility",           "HD",  false, false),
    ])
}
fn seg_pid() -> SegmentSpec {
    seg("PID", "Patient Identification", vec![
        f(1,  "Set ID - PID",              "SI",  false, false),
        f(2,  "Patient ID",                "CX",  false, false),
        f(3,  "Patient Identifier List",   "CX",  true,  true),
        f(4,  "Alternate Patient ID - PID","CX",  false, true),
        f(5,  "Patient Name",              "XPN", true,  true),
        f(6,  "Mother's Maiden Name",      "XPN", false, true),
        f(7,  "Date/Time of Birth",        "TS",  false, false),
        f(8,  "Administrative Sex",        "IS",  false, false),
        f(9,  "Patient Alias",             "XPN", false, true),
        f(10, "Race",                      "CE",  false, true),
        f(11, "Patient Address",           "XAD", false, true),
        f(12, "County Code",               "IS",  false, false),
        f(13, "Phone Number - Home",       "XTN", false, true),
        f(14, "Phone Number - Business",   "XTN", false, true),
        f(15, "Primary Language",          "CE",  false, false),
        f(16, "Marital Status",            "CE",  false, false),
        f(17, "Religion",                  "CE",  false, false),
        f(18, "Patient Account Number",    "CX",  false, false),
        f(19, "SSN Number - Patient",      "ST",  false, false),
        f(20, "Driver's License Number",   "DLN", false, false),
        f(21, "Mother's Identifier",       "CX",  false, true),
        f(22, "Ethnic Group",              "CE",  false, true),
        f(23, "Birth Place",               "ST",  false, false),
        f(24, "Multiple Birth Indicator",  "ID",  false, false),
        f(25, "Birth Order",               "NM",  false, false),
        f(26, "Citizenship",               "CE",  false, true),
        f(27, "Veterans Military Status",  "CE",  false, false),
        f(28, "Nationality",               "CE",  false, false),
        f(29, "Patient Death Date and Time","TS", false, false),
        f(30, "Patient Death Indicator",   "ID",  false, false),
        f(31, "Identity Unknown Indicator","ID",  false, false),
        f(32, "Identity Reliability Code", "IS",  false, true),
        f(33, "Last Update Date/Time",     "TS",  false, false),
        f(34, "Last Update Facility",      "HD",  false, false),
        f(35, "Species Code",              "CE",  false, false),
        f(36, "Breed Code",                "CE",  false, false),
        f(37, "Strain",                    "ST",  false, false),
        f(38, "Production Class Code",     "CE",  false, false),
        f(39, "Tribal Citizenship",        "CWE", false, true),
    ])
}

fn seg_pd1() -> SegmentSpec {
    seg("PD1", "Patient Additional Demographic", vec![
        f(1,  "Living Dependency",            "IS",  false, true),
        f(2,  "Living Arrangement",           "IS",  false, false),
        f(3,  "Patient Primary Facility",     "XON", false, true),
        f(4,  "Patient Primary Care Provider","XCN", false, true),
        f(5,  "Student Indicator",            "IS",  false, false),
        f(6,  "Handicap",                     "IS",  false, false),
        f(7,  "Living Will Code",             "IS",  false, false),
        f(8,  "Organ Donor Code",             "IS",  false, false),
        f(9,  "Separate Bill",                "ID",  false, false),
        f(10, "Duplicate Patient",            "CX",  false, true),
        f(11, "Publicity Code",               "CE",  false, false),
        f(12, "Protection Indicator",         "ID",  false, false),
        f(13, "Protection Indicator Effective Date","DT", false, false),
        f(14, "Place of Worship",             "XON", false, true),
        f(15, "Advance Directive Code",       "CE",  false, true),
        f(16, "Immunization Registry Status", "IS",  false, false),
        f(17, "Immunization Registry Status Effective Date","DT", false, false),
        f(18, "Publicity Code Effective Date","DT",  false, false),
        f(19, "Military Branch",              "IS",  false, false),
        f(20, "Military Rank/Grade",          "IS",  false, false),
        f(21, "Military Status",              "IS",  false, false),
    ])
}
fn seg_nk1() -> SegmentSpec {
    seg("NK1", "Next of Kin / Associated Parties", vec![
        f(1,  "Set ID - NK1",            "SI",  true,  false),
        f(2,  "Name",                    "XPN", false, true),
        f(3,  "Relationship",            "CE",  false, false),
        f(4,  "Address",                 "XAD", false, true),
        f(5,  "Phone Number",            "XTN", false, true),
        f(6,  "Business Phone Number",   "XTN", false, true),
        f(7,  "Contact Role",            "CE",  false, false),
        f(8,  "Start Date",              "DT",  false, false),
        f(9,  "End Date",                "DT",  false, false),
        f(10, "Next of Kin / Associated Parties Job Title","ST", false, false),
        f(11, "Next of Kin / Associated Parties Job Code/Class","JCC", false, false),
        f(12, "Next of Kin / Associated Parties Employee Number","CX", false, false),
        f(13, "Organization Name - NK1", "XON", false, true),
        f(14, "Marital Status",          "CE",  false, false),
        f(15, "Administrative Sex",      "IS",  false, false),
        f(16, "Date/Time of Birth",      "TS",  false, false),
        f(17, "Living Dependency",       "IS",  false, true),
        f(18, "Ambulatory Status",       "IS",  false, true),
        f(19, "Citizenship",             "CE",  false, true),
        f(20, "Primary Language",        "CE",  false, false),
        f(21, "Living Arrangement",      "IS",  false, false),
        f(22, "Publicity Code",          "CE",  false, false),
        f(23, "Protection Indicator",    "ID",  false, false),
        f(24, "Student Indicator",       "IS",  false, false),
        f(25, "Religion",                "CE",  false, false),
        f(26, "Mother's Maiden Name",    "XPN", false, true),
        f(27, "Nationality",             "CE",  false, false),
        f(28, "Ethnic Group",            "CE",  false, true),
        f(29, "Contact Reason",          "CE",  false, true),
        f(30, "Contact Person's Name",   "XPN", false, true),
        f(31, "Contact Person's Telephone Number","XTN", false, true),
        f(32, "Contact Person's Address","XAD", false, true),
        f(33, "Next of Kin/Associated Party's Identifiers","CX", false, true),
        f(34, "Job Status",              "IS",  false, false),
        f(35, "Race",                    "CE",  false, true),
        f(36, "Handicap",                "IS",  false, false),
        f(37, "Contact Person Social Security Number","ST", false, false),
        f(38, "Next of Kin Birth Place", "ST",  false, false),
        f(39, "VIP Indicator",           "IS",  false, false),
    ])
}
fn seg_pv1() -> SegmentSpec {
    seg("PV1", "Patient Visit", vec![
        f(1,  "Set ID - PV1",                "SI",  false, false),
        f(2,  "Patient Class",               "IS",  true,  false),
        f(3,  "Assigned Patient Location",   "PL",  false, false),
        f(4,  "Admission Type",              "IS",  false, false),
        f(5,  "Preadmit Number",             "CX",  false, false),
        f(6,  "Prior Patient Location",      "PL",  false, false),
        f(7,  "Attending Doctor",            "XCN", false, true),
        f(8,  "Referring Doctor",            "XCN", false, true),
        f(9,  "Consulting Doctor",           "XCN", false, true),
        f(10, "Hospital Service",            "IS",  false, false),
        f(11, "Temporary Location",          "PL",  false, false),
        f(12, "Preadmit Test Indicator",     "IS",  false, false),
        f(13, "Re-admission Indicator",      "IS",  false, false),
        f(14, "Admit Source",                "IS",  false, false),
        f(15, "Ambulatory Status",           "IS",  false, true),
        f(16, "VIP Indicator",               "IS",  false, false),
        f(17, "Admitting Doctor",            "XCN", false, true),
        f(18, "Patient Type",                "IS",  false, false),
        f(19, "Visit Number",                "CX",  false, false),
        f(20, "Financial Class",             "FC",  false, true),
        f(21, "Charge Price Indicator",      "IS",  false, false),
        f(22, "Courtesy Code",               "IS",  false, false),
        f(23, "Credit Rating",               "IS",  false, false),
        f(24, "Contract Code",               "IS",  false, true),
        f(25, "Contract Effective Date",     "DT",  false, true),
        f(26, "Contract Amount",             "NM",  false, true),
        f(27, "Contract Period",             "NM",  false, true),
        f(28, "Interest Code",               "IS",  false, false),
        f(29, "Transfer to Bad Debt Code",   "IS",  false, false),
        f(30, "Transfer to Bad Debt Date",   "DT",  false, false),
        f(31, "Bad Debt Agency Code",        "IS",  false, false),
        f(32, "Bad Debt Transfer Amount",    "NM",  false, false),
        f(33, "Bad Debt Recovery Amount",    "NM",  false, false),
        f(34, "Delete Account Indicator",    "IS",  false, false),
        f(35, "Delete Account Date",         "DT",  false, false),
        f(36, "Discharge Disposition",       "IS",  false, false),
        f(37, "Discharged to Location",      "DLD", false, false),
        f(38, "Diet Type",                   "CE",  false, false),
        f(39, "Servicing Facility",          "IS",  false, false),
        f(40, "Bed Status",                  "IS",  false, false),
        f(41, "Account Status",              "IS",  false, false),
        f(42, "Pending Location",            "PL",  false, false),
        f(43, "Prior Temporary Location",    "PL",  false, false),
        f(44, "Admit Date/Time",             "TS",  false, false),
        f(45, "Discharge Date/Time",         "TS",  false, true),
        f(46, "Current Patient Balance",     "NM",  false, false),
        f(47, "Total Charges",               "NM",  false, false),
        f(48, "Total Adjustments",           "NM",  false, false),
        f(49, "Total Payments",              "NM",  false, false),
        f(50, "Alternate Visit ID",          "CX",  false, false),
        f(51, "Visit Indicator",             "IS",  false, false),
        f(52, "Other Healthcare Provider",   "XCN", false, true),
    ])
}

fn seg_pv2() -> SegmentSpec {
    seg("PV2", "Patient Visit - Additional Information", vec![
        f(1,  "Prior Pending Location",        "PL",  false, false),
        f(2,  "Accommodation Code",            "CE",  false, false),
        f(3,  "Admit Reason",                  "CE",  false, false),
        f(4,  "Transfer Reason",               "CE",  false, false),
        f(5,  "Patient Valuables",             "ST",  false, true),
        f(6,  "Patient Valuables Location",    "ST",  false, false),
        f(7,  "Visit User Code",               "IS",  false, true),
        f(8,  "Expected Admit Date/Time",      "TS",  false, false),
        f(9,  "Expected Discharge Date/Time",  "TS",  false, false),
        f(10, "Estimated Length of Inpatient Stay","NM", false, false),
        f(11, "Actual Length of Inpatient Stay","NM",  false, false),
        f(12, "Visit Description",             "ST",  false, false),
        f(13, "Referral Source Code",          "XCN", false, true),
        f(14, "Previous Service Date",         "DT",  false, false),
        f(15, "Employment Illness Related Indicator","ID", false, false),
        f(16, "Purge Status Code",             "IS",  false, false),
        f(17, "Purge Status Date",             "DT",  false, false),
        f(18, "Special Program Code",          "IS",  false, false),
        f(19, "Retention Indicator",           "ID",  false, false),
        f(20, "Expected Number of Insurance Plans","NM", false, false),
        f(21, "Visit Publicity Code",          "IS",  false, false),
        f(22, "Visit Protection Indicator",    "ID",  false, false),
        f(23, "Clinic Organization Name",      "XON", false, true),
        f(24, "Patient Status Code",           "IS",  false, false),
        f(25, "Visit Priority Code",           "IS",  false, false),
        f(26, "Previous Treatment Date",       "DT",  false, false),
        f(27, "Expected Discharge Disposition","IS",  false, false),
        f(28, "Signature on File Date",        "DT",  false, false),
        f(29, "First Similar Illness Date",    "DT",  false, false),
        f(30, "Patient Charge Adjustment Code","CE",  false, false),
        f(31, "Recurring Service Code",        "IS",  false, false),
        f(32, "Billing Media Code",            "ID",  false, false),
        f(33, "Expected Surgery Date and Time","TS",  false, false),
        f(34, "Military Partnership Code",     "ID",  false, false),
        f(35, "Military Non-Availability Code","ID",  false, false),
        f(36, "Newborn Baby Indicator",        "ID",  false, false),
        f(37, "Baby Detained Indicator",       "ID",  false, false),
        f(38, "Mode of Arrival Code",          "CE",  false, false),
        f(39, "Recreational Drug Use Code",    "CE",  false, true),
        f(40, "Admission Level of Care Code",  "CE",  false, false),
        f(41, "Precaution Code",               "CE",  false, true),
        f(42, "Patient Condition Code",        "CE",  false, false),
        f(43, "Living Will Code",              "IS",  false, false),
        f(44, "Organ Donor Code",              "IS",  false, false),
        f(45, "Advance Directive Code",        "CE",  false, true),
        f(46, "Patient Status Effective Date", "DT",  false, false),
        f(47, "Expected LOA Return Date/Time", "TS",  false, false),
        f(48, "Expected Pre-admission Testing Date/Time","TS", false, false),
        f(49, "Notify Clergy Code",            "IS",  false, true),
    ])
}
fn seg_obx() -> SegmentSpec {
    seg("OBX", "Observation / Result", vec![
        f(1,  "Set ID - OBX",               "SI",     false, false),
        f(2,  "Value Type",                 "ID",     false, false),
        f(3,  "Observation Identifier",     "CE",     true,  false),
        f(4,  "Observation Sub-ID",         "ST",     false, false),
        f(5,  "Observation Value",          "VARIES", false, true),
        f(6,  "Units",                      "CE",     false, false),
        f(7,  "References Range",           "ST",     false, false),
        f(8,  "Abnormal Flags",             "IS",     false, true),
        f(9,  "Probability",                "NM",     false, false),
        f(10, "Nature of Abnormal Test",    "ID",     false, true),
        f(11, "Observation Result Status",  "ID",     true,  false),
        f(12, "Effective Date of Reference Range","TS", false, false),
        f(13, "User Defined Access Checks", "ST",     false, false),
        f(14, "Date/Time of the Observation","TS",    false, false),
        f(15, "Producer's ID",              "CE",     false, false),
        f(16, "Responsible Observer",       "XCN",    false, true),
        f(17, "Observation Method",         "CE",     false, true),
        f(18, "Equipment Instance Identifier","EI",   false, true),
        f(19, "Date/Time of the Analysis",  "TS",     false, false),
    ])
}
fn seg_al1() -> SegmentSpec {
    seg("AL1", "Patient Allergy Information", vec![
        f(1, "Set ID - AL1",              "SI", true,  false),
        f(2, "Allergen Type Code",        "CE", false, false),
        f(3, "Allergen Code/Mnemonic/Description","CE", true, false),
        f(4, "Allergy Severity Code",     "CE", false, false),
        f(5, "Allergy Reaction Code",     "ST", false, true),
        f(6, "Identification Date",       "DT", false, false),
    ])
}

fn seg_dg1() -> SegmentSpec {
    seg("DG1", "Diagnosis", vec![
        f(1,  "Set ID - DG1",             "SI",  true,  false),
        f(2,  "Diagnosis Coding Method",  "ID",  false, false),
        f(3,  "Diagnosis Code",           "CE",  false, false),
        f(4,  "Diagnosis Description",    "ST",  false, false),
        f(5,  "Diagnosis Date/Time",      "TS",  false, false),
        f(6,  "Diagnosis Type",           "IS",  true,  false),
        f(7,  "Major Diagnostic Category","CE",  false, false),
        f(8,  "Diagnostic Related Group", "CE",  false, false),
        f(9,  "DRG Approval Indicator",   "ID",  false, false),
        f(10, "DRG Grouper Review Code",  "IS",  false, false),
        f(11, "Outlier Type",             "CE",  false, false),
        f(12, "Outlier Days",             "NM",  false, false),
        f(13, "Outlier Cost",             "CP",  false, false),
        f(14, "Grouper Version And Type", "ST",  false, false),
        f(15, "Diagnosis Priority",       "ID",  false, false),
        f(16, "Diagnosing Clinician",     "XCN", false, true),
        f(17, "Diagnosis Classification", "IS",  false, false),
        f(18, "Confidential Indicator",   "ID",  false, false),
        f(19, "Attestation Date/Time",    "TS",  false, false),
        f(20, "Diagnosis Identifier",     "EI",  false, false),
        f(21, "Diagnosis Action Code",    "ID",  false, false),
    ])
}
fn seg_gt1() -> SegmentSpec {
    seg("GT1", "Guarantor", vec![
        f(1,  "Set ID - GT1",            "SI",  true,  false),
        f(2,  "Guarantor Number",        "CX",  false, true),
        f(3,  "Guarantor Name",          "XPN", true,  true),
        f(4,  "Guarantor Spouse Name",   "XPN", false, true),
        f(5,  "Guarantor Address",       "XAD", false, true),
        f(6,  "Guarantor Ph Num - Home", "XTN", false, true),
        f(7,  "Guarantor Ph Num - Business","XTN", false, true),
        f(8,  "Guarantor Date/Time of Birth","TS", false, false),
        f(9,  "Guarantor Administrative Sex","IS", false, false),
        f(10, "Guarantor Type",          "IS",  false, false),
        f(11, "Guarantor Relationship",  "CE",  false, false),
        f(12, "Guarantor SSN",           "ST",  false, false),
        f(13, "Guarantor Date - Begin",  "DT",  false, false),
        f(14, "Guarantor Date - End",    "DT",  false, false),
        f(15, "Guarantor Priority",      "NM",  false, false),
        f(16, "Guarantor Employer Name", "XPN", false, true),
        f(17, "Guarantor Employer Address","XAD", false, true),
        f(18, "Guarantor Employer Phone Number","XTN", false, true),
        f(19, "Guarantor Employee ID Number","CX", false, true),
        f(20, "Guarantor Employment Status","IS", false, false),
        f(21, "Guarantor Organization Name","XON", false, true),
        f(22, "Guarantor Billing Hold Flag","ID", false, false),
        f(23, "Guarantor Credit Rating Code","CE", false, false),
        f(24, "Guarantor Death Date And Time","TS", false, false),
        f(25, "Guarantor Death Flag",    "ID",  false, false),
        f(26, "Guarantor Charge Adjustment Code","CE", false, false),
        f(27, "Guarantor Household Annual Income","CP", false, false),
        f(28, "Guarantor Household Size","NM",  false, false),
        f(29, "Guarantor Employer ID Number","CX", false, true),
        f(30, "Guarantor Marital Status Code","CE", false, false),
        f(31, "Guarantor Hire Effective Date","DT", false, false),
        f(32, "Employment Stop Date",    "DT",  false, false),
        f(33, "Living Dependency",       "IS",  false, false),
        f(34, "Ambulatory Status",       "IS",  false, true),
        f(35, "Citizenship",             "CE",  false, true),
        f(36, "Primary Language",        "CE",  false, false),
        f(37, "Living Arrangement",      "IS",  false, false),
        f(38, "Publicity Code",          "CE",  false, false),
        f(39, "Protection Indicator",    "ID",  false, false),
        f(40, "Student Indicator",       "IS",  false, false),
        f(41, "Religion",                "CE",  false, false),
        f(42, "Mother's Maiden Name",    "XPN", false, true),
        f(43, "Nationality",             "CE",  false, false),
        f(44, "Ethnic Group",            "CE",  false, true),
        f(45, "Contact Person's Name",   "XPN", false, true),
        f(46, "Contact Person's Telephone Number","XTN", false, true),
        f(47, "Contact Reason",          "CE",  false, false),
        f(48, "Contact Relationship Code","IS", false, false),
        f(49, "Job Title",               "ST",  false, false),
        f(50, "Job Code/Class",          "JCC", false, false),
        f(51, "Guarantor Employer's Organization Name","XON", false, true),
        f(52, "Handicap",                "IS",  false, false),
        f(53, "Job Status",              "IS",  false, false),
        f(54, "Guarantor Financial Class","FC", false, false),
        f(55, "Guarantor Race",          "CE",  false, true),
        f(56, "Guarantor Birth Place",   "ST",  false, false),
        f(57, "VIP Indicator",           "IS",  false, false),
    ])
}
fn seg_in1() -> SegmentSpec {
    seg("IN1", "Insurance", vec![
        f(1,  "Set ID - IN1",              "SI",  true,  false),
        f(2,  "Insurance Plan ID",         "CE",  true,  false),
        f(3,  "Insurance Company ID",      "CX",  true,  true),
        f(4,  "Insurance Company Name",    "XON", false, true),
        f(5,  "Insurance Company Address", "XAD", false, true),
        f(6,  "Insurance Co Contact Person","XPN",false, true),
        f(7,  "Insurance Co Phone Number", "XTN", false, true),
        f(8,  "Group Number",              "ST",  false, false),
        f(9,  "Group Name",                "XON", false, true),
        f(10, "Insured's Group Emp ID",    "CX",  false, true),
        f(11, "Insured's Group Emp Name",  "XON", false, true),
        f(12, "Plan Effective Date",       "DT",  false, false),
        f(13, "Plan Expiration Date",      "DT",  false, false),
        f(14, "Authorization Information", "AUI", false, false),
        f(15, "Plan Type",                 "IS",  false, false),
        f(16, "Name Of Insured",           "XPN", false, true),
        f(17, "Insured's Relationship To Patient","CE", false, false),
        f(18, "Insured's Date Of Birth",   "TS",  false, false),
        f(19, "Insured's Address",         "XAD", false, true),
        f(20, "Assignment Of Benefits",    "IS",  false, false),
        f(21, "Coordination Of Benefits",  "IS",  false, false),
        f(22, "Coord Of Ben Priority",     "ST",  false, false),
        f(23, "Notice Of Admission Flag",  "ID",  false, false),
        f(24, "Notice Of Admission Date",  "DT",  false, false),
        f(25, "Report Of Eligibility Flag","ID",  false, false),
        f(26, "Report Of Eligibility Date","DT",  false, false),
        f(27, "Release Information Code",  "IS",  false, false),
        f(28, "Pre-Admit Cert (PAC)",      "ST",  false, false),
        f(29, "Verification Date/Time",    "TS",  false, false),
        f(30, "Verification By",           "XCN", false, true),
        f(31, "Type Of Agreement Code",    "IS",  false, false),
        f(32, "Billing Status",            "IS",  false, false),
        f(33, "Lifetime Reserve Days",     "NM",  false, false),
        f(34, "Delay Before L.R. Day",     "NM",  false, false),
        f(35, "Company Plan Code",         "IS",  false, false),
        f(36, "Policy Number",             "ST",  false, false),
        f(37, "Policy Deductible",         "CP",  false, false),
        f(38, "Policy Limit - Amount",     "CP",  false, false),
        f(39, "Policy Limit - Days",       "NM",  false, false),
        f(40, "Room Rate - Semi-Private",  "CP",  false, false),
        f(41, "Room Rate - Private",       "CP",  false, false),
        f(42, "Insured's Employment Status","CE", false, false),
        f(43, "Insured's Administrative Sex","IS", false, false),
        f(44, "Insured's Employer's Address","XAD", false, true),
        f(45, "Verification Status",       "ST",  false, false),
        f(46, "Prior Insurance Plan ID",   "IS",  false, false),
        f(47, "Coverage Type",             "IS",  false, false),
        f(48, "Handicap",                  "IS",  false, false),
        f(49, "Insured's ID Number",       "CX",  false, true),
        f(50, "Signature Code",            "IS",  false, false),
        f(51, "Signature Code Date",       "DT",  false, false),
        f(52, "Insured's Birth Place",     "ST",  false, false),
        f(53, "VIP Indicator",             "IS",  false, false),
    ])
}
fn seg_in2() -> SegmentSpec {
    seg("IN2", "Insurance Additional Information", vec![
        f(1,  "Insured's Employee ID",        "CX",  false, true),
        f(2,  "Insured's Social Security Number","ST", false, false),
        f(3,  "Insured's Employer's Name and ID","XCN", false, true),
        f(4,  "Employer Information Data",    "IS",  false, false),
        f(5,  "Mail Claim Party",             "IS",  false, true),
        f(6,  "Medicare Health Ins Card Number","ST", false, false),
        f(7,  "Medicaid Case Name",           "XPN", false, true),
        f(8,  "Medicaid Case Number",         "ST",  false, false),
        f(9,  "Military Sponsor Name",        "XPN", false, true),
        f(10, "Military ID Number",           "ST",  false, false),
        f(11, "Dependent Of Military Recipient","CE",false, false),
        f(12, "Military Organization",        "ST",  false, false),
        f(13, "Military Station",             "ST",  false, false),
        f(14, "Military Service",             "IS",  false, false),
        f(15, "Military Rank/Grade",          "IS",  false, false),
        f(16, "Military Status",              "IS",  false, false),
        f(17, "Military Retire Date",         "DT",  false, false),
        f(18, "Military Non-Avail Cert On File","ID",false, false),
        f(19, "Baby Coverage",                "ID",  false, false),
        f(20, "Combine Baby Bill",            "ID",  false, false),
        f(21, "Blood Deductible",             "ST",  false, false),
        f(22, "Special Coverage Approval Name","XPN",false, true),
        f(23, "Special Coverage Approval Title","ST",false, false),
        f(24, "Non-Covered Insurance Code",   "IS",  false, true),
        f(25, "Payor ID",                     "CX",  false, true),
        f(26, "Payor Subscriber ID",          "CX",  false, true),
        f(27, "Eligibility Source",           "IS",  false, false),
        f(28, "Room Coverage Type/Amount",    "RMC", false, true),
        f(29, "Policy Type/Amount",           "PTA", false, true),
        f(30, "Daily Deductible",             "DDI", false, false),
        f(31, "Living Dependency",            "IS",  false, false),
        f(32, "Ambulatory Status",            "IS",  false, true),
        f(33, "Citizenship",                  "CE",  false, true),
        f(34, "Primary Language",             "CE",  false, false),
        f(35, "Living Arrangement",           "IS",  false, false),
        f(36, "Publicity Code",               "CE",  false, false),
        f(37, "Protection Indicator",         "ID",  false, false),
        f(38, "Student Indicator",            "IS",  false, false),
        f(39, "Religion",                     "CE",  false, false),
        f(40, "Mother's Maiden Name",         "XPN", false, true),
        f(41, "Nationality",                  "CE",  false, false),
        f(42, "Ethnic Group",                 "CE",  false, true),
        f(43, "Marital Status",               "CE",  false, true),
        f(44, "Insured's Employment Start Date","DT",false, false),
        f(45, "Employment Stop Date",         "DT",  false, false),
        f(46, "Job Title",                    "ST",  false, false),
        f(47, "Job Code/Class",               "JCC", false, false),
        f(48, "Job Status",                   "IS",  false, false),
        f(49, "Employer Contact Person Name", "XPN", false, true),
        f(50, "Employer Contact Person Phone Number","XTN", false, true),
        f(51, "Employer Contact Reason",      "IS",  false, false),
        f(52, "Insured's Contact Person's Name","XPN",false,true),
        f(53, "Insured's Contact Person Telephone Number","XTN",false,true),
        f(54, "Insured's Contact Person Reason","IS",false, true),
        f(55, "Relationship To The Patient Start Date","DT",false,false),
        f(56, "Relationship To The Patient Stop Date","DT",false,true),
        f(57, "Insurance Co Contact Reason",  "IS",  false, false),
        f(58, "Insurance Co Contact Phone Number","XTN", false, false),
        f(59, "Policy Scope",                 "IS",  false, false),
        f(60, "Policy Source",                "IS",  false, false),
        f(61, "Patient Member Number",        "CX",  false, false),
        f(62, "Guarantor's Relationship to Insured","CE", false, false),
        f(63, "Insured's Phone Number - Home","XTN", false, true),
        f(64, "Insured's Employer Phone Number","XTN",false,true),
        f(65, "Military Handicapped Program", "CE",  false, false),
        f(66, "Suspend Flag",                 "ID",  false, false),
        f(67, "Copay Limit Flag",             "ID",  false, false),
        f(68, "Stoploss Limit Flag",          "ID",  false, false),
        f(69, "Insured Organization Name And ID","XON", false, true),
        f(70, "Insured Employer Organization Name And ID","XON", false, true),
        f(71, "Race",                         "CE",  false, true),
        f(72, "Patient's Relationship to Insured","CE", false, false),
    ])
}

fn seg_in3() -> SegmentSpec {
    seg("IN3", "Insurance Additional Information, Certification", vec![
        f(1,  "Set ID - IN3",             "SI",  true,  false),
        f(2,  "Certification Number",     "CX",  false, false),
        f(3,  "Certified By",             "XCN", false, true),
        f(4,  "Certification Required",   "ID",  false, false),
        f(5,  "Penalty",                  "MOP", false, false),
        f(6,  "Certification Date/Time",  "TS",  false, false),
        f(7,  "Certification Modify Date/Time","TS", false, false),
        f(8,  "Operator",                 "XCN", false, true),
        f(9,  "Certification Begin Date", "DT",  false, false),
        f(10, "Certification End Date",   "DT",  false, false),
        f(11, "Days",                     "DTN", false, false),
        f(12, "Non-Concur Code/Description","CE",false, false),
        f(13, "Non-Concur Effective Date/Time","TS", false, false),
        f(14, "Physician Reviewer",       "XCN", false, true),
        f(15, "Certification Contact",    "ST",  false, false),
        f(16, "Certification Contact Phone Number","XTN", false, true),
        f(17, "Appeal Reason",            "CE",  false, false),
        f(18, "Certification Agency",     "CE",  false, false),
        f(19, "Certification Agency Phone Number","XTN", false, true),
        f(20, "Pre-Certification Requirement","ICD", false, true),
        f(21, "Case Manager",             "ST",  false, false),
        f(22, "Second Opinion Date",      "DT",  false, false),
        f(23, "Second Opinion Status",    "IS",  false, false),
        f(24, "Second Opinion Documentation Received","IS", false, true),
        f(25, "Second Opinion Physician", "XCN", false, true),
    ])
}
fn seg_nte() -> SegmentSpec {
    seg("NTE", "Notes and Comments", vec![
        f(1, "Set ID - NTE",     "SI", false, false),
        f(2, "Source of Comment","ID", false, false),
        f(3, "Comment",          "FT", false, true),
        f(4, "Comment Type",     "CE", false, false),
    ])
}

fn seg_mrg() -> SegmentSpec {
    seg("MRG", "Merge Patient Information", vec![
        f(1, "Prior Patient Identifier List","CX",  true,  true),
        f(2, "Prior Alternate Patient ID",   "CX",  false, true),
        f(3, "Prior Patient Account Number", "CX",  false, false),
        f(4, "Prior Patient ID",             "CX",  false, false),
        f(5, "Prior Visit Number",           "CX",  false, false),
        f(6, "Prior Alternate Visit ID",     "CX",  false, false),
        f(7, "Prior Patient Name",           "XPN", false, true),
    ])
}

fn seg_dsc() -> SegmentSpec {
    seg("DSC", "Continuation Pointer", vec![
        f(1, "Continuation Pointer",    "ST", false, false),
        f(2, "Continuation Style",      "ID", false, false),
    ])
}

fn seg_sft() -> SegmentSpec {
    seg("SFT", "Software Segment", vec![
        f(1, "Software Vendor Organization","XON", true,  false),
        f(2, "Software Certified Version or Release Number","ST", true, false),
        f(3, "Software Product Name",   "ST", true,  false),
        f(4, "Software Binary ID",      "ST", true,  false),
        f(5, "Software Product Information","TX", false, false),
        f(6, "Software Install Date",   "TS", false, false),
    ])
}
fn seg_orc() -> SegmentSpec {
    seg("ORC", "Common Order", vec![
        f(1,  "Order Control",             "ID",  true,  false),
        f(2,  "Placer Order Number",       "EI",  false, false),
        f(3,  "Filler Order Number",       "EI",  false, false),
        f(4,  "Placer Group Number",       "EI",  false, false),
        f(5,  "Order Status",              "ID",  false, false),
        f(6,  "Response Flag",             "ID",  false, false),
        f(7,  "Quantity/Timing",           "TQ",  false, true),
        f(8,  "Parent",                    "EIP", false, false),
        f(9,  "Date/Time of Transaction",  "TS",  false, false),
        f(10, "Entered By",                "XCN", false, true),
        f(11, "Verified By",               "XCN", false, true),
        f(12, "Ordering Provider",         "XCN", false, true),
        f(13, "Enterer's Location",        "PL",  false, false),
        f(14, "Call Back Phone Number",    "XTN", false, true),
        f(15, "Order Effective Date/Time", "TS",  false, false),
        f(16, "Order Control Code Reason", "CE",  false, false),
        f(17, "Entering Organization",     "CE",  false, false),
        f(18, "Entering Device",           "CE",  false, false),
        f(19, "Action By",                 "XCN", false, true),
        f(20, "Advanced Beneficiary Notice Code","CE", false, false),
        f(21, "Ordering Facility Name",    "XON", false, true),
        f(22, "Ordering Facility Address", "XAD", false, true),
        f(23, "Ordering Facility Phone Number","XTN", false, true),
        f(24, "Ordering Provider Address", "XAD", false, true),
        f(25, "Order Status Modifier",     "CWE", false, false),
        f(26, "Advanced Beneficiary Notice Override Reason","CWE", false, false),
        f(27, "Filler's Expected Availability Date/Time","TS", false, false),
        f(28, "Confidentiality Code",      "CWE", false, false),
        f(29, "Order Type",                "CWE", false, false),
        f(30, "Enterer Authorization Mode","CNE", false, false),
    ])
}
fn seg_obr() -> SegmentSpec {
    seg("OBR", "Observation Request", vec![
        f(1,  "Set ID - OBR",              "SI",  false, false),
        f(2,  "Placer Order Number",       "EI",  false, false),
        f(3,  "Filler Order Number",       "EI",  false, false),
        f(4,  "Universal Service Identifier","CE", true, false),
        f(5,  "Priority - OBR",            "ID",  false, false),
        f(6,  "Requested Date/Time",       "TS",  false, false),
        f(7,  "Observation Date/Time",     "TS",  false, false),
        f(8,  "Observation End Date/Time", "TS",  false, false),
        f(9,  "Collection Volume",         "CQ",  false, false),
        f(10, "Collector Identifier",      "XCN", false, true),
        f(11, "Specimen Action Code",      "ID",  false, false),
        f(12, "Danger Code",               "CE",  false, false),
        f(13, "Relevant Clinical Information","ST", false, false),
        f(14, "Specimen Received Date/Time","TS", false, false),
        f(15, "Specimen Source",           "SPS", false, false),
        f(16, "Ordering Provider",         "XCN", false, true),
        f(17, "Order Callback Phone Number","XTN", false, true),
        f(18, "Placer Field 1",            "ST",  false, false),
        f(19, "Placer Field 2",            "ST",  false, false),
        f(20, "Filler Field 1",            "ST",  false, false),
        f(21, "Filler Field 2",            "ST",  false, false),
        f(22, "Results Rpt/Status Chng - Date/Time","TS", false, false),
        f(23, "Charge to Practice",        "MOC", false, false),
        f(24, "Diagnostic Serv Sect ID",   "ID",  false, false),
        f(25, "Result Status",             "ID",  false, false),
        f(26, "Parent Result",             "PRL", false, false),
        f(27, "Quantity/Timing",           "TQ",  false, true),
        f(28, "Result Copies To",          "XCN", false, true),
        f(29, "Parent",                    "EIP", false, false),
        f(30, "Transportation Mode",       "ID",  false, false),
        f(31, "Reason for Study",          "CE",  false, true),
        f(32, "Principal Result Interpreter","NDL",false, false),
        f(33, "Assistant Result Interpreter","NDL",false, true),
        f(34, "Technician",                "NDL", false, true),
        f(35, "Transcriptionist",          "NDL", false, true),
        f(36, "Scheduled Date/Time",       "TS",  false, false),
        f(37, "Number of Sample Containers","NM", false, false),
        f(38, "Transport Logistics of Collected Sample","CE", false, true),
        f(39, "Collector's Comment",       "CE",  false, true),
        f(40, "Transport Arrangement Responsibility","CE", false, false),
        f(41, "Transport Arranged",        "ID",  false, false),
        f(42, "Escort Required",           "ID",  false, false),
        f(43, "Planned Patient Transport Comment","CE", false, true),
        f(44, "Procedure Code",            "CE",  false, false),
        f(45, "Procedure Code Modifier",   "CE",  false, true),
        f(46, "Placer Supplemental Service Information","CE", false, true),
        f(47, "Filler Supplemental Service Information","CE", false, true),
        f(48, "Medically Necessary Duplicate Procedure Reason","CWE", false, false),
        f(49, "Result Handling",           "IS",  false, false),
        f(50, "Parent Universal Service Identifier","CWE", false, false),
    ])
}
fn seg_ctd() -> SegmentSpec {
    seg("CTD", "Contact Data", vec![
        f(1, "Contact Role",                 "CE",  true,  true),
        f(2, "Contact Name",                 "XPN", false, true),
        f(3, "Contact Address",              "XAD", false, true),
        f(4, "Contact Location",             "PL",  false, false),
        f(5, "Contact Communication Information","XTN", false, true),
        f(6, "Preferred Method of Contact",  "CE",  false, false),
        f(7, "Contact Identifiers",          "PLN", false, true),
    ])
}

fn seg_cti() -> SegmentSpec {
    seg("CTI", "Clinical Trial Identification", vec![
        f(1, "Sponsor Study ID",             "EI", true,  false),
        f(2, "Study Phase Identifier",       "CE", false, false),
        f(3, "Study Scheduled Time Point",   "CE", false, false),
    ])
}
fn seg_ft1() -> SegmentSpec {
    seg("FT1", "Financial Transaction", vec![
        f(1,  "Set ID - FT1",              "SI",  false, false),
        f(2,  "Transaction ID",            "ST",  false, false),
        f(3,  "Transaction Batch ID",      "ST",  false, false),
        f(4,  "Transaction Date",          "DR",  true,  false),
        f(5,  "Transaction Posting Date",  "TS",  false, false),
        f(6,  "Transaction Type",          "IS",  true,  false),
        f(7,  "Transaction Code",          "CE",  true,  false),
        f(8,  "Transaction Description",   "ST",  false, false),
        f(9,  "Transaction Description - Alt","ST",false, false),
        f(10, "Transaction Quantity",      "NM",  false, false),
        f(11, "Transaction Amount - Extended","CP",false, false),
        f(12, "Transaction Amount - Unit", "CP",  false, false),
        f(13, "Department Code",           "CE",  false, false),
        f(14, "Insurance Plan ID",         "CE",  false, false),
        f(15, "Insurance Amount",          "CP",  false, false),
        f(16, "Assigned Patient Location", "PL",  false, false),
        f(17, "Fee Schedule",              "IS",  false, false),
        f(18, "Patient Type",              "IS",  false, false),
        f(19, "Diagnosis Code - FT1",      "CE",  false, true),
        f(20, "Performed By Code",         "XCN", false, true),
        f(21, "Ordered By Code",           "XCN", false, true),
        f(22, "Unit Cost",                 "CP",  false, false),
        f(23, "Filler Order Number",       "EI",  false, false),
        f(24, "Entered By Code",           "XCN", false, true),
        f(25, "Procedure Code",            "CE",  false, false),
        f(26, "Procedure Code Modifier",   "CE",  false, true),
        f(27, "Advanced Beneficiary Notice Code","CE", false, false),
        f(28, "Medically Necessary Duplicate Procedure Reason","CWE", false, false),
        f(29, "NDC Code",                  "CNE", false, false),
        f(30, "Payment Reference ID",      "CX",  false, false),
        f(31, "Transaction Reference Key", "SI",  false, true),
    ])
}
fn seg_tq1() -> SegmentSpec {
    seg("TQ1", "Timing / Quantity", vec![
        f(1,  "Set ID - TQ1",          "SI",  false, false),
        f(2,  "Quantity",              "CQ",  false, false),
        f(3,  "Repeat Pattern",        "RPT", false, true),
        f(4,  "Explicit Time",         "TM",  false, true),
        f(5,  "Relative Time and Units","CQ", false, true),
        f(6,  "Service Duration",      "CQ",  false, false),
        f(7,  "Start date/time",       "TS",  false, false),
        f(8,  "End date/time",         "TS",  false, false),
        f(9,  "Priority",              "CWE", false, true),
        f(10, "Condition text",        "TX",  false, false),
        f(11, "Text instruction",      "TX",  false, false),
        f(12, "Conjunction",           "ID",  false, false),
        f(13, "Occurrence duration",   "CQ",  false, false),
        f(14, "Total occurrences",     "NM",  false, false),
    ])
}

fn seg_tq2() -> SegmentSpec {
    seg("TQ2", "Timing / Quantity Relationship", vec![
        f(1,  "Set ID - TQ2",             "SI", false, false),
        f(2,  "Sequence Results Flag",    "ID", false, false),
        f(3,  "Related Placer Number",    "EI", false, true),
        f(4,  "Related Filler Number",    "EI", false, true),
        f(5,  "Related Placer Group Number","EI",false,true),
        f(6,  "Sequence Condition Code",  "ID", false, false),
        f(7,  "Cyclic Entry/Exit Indicator","ID", false, false),
        f(8,  "Sequence Condition Time Interval","CQ", false, false),
        f(9,  "Cyclic Group Maximum Number of Repeats","NM", false, false),
        f(10, "Special Service Request Relationship","ID", false, false),
    ])
}

fn seg_spm() -> SegmentSpec {
    seg("SPM", "Specimen", vec![
        f(1,  "Set ID - SPM",                   "SI",  false, false),
        f(2,  "Specimen ID",                    "EIP", false, false),
        f(3,  "Specimen Parent IDs",            "EIP", false, true),
        f(4,  "Specimen Type",                  "CWE", true,  false),
        f(5,  "Specimen Type Modifier",         "CWE", false, true),
        f(6,  "Specimen Additives",             "CWE", false, true),
        f(7,  "Specimen Collection Method",     "CWE", false, false),
        f(8,  "Specimen Source Site",           "CWE", false, false),
        f(9,  "Specimen Source Site Modifier",  "CWE", false, true),
        f(10, "Specimen Collection Site",       "CWE", false, false),
        f(11, "Specimen Role",                  "CWE", false, true),
        f(12, "Specimen Collection Amount",     "CQ",  false, false),
        f(13, "Grouped Specimen Count",         "NM",  false, false),
        f(14, "Specimen Description",           "ST",  false, true),
        f(15, "Specimen Handling Code",         "CWE", false, true),
        f(16, "Specimen Risk Code",             "CWE", false, true),
        f(17, "Specimen Collection Date/Time",  "DR",  false, false),
        f(18, "Specimen Received Date/Time",    "TS",  false, false),
        f(19, "Specimen Expiration Date/Time",  "TS",  false, false),
        f(20, "Specimen Availability",          "ID",  false, false),
        f(21, "Specimen Reject Reason",         "CWE", false, true),
        f(22, "Specimen Quality",               "CWE", false, false),
        f(23, "Specimen Appropriateness",       "CWE", false, false),
        f(24, "Specimen Condition",             "CWE", false, true),
        f(25, "Specimen Current Quantity",      "CQ",  false, false),
        f(26, "Number of Specimen Containers",  "NM",  false, false),
        f(27, "Container Type",                 "CWE", false, false),
        f(28, "Container Condition",            "CWE", false, false),
        f(29, "Specimen Child Role",            "CWE", false, false),
    ])
}
fn seg_blg() -> SegmentSpec {
    seg("BLG", "Billing", vec![
        f(1, "When to Charge",     "CCD", false, false),
        f(2, "Charge Type",        "ID",  false, false),
        f(3, "Account ID",         "CX",  false, false),
        f(4, "Charge Type Reason", "CWE", false, false),
    ])
}

fn seg_rqd() -> SegmentSpec {
    seg("RQD", "Requisition Detail", vec![
        f(1,  "Requisition Line Number",        "SI", false, false),
        f(2,  "Item Code - Internal",           "CE", false, false),
        f(3,  "Item Code - External",           "CE", false, false),
        f(4,  "Hospital Item Code",             "CE", false, false),
        f(5,  "Requisition Quantity",           "NM", false, false),
        f(6,  "Requisition Unit of Measure",    "CE", false, false),
        f(7,  "Dept. Cost Center",              "IS", false, false),
        f(8,  "Item Natural Account Code",      "IS", false, false),
        f(9,  "Deliver To ID",                  "CE", false, false),
        f(10, "Date Needed",                    "DT", false, false),
    ])
}

fn seg_rq1() -> SegmentSpec {
    seg("RQ1", "Requisition Detail-1", vec![
        f(1, "Anticipated Price",            "ST", false, false),
        f(2, "Manufacturer Identifier",      "CE", false, false),
        f(3, "Manufacturer's Catalog",       "ST", false, false),
        f(4, "Vendor ID",                    "CE", false, false),
        f(5, "Vendor Catalog",               "ST", false, false),
        f(6, "Taxable",                      "ID", false, false),
        f(7, "Substitute Allowed",           "ID", false, false),
    ])
}
fn seg_rxo() -> SegmentSpec {
    seg("RXO", "Pharmacy/Treatment Order", vec![
        f(1,  "Requested Give Code",              "CE",  false, false),
        f(2,  "Requested Give Amount - Minimum",  "NM",  false, false),
        f(3,  "Requested Give Amount - Maximum",  "NM",  false, false),
        f(4,  "Requested Give Units",             "CE",  false, false),
        f(5,  "Requested Dosage Form",            "CE",  false, false),
        f(6,  "Provider's Pharmacy/Treatment Instructions","CE", false, true),
        f(7,  "Provider's Administration Instructions","CE", false, true),
        f(8,  "Deliver-To Location",              "LA1", false, false),
        f(9,  "Allow Substitutions",              "ID",  false, false),
        f(10, "Requested Dispense Code",          "CE",  false, false),
        f(11, "Requested Dispense Amount",        "NM",  false, false),
        f(12, "Requested Dispense Units",         "CE",  false, false),
        f(13, "Number Of Refills",                "NM",  false, false),
        f(14, "Ordering Provider's DEA Number",   "XCN", false, true),
        f(15, "Pharmacist/Treatment Supplier's Verifier ID","XCN", false, true),
        f(16, "Needs Human Review",               "ID",  false, false),
        f(17, "Requested Give Per (Time Unit)",   "ST",  false, false),
        f(18, "Requested Give Strength",          "NM",  false, false),
        f(19, "Requested Give Strength Units",    "CE",  false, false),
        f(20, "Indication",                       "CE",  false, true),
        f(21, "Requested Give Rate Amount",       "ST",  false, false),
        f(22, "Requested Give Rate Units",        "CE",  false, false),
        f(23, "Total Daily Dose",                 "CQ",  false, false),
        f(24, "Supplementary Code",               "CE",  false, true),
        f(25, "Requested Drug Strength Volume",   "NM",  false, false),
        f(26, "Requested Drug Strength Volume Units","CWE", false, false),
        f(27, "Pharmacy Order Type",              "ID",  false, false),
        f(28, "Dispensing Interval",              "NM",  false, false),
    ])
}

fn seg_ods() -> SegmentSpec {
    seg("ODS", "Dietary Orders, Supplements, Preferences", vec![
        f(1, "Type",              "ID", true,  false),
        f(2, "Service Period",    "CE", false, true),
        f(3, "Diet, Supplement, or Preference Code","CE", true, true),
        f(4, "Text Instruction",  "ST", false, true),
    ])
}

fn seg_odt() -> SegmentSpec {
    seg("ODT", "Diet Tray Instructions", vec![
        f(1, "Tray Type",        "CE", true,  false),
        f(2, "Service Period",   "CE", false, true),
        f(3, "Text Instruction", "ST", false, false),
    ])
}

// --------- composites + primitives (populated in follow-up chunks) ----------

fn composites() -> Vec<CompositeType> {
    let mut all = Vec::new();
    all.extend(composites_chunk_1_core());
    all.extend(composites_chunk_2_name_addr());
    all.extend(composites_chunk_3_financial());
    all.extend(composites_chunk_4_misc());
    all
}

/// Core composites: identifier, codes, hierarchic designator, message/version.
fn composites_chunk_1_core() -> Vec<CompositeType> {
    vec![
        comp("HD", vec![
            c(1, "Namespace ID",         "IS", false),
            c(2, "Universal ID",         "ST", false),
            c(3, "Universal ID Type",    "ID", false),
        ]),
        comp("MSG", vec![
            c(1, "Message Code",         "ID", true),
            c(2, "Trigger Event",        "ID", false),
            c(3, "Message Structure",    "ID", false),
        ]),
        comp("VID", vec![
            c(1, "Version ID",           "ID", false),
            c(2, "Internationalization Code", "CE", false),
            c(3, "International Version ID", "CE", false),
        ]),
        comp("PT", vec![
            c(1, "Processing ID",        "ID", false),
            c(2, "Processing Mode",      "ID", false),
        ]),
        comp("CE", vec![
            c(1, "Identifier",           "ST", false),
            c(2, "Text",                 "ST", false),
            c(3, "Name of Coding System","ID", false),
            c(4, "Alternate Identifier", "ST", false),
            c(5, "Alternate Text",       "ST", false),
            c(6, "Name of Alternate Coding System","ID", false),
        ]),
        comp("CWE", vec![
            c(1, "Identifier",           "ST", false),
            c(2, "Text",                 "ST", false),
            c(3, "Name of Coding System","ID", false),
            c(4, "Alternate Identifier", "ST", false),
            c(5, "Alternate Text",       "ST", false),
            c(6, "Name of Alternate Coding System","ID", false),
            c(7, "Coding System Version ID","ST", false),
            c(8, "Alternate Coding System Version ID","ST", false),
            c(9, "Original Text",        "ST", false),
        ]),
        comp("CNE", vec![
            c(1, "Identifier",           "ST", true),
            c(2, "Text",                 "ST", false),
            c(3, "Name of Coding System","ID", false),
            c(4, "Alternate Identifier", "ST", false),
            c(5, "Alternate Text",       "ST", false),
            c(6, "Name of Alternate Coding System","ID", false),
            c(7, "Coding System Version ID","ST", false),
            c(8, "Alternate Coding System Version ID","ST", false),
            c(9, "Original Text",        "ST", false),
        ]),
        comp("CNN", vec![
            c(1, "ID Number",            "ST", false),
            c(2, "Family Name",          "ST", false),
            c(3, "Given Name",           "ST", false),
            c(4, "Second and Further Given Names","ST", false),
            c(5, "Suffix",               "ST", false),
            c(6, "Prefix",               "ST", false),
            c(7, "Degree",               "IS", false),
            c(8, "Source Table",         "IS", false),
            c(9, "Assigning Authority - Namespace ID","IS", false),
            c(10,"Assigning Authority - Universal ID","ST", false),
            c(11,"Assigning Authority - Universal ID Type","ID", false),
        ]),
        comp("CX", vec![
            c(1, "ID Number",            "ST", true),
            c(2, "Check Digit",          "ST", false),
            c(3, "Check Digit Scheme",   "ID", false),
            c(4, "Assigning Authority",  "HD", false),
            c(5, "Identifier Type Code", "ID", false),
            c(6, "Assigning Facility",   "HD", false),
            c(7, "Effective Date",       "DT", false),
            c(8, "Expiration Date",      "DT", false),
            c(9, "Assigning Jurisdiction","CWE", false),
            c(10,"Assigning Agency or Department","CWE", false),
        ]),
    ]
}

fn composites_chunk_2_name_addr() -> Vec<CompositeType> {
    vec![
        comp("FN", vec![
            c(1, "Surname",                        "ST", true),
            c(2, "Own Surname Prefix",             "ST", false),
            c(3, "Own Surname",                    "ST", false),
            c(4, "Surname Prefix From Partner/Spouse","ST", false),
            c(5, "Surname From Partner/Spouse",    "ST", false),
        ]),
        comp("XPN", vec![
            c(1, "Family Name",         "FN", false),
            c(2, "Given Name",          "ST", false),
            c(3, "Second and Further Given Names","ST", false),
            c(4, "Suffix",              "ST", false),
            c(5, "Prefix",              "ST", false),
            c(6, "Degree",              "IS", false),
            c(7, "Name Type Code",      "ID", false),
            c(8, "Name Representation Code","ID", false),
            c(9, "Name Context",        "CE", false),
            c(10,"Name Validity Range", "DR", false),
            c(11,"Name Assembly Order", "ID", false),
            c(12,"Effective Date",      "TS", false),
            c(13,"Expiration Date",     "TS", false),
            c(14,"Professional Suffix", "ST", false),
        ]),
        comp("SAD", vec![
            c(1, "Street or Mailing Address","ST", false),
            c(2, "Street Name",         "ST", false),
            c(3, "Dwelling Number",     "ST", false),
        ]),
        comp("XAD", vec![
            c(1, "Street Address",      "SAD", false),
            c(2, "Other Designation",   "ST", false),
            c(3, "City",                "ST", false),
            c(4, "State or Province",   "ST", false),
            c(5, "Zip or Postal Code",  "ST", false),
            c(6, "Country",             "ID", false),
            c(7, "Address Type",        "ID", false),
            c(8, "Other Geographic Designation","ST", false),
            c(9, "County/Parish Code",  "IS", false),
            c(10,"Census Tract",        "IS", false),
            c(11,"Address Representation Code","ID", false),
            c(12,"Address Validity Range","DR", false),
            c(13,"Effective Date",      "TS", false),
            c(14,"Expiration Date",     "TS", false),
        ]),
        comp("XTN", vec![
            c(1, "Telephone Number",     "ST", false),
            c(2, "Telecommunication Use Code","ID", false),
            c(3, "Telecommunication Equipment Type","ID", false),
            c(4, "Email Address",        "ST", false),
            c(5, "Country Code",         "NM", false),
            c(6, "Area/City Code",       "NM", false),
            c(7, "Local Number",         "NM", false),
            c(8, "Extension",            "NM", false),
            c(9, "Any Text",             "ST", false),
            c(10,"Extension Prefix",     "ST", false),
            c(11,"Speed Dial Code",      "ST", false),
            c(12,"Unformatted Telephone Number","ST", false),
        ]),
        comp("XCN", vec![
            c(1, "ID Number",            "ST", false),
            c(2, "Family Name",          "FN", false),
            c(3, "Given Name",           "ST", false),
            c(4, "Second and Further Given Names","ST", false),
            c(5, "Suffix",               "ST", false),
            c(6, "Prefix",               "ST", false),
            c(7, "Degree",               "IS", false),
            c(8, "Source Table",         "IS", false),
            c(9, "Assigning Authority",  "HD", false),
            c(10,"Name Type Code",       "ID", false),
            c(11,"Identifier Check Digit","ST", false),
            c(12,"Check Digit Scheme",   "ID", false),
            c(13,"Identifier Type Code", "ID", false),
            c(14,"Assigning Facility",   "HD", false),
            c(15,"Name Representation Code","ID", false),
            c(16,"Name Context",         "CE", false),
            c(17,"Name Validity Range",  "DR", false),
            c(18,"Name Assembly Order",  "ID", false),
            c(19,"Effective Date",       "TS", false),
            c(20,"Expiration Date",      "TS", false),
            c(21,"Professional Suffix",  "ST", false),
            c(22,"Assigning Jurisdiction","CWE", false),
            c(23,"Assigning Agency or Department","CWE", false),
        ]),
        comp("XON", vec![
            c(1, "Organization Name",    "ST", false),
            c(2, "Organization Name Type Code","IS", false),
            c(3, "ID Number",            "NM", false),
            c(4, "Check Digit",          "NM", false),
            c(5, "Check Digit Scheme",   "ID", false),
            c(6, "Assigning Authority",  "HD", false),
            c(7, "Identifier Type Code", "ID", false),
            c(8, "Assigning Facility",   "HD", false),
            c(9, "Name Representation Code","ID", false),
            c(10,"Organization Identifier","ST", false),
        ]),
    ]
}
fn composites_chunk_3_financial() -> Vec<CompositeType> {
    vec![
        comp("MO", vec![
            c(1, "Quantity",             "NM", false),
            c(2, "Denomination",         "ID", false),
        ]),
        comp("CP", vec![
            c(1, "Price",                "MO", true),
            c(2, "Price Type",           "ID", false),
            c(3, "From Value",           "NM", false),
            c(4, "To Value",             "NM", false),
            c(5, "Range Units",          "CE", false),
            c(6, "Range Type",           "ID", false),
        ]),
        comp("CQ", vec![
            c(1, "Quantity",             "NM", false),
            c(2, "Units",                "CE", false),
        ]),
        comp("MOC", vec![
            c(1, "Monetary Amount",      "MO", false),
            c(2, "Charge Code",          "CE", false),
        ]),
        comp("MOP", vec![
            c(1, "Money or Percentage Indicator","ID", true),
            c(2, "Money or Percentage Quantity","NM", true),
            c(3, "Monetary Denomination","ID", false),
        ]),
        comp("DR", vec![
            c(1, "Range Start Date/Time","TS", false),
            c(2, "Range End Date/Time",  "TS", false),
        ]),
        comp("DLD", vec![
            c(1, "Discharge Location",   "IS", true),
            c(2, "Effective Date",       "TS", false),
        ]),
        comp("DLN", vec![
            c(1, "License Number",       "ST", true),
            c(2, "Issuing State/Province/Country","IS", false),
            c(3, "Expiration Date",      "DT", false),
        ]),
        comp("DTN", vec![
            c(1, "Day Type",             "IS", true),
            c(2, "Number of Days",       "NM", true),
        ]),
        comp("TS", vec![
            c(1, "Time",                 "DTM", true),
            c(2, "Degree of Precision",  "ID", false),
        ]),
        comp("FC", vec![
            c(1, "Financial Class Code", "IS", true),
            c(2, "Effective Date",       "TS", false),
        ]),
    ]
}
fn composites_chunk_4_misc() -> Vec<CompositeType> {
    let mut v = Vec::new();
    v.extend(composites_chunk_4a());
    v.extend(composites_chunk_4b());
    v.extend(composites_chunk_4c());
    v
}

fn composites_chunk_4a() -> Vec<CompositeType> {
    vec![
        comp("EI", vec![
            c(1, "Entity Identifier",    "ST", false),
            c(2, "Namespace ID",         "IS", false),
            c(3, "Universal ID",         "ST", false),
            c(4, "Universal ID Type",    "ID", false),
        ]),
        comp("EIP", vec![
            c(1, "Placer Assigned Identifier","EI", false),
            c(2, "Filler Assigned Identifier","EI", false),
        ]),
        comp("ELD", vec![
            c(1, "Segment ID",           "ST", false),
            c(2, "Segment Sequence",     "NM", false),
            c(3, "Field Position",       "NM", false),
            c(4, "Code Identifying Error","CE", false),
        ]),
        comp("ERL", vec![
            c(1, "Segment ID",           "ST", true),
            c(2, "Segment Sequence",     "NM", true),
            c(3, "Field Position",       "NM", false),
            c(4, "Field Repetition",     "NM", false),
            c(5, "Component Number",     "NM", false),
            c(6, "Sub-Component Number", "NM", false),
        ]),
        comp("JCC", vec![
            c(1, "Job Code",             "IS", false),
            c(2, "Job Class",            "IS", false),
            c(3, "Job Description Text", "TX", false),
        ]),
        comp("NDL", vec![
            c(1, "Name",                 "CNN", false),
            c(2, "Start Date/Time",      "TS", false),
            c(3, "End Date/Time",        "TS", false),
            c(4, "Point of Care",        "IS", false),
            c(5, "Room",                 "IS", false),
            c(6, "Bed",                  "IS", false),
            c(7, "Facility",             "HD", false),
            c(8, "Location Status",      "IS", false),
            c(9, "Patient Location Type","IS", false),
            c(10,"Building",             "IS", false),
            c(11,"Floor",                "IS", false),
        ]),
        comp("OSD", vec![
            c(1, "Sequence/Results Flag","ID", true),
            c(2, "Placer Order Number: Entity Identifier","ST", true),
            c(3, "Placer Order Number: Namespace ID","IS", false),
            c(4, "Filler Order Number: Entity Identifier","ST", true),
            c(5, "Filler Order Number: Namespace ID","IS", false),
            c(6, "Sequence Condition Value","ST", false),
            c(7, "Maximum Number of Repeats","NM", false),
            c(8, "Placer Order Number: Universal ID","ST", true),
            c(9, "Placer Order Number: Universal ID Type","ID", false),
            c(10,"Filler Order Number: Universal ID","ST", true),
            c(11,"Filler Order Number: Universal ID Type","ID", false),
        ]),
    ]
}

fn composites_chunk_4b() -> Vec<CompositeType> {
    vec![
        comp("PL", vec![
            c(1, "Point of Care",        "IS", false),
            c(2, "Room",                 "IS", false),
            c(3, "Bed",                  "IS", false),
            c(4, "Facility",             "HD", false),
            c(5, "Location Status",      "IS", false),
            c(6, "Person Location Type", "IS", false),
            c(7, "Building",             "IS", false),
            c(8, "Floor",                "IS", false),
            c(9, "Location Description", "ST", false),
            c(10,"Comprehensive Location Identifier","EI", false),
            c(11,"Assigning Authority for Location","HD", false),
        ]),
        comp("PLN", vec![
            c(1, "ID Number",            "ST", true),
            c(2, "Type of ID Number",    "IS", true),
            c(3, "State/Other Qualifying Information","ST", false),
            c(4, "Expiration Date",      "DT", false),
        ]),
        comp("PRL", vec![
            c(1, "Parent Observation Identifier","CE", true),
            c(2, "Parent Observation Sub-ID","ST", false),
            c(3, "Parent Observation Value Descriptor","TX", false),
        ]),
        comp("RI", vec![
            c(1, "Repeat Pattern",       "IS", false),
            c(2, "Explicit Time Interval","ST", false),
        ]),
        comp("RMC", vec![
            c(1, "Room Type",            "IS", true),
            c(2, "Amount Type",          "IS", false),
            c(3, "Coverage Amount",      "NM", false),
            c(4, "Money or Percentage",  "MOP", true),
        ]),
        comp("RPT", vec![
            c(1, "Repeat Pattern Code",  "CWE", true),
            c(2, "Calendar Alignment",   "ID", false),
            c(3, "Phase Range Begin Value","NM", false),
            c(4, "Phase Range End Value","NM", false),
            c(5, "Period Quantity",      "NM", false),
            c(6, "Period Units",         "IS", false),
            c(7, "Institution Specified Time","ID", false),
            c(8, "Event",                "ID", false),
            c(9, "Event Offset Quantity","NM", false),
            c(10,"Event Offset Units",   "IS", false),
            c(11,"General Timing Specification","GTS", false),
        ]),
        comp("SPS", vec![
            c(1, "Specimen Source Name or Code","CWE", false),
            c(2, "Additives",            "CWE", false),
            c(3, "Specimen Collection Method","TX", false),
            c(4, "Body Site",            "CWE", false),
            c(5, "Site Modifier",        "CWE", false),
            c(6, "Collection Method Modifier Code","CWE", false),
            c(7, "Specimen Role",        "CWE", false),
        ]),
        comp("TQ", vec![
            c(1, "Quantity",             "CQ", false),
            c(2, "Interval",             "RI", false),
            c(3, "Duration",             "ST", false),
            c(4, "Start Date/Time",      "TS", false),
            c(5, "End Date/Time",        "TS", false),
            c(6, "Priority",             "ST", false),
            c(7, "Condition",            "ST", false),
            c(8, "Text",                 "TX", false),
            c(9, "Conjunction",          "ID", false),
            c(10,"Order Sequencing",     "OSD", false),
            c(11,"Occurrence Duration",  "CE", false),
            c(12,"Total Occurrences",    "NM", false),
        ]),
    ]
}

fn composites_chunk_4c() -> Vec<CompositeType> {
    vec![
        comp("AD", vec![
            c(1, "Street Address",       "ST", false),
            c(2, "Other Designation",    "ST", false),
            c(3, "City",                 "ST", false),
            c(4, "State or Province",    "ST", false),
            c(5, "Zip or Postal Code",   "ST", false),
            c(6, "Country",              "ID", false),
            c(7, "Address Type",         "ID", false),
            c(8, "Other Geographic Designation","ST", false),
        ]),
        comp("AUI", vec![
            c(1, "Authorization Number", "ST", false),
            c(2, "Date",                 "DT", false),
            c(3, "Source",               "ST", false),
        ]),
        comp("CCD", vec![
            c(1, "Invocation Event",     "ID", true),
            c(2, "Date/Time",            "TS", false),
        ]),
        comp("DDI", vec![
            c(1, "Delay Days",           "NM", false),
            c(2, "Monetary Amount",      "MO", true),
            c(3, "Number of Days",       "NM", false),
        ]),
        comp("ICD", vec![
            c(1, "Certification Patient Type","IS", false),
            c(2, "Certification Required","ID", true),
            c(3, "Date/Time Certification Required","TS", false),
        ]),
        comp("LA1", vec![
            c(1, "Point of Care",        "IS", false),
            c(2, "Room",                 "IS", false),
            c(3, "Bed",                  "IS", false),
            c(4, "Facility",             "HD", false),
            c(5, "Location Status",      "IS", false),
            c(6, "Patient Location Type","IS", false),
            c(7, "Building",             "IS", false),
            c(8, "Floor",                "IS", false),
            c(9, "Address",              "AD", false),
        ]),
        comp("PTA", vec![
            c(1, "Policy Type",          "IS", true),
            c(2, "Amount Class",         "IS", false),
            c(3, "Money or Percentage Quantity","NM", false),
            c(4, "Money or Percentage",  "MOP", true),
        ]),
    ]
}
fn primitives() -> Vec<PrimitiveType> {
    vec![
        primitive("DT"),
        primitive("DTM"),
        primitive("FT"),
        primitive("GTS"),
        primitive("ID"),
        primitive("IS"),
        primitive("NM"),
        primitive("SI"),
        primitive("ST"),
        primitive("TM"),
        primitive("TX"),
        primitive("VARIES"),
    ]
}
