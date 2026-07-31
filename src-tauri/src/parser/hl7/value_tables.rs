//! HL7 value tables ("HL7 tables") for coded fields.
//!
//! A pragmatic subset of the official tables covering the coded fields an
//! integration engineer hits daily (PID-8 sex, PV1-2 patient class, MSA-1
//! ack code, ORC-1 order control, ...). The mapping from (segment,
//! position) to table id lives here too, so `tables::get_field_info` can
//! attach a `table_id` without touching every hardcoded FieldDef.
//!
//! Values are stable across v2.3–v2.8 for this subset; version-specific
//! tables can be layered in later via the hl7-schema-importer.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TableValue {
    pub code: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValueTable {
    pub id: String,
    pub name: &'static str,
    pub values: Vec<TableValue>,
}

/// Map a (segment, field position) to the HL7 table id its values come from.
pub fn table_for_field(segment: &str, position: usize) -> Option<&'static str> {
    Some(match (segment, position) {
        ("PID", 8) => "0001",
        ("PID", 16) => "0002",
        ("PID", 24) | ("PID", 30) => "0136",
        ("PV1", 2) => "0004",
        ("PV1", 4) => "0007",
        ("MSH", 9) => "0076",
        ("MSH", 11) => "0103",
        ("MSA", 1) => "0008",
        ("ORC", 1) => "0119",
        ("ORC", 5) => "0038",
        ("OBX", 11) => "0085",
        ("OBR", 25) => "0123",
        ("AL1", 2) => "0127",
        ("DG1", 6) => "0052",
        _ => return None,
    })
}

fn v(code: &'static str, description: &'static str) -> TableValue {
    TableValue { code, description }
}

