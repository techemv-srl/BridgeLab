use crate::database::Database;
use crate::licensing::{self, online, telemetry, LicenseStatus};
use tauri::State;

/// Check current license status.
#[tauri::command]
pub fn check_license() -> Result<LicenseStatus, String> {
    Ok(licensing::check_license_status())
}

/// Activate a license key (offline path). Only accepts Ed25519-signed keys.
/// The simple BL-TYPE-CODE format is only available in debug builds.
#[tauri::command]
pub fn activate_license(
    key: String,
    licensee: String,
    email: String,
) -> Result<LicenseStatus, String> {
    // licensee/email feed only the debug-only simple-key fallback below;
    // keep release builds warning-free.
    #[cfg(not(debug_assertions))]
    let _ = (&licensee, &email);

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

/// Exchange an activation code (BL-PRO/ENT-XXXX-XXXX-XXXX) for a signed
/// license via the TECHEMV license server. Verification of the returned key
/// happens locally, exactly like the offline path.
#[tauri::command]
pub async fn activate_license_online(
    code: String,
    app: tauri::AppHandle,
    db: State<'_, Database>,
) -> Result<LicenseStatus, String> {
    if !online::looks_like_activation_code(&code) {
        return Err("Not a valid activation code. Expected format: BL-PRO-XXXX-XXXX-XXXX".into());
    }
    let installation_id = telemetry::installation_id(&db);
    let locale = db.get_preference("language").ok().flatten();
    let app_version = app.package_info().version.to_string();
    online::activate_online(&code, &installation_id, &app_version, locale).await?;
    Ok(licensing::check_license_status())
}

/// Deactivate the current license. When the license was activated online,
/// tell the server first so the seat is freed — best-effort: the local
/// removal proceeds even if the server is unreachable.
#[tauri::command]
pub async fn deactivate_license(app: tauri::AppHandle) -> Result<LicenseStatus, String> {
    if let Some(license) = licensing::load_license() {
        if let Some(code) = license.activation_code {
            let hardware_id = licensing::get_hardware_id();
            let app_version = app.package_info().version.to_string();
            if let Err(_e) = online::deactivate_online(&code, &hardware_id, &app_version).await {
                #[cfg(debug_assertions)]
                eprintln!("[licensing] online deactivation failed (ignored): {}", _e);
            }
        }
    }
    licensing::remove_license()?;
    Ok(licensing::check_license_status())
}

/// True when the input looks like an online activation code (authoritative
/// twin of the frontend heuristic).
#[tauri::command]
pub fn is_activation_code(input: String) -> bool {
    online::looks_like_activation_code(&input)
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

// =============================================================================
// Telemetry (Settings → Privacy)
// =============================================================================

#[tauri::command]
pub fn get_telemetry_settings(
    db: State<'_, Database>,
    counters: State<'_, telemetry::UsageCounters>,
) -> telemetry::TelemetrySettings {
    telemetry::settings(&db, &counters)
}

#[tauri::command]
pub fn set_telemetry_enabled(enabled: bool, db: State<'_, Database>) -> Result<(), String> {
    telemetry::set_enabled(&db, enabled)
}

#[derive(serde::Serialize)]
pub struct TelemetrySendResult {
    pub message: String,
    /// The exact JSON payload that was transmitted, so the Privacy UI can
    /// show precisely what left the machine.
    pub payload: serde_json::Value,
}

/// Manual send from Settings; returns the server's message plus the payload
/// that was actually sent, for inline display.
#[tauri::command]
pub async fn send_telemetry_now(
    app: tauri::AppHandle,
    db: State<'_, Database>,
) -> Result<TelemetrySendResult, String> {
    if !telemetry::is_enabled(&db) {
        return Err("Telemetry is disabled".into());
    }
    let (message, payload) = telemetry::send(&app).await?;
    Ok(TelemetrySendResult { message, payload })
}

/// The exact JSON payload a telemetry send would transmit (transparency).
#[tauri::command]
pub fn get_telemetry_preview(app: tauri::AppHandle) -> serde_json::Value {
    telemetry::build_payload(&app)
}
