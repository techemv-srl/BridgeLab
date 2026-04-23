pub mod commands;
pub mod anonymization;
pub mod communication;
pub mod database;
pub mod licensing;
pub mod message_store;
pub mod parser;
pub mod plugins;
pub mod templates;
pub mod utils;
pub mod validation;

use database::Database;
use message_store::MessageStore;
use plugins::PluginRegistry;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = Database::new().expect("Failed to initialize database");
    let plugins = PluginRegistry::new();
    // Best-effort plugin load; failures surface per-file via PluginInfo.error.
    let _ = plugins.reload();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(MessageStore::new())
        .manage(db)
        .manage(plugins)
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
            commands::database::save_session,
            commands::database::load_session,
            commands::database::clear_session,
            commands::tables::get_segment_info,
            commands::tables::get_field_info,
            commands::validation::validate_message,
            commands::validation::validate_fhir,
            commands::parser::parse_fhir_message,
            commands::parser::get_fhir_tree_children,
            commands::parser::analyze_fhir_bundle,
            commands::parser::get_fhir_bundle_entry,
            commands::parser::evaluate_fhirpath,
            commands::parser::expand_field_inline,
            commands::parser::expand_all_fields,
            commands::parser::collapse_all_fields,
            commands::communication::mllp_send,
            commands::communication::mllp_receive,
            commands::communication::http_request,
            commands::communication::generate_ack,
            commands::communication::save_connection_profile,
            commands::communication::get_connection_profiles,
            commands::communication::delete_connection_profile,
            commands::communication::get_request_history,
            commands::communication::clear_request_history,
            commands::anonymization::detect_phi,
            commands::anonymization::anonymize_message,
            commands::anonymization::get_message_full_text,
            commands::anonymization::get_message_truncated_text,
            commands::anonymization::export_as_json,
            commands::anonymization::export_as_csv,
            commands::licensing::check_license,
            commands::licensing::activate_license,
            commands::licensing::deactivate_license,
            commands::licensing::get_hardware_id,
            commands::licensing::get_available_features,
            commands::templates::get_templates,
            commands::templates::get_templates_grouped,
            commands::test_cases::save_test_case,
            commands::test_cases::get_test_cases,
            commands::test_cases::delete_test_case,
            commands::plugins::list_plugins,
            commands::plugins::reload_plugins,
            commands::plugins::set_plugin_enabled,
            commands::plugins::apply_plugin_overrides,
            commands::plugins::get_plugins_dir,
            commands::plugins::open_plugins_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running BridgeLab");
}
