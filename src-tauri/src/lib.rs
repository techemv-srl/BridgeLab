pub mod commands;
pub mod message_store;
pub mod parser;
pub mod utils;

use message_store::MessageStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(MessageStore::new())
        .invoke_handler(tauri::generate_handler![
            commands::parser::parse_message,
            commands::parser::get_tree_children,
            commands::parser::get_field_content,
            commands::fileio::open_file,
            commands::fileio::save_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running BridgeLab");
}
