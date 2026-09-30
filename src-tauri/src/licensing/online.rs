//! Online license activation against the TECHEMV license server.
//!
//! The buyer receives a short activation code (`BL-PRO-XXXX-XXXX-XXXX`);
//! the app exchanges it — together with the hardware ID — for a signed
//! `LicenseFile` that the existing offline verification then validates and
//! stores. After that HTTPS call everything works offline exactly like a
//! hand-issued key. An online-activated license is re-checked with the
//! server now and then (see `maybe_refresh_on_startup`): that picks up
//! renewals and, when the server answers that the code was revoked (a
//! refund, say), returns the app to Community. Offline sites are never
//! affected: an unreachable server changes nothing.

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

/// Why an activation did not produce a license.
#[derive(Debug)]
pub(crate) enum ActivationFailure {
    /// The server answered with `success: false`: its machine code (e.g.
    /// `REVOKED`) and user-facing message.
    Refused { error_code: Option<String>, message: String },
    /// Anything else: unreachable server, proxy page, bad key, …
    Other(String),
}

impl ActivationFailure {
    fn into_message(self) -> String {
        match self {
            ActivationFailure::Refused { message, .. } | ActivationFailure::Other(message) => message,
        }
    }
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
    request_license(code, installation_id, app_version, locale)
        .await
        .map_err(ActivationFailure::into_message)
}

async fn request_license(
    code: &str,
    installation_id: &str,
    app_version: &str,
    locale: Option<String>,
) -> Result<LicenseFile, ActivationFailure> {
    let code = normalize_activation_code(code);
    // Re-activating the code this machine already holds (a renewal, or
    // "Update license") sends the ID the seat was taken with: a license
    // bound to the pre-1.9 ID must not use up a second activation.
    let hardware_id = crate::licensing::load_license()
        .filter(|l| l.activation_code.as_deref() == Some(code.as_str()))
        .map(|l| l.payload.hardware_id)
        .filter(|h| !h.is_empty() && crate::licensing::is_this_machine(h))
        .unwrap_or_else(get_hardware_id);
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
        .map_err(|e| ActivationFailure::Other(unreachable_err(e)))?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| ActivationFailure::Other(unreachable_err(e)))?;
    // A proxy block page or a server error page is not "no internet": say
    // what happened, so the user (or their IT) can act on it.
    let body: ActivateResponse = serde_json::from_str(&text).map_err(|_| {
        ActivationFailure::Other(if status.is_success() {
            "The license server sent an answer the app does not understand. Try again later.".to_string()
        } else {
            format!(
                "The license server, or a proxy in between, answered HTTP {}. Try again later; if it persists, check with your IT whether {} is reachable.",
                status.as_u16(),
                license_server_base()
            )
        })
    })?;

    if !body.success {
        // The server's message is already user-facing; pass it through.
        let message = body.message.unwrap_or_else(|| {
            format!(
                "Activation failed ({})",
                body.error_code.as_deref().unwrap_or("SERVER_ERROR")
            )
        });
        return Err(ActivationFailure::Refused { error_code: body.error_code, message });
    }

    let key = body
        .license_key
        .ok_or_else(|| ActivationFailure::Other("The license server returned no license key".to_string()))?;

    let mut license = parse_and_verify_key(&key).map_err(ActivationFailure::Other)?;
    license.activation_code = Some(code);
    license.activated_at = Some(chrono::Utc::now().to_rfc3339());
    save_license(&license).map_err(ActivationFailure::Other)?;
    Ok(license)
}

// =============================================================================
// Silent license re-check (renewals and revocations)
// =============================================================================

/// Days before expiry at which the app re-checks every day (renewals extend
/// the code server-side; a reactivation picks the new expiry up without any
/// user action). Past-expiry licenses keep being retried too — renewing
/// after the deadline is common.
const REFRESH_WINDOW_DAYS: i64 = 14;
/// Outside that window (and for licenses without an expiry) the check runs
/// once a week, so a revoked code stops working within about a week of
/// the next start, not at its expiry.
const RECHECK_EVERY_DAYS: i64 = 7;
pub(crate) const PREF_REFRESH_LAST: &str = "license_refresh_last_attempt";
/// The server's machine code for a revoked activation code.
const REVOKED: &str = "REVOKED";

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

/// How long to wait after the last attempt before checking again.
fn recheck_interval(expires_at: Option<&str>) -> chrono::Duration {
    match expires_at {
        Some(exp) if within_refresh_window(exp) => chrono::Duration::hours(24),
        _ => chrono::Duration::days(RECHECK_EVERY_DAYS),
    }
}

