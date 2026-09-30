use std::collections::HashMap;
use std::path::PathBuf;

use tauri::State;

use crate::licensing::feature_gate;
use crate::plugins::{PluginInfo, PluginRegistry, plugins_root};

/// Return all installed plugin packs, including ones that failed to parse
/// (those are surfaced with an `error` field set).
#[tauri::command]
pub fn list_plugins(registry: State<'_, PluginRegistry>) -> Result<Vec<PluginInfo>, String> {
    Ok(registry.list(feature_gate::active_plugin_limit()))
}

/// Rescan the plugins directory and return the fresh list.
#[tauri::command]
pub fn reload_plugins(registry: State<'_, PluginRegistry>) -> Result<Vec<PluginInfo>, String> {
    registry.reload()?;
    Ok(registry.list(feature_gate::active_plugin_limit()))
}

/// Switch one pack on or off. `key` is the pack's `<kind>/<id>` (see
/// `PluginInfo.key`); the choice is saved in the plugins folder, where the
/// CLI reads it too.
///
/// Enabling a pack beyond the Community cap is refused with a message that
/// names the cap; disabling is always allowed.
#[tauri::command]
pub fn set_plugin_enabled(
    key: String,
    enabled: bool,
    registry: State<'_, PluginRegistry>,
) -> Result<(), String> {
    if enabled {
        if let Some(max) = feature_gate::active_plugin_limit() {
            let others = registry
                .list(None)
                .iter()
                .filter(|p| p.enabled && p.key != key)
                .count();
            if others >= max {
                return Err(format!(
                    "PLUGIN_CAP:{}: the Community edition runs up to {} plugin packs at a time. Switch another pack off first, or upgrade to Professional for no limit.",
                    max, max
                ));
            }
        }
    }
    registry.set_enabled(&key, enabled)
}

/// Choices an older version saved as `plugin_enabled:<id>` preferences,
/// applied on startup until each pack has a choice of its own.
#[tauri::command]
pub fn apply_plugin_overrides(
    overrides: HashMap<String, bool>,
    registry: State<'_, PluginRegistry>,
) -> Result<(), String> {
    registry.apply_legacy_overrides(overrides);
    Ok(())
}

/// Return the absolute path to the plugins root directory, creating it if
/// necessary. Used by the UI to offer an "Open plugins folder" button.
#[tauri::command]
pub fn get_plugins_dir() -> Result<String, String> {
    let root: PathBuf = plugins_root()
        .ok_or_else(|| "Could not determine config directory".to_string())?;
    // Ensure subdirs exist so the UI finds an expected layout
    for sub in ["validation", "fhir", "anonymization"] {
        let _ = std::fs::create_dir_all(root.join(sub));
    }
    Ok(root.display().to_string())
}

/// Reveal the plugins directory in the host OS file manager.
#[tauri::command]
pub fn open_plugins_folder() -> Result<(), String> {
    let root: PathBuf = plugins_root()
        .ok_or_else(|| "Could not determine config directory".to_string())?;
    for sub in ["validation", "fhir", "anonymization"] {
        let _ = std::fs::create_dir_all(root.join(sub));
    }

    #[cfg(target_os = "windows")]
    let cmd = ("explorer", vec![root.display().to_string()]);
    #[cfg(target_os = "macos")]
    let cmd = ("open", vec![root.display().to_string()]);
    #[cfg(all(unix, not(target_os = "macos")))]
    let cmd = ("xdg-open", vec![root.display().to_string()]);

    std::process::Command::new(cmd.0)
        .args(&cmd.1)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to open folder: {}", e))
}
