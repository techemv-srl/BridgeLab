use ed25519_dalek::{Signature, Verifier, VerifyingKey, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// =============================================================================
// The PUBLIC key is embedded in the app for offline verification.
// The PRIVATE key is kept secret in the CLI tool only.
// Generate a new keypair with: bridgelab-keygen generate-keypair
// =============================================================================
const PUBLIC_KEY_HEX: &str = "PLACEHOLDER_GENERATE_WITH_CLI";

/// License payload (the data that gets signed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePayload {
    pub license_type: LicenseType,
    pub licensee: String,
    pub email: String,
    pub hardware_id: String,
    pub issued_at: String,
    pub expires_at: Option<String>,
    pub features: Vec<String>,
}

/// A complete license = payload + signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseFile {
    pub payload: LicensePayload,
    /// Hex-encoded Ed25519 signature of the JSON-serialized payload
    pub signature: String,
}

/// License status returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseStatus {
    pub is_valid: bool,
    pub license_type: LicenseType,
    pub days_remaining: Option<i64>,
    pub licensee: String,
    pub email: String,
    pub features: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LicenseType {
    Trial,
    Free,
    Professional,
    Enterprise,
    Expired,
}

/// Trial tracking data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialData {
    pub started_at: String,
    pub trial_days: i64,
}

// =============================================================================
// Hardware ID
// =============================================================================

pub fn get_hardware_id() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    if let Ok(hostname) = hostname::get() {
        hostname.to_string_lossy().hash(&mut hasher);
    }
    std::env::consts::OS.hash(&mut hasher);
    std::env::consts::ARCH.hash(&mut hasher);
    if let Ok(user) = std::env::var("USERNAME").or_else(|_| std::env::var("USER")) {
        user.hash(&mut hasher);
    }
    format!("BL-{:016X}", hasher.finish())
}

// =============================================================================
// File paths
// =============================================================================

fn data_dir() -> Result<PathBuf, String> {
    let dir = dirs::data_dir()
        .ok_or_else(|| "Could not determine data directory".to_string())?;
    Ok(dir.join("BridgeLab"))
}

fn license_file_path() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("license.json"))
}

fn trial_file_path() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("trial.json"))
}

// =============================================================================
// License verification (Ed25519 signature check)
// =============================================================================

/// Verify an Ed25519 signature on a license payload.
fn verify_signature(payload: &LicensePayload, signature_hex: &str) -> bool {
    // If public key is placeholder, accept any well-formed license (dev mode)
    if PUBLIC_KEY_HEX == "PLACEHOLDER_GENERATE_WITH_CLI" {
        return true;
    }

    let pub_bytes = match hex::decode(PUBLIC_KEY_HEX) {
        Ok(b) if b.len() == PUBLIC_KEY_LENGTH => b,
        _ => return false,
    };

    let pub_key = match VerifyingKey::from_bytes(
        pub_bytes.as_slice().try_into().unwrap_or(&[0u8; PUBLIC_KEY_LENGTH])
    ) {
        Ok(k) => k,
        Err(_) => return false,
    };

    let sig_bytes = match hex::decode(signature_hex) {
        Ok(b) if b.len() == SIGNATURE_LENGTH => b,
        _ => return false,
    };

    let signature = match Signature::from_bytes(
        sig_bytes.as_slice().try_into().unwrap_or(&[0u8; SIGNATURE_LENGTH])
    ) {
        sig => sig,
    };

    let payload_json = match serde_json::to_string(payload) {
        Ok(j) => j,
        Err(_) => return false,
    };

    pub_key.verify(payload_json.as_bytes(), &signature).is_ok()
}

// =============================================================================
// License persistence
// =============================================================================

