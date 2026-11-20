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
#[cfg(feature = "desktop")]
mod navigation;
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

/// Build code shown next to the version, so two builds of the same version
/// can be told apart: the commit date and short hash (`20261117.762cc1c`),
/// or `BRIDGELAB_BUILD` when the build sets it. See build.rs.
pub const BUILD_ID: &str = env!("BRIDGELAB_BUILD");

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
///
/// A path that is not valid Unicode (a file or folder named in Latin-1 on
/// Linux) is kept too, with the bytes it cannot show as U+FFFD. The
/// frontend handles paths as text, so it cannot open that file, but it says
/// so: reading such an argument as a `String` ended the launch in a panic
/// before any window, and a second launch the same way.
#[cfg(feature = "desktop")]
fn collect_file_args<A: AsRef<std::ffi::OsStr>>(argv: &[A], cwd: &std::path::Path) -> Vec<String> {
    argv.iter()
        .skip(1)
        .map(AsRef::as_ref)
        .filter(|a| !a.is_empty() && !a.as_encoded_bytes().starts_with(b"-"))
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

/// A launch forwarded by a second instance, or a Finder open on macOS:
/// bring the main window to the front and open `files` in it.
///
/// The main window is there for as long as the app runs (the app ends
/// when it closes), but a launch must not be dropped without a trace when
/// it is not: the files wait in the launch queue until a main window's
/// page drains it. With no window at all the app is still starting (macOS
/// can hand over a Finder open before the configured windows exist) and
/// the main window is on its way. With other windows up and no main one,
/// a new one is made from the app's configuration.
#[cfg(feature = "desktop")]
fn show_main_window(app: &tauri::AppHandle, files: Vec<String>) {
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.set_focus();
        deliver_files(app, files);
        return;
    }
    // A new page asks for the queue once its listener is in place; an
    // event emitted before that would be lost.
    if let Ok(mut queue) = app.state::<LaunchFiles>().0.lock() {
        queue.drained = false;
        queue.files.extend(files);
    }
    // Before startup has made the main window, making one here would take
    // its label, and startup would fail on it.
    if app.webview_windows().is_empty() {
        return;
    }
    let app = app.clone();
    // Not built here: this runs inside the event loop's own handlers, where
    // creating a webview can deadlock on Windows.
    tauri::async_runtime::spawn(async move {
        let Some(config) = app.config().app.windows.iter().find(|w| w.label == "main").cloned() else {
            return;
        };
        if app.get_webview_window("main").is_none() {
            let _ = tauri::WebviewWindowBuilder::from_config(&app, &config).and_then(|b| b.build());
        }
    });
}

/// Linux: the single-instance plugin finds the running BridgeLab over the
/// D-Bus session bus. Without a usable bus (none, an address that points
/// nowhere, a malformed one) it lets every launch start, without a word,
/// and two or three BridgeLab windows then shared one database, each
/// writing its own tabs over the others' saved session.
///
/// A lock file, held for as long as the app runs, catches that case: a
/// launch that finds it taken is a second BridgeLab the guard did not stop
/// (with a working bus it would have handed its files over and exited
/// before getting here). It still opens its files, so nothing the user
/// asked for is dropped, but on a database in memory, so it cannot
/// overwrite the first one's session and settings; it logs why and the
/// window says so. The lock is an `flock`, released by the kernel when the
/// process ends however it ends, so a crash leaves nothing stale behind.
#[cfg(all(feature = "desktop", target_os = "linux"))]
mod instance_lock {
    use std::fs::File;
    use std::os::unix::io::AsRawFd;
    use std::path::Path;
    use std::time::{Duration, Instant};