/// Return the named value table, or None for unknown ids.
pub fn get_table(id: &str) -> Option<ValueTable> {
    let (name, values): (&'static str, Vec<TableValue>) = match id {
        "0001" => ("Administrative Sex", vec![
            v("F", "Female"), v("M", "Male"), v("O", "Other"),
            v("U", "Unknown"), v("A", "Ambiguous"), v("N", "Not applicable"),
        ]),
        "0002" => ("Marital Status", vec![
            v("S", "Single"), v("M", "Married"), v("D", "Divorced"),
            v("W", "Widowed"), v("A", "Separated"), v("C", "Common law"),
            v("G", "Living together"), v("P", "Domestic partner"),
            v("E", "Legally separated"), v("U", "Unknown"), v("O", "Other"),
        ]),
        "0004" => ("Patient Class", vec![
            v("I", "Inpatient"), v("O", "Outpatient"), v("E", "Emergency"),
            v("P", "Preadmit"), v("R", "Recurring patient"), v("B", "Obstetrics"),
            v("C", "Commercial account"), v("N", "Not applicable"), v("U", "Unknown"),
        ]),
        "0007" => ("Admission Type", vec![
            v("A", "Accident"), v("C", "Elective"), v("E", "Emergency"),
            v("L", "Labor and delivery"), v("N", "Newborn"), v("R", "Routine"),
            v("U", "Urgent"),
        ]),
        "0008" => ("Acknowledgment Code", vec![
            v("AA", "Application Accept"), v("AE", "Application Error"),
            v("AR", "Application Reject"), v("CA", "Commit Accept"),
            v("CE", "Commit Error"), v("CR", "Commit Reject"),
        ]),
        "0038" => ("Order Status", vec![
            v("A", "Some, but not all, results available"), v("CA", "Order was canceled"),
            v("CM", "Order is completed"), v("DC", "Order was discontinued"),
            v("ER", "Error, order not found"), v("HD", "Order is on hold"),
            v("IP", "In process, unspecified"), v("RP", "Order has been replaced"),
            v("SC", "In process, scheduled"),
        ]),
        "0052" => ("Diagnosis Type", vec![
            v("A", "Admitting"), v("W", "Working"), v("F", "Final"),
        ]),
        "0076" => ("Message Type", vec![
            v("ACK", "General acknowledgment"), v("ADT", "Admit/discharge/transfer"),
            v("BAR", "Billing account record"), v("DFT", "Detailed financial transaction"),
            v("MDM", "Medical document management"), v("OML", "Laboratory order"),
            v("ORM", "Order message"), v("ORU", "Observation result / unsolicited"),
            v("OUL", "Unsolicited laboratory observation"), v("QRY", "Query"),
            v("RAS", "Pharmacy/treatment administration"), v("RDE", "Pharmacy/treatment encoded order"),
            v("RSP", "Segment pattern response"), v("SIU", "Scheduling information unsolicited"),
            v("VXU", "Unsolicited vaccination record update"),
        ]),
        "0085" => ("Observation Result Status", vec![
            v("C", "Correction to results"), v("D", "Deleted"),
            v("F", "Final results"), v("I", "Specimen in lab, pending"),
            v("P", "Preliminary results"), v("R", "Results entered, not verified"),
            v("S", "Partial results"), v("U", "Change to final without retransmit"),
            v("W", "Post original as wrong"), v("X", "Results cannot be obtained"),
        ]),
        "0103" => ("Processing ID", vec![
            v("D", "Debugging"), v("P", "Production"), v("T", "Training"),
        ]),
        "0119" => ("Order Control Codes", vec![
            v("NW", "New order/service"), v("OK", "Order accepted"),
            v("CA", "Cancel order request"), v("CR", "Canceled as requested"),
            v("DC", "Discontinue order request"), v("HD", "Hold order request"),
            v("RL", "Release previous hold"), v("RE", "Observations to follow"),
            v("SC", "Status changed"), v("XO", "Change order request"),
            v("XX", "Order changed, unsolicited"),
        ]),
        "0123" => ("Result Status", vec![
            v("O", "Order received, specimen not yet received"),
            v("I", "No results, specimen in lab"),
            v("S", "Procedure scheduled, not done"),
            v("A", "Some results available"),
            v("P", "Preliminary results"),
            v("C", "Correction of previously transmitted results"),
            v("R", "Results stored, not yet verified"),
            v("F", "Final results, verified"),
            v("X", "Results cannot be obtained"),
            v("Y", "No order on record"),
            v("Z", "No record of the patient"),
        ]),
        "0127" => ("Allergen Type", vec![
            v("DA", "Drug allergy"), v("FA", "Food allergy"),
            v("MA", "Miscellaneous allergy"), v("MC", "Miscellaneous contraindication"),
            v("EA", "Environmental allergy"), v("AA", "Animal allergy"),
            v("PA", "Plant allergy"), v("LA", "Pollen allergy"),
        ]),
        "0136" => ("Yes/No Indicator", vec![
            v("Y", "Yes"), v("N", "No"),
        ]),
        _ => return None,
    };
    Some(ValueTable { id: id.to_string(), name, values })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_mapping_hits() {
        assert_eq!(table_for_field("PID", 8), Some("0001"));
        assert_eq!(table_for_field("PV1", 2), Some("0004"));
        assert_eq!(table_for_field("MSA", 1), Some("0008"));
        assert_eq!(table_for_field("PID", 5), None);
    }

    #[test]
    fn test_every_mapped_table_exists() {
        // Every table id reachable from the field mapping must resolve.
        let mapped = [
            "0001", "0002", "0004", "0007", "0008", "0038", "0052",
            "0076", "0085", "0103", "0119", "0123", "0127", "0136",
        ];
        for id in mapped {
            let t = get_table(id).unwrap_or_else(|| panic!("table {} missing", id));
            assert!(!t.values.is_empty(), "table {} empty", id);
            assert_eq!(t.id, id);
        }
    }

    #[test]
    fn test_sex_table_contents() {
        let t = get_table("0001").unwrap();
        assert_eq!(t.name, "Administrative Sex");
        assert!(t.values.iter().any(|tv| tv.code == "F" && tv.description == "Female"));
        assert!(t.values.iter().any(|tv| tv.code == "M"));
    }

    #[test]
    fn test_unknown_table() {
        assert!(get_table("9999").is_none());
    }
}
