use tauri::State;
use crate::database::{Database, RecentFile, Preference};

#[tauri::command]
pub fn get_recent_files(limit: usize, db: State<'_, Database>) -> Result<Vec<RecentFile>, String> {
    db.get_recent_files(limit)
}

#[tauri::command]
pub fn add_recent_file(
    path: String,
    filename: String,
    message_type: String,
    version: String,
    file_size: u64,
    db: State<'_, Database>,
) -> Result<(), String> {
    db.add_recent_file(&path, &filename, &message_type, &version, file_size)
}

#[tauri::command]
pub fn remove_recent_file(path: String, db: State<'_, Database>) -> Result<(), String> {
    db.remove_recent_file(&path)
}

#[tauri::command]
pub fn clear_recent_files(db: State<'_, Database>) -> Result<(), String> {
    db.clear_recent_files()
}

#[tauri::command]
pub fn get_preference(key: String, db: State<'_, Database>) -> Result<Option<String>, String> {
    db.get_preference(&key)
}

#[tauri::command]
pub fn set_preference(key: String, value: String, db: State<'_, Database>) -> Result<(), String> {
    db.set_preference(&key, &value)
}

#[tauri::command]
pub fn get_all_preferences(db: State<'_, Database>) -> Result<Vec<Preference>, String> {
    db.get_all_preferences()
}
