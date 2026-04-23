use crate::licensing::{self, LicenseStatus};

/// Check current license status.
#[tauri::command]
pub fn check_license() -> Result<LicenseStatus, String> {
    Ok(licensing::check_license_status())
}

/// Activate a license key. Tries Base64 signed key first, then simple format.
#[tauri::command]
pub fn activate_license(
    key: String,
    licensee: String,
    email: String,
) -> Result<LicenseStatus, String> {
    // Try Base64-encoded signed license first
    match licensing::activate_from_key(&key) {
        Ok(_) => return Ok(licensing::check_license_status()),
        Err(_) => {}
    }

    // Fallback to simple BL-TYPE-CODE format
    licensing::activate_simple_key(&key, &licensee, &email)?;
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

/// Get the list of features available to the current user.
#[tauri::command]
pub fn get_available_features() -> Result<Vec<String>, String> {
    Ok(licensing::feature_gate::available_features())
}
