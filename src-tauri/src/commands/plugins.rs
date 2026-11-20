use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tauri::{Manager, State};

use crate::licensing::feature_gate;
use crate::plugins::{PluginInfo, PluginRegistry, RuleKind, plugins_root};

// The commands that read the packs are async, so they run off the main
// thread: right after startup they wait for the packs to finish loading,
// and the window must not freeze meanwhile.

/// Return all installed plugin packs, including ones that failed to parse
/// (those are surfaced with an `error` field set).
#[tauri::command]
pub async fn list_plugins(registry: State<'_, PluginRegistry>) -> Result<Vec<PluginInfo>, String> {
    Ok(registry.list(feature_gate::active_plugin_limit()))
}

/// Rescan the plugins directory and return the fresh list.
///
/// On a blocking-task thread: compiling the patterns of a large pack can
/// take seconds, which used to freeze the window for as long.
#[tauri::command]
pub async fn reload_plugins(app: tauri::AppHandle) -> Result<Vec<PluginInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let registry = app.state::<PluginRegistry>();
        registry.reload()?;
        Ok(registry.list(feature_gate::active_plugin_limit()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Switch one pack on or off. `key` is the pack's `<kind>/<id>` (see
/// `PluginInfo.key`); the choice is saved in the plugins folder, where the
/// CLI reads it too.
///
/// Enabling a pack beyond the Community cap is refused with a message that
/// names the cap; disabling is always allowed. Packs that hold no rules
/// take no slot (see `PluginRegistry::active_keys`), so they neither count
/// nor are refused.
#[tauri::command]
pub async fn set_plugin_enabled(
    key: String,
    enabled: bool,
    registry: State<'_, PluginRegistry>,
) -> Result<(), String> {
    if enabled {
        if let Some(max) = feature_gate::active_plugin_limit() {
            let packs = registry.list(None);
            let holds_rules = packs.iter().any(|p| p.key == key && p.rule_count > 0);
            let others = packs
                .iter()
                .filter(|p| p.enabled && p.key != key && p.rule_count > 0)
                .count();
            if holds_rules && others >= max {
                return Err(format!(
                    "PLUGIN_CAP:{}: the Community edition runs up to {} plugin packs at a time. Switch another pack off first, or upgrade to Professional for no limit.",
                    max, max
                ));
            }
        }
    }
    registry.set_enabled(&key, enabled)
}

/// Why the saved on/off choices could not be read (`None` when they
/// could): Settings → Plugins says so, since a pack switched off may be
/// running.
#[tauri::command]
pub async fn plugin_state_problem(registry: State<'_, PluginRegistry>) -> Result<Option<String>, String> {
    Ok(registry.state_problem())
}

/// How many packs switched on, with rules of `kind` ("hl7", "fhir" or
/// "phi"), the cap leaves out: the batch and anonymization dialogs show a
/// note, as validation reports carry one.
#[tauri::command]
pub async fn plugins_not_applied(kind: String, registry: State<'_, PluginRegistry>) -> Result<usize, String> {
    let kind = match kind.as_str() {
        "hl7" => RuleKind::Hl7,
        "fhir" => RuleKind::Fhir,
        "phi" => RuleKind::Phi,
        other => return Err(format!("unknown rule kind: {}", other)),
    };
    Ok(registry.gated_count(feature_gate::active_plugin_limit(), kind))
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

/// Reveal the plugins directory in the host OS file manager, through the
/// opener the links use: on Linux it tries xdg-open, then gio and the
/// desktops' own openers. Starting xdg-open alone failed on a system
/// without xdg-utils (which the packages do not require) with a bare "No
/// such file or directory (os error 2)". When nothing can open it, the
/// error names the folder; Settings then says so with the path.
#[tauri::command]
pub fn open_plugins_folder() -> Result<(), String> {
    let root: PathBuf = plugins_root()
        .ok_or_else(|| "Could not determine config directory".to_string())?;
    for sub in ["validation", "fhir", "anonymization"] {
        let _ = std::fs::create_dir_all(root.join(sub));
    }
    tauri_plugin_opener::open_path(&root, None::<&str>).map_err(|e| open_folder_error(&root, &e.to_string()))
}

/// The error for a folder no program could open: which folder, what is
/// missing on Linux, and the system's reason.
fn open_folder_error(root: &Path, reason: &str) -> String {
    let needs = if cfg!(any(target_os = "windows", target_os = "macos")) {
        ""
    } else {
        " (it needs xdg-open, from the xdg-utils package, or gio)"
    };
    format!(
        "No program could open the folder {}{}: {}. Open it yourself in a file manager.",
        root.display(),
        needs,
        reason
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_that_cannot_be_opened_is_named_with_what_is_missing() {
        let msg = open_folder_error(Path::new("/home/u/.config/bridgelab/plugins"), "No such file or directory (os error 2)");
        assert!(msg.contains("/home/u/.config/bridgelab/plugins"), "{msg}");
        assert!(msg.contains("os error 2"), "{msg}");
        #[cfg(target_os = "linux")]
        assert!(msg.contains("xdg-open") && msg.contains("gio"), "{msg}");
    }
}
