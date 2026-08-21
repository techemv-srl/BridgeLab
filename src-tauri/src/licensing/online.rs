//! Online license activation against the TECHEMV license server.
//!
//! The buyer receives a short activation code (`BL-PRO-XXXX-XXXX-XXXX`);
//! the app exchanges it — together with the hardware ID — for a signed
//! `LicenseFile` that the existing offline verification then validates and
//! stores. After that single HTTPS call everything works offline exactly
//! like a hand-issued key: no heartbeat, no phone-home.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;

use super::{get_hardware_id, parse_and_verify_key, save_license, LicenseFile};

/// Base URL of the TECHEMV license server (shared with Aurora).
/// Override for dev/staging with env var BRIDGELAB_LICENSE_SERVER.
pub const DEFAULT_LICENSE_SERVER: &str = "https://license-api.techemv.it/api/v1";

pub fn license_server_base() -> String {
    std::env::var("BRIDGELAB_LICENSE_SERVER")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim_end_matches('/').to_string())
        .unwrap_or_else(|| DEFAULT_LICENSE_SERVER.to_string())
}

/// Shared HTTP client; per-request timeouts are set at the call sites.
fn http_client(app_version: &str) -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(format!(
                "BridgeLab/{} ({}; {})",
                app_version,
                std::env::consts::OS,
                std::env::consts::ARCH
            ))
            .build()
            .unwrap_or_default()
    })
}

// =============================================================================
// Activation code format: BL-<PRO|ENT>-XXXX-XXXX-XXXX, Crockford Base32
// =============================================================================

/// Uppercase and drop spaces; dashes are kept so that the debug simple-key
/// format (`BL-PRO-ABCD1234EFGH`, one 12-char group) never matches the
/// three-group activation-code pattern below.
fn canon(input: &str) -> String {
    input
        .trim()
        .to_ascii_uppercase()
        .chars()
        .filter(|c| *c != ' ')
        .collect()
}

fn code_regex() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Crockford Base32 alphabet: 0-9 A-H J K M N P-T V-Z (no I, L, O, U)
        regex::Regex::new(r"^BL-(PRO|ENT)-[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$")
            .expect("static regex")
    })
}

pub fn looks_like_activation_code(input: &str) -> bool {
    code_regex().is_match(&canon(input))
}

/// Canonical form used for server calls and stored metadata.
pub fn normalize_activation_code(input: &str) -> String {
    canon(input)
}

// =============================================================================
// Server API
// =============================================================================

#[derive(Serialize)]
struct ActivateRequest<'a> {
    code: &'a str,
    hardware_id: &'a str,
    installation_id: &'a str,
    app_version: &'a str,
    os: &'a str,
    arch: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    locale: Option<&'a str>,
}

#[derive(Deserialize)]
struct ActivateResponse {
    success: bool,
    #[serde(default)]
    license_key: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    error_code: Option<String>,
    // seats_used / seats_total are informational; the server already folds
    // them into `message` when relevant (e.g. NO_SEATS).
    #[serde(default)]
    #[allow(dead_code)]
    seats_used: Option<u32>,
    #[serde(default)]
    #[allow(dead_code)]
    seats_total: Option<u32>,
}

#[derive(Serialize)]
struct DeactivateRequest<'a> {
    code: &'a str,
    hardware_id: &'a str,
}

/// Stable sentinel for "the license server cannot be reached". The frontend
/// maps it to a localized, non-technical message that points isolated
/// (air-gapped) sites at the offline-key flow — raw network details would
/// only mislead users on machines that are offline by design.
pub const ERR_SERVER_UNREACHABLE: &str = "ERR_SERVER_UNREACHABLE";

fn unreachable_err(_e: impl std::fmt::Display) -> String {
    #[cfg(debug_assertions)]
    eprintln!("[licensing] license server unreachable: {}", _e);
    ERR_SERVER_UNREACHABLE.to_string()
}

