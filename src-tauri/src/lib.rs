pub mod commands;
pub mod database;
pub mod message_store;
pub mod parser;
pub mod utils;
pub mod validation;

use database::Database;
use message_store::MessageStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = Database::new().expect("Failed to initialize database");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(MessageStore::new())
        .manage(db)
        .invoke_handler(tauri::generate_handler![
            commands::parser::parse_message,
            commands::parser::get_tree_children,
            commands::parser::get_field_content,
            commands::fileio::open_file,
            commands::fileio::save_file,
            commands::database::get_recent_files,
            commands::database::add_recent_file,
            commands::database::remove_recent_file,
            commands::database::clear_recent_files,
            commands::database::get_preference,
            commands::database::set_preference,
            commands::database::get_all_preferences,
            commands::tables::get_segment_info,
            commands::tables::get_field_info,
            commands::validation::validate_message,
            commands::validation::validate_fhir,
            commands::parser::parse_fhir_message,
            commands::parser::get_fhir_tree_children,
        ])
        .run(tauri::generate_context!())
        .expect("error while running BridgeLab");
}
