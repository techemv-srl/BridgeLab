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