    /// Held by the managed state until the process ends.
    pub struct InstanceLock(#[allow(dead_code)] pub File);

    /// Take the lock at `path`. `Ok(None)` when another process holds it
    /// for longer than `wait` (a restart starts the new process while the
    /// old one is still closing, which takes a moment).
    pub fn acquire(path: &Path, wait: Duration) -> std::io::Result<Option<File>> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        // std opens with O_CLOEXEC: a program the app starts (xdg-open, a
        // file manager that stays open) must not inherit the lock and keep
        // it after BridgeLab has exited.
        let file = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path)?;
        let deadline = Instant::now() + wait;
        loop {
            // SAFETY: a valid descriptor owned by `file`, which outlives the call.
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
                return Ok(Some(file));
            }
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() != Some(libc::EWOULDBLOCK) {
                return Err(err);
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// What the session bus looked like to this launch, for the log line
    /// and the notice's details.
    pub fn bus_description() -> String {
        match std::env::var_os("DBUS_SESSION_BUS_ADDRESS") {
            None => "DBUS_SESSION_BUS_ADDRESS is not set".to_string(),
            Some(a) => format!("DBUS_SESSION_BUS_ADDRESS is \"{}\"", a.to_string_lossy()),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_second_holder_is_told_the_lock_is_taken_until_the_first_lets_go() {
            let dir = std::env::temp_dir().join(format!("bl-instance-lock-{}", std::process::id()));
            let path = dir.join("BridgeLab").join("instance.lock");
            let first = acquire(&path, Duration::ZERO).unwrap().expect("free lock");
            // flock belongs to the open file, so a second opening in this
            // process is refused just as another process would be.
            let started = Instant::now();
            assert!(acquire(&path, Duration::from_millis(120)).unwrap().is_none());
            assert!(started.elapsed() >= Duration::from_millis(120), "waited for the holder");
            drop(first);
            assert!(acquire(&path, Duration::ZERO).unwrap().is_some(), "free again once the holder is gone");
            std::fs::remove_dir_all(&dir).ok();
        }
    }
}

/// Linux: take the instance lock, or, when another BridgeLab holds it, go on
/// as a second window that keeps nothing (see [`instance_lock`]).
#[cfg(all(feature = "desktop", target_os = "linux"))]
fn guard_second_instance(app: &tauri::App) {
    use tauri::Manager;
    let Some(path) = Database::db_path().ok().and_then(|p| p.parent().map(|d| d.join("instance.lock"))) else {
        return;
    };
    match instance_lock::acquire(&path, std::time::Duration::from_secs(2)) {
        Ok(Some(file)) => {
            app.manage(instance_lock::InstanceLock(file));
        }
        Ok(None) => {
            let reason = format!(
                "Another BridgeLab holds {} and could not be reached: the single-instance check needs a D-Bus session bus ({}).",
                path.display(),
                instance_lock::bus_description()
            );
            eprintln!("BridgeLab: {reason} This window works on a database in memory.");
            app.state::<Database>().switch_to_memory();
            if let Ok(mut notice) = app.state::<commands::database::DatabaseFallbackNotice>().0.lock() {
                let db = Database::db_path().map(|p| p.display().to_string()).unwrap_or_default();
                *notice = Some(database::DatabaseFallback { path: db, reason, second_instance: true });
            }
        }
        // A data folder that cannot be written: the database could not be
        // opened either, and the window already says so.
        Err(e) => eprintln!("BridgeLab: the instance lock {} could not be taken: {e}", path.display()),
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

/// The build code ([`BUILD_ID`]) for the About dialog.
#[cfg(feature = "desktop")]
#[tauri::command]
fn get_build_id() -> &'static str {
    BUILD_ID
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

/// Wakes the signal handler once the window has written what it holds.
#[cfg(feature = "desktop")]
#[derive(Default)]
pub struct OsEnding(tokio::sync::Notify);

/// Called by the window after `app://os-ending`, once it has handed over its
/// latest tabs (and, with restore on, written the session).
#[cfg(feature = "desktop")]
#[tauri::command]
fn os_ending_ready(state: tauri::State<'_, OsEnding>) {
    // A permit is kept if the handler is not waiting yet.
    state.0.notify_one();
}

/// How long the window gets to write what it holds when the system ends
/// the app.
#[cfg(all(feature = "desktop", unix))]
const OS_ENDING_WAIT: std::time::Duration = std::time::Duration::from_secs(3);

/// Linux and macOS: SIGTERM and SIGHUP (log out or shut down through the
/// session manager or systemd, `kill`, a closed terminal) ended the app at
/// once, without the window's close request: with session restore off the
/// unsaved tabs were gone without a question, and with it on the edits
/// since the last autosave were lost. Now the window is told
/// (`app://os-ending`) and hands over its latest tabs (and writes the
/// session, with restore on), then the app exits the normal way, where the
/// exit event keeps that snapshot (see [`commands::database::ExitSnapshot`]).
/// Unlike the Windows end of session, the window can be waited for here:
/// this runs on a thread of its own. A window that does not answer within
/// [`OS_ENDING_WAIT`] does not hold the app up, and a second signal, or a
/// stuck exit, ends it at once, keeping the last snapshot first.
#[cfg(all(feature = "desktop", unix))]
fn watch_termination_signals(app: tauri::AppHandle) {
    use tauri::{Emitter, Manager};
    use tokio::signal::unix::{signal, SignalKind};
    tauri::async_runtime::spawn(async move {
        let (Ok(mut term), Ok(mut hup)) = (signal(SignalKind::terminate()), signal(SignalKind::hangup())) else {
            return;
        };
        tokio::select! {
            _ = term.recv() => {}
            _ = hup.recv() => {}
        }
        let watchdog = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::select! {
                _ = term.recv() => {}
                _ = hup.recv() => {}
                _ = tokio::time::sleep(OS_ENDING_WAIT * 3) => {}
            }
            let db = watchdog.state::<Database>();
            watchdog.state::<commands::database::ExitSnapshot>().save_to(&db);
            std::process::exit(0);
        });
        if app.emit_to("main", "app://os-ending", ()).is_ok() {
            let ending = app.state::<OsEnding>();
            let _ = tokio::time::timeout(OS_ENDING_WAIT, ending.0.notified()).await;
        }
        app.exit(0);
    });
}

/// Id of the macOS "Quit BridgeLab" menu item that replaces the standard one.
#[cfg(all(feature = "desktop", target_os = "macos"))]
const QUIT_MENU_ID: &str = "bridgelab-quit";

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
        // The app exits once the window has closed (see `on_window_event`).
        Some(w) => {
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
    // A database file that cannot be opened must not stop the app: it runs
    // on one in memory, and the window says why once it is up.
    let (db, db_fallback) = database::open_or_in_memory(Database::db_path());
    let plugins = PluginRegistry::new();
    // Best-effort plugin load; failures surface per-file via PluginInfo.error.
    // It runs on a thread of its own once the app is set up (below), so the
    // window does not wait for it: compiling the patterns of a large pack
    // once held startup back by twenty seconds. Until it is done, whatever
    // reads the packs waits for it rather than run without them.
    plugins.begin_background_load();

    // Installed FHIR profile packages. Loading is best-effort too: a package
    // that fails to read leaves the others working rather than blocking
    // startup.
    let profiles = parser::fhir::profile::ProfileRegistry::new();
    let _ = profiles.reload();

    let cwd = std::env::current_dir().unwrap_or_default();
    let launch_files = collect_file_args(&std::env::args_os().collect::<Vec<_>>(), &cwd);

    with_macos_quit(tauri::Builder::default())
        // Must be the first registered plugin: a second launch (e.g. the user
        // double-clicks another .hl7 file) forwards its args here and exits,
        // instead of opening a second BridgeLab window. On Linux it needs
        // the D-Bus session bus; see `guard_second_instance` for a launch
        // it could not check.
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            show_main_window(app, collect_file_args(&argv, std::path::Path::new(&cwd)));
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        // The app's windows show its own pages only: a link the webview
        // follows by itself opens in the browser or mail client instead.
        .plugin(navigation::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(MessageStore::new())
        .on_window_event(|window, event| {
            // The main window is the app: once it has closed (its close
            // request has saved the session and asked about unsaved tabs),
            // the app ends, the manual window included. Left running with
            // only the manual, it kept the MLLP listener bound, and every
            // later launch, a double-clicked file too, went to a main
            // window that was no longer there.
            if matches!(event, tauri::WindowEvent::Destroyed) && window.label() == "main" {
                use tauri::Manager;
                window.app_handle().exit(0);
            }
        })
        .manage(LaunchFiles(Mutex::new(LaunchQueue { files: launch_files, drained: false })))
        .manage(db)
        .manage(commands::database::DatabaseFallbackNotice(Mutex::new(db_fallback)))
        .manage(plugins)
        .manage(profiles)
        .manage(ListenerState::new())
        .manage(licensing::telemetry::UsageCounters::new())
        .manage(commands::database::ExitSnapshot::default())
        .manage(OsEnding::default())
        .setup(|app| {
            use tauri::Manager;
            // First: the plugins are set up by now, so a launch the
            // single-instance plugin let through is either the only one or
            // one it could not check; nothing has used the database yet.
            #[cfg(target_os = "linux")]
            guard_second_instance(app);
            #[cfg(unix)]
            watch_termination_signals(app.handle().clone());
            let handle = app.handle().clone();
            let spawned = std::thread::Builder::new()
                .name("plugin-packs".into())
                .spawn(move || {
                    let _ = handle.state::<PluginRegistry>().reload();
                });
            if spawned.is_err() {
                // Loaded here rather than leave every reader waiting.
                let _ = app.state::<PluginRegistry>().reload();
            }

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
            commands::parser::interpret_message,
            commands::parser::get_field_content,
            commands::parser::get_field_length,
            commands::parser::search_message,
            commands::fileio::open_file,
            get_launch_files,
            get_build_id,
            commands::fileio::save_file,
            commands::fileio::file_stat,
            commands::fileio::canonical_path,
            commands::fileio::allow_save_with_extension,
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
            commands::database::take_database_fallback,
            commands::database::set_exit_snapshot,
            commands::database::load_recovery_tabs,
            commands::database::clear_recovery_tabs,
            os_ending_ready,
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
            commands::licensing::get_hardware_id_info,
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
            commands::plugins::plugin_state_problem,
            commands::plugins::plugins_not_applied,
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
            commands::soap::soap_envelope_size,
            commands::communication::mllp_wire_size,
        ]))
        .build(tauri::generate_context!())
        .expect("error while running BridgeLab")
        .run(|app_handle, event| {
            // macOS delivers Finder opens (double-click, Open With, a drop on
            // the Dock icon) as an event, never as launch arguments.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Opened { urls } = &event {
                show_main_window(app_handle, file_url_paths(urls));
            }
            // Flush the in-memory usage deltas on normal exit so short
            // sessions don't lose their counters (the periodic flusher only
            // covers sessions longer than its interval).
            if let tauri::RunEvent::Exit = event {
                use tauri::Manager;
                let db = app_handle.state::<database::Database>();
                // Reached without the window's close request (it empties
                // the snapshot when it closes the normal way): the system
                // ended the app. The tabs the window last handed over are
                // kept, as the session or as the recovery copy. This is the
                // one hook macOS (logout, the Dock's or the standard Quit)
                // and Windows (end of session, Restart Manager) give before
                // the process ends. It runs on the thread the webview's IPC
                // comes through, so the window cannot be asked for newer
                // text here (waiting for it would deadlock): it hands its
                // tabs over 300 ms after an edit and at once when it loses
                // focus or is hidden, which a shutdown from the Start menu
                // or the Dock does first.
                app_handle.state::<commands::database::ExitSnapshot>().save_to(&db);
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

    /// A file name in Latin-1 on the command line (a double-click on Linux)
    /// is passed on, shown with U+FFFD, instead of ending the launch.
    #[cfg(unix)]
    #[test]
    fn a_launch_argument_that_is_not_unicode_is_kept() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        let argv = vec![
            OsString::from("bridgelab"),
            OsString::from_vec(b"M\xfcller.hl7".to_vec()),
            OsString::from_vec(b"-\xff".to_vec()),
            OsString::from("ok.hl7"),
        ];
        assert_eq!(
            collect_file_args(&argv, Path::new("/work")),
            vec!["/work/M\u{FFFD}ller.hl7", "/work/ok.hl7"],
        );
        // A valid name in a folder whose name is not Unicode.
        let cwd = std::path::PathBuf::from(OsString::from_vec(b"/data/caf\xe9".to_vec()));
        assert_eq!(collect_file_args(&["bridgelab", "a.hl7"], &cwd), vec!["/data/caf\u{FFFD}/a.hl7"]);
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