/// Exchange an activation code for a signed license, verify it locally
/// (signature + hardware binding — never trust the server blindly), attach
/// the unsigned activation metadata and save it.
pub async fn activate_online(
    code: &str,
    installation_id: &str,
    app_version: &str,
    locale: Option<String>,
) -> Result<LicenseFile, String> {
    let code = normalize_activation_code(code);
    let hardware_id = get_hardware_id();
    let req = ActivateRequest {
        code: &code,
        hardware_id: &hardware_id,
        installation_id,
        app_version,
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        locale: locale.as_deref(),
    };

    let url = format!("{}/bridgelab/activate", license_server_base());
    let resp = http_client(app_version)
        .post(&url)
        .timeout(Duration::from_secs(10))
        .json(&req)
        .send()
        .await
        .map_err(unreachable_err)?;

    let body: ActivateResponse = resp.json().await.map_err(unreachable_err)?;

    if !body.success {
        // The server's message is already user-facing; pass it through.
        return Err(body.message.unwrap_or_else(|| {
            format!(
                "Activation failed ({})",
                body.error_code.as_deref().unwrap_or("SERVER_ERROR")
            )
        }));
    }

    let key = body
        .license_key
        .ok_or_else(|| "The license server returned no license key".to_string())?;

    let mut license = parse_and_verify_key(&key)?;
    license.activation_code = Some(code);
    license.activated_at = Some(chrono::Utc::now().to_rfc3339());
    save_license(&license)?;
    Ok(license)
}

// =============================================================================
// Silent license refresh (renewals)
// =============================================================================

/// Days before expiry at which the app starts trying to refresh an
/// online-activated license in the background (renewals extend the code
/// server-side; a reactivation picks the new expiry up without any user
/// action). Past-expiry licenses keep being retried too — renewing after
/// the deadline is common.
const REFRESH_WINDOW_DAYS: i64 = 14;
const PREF_REFRESH_LAST: &str = "license_refresh_last_attempt";

/// True when the expiry is close enough (or past) to warrant a refresh.
fn within_refresh_window(expires_at: &str) -> bool {
    match chrono::DateTime::parse_from_rfc3339(expires_at) {
        Ok(exp) => {
            let days_left = (exp.with_timezone(&chrono::Utc) - chrono::Utc::now()).num_days();
            days_left <= REFRESH_WINDOW_DAYS
        }
        Err(_) => false,
    }
}

/// Startup task: if an online-activated license is within the refresh
/// window, silently re-activate with the stored code (at most one attempt
/// per 24 h). Every failure is ignored — offline sites must never notice
/// this exists. On success a `license://refreshed` event lets the UI
/// reload the license status.
pub async fn maybe_refresh_on_startup(app: tauri::AppHandle) {
    use tauri::{Emitter, Manager};

    let Some(license) = super::load_license() else { return };
    let Some(code) = license.activation_code.clone() else { return };
    // Perpetual licenses (no expiry) never need refreshing.
    let Some(expires) = license.payload.expires_at.as_deref() else { return };
    if !within_refresh_window(expires) {
        return;
    }

    let (installation_id, locale) = {
        let db = app.state::<crate::database::Database>();
        // Throttle: one attempt per day, recorded BEFORE the call so a
        // crash loop can't hammer the server.
        if let Ok(Some(last)) = db.get_preference(PREF_REFRESH_LAST) {
            if let Ok(t) = chrono::DateTime::parse_from_rfc3339(&last) {
                if chrono::Utc::now() - t.with_timezone(&chrono::Utc) < chrono::Duration::hours(24) {
                    return;
                }
            }
        }
        let _ = db.set_preference(PREF_REFRESH_LAST, &chrono::Utc::now().to_rfc3339());
        (
            super::telemetry::installation_id(&db),
            db.get_preference("language").ok().flatten(),
        )
    };

    let app_version = app.package_info().version.to_string();
    match activate_online(&code, &installation_id, &app_version, locale).await {
        Ok(lic) => {
            let _ = app.emit("license://refreshed", lic.payload.expires_at);
            #[cfg(debug_assertions)]
            eprintln!("[licensing] silent refresh: license updated");
        }
        Err(_e) => {
            // Unreachable server, revoked code, whatever: stay silent. The
            // license on disk keeps working until its own expiry, and the
            // user can always refresh manually from the License dialog.
            #[cfg(debug_assertions)]
            eprintln!("[licensing] silent refresh failed (ignored): {}", _e);
        }
    }
}

