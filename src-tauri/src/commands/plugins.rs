use std::collections::HashMap;
use std::path::PathBuf;

use tauri::State;

use crate::plugins::{PluginInfo, PluginRegistry, plugins_root};

/// Return all installed plugin packs, including ones that failed to parse
/// (those are surfaced with an `error` field set).
#[tauri::command]
pub fn list_plugins(registry: State<'_, PluginRegistry>) -> Result<Vec<PluginInfo>, String> {
    Ok(registry.list())
}

/// Rescan the plugins directory and return the fresh list.
#[tauri::command]
pub fn reload_plugins(registry: State<'_, PluginRegistry>) -> Result<Vec<PluginInfo>, String> {
    registry.reload()?;
    Ok(registry.list())
}

/// Persist / apply an override for a specific plugin id. The frontend is
/// responsible for also writing the preference (`plugin_enabled:<id>`) so
/// the override survives restarts.
#[tauri::command]
pub fn set_plugin_enabled(
    id: String,
    enabled: bool,
    registry: State<'_, PluginRegistry>,
) -> Result<(), String> {
    registry.set_override(&id, enabled);
    Ok(())
}

/// Bulk-apply overrides (called on startup after the frontend has read the
/// preferences table). Keys are plugin ids.
#[tauri::command]
pub fn apply_plugin_overrides(
    overrides: HashMap<String, bool>,
    registry: State<'_, PluginRegistry>,
) -> Result<(), String> {
    registry.set_overrides(overrides);
    Ok(())
}

/// Return the absolute path to the plugins root directory, creating it if
/// necessary. Used by the UI to offer an "Open plugins folder" button.
#[tauri::command]
pub fn get_plugins_dir() -> Result<String, String> {
    let root: PathBuf = plugins_root()
        .ok_or_else(|| "Could not determine config directory".to_string())?;
    // Ensure subdirs exist so the UI finds an expected layout
    let _ = std::fs::create_dir_all(root.join("validation"));
    let _ = std::fs::create_dir_all(root.join("anonymization"));
    Ok(root.display().to_string())
}

/// Reveal the plugins directory in the host OS file manager.
#[tauri::command]
pub fn open_plugins_folder() -> Result<(), String> {
    let root: PathBuf = plugins_root()
        .ok_or_else(|| "Could not determine config directory".to_string())?;
    let _ = std::fs::create_dir_all(root.join("validation"));
    let _ = std::fs::create_dir_all(root.join("anonymization"));

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