pub fn load_license() -> Option<LicenseFile> {
    let path = license_file_path().ok()?;
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_license(license: &LicenseFile) -> Result<(), String> {
    let path = license_file_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(license)
        .map_err(|e| format!("Serialize failed: {}", e))?;
    std::fs::write(path, json).map_err(|e| format!("Write failed: {}", e))
}

pub fn remove_license() -> Result<(), String> {
    let path = license_file_path()?;
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// =============================================================================
// Trial management
// =============================================================================

pub fn load_or_init_trial() -> TrialData {
    let path = match trial_file_path() {
        Ok(p) => p,
        Err(_) => return new_trial(),
    };

    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(trial) = serde_json::from_str::<TrialData>(&content) {
            return trial;
        }
    }

    let trial = new_trial();
    save_trial(&trial).ok();
    trial
}

fn new_trial() -> TrialData {
    TrialData {
        started_at: chrono::Utc::now().to_rfc3339(),
        trial_days: 30,
    }
}

fn save_trial(trial: &TrialData) -> Result<(), String> {
    let path = trial_file_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(trial)
        .map_err(|e| format!("Serialize failed: {}", e))?;
    std::fs::write(path, json).map_err(|e| format!("Write failed: {}", e))
}

pub fn trial_days_remaining(trial: &TrialData) -> i64 {
    let started = chrono::DateTime::parse_from_rfc3339(&trial.started_at)
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    let expires = started + chrono::Duration::days(trial.trial_days);
    (expires - chrono::Utc::now()).num_days().max(0)
}

// =============================================================================
// Activate from license key (Base64-encoded JSON)
// =============================================================================

/// Activate a license from a key string.
/// The key is a Base64-encoded JSON LicenseFile.
pub fn activate_from_key(key: &str) -> Result<LicenseFile, String> {
    // Try Base64 decode
    let decoded = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        key.trim(),
    ).map_err(|_| {
        // Fallback: try as a simple format key BL-TYPE-CODE
        return format!("Invalid key format");
    })?;

    let license: LicenseFile = serde_json::from_slice(&decoded)
        .map_err(|e| format!("Invalid license data: {}", e))?;

    // Verify signature
    if !verify_signature(&license.payload, &license.signature) {
        return Err("License signature verification failed".to_string());
    }

    // Verify hardware
    let hw_id = get_hardware_id();
    if !license.payload.hardware_id.is_empty() && license.payload.hardware_id != hw_id {
        return Err(format!(
            "License is bound to a different machine. Expected: {}, Got: {}",
            license.payload.hardware_id, hw_id
        ));
    }

    // Save the license
    save_license(&license)?;

    Ok(license)
}

/// Simple key activation (BL-TYPE-CODE format, dev mode).
pub fn activate_simple_key(key: &str, licensee: &str, email: &str) -> Result<LicenseFile, String> {
    let parts: Vec<&str> = key.split('-').collect();
    if parts.len() < 3 || parts[0] != "BL" {
        return Err("Invalid key format. Expected: BL-FREE/PRO/ENT-{code}".into());
    }

    let license_type = match parts[1] {
        "FREE" => LicenseType::Free,
        "PRO" => LicenseType::Professional,
        "ENT" => LicenseType::Enterprise,
        _ => return Err("Unknown license type. Use FREE, PRO, or ENT.".into()),
    };

    let key_body = parts[2..].join("-");
    if key_body.len() < 8 {
        return Err("License key too short (minimum 8 characters)".into());
    }

    let features = match license_type {
        LicenseType::Free => vec!["core".into(), "hl7v2".into()],
        LicenseType::Professional => vec![
            "core".into(), "hl7v2".into(), "fhir".into(),
            "mllp".into(), "http".into(), "anonymize".into(), "export".into(),
        ],
        LicenseType::Enterprise => vec![
            "core".into(), "hl7v2".into(), "fhir".into(),
            "mllp".into(), "http".into(), "anonymize".into(), "export".into(),
            "soap".into(), "plugins".into(), "priority_support".into(),
        ],
        _ => vec!["core".into()],
    };

    let expires_at = match license_type {
        LicenseType::Free => None,
        _ => Some((chrono::Utc::now() + chrono::Duration::days(365)).to_rfc3339()),
    };

    let license = LicenseFile {
        payload: LicensePayload {
            license_type,
            licensee: licensee.to_string(),
            email: email.to_string(),
            hardware_id: get_hardware_id(),
            issued_at: chrono::Utc::now().to_rfc3339(),
            expires_at,
            features,
        },
        signature: "dev-mode-no-signature".to_string(),
    };

    save_license(&license)?;
    Ok(license)
}

// =============================================================================
// Check current status
// =============================================================================