/// Tell the server this machine's seat is being freed. Best-effort: the
/// caller removes the local license regardless of the outcome here.
pub async fn deactivate_online(code: &str, hardware_id: &str, app_version: &str) -> Result<(), String> {
    let url = format!("{}/bridgelab/deactivate", license_server_base());
    http_client(app_version)
        .post(&url)
        .timeout(Duration::from_secs(5))
        .json(&DeactivateRequest {
            code: &normalize_activation_code(code),
            hardware_id,
        })
        .send()
        .await
        .map_err(unreachable_err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_codes() {
        assert!(looks_like_activation_code("BL-PRO-2345-ABCD-WXYZ"));
        assert!(looks_like_activation_code("BL-ENT-0000-9999-ZZZZ"));
        // Case-insensitive and space tolerant
        assert!(looks_like_activation_code("  bl-pro-2345-abcd-wxyz "));
        assert!(looks_like_activation_code("BL-PRO-2345 -ABCD- WXYZ"));
    }

    #[test]
    fn test_invalid_codes() {
        // Debug simple-key format: one 12-char group, must fall through
        assert!(!looks_like_activation_code("BL-PRO-ABCD1234EFGH"));
        // Excluded Crockford letters I, L, O, U
        assert!(!looks_like_activation_code("BL-PRO-ILOU-ABCD-2345"));
        // Wrong tier
        assert!(!looks_like_activation_code("BL-FREE-2345-ABCD-WXYZ"));
        // Wrong group count / length
        assert!(!looks_like_activation_code("BL-PRO-2345-ABCD"));
        assert!(!looks_like_activation_code("BL-PRO-2345-ABCD-WXYZ-2345"));
        assert!(!looks_like_activation_code("BL-PRO-234-ABCD-WXYZ"));
        // Base64 offline keys must never match
        assert!(!looks_like_activation_code("eyJwYXlsb2FkIjp7fX0="));
        assert!(!looks_like_activation_code(""));
    }

    #[test]
    fn test_normalize() {
        assert_eq!(
            normalize_activation_code("  bl-pro-2345-abcd-wxyz "),
            "BL-PRO-2345-ABCD-WXYZ"
        );
        assert_eq!(
            normalize_activation_code("BL-ENT-0000 -9999- ZZZZ"),
            "BL-ENT-0000-9999-ZZZZ"
        );
    }

    #[test]
    fn test_refresh_window() {
        let soon = (chrono::Utc::now() + chrono::Duration::days(5)).to_rfc3339();
        let past = (chrono::Utc::now() - chrono::Duration::days(30)).to_rfc3339();
        let far = (chrono::Utc::now() + chrono::Duration::days(200)).to_rfc3339();
        assert!(within_refresh_window(&soon), "5 days out is within the window");
        assert!(within_refresh_window(&past), "expired licenses keep retrying");
        assert!(!within_refresh_window(&far), "200 days out must not refresh");
        assert!(!within_refresh_window("not-a-date"), "unparseable fails closed");
    }

    #[test]
    fn test_server_base_default() {
        // Env override is exercised manually; here just pin the default.
        if std::env::var("BRIDGELAB_LICENSE_SERVER").is_err() {
            assert_eq!(license_server_base(), DEFAULT_LICENSE_SERVER);
        }
    }
}
