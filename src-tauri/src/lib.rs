//! BridgeLab's library crate.
//!
//! Two layers live here. The headless core — parsers, validators,
//! anonymisation, plugin packs, FHIR profiles — is plain Rust and is what
//! `bridgelab-cli` is built on. The desktop shell — IPC commands, the MLLP
//! listener, telemetry, online activation and [`run`] — needs Tauri and is
//! compiled only with the `desktop` feature (on by default).

#[cfg(feature = "desktop")]
pub mod commands;
pub mod anonymization;
pub mod communication;
pub mod database;
pub mod licensing;
pub mod message_store;
pub mod parser;
pub mod plugins;
/// Proprietary (BUSL-1.1) feature implementations — see src/pro/LICENSE.
/// Compiled out entirely in Community-only builds (without the `pro`
/// feature); the MIT command shims in commands/ return a clear error
/// instead.
#[cfg(feature = "pro")]
pub mod pro;
pub mod templates;
pub mod utils;
pub mod validation;

#[cfg(feature = "desktop")]
use std::sync::Mutex;

#[cfg(feature = "desktop")]
use communication::mllp_listener::ListenerState;
#[cfg(feature = "desktop")]
use database::Database;
#[cfg(feature = "desktop")]
use message_store::MessageStore;
#[cfg(feature = "desktop")]
use plugins::PluginRegistry;

/// File paths the app was launched with (double-clicked / "open with").
/// Drained once by the frontend via `get_launch_files`.
#[cfg(feature = "desktop")]
pub struct LaunchFiles(pub Mutex<Vec<String>>);

/// Keep only arguments that are real files (skips the executable path and
/// any flags a launcher might add).
#[cfg(feature = "desktop")]
fn collect_file_args(argv: &[String]) -> Vec<String> {
    argv.iter()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .filter(|a| std::path::Path::new(a).is_file())
        .cloned()
        .collect()
}

#[cfg(feature = "desktop")]
#[tauri::command]
fn get_launch_files(state: tauri::State<'_, LaunchFiles>) -> Vec<String> {
    state.0.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default()
}

#[cfg(feature = "desktop")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = Database::new().expect("Failed to initialize database");
    let plugins = PluginRegistry::new();
    // Best-effort plugin load; failures surface per-file via PluginInfo.error.
    let _ = plugins.reload();

    // Installed FHIR profile packages. Loading is best-effort too: a package
    // that fails to read leaves the others working rather than blocking
    // startup.
    let profiles = parser::fhir::profile::ProfileRegistry::new();
    let _ = profiles.reload();

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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(MessageStore::new())
        .manage(LaunchFiles(Mutex::new(launch_files)))
        .manage(db)
        .manage(plugins)
        .manage(profiles)
        .manage(ListenerState::new())
        .manage(licensing::telemetry::UsageCounters::new())
        .setup(|app| {
            // Establish install identity, count the session and run the
            // opt-in telemetry loop (no-op while disabled; every network
            // failure is silent — telemetry must never affect the app).
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(licensing::telemetry::maybe_send_on_startup(handle));
            // Silent renewal pickup for online-activated licenses near/past
            // expiry (no-op otherwise; every failure is ignored).
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(licensing::online::maybe_refresh_on_startup(handle));
            Ok(())
        })
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
            commands::update_policy::get_update_policy,
            commands::tables::get_expected_segments,
            commands::tables::get_segment_schema,
            commands::tables::get_composite_components,
            commands::validation::validate_message,
            commands::validation::validate_fhir,
            commands::fhir_packages::fhir_packages_list,
            commands::fhir_packages::fhir_packages_reload,
            commands::fhir_packages::fhir_packages_install,
            commands::fhir_packages::fhir_packages_remove,
            commands::fhir_packages::fhir_packages_dir,
            commands::fhir_rules::fhir_rules_list,
            commands::fhir_rules::fhir_rules_save,
            commands::fhir_rules::fhir_rule_check,
            commands::fhir_rules::fhir_rule_test,
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
            commands::licensing::activate_license_online,
            commands::licensing::deactivate_license,
            commands::licensing::is_activation_code,
            commands::licensing::get_hardware_id,
            commands::licensing::get_available_features,
            commands::licensing::get_telemetry_settings,
            commands::licensing::set_telemetry_enabled,
            commands::licensing::send_telemetry_now,
            commands::licensing::get_telemetry_preview,
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
            commands::soap::soap_send,
        ])
        .build(tauri::generate_context!())
        .expect("error while running BridgeLab")
        .run(|app_handle, event| {
            // Flush the in-memory usage deltas on normal exit so short
            // sessions don't lose their counters (the periodic flusher only
            // covers sessions longer than its interval).
            if let tauri::RunEvent::Exit = event {
                use tauri::Manager;
                let db = app_handle.state::<database::Database>();
                let counters = app_handle.state::<licensing::telemetry::UsageCounters>();
                counters.flush(&db);
            }
        });
}
