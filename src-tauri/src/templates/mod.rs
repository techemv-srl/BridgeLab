use serde::{Deserialize, Serialize};
use chrono::Utc;

/// A message template definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTemplate {
    pub id: String,
    pub name: String,
    pub message_type: String,
    pub description: String,
    pub category: String,
    pub content: String,
}

/// Get all built-in templates with placeholders filled with current timestamps.
pub fn get_builtin_templates() -> Vec<MessageTemplate> {
    let now = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let msg_id = format!("MSG{}", Utc::now().format("%Y%m%d%H%M%S"));

    vec![
        MessageTemplate {
            id: "adt-a01".into(),
            name: "ADT^A01 - Patient Admission".into(),
            message_type: "ADT".into(),
            category: "Admission / Discharge / Transfer".into(),
            description: "Patient admission / visit notification".into(),
            content: format!(
                "MSH|^~\\&|SENDING_APP|SENDING_FAC|RECEIVING_APP|RECEIVING_FAC|{now}||ADT^A01|{msg_id}|P|2.5\r\
                EVN|A01|{now}\r\
                PID|1||MRN001^^^HOSPITAL^MR||DOE^JOHN^A||19800101|M|||123 MAIN ST^^CITY^ST^12345||555-0100|||M|\r\
                NK1|1|DOE^JANE|SPO|123 MAIN ST^^CITY^ST^12345|555-0101|\r\
                PV1|1|I|WARD01^101^A||||ATTENDING^DOC^M|||MED|||||||INS001||||||||||||||||||||||||||{now}\r",
                now = now, msg_id = msg_id
            ),
        },
        MessageTemplate {
            id: "adt-a03".into(),
            name: "ADT^A03 - Patient Discharge".into(),
            message_type: "ADT".into(),
            category: "Admission / Discharge / Transfer".into(),
            description: "Patient discharge notification".into(),
            content: format!(
                "MSH|^~\\&|SENDING_APP|SENDING_FAC|RECEIVING_APP|RECEIVING_FAC|{now}||ADT^A03|{msg_id}|P|2.5\r\
                EVN|A03|{now}\r\
                PID|1||MRN001^^^HOSPITAL^MR||DOE^JOHN^A||19800101|M|||123 MAIN ST^^CITY^ST^12345|\r\
                PV1|1|I|WARD01^101^A||||ATTENDING^DOC^M|||MED|||||||||||||||||||||||||||||||||{now}|{now}\r",
                now = now, msg_id = msg_id
            ),
        },
        MessageTemplate {
            id: "adt-a04".into(),
            name: "ADT^A04 - Patient Registration".into(),
            message_type: "ADT".into(),
            category: "Admission / Discharge / Transfer".into(),
            description: "Outpatient registration".into(),
            content: format!(
                "MSH|^~\\&|SENDING_APP|SENDING_FAC|RECEIVING_APP|RECEIVING_FAC|{now}||ADT^A04|{msg_id}|P|2.5\r\
                EVN|A04|{now}\r\
                PID|1||MRN001^^^HOSPITAL^MR||DOE^JOHN^A||19800101|M|||123 MAIN ST^^CITY^ST^12345|\r\
                PV1|1|O|CLINIC01||||REFERRING^DOC^M|||OUT|\r",
                now = now, msg_id = msg_id
            ),
        },
        MessageTemplate {
            id: "oru-r01".into(),
            name: "ORU^R01 - Lab Results".into(),
            message_type: "ORU".into(),
            category: "Observation / Result".into(),
            description: "Unsolicited observation message (lab results)".into(),
            content: format!(
                "MSH|^~\\&|LAB_SYS|LAB|HOSPITAL|MAIN|{now}||ORU^R01|{msg_id}|P|2.5\r\
                PID|1||MRN001^^^HOSPITAL^MR||DOE^JOHN||19800101|M|\r\
                PV1|1|O|CLINIC01||||REFERRING^DOC^M|\r\
                OBR|1|ORDER001|FILLER001|CBC^Complete Blood Count^L|||{now}|||||||||REFERRING^DOC^M|||||||||F\r\
                OBX|1|NM|WBC^White Blood Cell^L||7.5|10*3/uL|4.5-11.0|N|||F\r\
                OBX|2|NM|RBC^Red Blood Cell^L||4.8|10*6/uL|4.2-5.4|N|||F\r\
                OBX|3|NM|HGB^Hemoglobin^L||14.2|g/dL|13.5-17.5|N|||F\r\
                OBX|4|NM|PLT^Platelets^L||250|10*3/uL|150-400|N|||F\r",
                now = now, msg_id = msg_id
            ),
        },
        MessageTemplate {
            id: "orm-o01".into(),
            name: "ORM^O01 - General Order".into(),
            message_type: "ORM".into(),
            category: "Order Management".into(),
            description: "New laboratory or radiology order".into(),
            content: format!(
                "MSH|^~\\&|ORDERING_APP|CLINIC|LAB_SYS|LAB|{now}||ORM^O01|{msg_id}|P|2.5\r\
                PID|1||MRN001^^^HOSPITAL^MR||DOE^JOHN||19800101|M|\r\
                PV1|1|O|CLINIC01||||ORDERING^DOC^M|\r\
                ORC|NW|ORDER001|||IP||^^^{now}||{now}|||ORDERING^DOC^M\r\
                OBR|1|ORDER001||CBC^Complete Blood Count^L|R|||||||||||ORDERING^DOC^M\r",
                now = now, msg_id = msg_id
            ),
        },
        MessageTemplate {
            id: "siu-s12".into(),
            name: "SIU^S12 - New Appointment".into(),
            message_type: "SIU".into(),
            category: "Scheduling".into(),
            description: "New appointment booking".into(),
            content: format!(
                "MSH|^~\\&|SCHED_APP|CLINIC|EMR|MAIN|{now}||SIU^S12|{msg_id}|P|2.5\r\
                SCH|APPT001|FILLER001|||||^^^NORMAL||||^^30^{now}||||PROVIDER^DOC^M\r\
                PID|1||MRN001^^^HOSPITAL^MR||DOE^JOHN||19800101|M|\r\
                PV1|1|O|CLINIC01|\r\
                RGS|1\r\
                AIS|1||VISIT^Office Visit\r\
                AIL|1||CLINIC01^^^CLINIC\r\
                AIP|1||PROVIDER^DOC^M|\r",
                now = now, msg_id = msg_id
            ),
        },
        MessageTemplate {
            id: "mdm-t02".into(),
            name: "MDM^T02 - Document Notification".into(),
            message_type: "MDM".into(),
            category: "Medical Document".into(),
            description: "Original document notification".into(),
            content: format!(
                "MSH|^~\\&|DOC_SYS|CLINIC|EMR|MAIN|{now}||MDM^T02|{msg_id}|P|2.5\r\
                EVN|T02|{now}\r\
                PID|1||MRN001^^^HOSPITAL^MR||DOE^JOHN||19800101|M|\r\
                PV1|1|O|CLINIC01||||ATTENDING^DOC^M|\r\
                TXA|1|DS|TX||{now}||||PROVIDER^DOC^M||||DOC001||AU\r\
                OBX|1|TX|NOTE||Patient presents with symptoms of...||||||F\r",
                now = now, msg_id = msg_id
            ),
        },
        MessageTemplate {
            id: "ack".into(),
            name: "ACK - Generic Acknowledgment".into(),
            message_type: "ACK".into(),
            category: "Acknowledgment".into(),
            description: "Generic acknowledgment message".into(),
            content: format!(
                "MSH|^~\\&|RECEIVER|FAC|SENDER|FAC|{now}||ACK|{msg_id}|P|2.5\r\
                MSA|AA|ORIGINAL_MSG_ID|Message processed successfully\r",
                now = now, msg_id = msg_id
            ),
        },
        MessageTemplate {
            id: "fhir-patient".into(),
            name: "FHIR Patient (JSON)".into(),
            message_type: "FHIR".into(),
            category: "FHIR".into(),
            description: "Minimal Patient resource".into(),
            content: r#"{
  "resourceType": "Patient",
  "id": "example",
  "identifier": [{ "system": "urn:oid:2.16.840.1.113883.2.9.4.3.2", "value": "MRN001" }],
  "name": [{ "family": "Doe", "given": ["John"] }],
  "gender": "male",
  "birthDate": "1980-01-01",
  "address": [{ "line": ["123 Main St"], "city": "City", "postalCode": "12345" }]
}"#.into(),
        },
        MessageTemplate {
            id: "fhir-observation".into(),
            name: "FHIR Observation - Blood Pressure (JSON)".into(),
            message_type: "FHIR".into(),
            category: "FHIR".into(),
            description: "Vital-signs Observation with two components".into(),
            content: r#"{
  "resourceType": "Observation",
  "id": "bp-example",
  "status": "final",
  "category": [{ "coding": [{ "system": "http://terminology.hl7.org/CodeSystem/observation-category", "code": "vital-signs" }] }],
  "code": { "coding": [{ "system": "http://loinc.org", "code": "85354-9", "display": "Blood pressure panel" }] },
  "subject": { "reference": "Patient/example" },
  "effectiveDateTime": "2026-01-15T09:30:00Z",
  "component": [
    { "code": { "coding": [{ "system": "http://loinc.org", "code": "8480-6", "display": "Systolic" }] }, "valueQuantity": { "value": 120, "unit": "mmHg" } },
    { "code": { "coding": [{ "system": "http://loinc.org", "code": "8462-4", "display": "Diastolic" }] }, "valueQuantity": { "value": 80, "unit": "mmHg" } }
  ]
}"#.into(),
        },
        MessageTemplate {
            id: "fhir-bundle-transaction".into(),
            name: "FHIR Bundle - Transaction (JSON)".into(),
            message_type: "FHIR".into(),
            category: "FHIR".into(),
            description: "Transaction Bundle: Patient + two Observations referencing it (try the Bundle visualizer)".into(),
            content: r#"{
  "resourceType": "Bundle",
  "type": "transaction",
  "entry": [
    {
      "fullUrl": "urn:uuid:61ebe359-bfdc-4613-8bf2-c5e300945f0a",
      "resource": {
        "resourceType": "Patient",
        "name": [{ "family": "Doe", "given": ["John"] }],
        "gender": "male",
        "birthDate": "1980-01-01"
      },
      "request": { "method": "POST", "url": "Patient" }
    },
    {
      "fullUrl": "urn:uuid:3f2504e0-4f89-41d3-9a0c-0305e82c3301",
      "resource": {
        "resourceType": "Observation",
        "status": "final",
        "code": { "coding": [{ "system": "http://loinc.org", "code": "8867-4", "display": "Heart rate" }] },
        "subject": { "reference": "urn:uuid:61ebe359-bfdc-4613-8bf2-c5e300945f0a" },
        "valueQuantity": { "value": 72, "unit": "beats/minute" }
      },
      "request": { "method": "POST", "url": "Observation" }
    },
    {
      "fullUrl": "urn:uuid:9c47a2c8-2d1e-4d8a-9f3b-6a1e5b2c7d90",
      "resource": {
        "resourceType": "Observation",
        "status": "final",
        "code": { "coding": [{ "system": "http://loinc.org", "code": "8310-5", "display": "Body temperature" }] },
        "subject": { "reference": "urn:uuid:61ebe359-bfdc-4613-8bf2-c5e300945f0a" },
        "valueQuantity": { "value": 36.8, "unit": "Cel" }
      },
      "request": { "method": "POST", "url": "Observation" }
    }
  ]
}"#.into(),
        },
    ]
}

