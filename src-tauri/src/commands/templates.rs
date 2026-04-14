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
