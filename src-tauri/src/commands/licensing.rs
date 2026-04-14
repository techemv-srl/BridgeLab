use serde::Serialize;

use crate::licensing::{self, LicenseStatus};

/// Check current license status (trial, active, expired).
#[tauri::command]
pub fn check_license() -> Result<LicenseStatus, String> {
    Ok(licensing::check_license_status())
}

/// Activate a license key.
#[tauri::command]
pub fn activate_license(
    key: String,
    licensee: String,
    email: String,
) -> Result<LicenseStatus, String> {
    let hardware_id = licensing::get_hardware_id();
    let mut license = licensing::validate_key(&key, &hardware_id)?;
    license.licensee = licensee;
    license.email = email;
    licensing::save_license(&license)?;
    Ok(licensing::check_license_status())
}

/// Deactivate the current license.
#[tauri::command]
pub fn deactivate_license() -> Result<LicenseStatus, String> {
    licensing::remove_license()?;
    Ok(licensing::check_license_status())
}

/// Get hardware ID for display.
#[tauri::command]
pub fn get_hardware_id() -> Result<String, String> {
    Ok(licensing::get_hardware_id())
}