/// Get templates grouped by category.
pub fn get_templates_by_category() -> Vec<(String, Vec<MessageTemplate>)> {
    let templates = get_builtin_templates();
    let mut categories: std::collections::BTreeMap<String, Vec<MessageTemplate>> =
        std::collections::BTreeMap::new();

    for t in templates {
        categories.entry(t.category.clone()).or_default().push(t);
    }

    categories.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_templates_valid_msh() {
        for t in get_builtin_templates() {
            if t.category == "FHIR" {
                continue; // JSON resources, covered by test_fhir_templates_valid_json
            }
            assert!(t.content.starts_with("MSH|"), "Template {} must start with MSH", t.id);
            assert!(t.content.contains('\r'), "Template {} must use CR line endings", t.id);
        }
    }

    #[test]
    fn test_fhir_templates_valid_json() {
        let fhir: Vec<_> = get_builtin_templates()
            .into_iter()
            .filter(|t| t.category == "FHIR")
            .collect();
        assert!(fhir.len() >= 3, "expected the FHIR template set");
        for t in fhir {
            let v: serde_json::Value = serde_json::from_str(&t.content)
                .unwrap_or_else(|e| panic!("Template {} is not valid JSON: {}", t.id, e));
            assert!(
                v.get("resourceType").and_then(|r| r.as_str()).is_some(),
                "Template {} must declare resourceType",
                t.id
            );
            // urn:uuid fullUrls must carry real RFC 4122 UUIDs — servers
            // that validate URNs reject placeholders like urn:uuid:patient-1
            if let Some(entries) = v.get("entry").and_then(|e| e.as_array()) {
                for entry in entries {
                    if let Some(full_url) = entry.get("fullUrl").and_then(|u| u.as_str()) {
                        if let Some(uuid) = full_url.strip_prefix("urn:uuid:") {
                            let parts: Vec<usize> =
                                uuid.split('-').map(|p| p.len()).collect();
                            assert_eq!(
                                parts,
                                vec![8, 4, 4, 4, 12],
                                "Template {}: '{}' is not a valid UUID URN",
                                t.id, full_url
                            );
                            assert!(
                                uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-'),
                                "Template {}: '{}' contains non-hex characters",
                                t.id, full_url
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_templates_have_ids() {
        let templates = get_builtin_templates();
        let ids: std::collections::HashSet<&str> = templates.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids.len(), templates.len(), "All template IDs must be unique");
    }
}
