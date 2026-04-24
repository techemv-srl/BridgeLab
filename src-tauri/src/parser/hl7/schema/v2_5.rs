//! HL7 v2.5 hand-coded schema data for the F1 MVP.
//!
//! Covers four message types and the segments they reference. All metadata
//! is sourced from the HL7 v2.5 public specification (Chapters 2A, 3, 4,
//! 6, 7). Field lists are intentionally abbreviated to "safe" cardinality
//! — every field is modelled as optional/`xs:string` so the generated XSD
//! is permissive. F2 will re-import this with tighter types.

use super::{FieldSpec, Hl7Schema, Hl7Version, MessageElement, MessageStructure, SegmentSpec};

pub fn schema() -> Hl7Schema {
    Hl7Schema {
        version: Hl7Version::V2_5,
        messages: vec![adt_a01(), adt_a40(), orm_o01(), oru_r01()],
        segments: segments(),
    }
}

// ----- messages -------------------------------------------------------------

/// ADT^A01 — Admit / Visit Notification
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

/// ADT^A40 — Merge Patient Information - Patient Identifier List (MRG-1)
fn adt_a40() -> MessageStructure {
    use MessageElement::*;
    MessageStructure {
        code: "ADT_A40".into(),
        event: "ADT^A40".into(),
        description: "Merge Patient - Patient Identifier List".into(),
        elements: vec![
            Segment { code: "MSH".into(), required: true,  repeats: false },
            Segment { code: "EVN".into(), required: true,  repeats: false },
            Group {
                name: "PATIENT".into(),
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

/// ORM^O01 — Order Message
fn orm_o01() -> MessageStructure {
    use MessageElement::*;
    MessageStructure {
        code: "ORM_O01".into(),
        event: "ORM^O01".into(),
        description: "Order Message".into(),
        elements: vec![
            Segment { code: "MSH".into(), required: true, repeats: false },
            Segment { code: "NTE".into(), required: false, repeats: true },
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
                            Segment { code: "OBR".into(), required: true,  repeats: false },
                            Segment { code: "NTE".into(), required: false, repeats: true  },
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
                ],
            },
        ],
    }
}

/// ORU^R01 — Unsolicited Observation Result
fn oru_r01() -> MessageStructure {
    use MessageElement::*;
    MessageStructure {
        code: "ORU_R01".into(),
        event: "ORU^R01".into(),
        description: "Unsolicited Observation Result".into(),
        elements: vec![
            Segment { code: "MSH".into(), required: true, repeats: false },
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
                ],
            },
        ],
    }
}

// ----- segments -------------------------------------------------------------

fn segments() -> Vec<SegmentSpec> {
    vec![
        seg_msh(), seg_evn(), seg_pid(), seg_pd1(), seg_nk1(),
        seg_pv1(), seg_pv2(), seg_obx(), seg_al1(), seg_dg1(),
        seg_gt1(), seg_in1(), seg_in2(), seg_in3(), seg_mrg(),
        seg_orc(), seg_obr(), seg_nte(),
    ]
}

/// Helper to build a FieldSpec quickly.
fn f(position: usize, name: &str, dt: &str, max: Option<usize>, req: bool, rep: bool) -> FieldSpec {
    FieldSpec {
        position,
        name: name.into(),
        data_type: dt.into(),
        max_length: max,
        required: req,
        repeats: rep,
    }
}

fn seg_msh() -> SegmentSpec {
    SegmentSpec {
        code: "MSH".into(),
        name: "Message Header".into(),
        fields: vec![
            f(1,  "Field Separator",              "ST",  Some(1),   true,  false),
            f(2,  "Encoding Characters",          "ST",  Some(4),   true,  false),
            f(3,  "Sending Application",          "HD",  Some(227), false, false),
            f(4,  "Sending Facility",             "HD",  Some(227), false, false),
            f(5,  "Receiving Application",        "HD",  Some(227), false, false),
            f(6,  "Receiving Facility",           "HD",  Some(227), false, false),
            f(7,  "Date/Time Of Message",         "TS",  Some(26),  true,  false),
            f(8,  "Security",                     "ST",  Some(40),  false, false),
            f(9,  "Message Type",                 "MSG", Some(15),  true,  false),
            f(10, "Message Control ID",           "ST",  Some(20),  true,  false),
            f(11, "Processing ID",                "PT",  Some(3),   true,  false),
            f(12, "Version ID",                   "VID", Some(60),  true,  false),
            f(13, "Sequence Number",              "NM",  Some(15),  false, false),
            f(14, "Continuation Pointer",         "ST",  Some(180), false, false),
            f(15, "Accept Acknowledgment Type",   "ID",  Some(2),   false, false),
            f(16, "Application Acknowledgment Type","ID",Some(2),   false, false),
            f(17, "Country Code",                 "ID",  Some(3),   false, false),
            f(18, "Character Set",                "ID",  Some(16),  false, true ),
            f(19, "Principal Language Of Message","CE",  Some(250), false, false),
        ],
    }
}

