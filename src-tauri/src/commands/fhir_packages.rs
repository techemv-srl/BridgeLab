//! IPC commands for the FHIR profile package manager.

use std::path::PathBuf;

use tauri::State;

use crate::licensing::feature_gate;
use crate::parser::fhir::profile::{package, PackageInfo, ProfileRegistry};

/// Packages currently loaded.
#[tauri::command]
pub fn fhir_packages_list(registry: State<'_, ProfileRegistry>) -> Vec<PackageInfo> {
    registry.packages()
}

/// Re-read the installed packages from disk.
#[tauri::command]
pub fn fhir_packages_reload(registry: State<'_, ProfileRegistry>) -> Result<Vec<PackageInfo>, String> {
    registry.reload()?;
    Ok(registry.packages())
}

/// Install a `.tgz` the user picked, then reload.
///
/// Reading the archive is the slow part (a core package holds thousands of
/// files), so it happens once here and the distilled index is what the app
/// loads from then on.
#[tauri::command]
pub fn fhir_packages_install(
    path: String,
    registry: State<'_, ProfileRegistry>,
) -> Result<Vec<PackageInfo>, String> {
    feature_gate::require("fhir_profile_validation")?;
    package::install(&PathBuf::from(path))?;
    registry.reload()?;
    Ok(registry.packages())
}

#[tauri::command]
pub fn fhir_packages_remove(
    name: String,
    version: String,
    registry: State<'_, ProfileRegistry>,
) -> Result<Vec<PackageInfo>, String> {
    package::remove(&name, &version)?;
    registry.reload()?;
    Ok(registry.packages())
}

/// Directory the packages live in, so the UI can offer to open it.
#[tauri::command]
pub fn fhir_packages_dir() -> Result<String, String> {
    package::packages_root()
        .map(|p| p.display().to_string())
        .ok_or_else(|| "Could not determine the config directory".into())
}
