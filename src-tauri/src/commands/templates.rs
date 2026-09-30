use crate::templates::{self, MessageTemplate};

/// Get all built-in message templates.
#[tauri::command]
pub fn get_templates() -> Result<Vec<MessageTemplate>, String> {
    Ok(templates::get_builtin_templates())
}

/// Get templates grouped by category.
#[tauri::command]
pub fn get_templates_grouped() -> Result<Vec<(String, Vec<MessageTemplate>)>, String> {
    Ok(templates::get_templates_by_category())
}

/// The sample message library: complete HL7 v2 examples in several
/// versions, every edition.
#[tauri::command]
pub fn get_samples() -> Vec<crate::samples::Sample> {
    crate::samples::all()
}