fn seg_evn() -> SegmentSpec {
    SegmentSpec {
        code: "EVN".into(),
        name: "Event Type".into(),
        fields: vec![
            f(1, "Event Type Code",             "ID",  Some(3),   false, false),
            f(2, "Recorded Date/Time",          "TS",  Some(26),  true,  false),
            f(3, "Date/Time Planned Event",     "TS",  Some(26),  false, false),
            f(4, "Event Reason Code",           "IS",  Some(3),   false, false),
            f(5, "Operator ID",                 "XCN", Some(250), false, true ),
            f(6, "Event Occurred",              "TS",  Some(26),  false, false),
            f(7, "Event Facility",              "HD",  Some(241), false, false),
        ],
    }
}

fn seg_pid() -> SegmentSpec {
    SegmentSpec {
        code: "PID".into(),
        name: "Patient Identification".into(),
        fields: vec![
            f(1,  "Set ID - PID",              "SI",  Some(4),    false, false),
            f(2,  "Patient ID",                "CX",  Some(20),   false, false),
            f(3,  "Patient Identifier List",   "CX",  Some(250),  true,  true ),
            f(4,  "Alternate Patient ID - PID","CX",  Some(20),   false, true ),
            f(5,  "Patient Name",              "XPN", Some(250),  true,  true ),
            f(6,  "Mother's Maiden Name",      "XPN", Some(250),  false, true ),
            f(7,  "Date/Time of Birth",        "TS",  Some(26),   false, false),
            f(8,  "Administrative Sex",        "IS",  Some(1),    false, false),
            f(9,  "Patient Alias",             "XPN", Some(250),  false, true ),
            f(10, "Race",                      "CE",  Some(250),  false, true ),
            f(11, "Patient Address",           "XAD", Some(250),  false, true ),
            f(12, "County Code",               "IS",  Some(4),    false, false),
            f(13, "Phone Number - Home",       "XTN", Some(250),  false, true ),
            f(14, "Phone Number - Business",   "XTN", Some(250),  false, true ),
            f(15, "Primary Language",          "CE",  Some(250),  false, false),
            f(16, "Marital Status",            "CE",  Some(250),  false, false),
            f(17, "Religion",                  "CE",  Some(250),  false, false),
            f(18, "Patient Account Number",    "CX",  Some(250),  false, false),
            f(19, "SSN Number - Patient",      "ST",  Some(16),   false, false),
            f(20, "Driver's License Number",   "DLN", Some(25),   false, false),
            f(21, "Mother's Identifier",       "CX",  Some(250),  false, true ),
            f(22, "Ethnic Group",              "CE",  Some(250),  false, true ),
            f(23, "Birth Place",               "ST",  Some(250),  false, false),
            f(24, "Multiple Birth Indicator",  "ID",  Some(1),    false, false),
            f(25, "Birth Order",               "NM",  Some(2),    false, false),
            f(26, "Citizenship",               "CE",  Some(250),  false, true ),
            f(27, "Veterans Military Status",  "CE",  Some(250),  false, false),
            f(28, "Nationality",               "CE",  Some(250),  false, false),
            f(29, "Patient Death Date and Time","TS", Some(26),   false, false),
            f(30, "Patient Death Indicator",   "ID",  Some(1),    false, false),
        ],
    }
}

fn seg_pd1() -> SegmentSpec {
    SegmentSpec {
        code: "PD1".into(),
        name: "Patient Additional Demographic".into(),
        fields: vec![
            f(1, "Living Dependency",           "IS",  Some(2),   false, true ),
            f(2, "Living Arrangement",          "IS",  Some(2),   false, false),
            f(3, "Patient Primary Facility",    "XON", Some(250), false, true ),
            f(4, "Patient Primary Care Provider","XCN",Some(250), false, true ),
            f(5, "Student Indicator",           "IS",  Some(2),   false, false),
            f(6, "Handicap",                    "IS",  Some(2),   false, false),
        ],
    }
}

