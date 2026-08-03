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

use std::sync::Mutex;

use communication::mllp_listener::ListenerState;
use database::Database;
use message_store::MessageStore;
use plugins::PluginRegistry;

/// File paths the app was launched with (double-clicked / "open with").
/// Drained once by the frontend via `get_launch_files`.
pub struct LaunchFiles(pub Mutex<Vec<String>>);

/// Keep only arguments that are real files (skips the executable path and
/// any flags a launcher might add).
fn collect_file_args(argv: &[String]) -> Vec<String> {
    argv.iter()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .filter(|a| std::path::Path::new(a).is_file())
        .cloned()
        .collect()
}

#[tauri::command]
fn get_launch_files(state: tauri::State<'_, LaunchFiles>) -> Vec<String> {
    state.0.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = Database::new().expect("Failed to initialize database");
    let plugins = PluginRegistry::new();
    // Best-effort plugin load; failures surface per-file via PluginInfo.error.
    let _ = plugins.reload();

    let launch_files = collect_file_args(&std::env::args().collect::<Vec<_>>());

    tauri::Builder::default()
        // Must be the first registered plugin: a second launch (e.g. the user
        // double-clicks another .hl7 file) forwards its args here and exits,
        // instead of opening a second BridgeLab window.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            use tauri::{Emitter, Manager};
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
            let files = collect_file_args(&argv);
            if !files.is_empty() {
                let _ = app.emit("app://open-files", files);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(MessageStore::new())
        .manage(LaunchFiles(Mutex::new(launch_files)))
        .manage(db)
        .manage(plugins)
        .manage(ListenerState::new())
        .invoke_handler(tauri::generate_handler![
            commands::parser::parse_message,
            commands::parser::get_tree_children,
            commands::parser::get_field_content,
            commands::parser::search_message,
            commands::fileio::open_file,
            get_launch_files,
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
            commands::tables::get_hl7_table,
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
            commands::communication::mllp_listen_start,
            commands::communication::mllp_listen_stop,
            commands::communication::mllp_listen_status,
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
            commands::schema_export::hl7_schema_list_versions,
            commands::schema_export::hl7_schema_list_messages,
            commands::schema_export::hl7_schema_export_xsd,
            commands::batch::batch_validate,
            commands::batch::batch_anonymize,
            commands::generator::generate_test_messages,
        ])
        .run(tauri::generate_context!())
        .expect("error while running BridgeLab");
}
