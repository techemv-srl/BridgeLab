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

fn unreachable_err(e: impl std::fmt::Display) -> String {
    format!(
        "Could not reach the license server: {}. Check your connection or use an offline key.",
        e
    )
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
    fn test_server_base_default() {
        // Env override is exercised manually; here just pin the default.
        if std::env::var("BRIDGELAB_LICENSE_SERVER").is_err() {
            assert_eq!(license_server_base(), DEFAULT_LICENSE_SERVER);
        }
    }
}
