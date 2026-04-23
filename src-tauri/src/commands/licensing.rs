use crate::licensing::{self, LicenseStatus};

/// Check current license status.
#[tauri::command]
pub fn check_license() -> Result<LicenseStatus, String> {
    Ok(licensing::check_license_status())
}

/// Activate a license key. Only accepts Ed25519-signed keys.
/// The simple BL-TYPE-CODE format is only available in debug builds.
#[tauri::command]
pub fn activate_license(
    key: String,
    licensee: String,
    email: String,
) -> Result<LicenseStatus, String> {
    // Try Base64-encoded signed license
    match licensing::activate_from_key(&key) {
        Ok(_) => return Ok(licensing::check_license_status()),
        Err(signed_err) => {
            // In debug builds only, fall back to simple BL-TYPE-CODE format
            #[cfg(debug_assertions)]
            {
                match licensing::activate_simple_key(&key, &licensee, &email) {
                    Ok(_) => return Ok(licensing::check_license_status()),
                    Err(_) => {}
                }
            }
            return Err(format!("Invalid license key: {}", signed_err));
        }
    }
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