pub fn check_license_status() -> LicenseStatus {
    let hardware_id = get_hardware_id();

    if let Some(license) = load_license() {
        // Hardware check
        if !license.payload.hardware_id.is_empty() && license.payload.hardware_id != hardware_id {
            return LicenseStatus {
                is_valid: false,
                license_type: LicenseType::Expired,
                days_remaining: None,
                licensee: license.payload.licensee,
                email: license.payload.email,
                features: vec![],
                message: "License is bound to a different machine".into(),
            };
        }

        // Signature check (skip in dev mode)
        if license.signature != "dev-mode-no-signature" {
            if !verify_signature(&license.payload, &license.signature) {
                return LicenseStatus {
                    is_valid: false,
                    license_type: LicenseType::Expired,
                    days_remaining: None,
                    licensee: license.payload.licensee,
                    email: license.payload.email,
                    features: vec![],
                    message: "License signature is invalid".into(),
                };
            }
        }

        // Expiration check
        if let Some(ref expires) = license.payload.expires_at {
            if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(expires) {
                let days = (exp.with_timezone(&chrono::Utc) - chrono::Utc::now()).num_days();
                if days < 0 {
                    return LicenseStatus {
                        is_valid: false,
                        license_type: LicenseType::Expired,
                        days_remaining: Some(0),
                        licensee: license.payload.licensee,
                        email: license.payload.email,
                        features: vec![],
                        message: "License has expired".into(),
                    };
                }
                return LicenseStatus {
                    is_valid: true,
                    license_type: license.payload.license_type,
                    days_remaining: Some(days),
                    licensee: license.payload.licensee,
                    email: license.payload.email,
                    features: license.payload.features,
                    message: format!("{} days remaining", days),
                };
            }
        }

        // No expiration (Free license)
        return LicenseStatus {
            is_valid: true,
            license_type: license.payload.license_type,
            days_remaining: None,
            licensee: license.payload.licensee,
            email: license.payload.email,
            features: license.payload.features,
            message: "License is valid".into(),
        };
    }

    // No license - check trial
    let trial = load_or_init_trial();
    let days = trial_days_remaining(&trial);

    if days > 0 {
        LicenseStatus {
            is_valid: true,
            license_type: LicenseType::Trial,
            days_remaining: Some(days),
            licensee: String::new(),
            email: String::new(),
            features: vec!["core".into(), "hl7v2".into(), "fhir".into(), "mllp".into(), "http".into()],
            message: format!("Trial: {} days remaining", days),
        }
    } else {
        LicenseStatus {
            is_valid: false,
            license_type: LicenseType::Expired,
            days_remaining: Some(0),
            licensee: String::new(),
            email: String::new(),
            features: vec![],
            message: "Trial has expired. Please activate a license.".into(),
        }
    }
}

// Hex encode/decode helpers (avoid adding hex crate dependency)
mod hex {
    pub fn decode(s: &str) -> Result<Vec<u8>, ()> {
        if s.len() % 2 != 0 { return Err(()); }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
            .collect()
    }

    #[allow(dead_code)]
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_id_stable() {
        let id1 = get_hardware_id();
        let id2 = get_hardware_id();
        assert_eq!(id1, id2);
        assert!(id1.starts_with("BL-"));
    }

    #[test]
    fn test_simple_key_free() {
        let license = activate_simple_key("BL-FREE-ABCD1234EFGH", "Test", "test@test.com").unwrap();
        assert_eq!(license.payload.license_type, LicenseType::Free);
        // Cleanup
        remove_license().ok();
    }

    #[test]
    fn test_simple_key_invalid() {
        assert!(activate_simple_key("INVALID", "", "").is_err());
        assert!(activate_simple_key("BL-FREE-short", "", "").is_err());
    }

    #[test]
    fn test_trial_days() {
        let trial = TrialData {
            started_at: chrono::Utc::now().to_rfc3339(),
            trial_days: 30,
        };
        let days = trial_days_remaining(&trial);
        assert!(days >= 29 && days <= 30);
    }

    #[test]
    fn test_hex_roundtrip() {
        let data = b"hello world";
        let encoded = hex::encode(data);
        let decoded = hex::decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }
}