fn seg_nk1() -> SegmentSpec {
    SegmentSpec {
        code: "NK1".into(),
        name: "Next of Kin / Associated Parties".into(),
        fields: vec![
            f(1, "Set ID - NK1",                "SI",  Some(4),   true,  false),
            f(2, "Name",                        "XPN", Some(250), false, true ),
            f(3, "Relationship",                "CE",  Some(250), false, false),
            f(4, "Address",                     "XAD", Some(250), false, true ),
            f(5, "Phone Number",                "XTN", Some(250), false, true ),
            f(6, "Business Phone Number",       "XTN", Some(250), false, true ),
            f(7, "Contact Role",                "CE",  Some(250), false, false),
        ],
    }
}

fn seg_pv1() -> SegmentSpec {
    SegmentSpec {
        code: "PV1".into(),
        name: "Patient Visit".into(),
        fields: vec![
            f(1,  "Set ID - PV1",           "SI",  Some(4),   false, false),
            f(2,  "Patient Class",          "IS",  Some(1),   true,  false),
            f(3,  "Assigned Patient Location","PL",Some(80),  false, false),
            f(4,  "Admission Type",         "IS",  Some(2),   false, false),
            f(5,  "Preadmit Number",        "CX",  Some(250), false, false),
            f(6,  "Prior Patient Location", "PL",  Some(80),  false, false),
            f(7,  "Attending Doctor",       "XCN", Some(250), false, true ),
            f(8,  "Referring Doctor",       "XCN", Some(250), false, true ),
            f(9,  "Consulting Doctor",      "XCN", Some(250), false, true ),
            f(10, "Hospital Service",       "IS",  Some(3),   false, false),
            f(18, "Patient Type",           "IS",  Some(2),   false, false),
            f(19, "Visit Number",           "CX",  Some(250), false, false),
            f(44, "Admit Date/Time",        "TS",  Some(26),  false, false),
            f(45, "Discharge Date/Time",    "TS",  Some(26),  false, true ),
        ],
    }
}

fn seg_pv2() -> SegmentSpec {
    SegmentSpec {
        code: "PV2".into(),
        name: "Patient Visit - Additional Information".into(),
        fields: vec![
            f(1, "Prior Pending Location",      "PL",  Some(80),  false, false),
            f(2, "Accommodation Code",          "CE",  Some(250), false, false),
            f(3, "Admit Reason",                "CE",  Some(250), false, false),
        ],
    }
}

fn seg_obx() -> SegmentSpec {
    SegmentSpec {
        code: "OBX".into(),
        name: "Observation / Result".into(),
        fields: vec![
            f(1,  "Set ID - OBX",          "SI",  Some(4),    false, false),
            f(2,  "Value Type",            "ID",  Some(2),    false, false),
            f(3,  "Observation Identifier","CE",  Some(250),  true,  false),
            f(4,  "Observation Sub-ID",    "ST",  Some(20),   false, false),
            f(5,  "Observation Value",     "Varies", Some(65536), false, true),
            f(6,  "Units",                 "CE",  Some(250),  false, false),
            f(7,  "References Range",      "ST",  Some(60),   false, false),
            f(8,  "Abnormal Flags",        "IS",  Some(5),    false, true ),
            f(11, "Observation Result Status","ID",Some(1),   true,  false),
            f(14, "Date/Time of the Observation","TS",Some(26),false,false),
        ],
    }
}

fn seg_al1() -> SegmentSpec {
    SegmentSpec {
        code: "AL1".into(),
        name: "Patient Allergy Information".into(),
        fields: vec![
            f(1, "Set ID - AL1",              "SI",  Some(4),    true,  false),
            f(2, "Allergen Type Code",        "CE",  Some(250),  false, false),
            f(3, "Allergen Code/Mnemonic/Description","CE",Some(250),true,false),
            f(4, "Allergy Severity Code",     "CE",  Some(250),  false, false),
            f(5, "Allergy Reaction Code",     "ST",  Some(15),   false, true ),
            f(6, "Identification Date",       "DT",  Some(8),    false, false),
        ],
    }
}

fn seg_dg1() -> SegmentSpec {
    SegmentSpec {
        code: "DG1".into(),
        name: "Diagnosis".into(),
        fields: vec![
            f(1, "Set ID - DG1",          "SI",  Some(4),   true,  false),
            f(2, "Diagnosis Coding Method","ID", Some(2),   false, false),
            f(3, "Diagnosis Code",        "CE",  Some(250), false, false),
            f(6, "Diagnosis Type",        "IS",  Some(2),   true,  false),
        ],
    }
}

