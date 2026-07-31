use crate::parser::hl7::tables::{self, SegmentInfo, FieldInfo};

#[tauri::command]
pub fn get_segment_info(segment_type: String, version: String) -> Result<Option<SegmentInfo>, String> {
    Ok(tables::get_segment_info(&segment_type, &version))
}

#[tauri::command]
pub fn get_field_info(
    segment_type: String,
    field_position: usize,
    version: String,
) -> Result<Option<FieldInfo>, String> {
    Ok(tables::get_field_info(&segment_type, field_position, &version))
}

/// Return the values of an HL7 value table (e.g. "0001" Administrative Sex).
#[tauri::command]
pub fn get_hl7_table(table_id: String) -> Option<crate::parser::hl7::value_tables::ValueTable> {
    crate::parser::hl7::value_tables::get_table(&table_id)
}