/// What a re-check answer means for the license on disk.
#[derive(Debug, PartialEq)]
enum RecheckOutcome {
    /// The server issued a license again (renewal picked up, or unchanged).
    Refreshed,
    /// The server says the code is revoked: back to Community.
    Revoked,
    /// Anything else — unreachable, rate limited, server error, a proxy
    /// page: keep the license as it is.
    Unchanged,
}

fn classify(result: &Result<LicenseFile, ActivationFailure>) -> RecheckOutcome {
    match result {
        Ok(_) => RecheckOutcome::Refreshed,
        Err(ActivationFailure::Refused { error_code: Some(code), .. }) if code == REVOKED => RecheckOutcome::Revoked,
        Err(_) => RecheckOutcome::Unchanged,
    }
}

/// Startup task: re-activate an online-activated license with its stored
/// code — every day in the last 14 days before expiry (and after it), once
/// a week otherwise, perpetual licenses included. A new license replaces
/// the old one (renewal). A `REVOKED` answer puts the license aside, keeps
/// the server's message for the notice banner, and the app runs as
/// Community. Every other failure is ignored — offline sites must never
/// notice this exists. Either change emits `license://refreshed` so the
/// UI reloads the license status.
pub async fn maybe_refresh_on_startup(app: tauri::AppHandle) {
    use tauri::{Emitter, Manager};

    let Some(license) = super::load_license() else { return };
    let Some(code) = license.activation_code.clone() else { return };
    let interval = recheck_interval(license.payload.expires_at.as_deref());

    let (installation_id, locale) = {
        let db = app.state::<crate::database::Database>();
        // Throttle, recorded BEFORE the call so a crash loop can't hammer
        // the server.
        if let Ok(Some(last)) = db.get_preference(PREF_REFRESH_LAST) {
            if let Ok(t) = chrono::DateTime::parse_from_rfc3339(&last) {
                if chrono::Utc::now() - t.with_timezone(&chrono::Utc) < interval {
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
    let result = request_license(&code, &installation_id, &app_version, locale).await;
    match classify(&result) {
        RecheckOutcome::Refreshed => {
            let expires = result.ok().and_then(|lic| lic.payload.expires_at);
            let _ = app.emit("license://refreshed", expires);
        }
        RecheckOutcome::Revoked => {
            let message = match result {
                Err(ActivationFailure::Refused { message, .. }) => message,
                _ => String::new(),
            };
            if let Ok(true) = super::set_aside_revoked_license(&code) {
                let db = app.state::<crate::database::Database>();
                let notice = if message.trim().is_empty() {
                    "Your license was revoked, so BridgeLab now runs as Community. Contact info@techemv.it.".to_string()
                } else {
                    message
                };
                let _ = db.set_preference(super::telemetry::PREF_SERVER_NOTICE, &notice);
                let _ = app.emit("license://refreshed", Option::<String>::None);
            }
        }
        RecheckOutcome::Unchanged => {
            #[cfg(debug_assertions)]
            if let Err(e) = &result {
                eprintln!("[licensing] silent re-check failed (ignored): {:?}", e);
            }
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
    fn test_recheck_interval() {
        let soon = (chrono::Utc::now() + chrono::Duration::days(5)).to_rfc3339();
        let far = (chrono::Utc::now() + chrono::Duration::days(200)).to_rfc3339();
        assert_eq!(recheck_interval(Some(&soon)), chrono::Duration::hours(24));
        assert_eq!(recheck_interval(Some(&far)), chrono::Duration::days(7));
        assert_eq!(recheck_interval(None), chrono::Duration::days(7), "perpetual licenses are re-checked weekly");
    }

    #[test]
    fn only_a_revoked_answer_downgrades() {
        let refused = |code: Option<&str>| -> Result<LicenseFile, ActivationFailure> {
            Err(ActivationFailure::Refused { error_code: code.map(str::to_string), message: "m".into() })
        };
        assert_eq!(classify(&refused(Some("REVOKED"))), RecheckOutcome::Revoked);
        for code in [Some("RATE_LIMITED"), Some("SERVER_ERROR"), Some("NO_SEATS"), Some("EXPIRED"), Some("INVALID_CODE"), Some("revoked"), None] {
            assert_eq!(classify(&refused(code)), RecheckOutcome::Unchanged, "{:?} must keep the license", code);
        }
        assert_eq!(
            classify(&Err(ActivationFailure::Other(ERR_SERVER_UNREACHABLE.into()))),
            RecheckOutcome::Unchanged,
            "an unreachable server never downgrades"
        );
    }

    #[test]
    fn test_server_base_default() {
        // Env override is exercised manually; here just pin the default.
        if std::env::var("BRIDGELAB_LICENSE_SERVER").is_err() {
            assert_eq!(license_server_base(), DEFAULT_LICENSE_SERVER);
        }
    }
}
