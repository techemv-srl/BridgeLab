//! Sample messages: complete, realistic HL7 v2 examples shipped with the
//! app, for learning a message type, demoing a feature or seeding a test.
//!
//! Unlike the templates (skeletons in v2.5, to be filled in), a sample is a
//! finished message in the version it names, with fictional patients and
//! data, and it validates clean — a test below keeps it that way. The
//! files live in `resources/samples/` with LF line endings for readability;
//! they are served with the CR segment separator HL7 uses.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Sample {
    pub id: &'static str,
    pub name: &'static str,
    /// MSH-9 as it appears in the message, e.g. `ADT^A01^ADT_A01`.
    pub message_type: &'static str,
    /// MSH-12.
    pub version: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub content: String,
}

macro_rules! sample {
    ($id:expr, $name:expr, $ty:expr, $ver:expr, $cat:expr, $desc:expr, $file:expr) => {
        ($id, $name, $ty, $ver, $cat, $desc, include_str!(concat!("../resources/samples/", $file)))
    };
}

// Names, descriptions and categories are English here; the UI shows them
// translated (`samples.name.<id>`, `samples.desc.<id>`, `samples.cat.*` in
// src/lib/i18n/*.json). src/lib/ipc/samples.test.ts parses this file and
// fails when a sample or a category has no translation.
const ADT: &str = "Admission / Discharge / Transfer";
const RESULTS: &str = "Observations / Results";
const ORDERS: &str = "Orders";
const OTHER: &str = "Scheduling, documents, billing, immunization";
const ACK: &str = "Acknowledgment";

#[rustfmt::skip]
const SAMPLES: &[(&str, &str, &str, &str, &str, &str, &str)] = &[
    sample!("adt-a01-251", "Inpatient admission", "ADT^A01^ADT_A01", "2.5.1", ADT,
        "Emergency admission to cardiology, with next of kin, allergy and admitting diagnosis", "adt_a01_v2.5.1.hl7"),
    sample!("adt-a04-25", "Outpatient registration", "ADT^A04", "2.5", ADT,
        "Registration for a dermatology clinic visit", "adt_a04_v2.5.hl7"),
    sample!("adt-a08-25", "Update patient information", "ADT^A08", "2.5", ADT,
        "New address and phone number for an admitted patient", "adt_a08_v2.5.hl7"),
    sample!("adt-a03-25", "Discharge", "ADT^A03", "2.5", ADT,
        "Discharge home with admit and discharge dates and final diagnosis", "adt_a03_v2.5.hl7"),
    sample!("adt-a40-25", "Merge patient identifiers", "ADT^A40^ADT_A39", "2.5", ADT,
        "A duplicate medical record number merged into the surviving one (MRG)", "adt_a40_v2.5.hl7"),
    sample!("oru-r01-cbc-251", "Lab results: complete blood count", "ORU^R01^ORU_R01", "2.5.1", RESULTS,
        "Twelve LOINC-coded OBX with units, reference ranges and abnormal flags", "oru_r01_cbc_v2.5.1.hl7"),
    sample!("oru-r01-chem-23", "Lab results: metabolic panel", "ORU^R01", "2.3", RESULTS,
        "Chemistry panel in v2.3, one result flagged high", "oru_r01_chem_v2.3.hl7"),
    sample!("orm-o01-23", "Lab order", "ORM^O01", "2.3", ORDERS,
        "Two new orders (CBC and LDL cholesterol), each with ORC and OBR", "orm_o01_v2.3.hl7"),
    sample!("siu-s12-25", "New appointment", "SIU^S12^SIU_S12", "2.5", OTHER,
        "Follow-up visit booked with service, clinician and location resources", "siu_s12_v2.5.hl7"),
    sample!("mdm-t02-25", "Clinical document", "MDM^T02^MDM_T02", "2.5", OTHER,
        "Discharge summary notification with the text in OBX", "mdm_t02_v2.5.hl7"),
    sample!("dft-p03-25", "Charges", "DFT^P03^DFT_P03", "2.5", OTHER,
        "Two financial transactions (FT1) for an inpatient stay", "dft_p03_v2.5.hl7"),
    sample!("vxu-v04-251", "Immunization record", "VXU^V04^VXU_V04", "2.5.1", OTHER,
        "Seasonal influenza vaccine (CVX) with route and site", "vxu_v04_v2.5.1.hl7"),
    sample!("ack-aa-25", "Application accept ACK", "ACK^A01^ACK", "2.5", ACK,
        "Positive acknowledgment answering the admission sample", "ack_aa_v2.5.hl7"),
];

/// Every sample, in catalogue order, with CR segment separators.
pub fn all() -> Vec<Sample> {
    SAMPLES
        .iter()
        .map(|&(id, name, message_type, version, category, description, raw)| Sample {
            id,
            name,
            message_type,
            version,
            category,
            description,
            content: raw.trim_end().replace("\r\n", "\n").replace('\n', "\r") + "\r",
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::hl7::lexer::Hl7Lexer;
    use crate::validation::{self, Severity};

    #[test]
    fn every_sample_parses_as_declared_and_validates_clean() {
        let samples = all();
        assert!(samples.len() >= 12);
        let mut ids = std::collections::HashSet::new();
        for s in &samples {
            assert!(ids.insert(s.id), "duplicate id {}", s.id);
            assert!(s.content.starts_with("MSH|") && !s.content.contains('\n'), "{}", s.id);
            let msg = Hl7Lexer::new().parse(s.content.clone().into_bytes()).unwrap_or_else(|e| panic!("{}: {}", s.id, e));
            assert_eq!(msg.version, s.version, "{}: MSH-12", s.id);
            assert_eq!(msg.message_type, s.message_type, "{}: MSH-9", s.id);
            let report = validation::validate_hl7_message(&msg);
            let problems: Vec<_> = report
                .issues
                .iter()
                .filter(|i| matches!(i.severity, Severity::Error | Severity::Warning))
                .map(|i| format!("{} {}", i.rule_id, i.message))
                .collect();
            assert!(problems.is_empty(), "{} should validate clean: {:?}", s.id, problems);
        }
    }

    #[test]
    fn samples_cover_several_versions() {
        let versions: std::collections::HashSet<_> = all().iter().map(|s| s.version).collect();
        assert!(versions.len() >= 3, "{:?}", versions);
    }
}
