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
pub mod samples;
pub mod templates;
pub mod test_packs;
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

/// Files to open in the main window: launch arguments, files forwarded by a
/// second launch, and (macOS) Finder opens. Held here until the frontend
/// has its `app://open-files` listener and has drained the queue with
/// `get_launch_files`; after that they are emitted straight away.
#[cfg(feature = "desktop")]
pub struct LaunchFiles(pub Mutex<LaunchQueue>);

#[cfg(feature = "desktop")]
#[derive(Default)]
pub struct LaunchQueue {
    files: Vec<String>,
    drained: bool,
}

/// The file arguments of a launch as absolute paths. The executable path
/// and flags are skipped; a relative path is resolved against the
/// directory the launch ran in (for a forwarded launch, that one, not ours).
/// Paths that do not exist are kept, so the frontend can say so instead of
/// the launch doing nothing.
#[cfg(feature = "desktop")]
fn collect_file_args(argv: &[String], cwd: &std::path::Path) -> Vec<String> {
    argv.iter()
        .skip(1)
        .filter(|a| !a.is_empty() && !a.starts_with('-'))
        .map(|a| absolute_path(std::path::Path::new(a), cwd).to_string_lossy().into_owned())
        .collect()
}

/// `path` made absolute against `cwd`, with `.` and `..` removed lexically
/// (no filesystem access: the file may not exist).
#[cfg(feature = "desktop")]
fn absolute_path(path: &std::path::Path, cwd: &std::path::Path) -> std::path::PathBuf {
    use std::path::Component;
    let joined = if path.is_absolute() { path.to_path_buf() } else { cwd.join(path) };
    let mut out = std::path::PathBuf::new();
    for c in joined.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                // Never above the root (or drive/UNC prefix).
                if matches!(out.components().next_back(), Some(Component::Normal(_))) {
                    out.pop();
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Hand `files` to the frontend: queue them until it has drained the
/// launch queue (an event emitted before its listener exists is lost),
/// emit them afterwards.
#[cfg(feature = "desktop")]
fn deliver_files(app: &tauri::AppHandle, files: Vec<String>) {
    use tauri::{Emitter, Manager};
    if files.is_empty() {
        return;
    }
    let state = app.state::<LaunchFiles>();
    let Ok(mut queue) = state.0.lock() else { return };
    if queue.drained {
        drop(queue);
        let _ = app.emit("app://open-files", files);
    } else {
        queue.files.extend(files);
    }
}

/// Paths of the `file://` URLs macOS hands over for a Finder open.
#[cfg(feature = "desktop")]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn file_url_paths(urls: &[tauri::Url]) -> Vec<String> {
    urls.iter()
        .filter(|u| u.scheme() == "file")
        .filter_map(|u| u.to_file_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

/// Called by the frontend once its `app://open-files` listener is in place:
/// returns the files queued so far, and later ones are emitted instead.
#[cfg(feature = "desktop")]
#[tauri::command]
fn get_launch_files(state: tauri::State<'_, LaunchFiles>) -> Vec<String> {
    state
        .0
        .lock()
        .map(|mut q| {
            q.drained = true;
            std::mem::take(&mut q.files)
        })
        .unwrap_or_default()
}

/// Id of the macOS "Quit BridgeLab" menu item that replaces the standard one.
#[cfg(all(feature = "desktop", target_os = "macos"))]
const QUIT_MENU_ID: &str = "bridgelab-quit";

/// Set by Quit so the app exits once the main window has closed.
#[cfg(all(feature = "desktop", target_os = "macos"))]
static QUIT_REQUESTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Tauri's standard macOS menu, with its Quit item (Cmd+Q) replaced by one of
/// our own. The standard item sends `terminate:`, which ends the process
/// without the window's close request: no unsaved-changes prompt and no
/// final session save. Ours closes the main window the way its red button
/// does, and the app exits once the window has actually closed.
#[cfg(all(feature = "desktop", target_os = "macos"))]
fn macos_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem, MenuItemKind};
    use tauri::Manager;
    let menu = Menu::default(app)?;
    if let Some(MenuItemKind::Submenu(app_menu)) = menu.items()?.into_iter().next() {
        let items = app_menu.items()?;
        // The application menu ends with the predefined Quit.
        if let Some(MenuItemKind::Predefined(_)) = items.last() {
            app_menu.remove_at(items.len() - 1)?;
            let label = format!("Quit {}", app.package_info().name);
            app_menu.append(&MenuItem::with_id(app, QUIT_MENU_ID, label, true, Some("CmdOrCtrl+Q"))?)?;
        }
    }
    Ok(menu)
}

/// Quit from the menu: close the main window through its close request (the
/// frontend flushes the session and asks about unsaved tabs, and may cancel).
#[cfg(all(feature = "desktop", target_os = "macos"))]
fn request_quit(app: &tauri::AppHandle) {
    use tauri::Manager;
    match app.get_webview_window("main") {
        Some(w) => {
            QUIT_REQUESTED.store(true, std::sync::atomic::Ordering::SeqCst);
            let _ = w.set_focus();
            let _ = w.close();
        }
        None => app.exit(0),
    }
}

/// App commands answer the main window only. The manual window shows
/// static pages and needs none, but without an app manifest every command
/// (save_file, open_file…) was callable from it; its capability grants only
/// the opener plugin, whose commands do not come through here.
#[cfg(feature = "desktop")]
fn main_window_only<F>(handler: F) -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static
where
    F: Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static,
{
    move |invoke| {
        if invoke.message.webview_ref().label() != "main" {
            invoke.resolver.reject("BridgeLab commands are available to the main window only");
            return true;
        }
        handler(invoke)
    }
}

/// On macOS, installs the menu whose Quit goes through the window's close
/// request; elsewhere the builder is returned as it is (there is no native
/// menu there, and closing the window already goes through that request).
#[cfg(feature = "desktop")]
fn with_macos_quit(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    #[cfg(target_os = "macos")]
    let builder = builder.menu(macos_menu).on_menu_event(|app, event| {
        if event.id() == QUIT_MENU_ID {
            request_quit(app);
        }
    });
    builder
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

    let cwd = std::env::current_dir().unwrap_or_default();
    let launch_files = collect_file_args(&std::env::args().collect::<Vec<_>>(), &cwd);

    with_macos_quit(tauri::Builder::default())
        // Must be the first registered plugin: a second launch (e.g. the user
        // double-clicks another .hl7 file) forwards its args here and exits,
        // instead of opening a second BridgeLab window.
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            use tauri::Manager;
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
            deliver_files(app, collect_file_args(&argv, std::path::Path::new(&cwd)));
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(MessageStore::new())
        .on_window_event(|window, event| {
            // After Quit, the main window closing ends the app, the manual
            // window included. (If the user cancels the close, the flag
            // stays set: the next close of the main window quits too, which
            // on macOS only differs when the manual is open.)
            #[cfg(target_os = "macos")]
            if matches!(event, tauri::WindowEvent::Destroyed)
                && window.label() == "main"
                && QUIT_REQUESTED.load(std::sync::atomic::Ordering::SeqCst)
            {
                use tauri::Manager;
                window.app_handle().exit(0);
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (window, event);
        })
        .manage(LaunchFiles(Mutex::new(LaunchQueue { files: launch_files, drained: false })))
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
        .invoke_handler(main_window_only(tauri::generate_handler![
            commands::parser::parse_message,
            commands::parser::release_messages,
            commands::parser::get_tree_children,
            commands::parser::get_segment_counts,
            commands::parser::get_segment_grid,
            commands::parser::get_field_content,
            commands::parser::search_message,
            commands::fileio::open_file,
            get_launch_files,
            commands::fileio::save_file,
            commands::fileio::file_stat,
            commands::fileio::canonical_path,
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
            commands::templates::get_samples,
            commands::test_cases::save_test_case,
            commands::test_cases::get_test_cases,
            commands::test_cases::delete_test_case,
            commands::test_cases::scan_test_cases_phi,
            commands::test_cases::export_test_cases,
            commands::test_cases::preview_test_case_import,
            commands::test_cases::import_test_cases,
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
            commands::generator::save_generated_messages,
            commands::soap::soap_send,
        ]))
        .build(tauri::generate_context!())
        .expect("error while running BridgeLab")
        .run(|app_handle, event| {
            // macOS delivers Finder opens (double-click, Open With, a drop on
            // the Dock icon) as an event, never as launch arguments.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Opened { urls } = &event {
                use tauri::Manager;
                if let Some(w) = app_handle.get_webview_window("main") {
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
                deliver_files(app_handle, file_url_paths(urls));
            }
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

#[cfg(all(test, feature = "desktop"))]
mod launch_tests {
    use super::*;
    use std::path::Path;

    #[cfg(unix)]
    #[test]
    fn launch_args_resolve_against_the_launch_directory_and_keep_missing_files() {
        let argv: Vec<String> = ["bridgelab", "--flag", "rel.hl7", "../up.hl7", "./x/../y.hl7", "/abs/missing.hl7", ""]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            collect_file_args(&argv, Path::new("/work/sub")),
            vec!["/work/sub/rel.hl7", "/work/up.hl7", "/work/sub/y.hl7", "/abs/missing.hl7"],
        );
        assert_eq!(absolute_path(Path::new("../../../a"), Path::new("/w")), Path::new("/a"));
    }

    #[cfg(unix)]
    #[test]
    fn only_file_urls_become_paths() {
        let urls = vec![
            tauri::Url::parse("file:///Users/me/My%20Docs/a%20b.hl7").unwrap(),
            tauri::Url::parse("https://example.com/x.hl7").unwrap(),
        ];
        assert_eq!(file_url_paths(&urls), vec!["/Users/me/My Docs/a b.hl7"]);
    }
}