fn seg_gt1() -> SegmentSpec {
    SegmentSpec {
        code: "GT1".into(),
        name: "Guarantor".into(),
        fields: vec![
            f(1, "Set ID - GT1",     "SI",  Some(4),   true,  false),
            f(2, "Guarantor Number", "CX",  Some(250), false, true ),
            f(3, "Guarantor Name",   "XPN", Some(250), true,  true ),
            f(5, "Guarantor Address","XAD", Some(250), false, true ),
        ],
    }
}

fn seg_in1() -> SegmentSpec {
    SegmentSpec {
        code: "IN1".into(),
        name: "Insurance".into(),
        fields: vec![
            f(1, "Set ID - IN1",        "SI",  Some(4),   true,  false),
            f(2, "Insurance Plan ID",   "CE",  Some(250), true,  false),
            f(3, "Insurance Company ID","CX",  Some(250), true,  true ),
            f(4, "Insurance Company Name","XON",Some(250),false, true ),
        ],
    }
}

fn seg_in2() -> SegmentSpec {
    SegmentSpec {
        code: "IN2".into(),
        name: "Insurance Additional Information".into(),
        fields: vec![
            f(1, "Insured's Employee ID",      "CX",  Some(250), false, true ),
            f(2, "Insured's Social Security Number","ST",Some(11),false,false),
        ],
    }
}

fn seg_in3() -> SegmentSpec {
    SegmentSpec {
        code: "IN3".into(),
        name: "Insurance Additional Information, Certification".into(),
        fields: vec![
            f(1, "Set ID - IN3",              "SI", Some(4),   true,  false),
            f(2, "Certification Number",      "CX", Some(250), false, false),
        ],
    }
}

fn seg_mrg() -> SegmentSpec {
    SegmentSpec {
        code: "MRG".into(),
        name: "Merge Patient Information".into(),
        fields: vec![
            f(1, "Prior Patient Identifier List","CX",Some(250),true, true ),
            f(2, "Prior Alternate Patient ID",   "CX",Some(250),false,true ),
            f(3, "Prior Patient Account Number", "CX",Some(250),false,false),
            f(7, "Prior Patient Name",           "XPN",Some(250),false,true),
        ],
    }
}

fn seg_orc() -> SegmentSpec {
    SegmentSpec {
        code: "ORC".into(),
        name: "Common Order".into(),
        fields: vec![
            f(1, "Order Control",             "ID",  Some(2),   true,  false),
            f(2, "Placer Order Number",       "EI",  Some(22),  false, false),
            f(3, "Filler Order Number",       "EI",  Some(22),  false, false),
            f(4, "Placer Group Number",       "EI",  Some(22),  false, false),
            f(5, "Order Status",              "ID",  Some(2),   false, false),
            f(9, "Date/Time of Transaction",  "TS",  Some(26),  false, false),
            f(10,"Entered By",                "XCN", Some(250), false, true ),
            f(12,"Ordering Provider",         "XCN", Some(250), false, true ),
        ],
    }
}

fn seg_obr() -> SegmentSpec {
    SegmentSpec {
        code: "OBR".into(),
        name: "Observation Request".into(),
        fields: vec![
            f(1, "Set ID - OBR",              "SI",  Some(4),   false, false),
            f(2, "Placer Order Number",       "EI",  Some(22),  false, false),
            f(3, "Filler Order Number",       "EI",  Some(22),  false, false),
            f(4, "Universal Service Identifier","CE",Some(250), true,  false),
            f(7, "Observation Date/Time",     "TS",  Some(26),  false, false),
            f(8, "Observation End Date/Time", "TS",  Some(26),  false, false),
            f(16,"Ordering Provider",         "XCN", Some(250), false, true ),
            f(25,"Result Status",             "ID",  Some(1),   false, false),
        ],
    }
}

fn seg_nte() -> SegmentSpec {
    SegmentSpec {
        code: "NTE".into(),
        name: "Notes and Comments".into(),
        fields: vec![
            f(1, "Set ID - NTE",    "SI",  Some(4),     false, false),
            f(2, "Source of Comment","ID", Some(8),     false, false),
            f(3, "Comment",         "FT",  Some(65536), false, true ),
            f(4, "Comment Type",    "CE",  Some(250),   false, false),
        ],
    }
}
