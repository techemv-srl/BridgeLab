use ed25519_dalek::{Signature, Verifier, VerifyingKey, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod feature_gate;

// =============================================================================
// The PUBLIC key is embedded in the app for offline verification.
// The PRIVATE key is kept secret in the CLI tool only.
// Generate a new keypair with: bridgelab-keygen generate-keypair
// =============================================================================
const PUBLIC_KEY_HEX: &str = "cd9559f4beffe61a9c2878434a84fb2c3de85e36247c4188537c722d9fcc2649";

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
///
/// v2 binds the record to the machine (`hw`), carries a monotonic
/// high-water timestamp (`last_seen`) so rolling the system clock back
/// cannot extend the trial, and an integrity tag (`sig`) so the file
/// cannot simply be edited or copied from another machine. The tag is a
/// salted hash with the salt embedded in the binary — this stops casual
/// tampering, not a determined reverse engineer (nothing offline can).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialData {
    pub started_at: String,
    pub trial_days: i64,
    #[serde(default)]
    pub hw: String,
    #[serde(default)]
    pub last_seen: String,
    #[serde(default)]
    pub sig: String,
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

/// Redundant copy of the trial record in a second base directory
/// (cache dir ≠ data dir on every supported platform), so deleting
/// `trial.json` alone no longer restarts the trial.
fn trial_marker_path() -> Result<PathBuf, String> {
    let dir = dirs::cache_dir()
        .ok_or_else(|| "Could not determine cache directory".to_string())?;
    Ok(dir.join("BridgeLab").join(".bl-state.json"))
}

// =============================================================================
// License verification (Ed25519 signature check)
// =============================================================================

/// Verify an Ed25519 signature on a license payload.
fn verify_signature(payload: &LicensePayload, signature_hex: &str) -> bool {
    // Reject if no public key has been configured (shipping placeholder = no valid licenses)
    if PUBLIC_KEY_HEX == "PLACEHOLDER_GENERATE_WITH_CLI" {
        return false;
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

const TRIAL_DAYS: i64 = 7;

/// Salt for the trial integrity tag. Embedded in the binary: raises the
/// bar from "edit a JSON file" to "reverse engineer the executable".
const TRIAL_SIG_SALT: &[u8] = &[
    0x42, 0x4c, 0x54, 0x32, 0x9f, 0x4e, 0x7c, 0x11,
    0xd2, 0xa6, 0x4b, 0x08, 0x5e, 0x31, 0xc7, 0xe9,
];

fn trial_sig(started_at: &str, trial_days: i64, hw: &str, last_seen: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(TRIAL_SIG_SALT);
    for part in [started_at, hw, last_seen] {
        h.update(part.as_bytes());
        h.update([0x1f]);
    }
    h.update(trial_days.to_le_bytes());
    hex::encode(&h.finalize())
}

fn sign_trial(trial: &mut TrialData) {
    trial.sig = trial_sig(&trial.started_at, trial.trial_days, &trial.hw, &trial.last_seen);
}

/// A trial record is authentic when its tag matches, it is bound to this
/// machine, its duration was not inflated and its start is not in the future.
fn trial_is_authentic(trial: &TrialData, hw: &str) -> bool {
    if trial.sig.is_empty() || trial.hw != hw {
        return false;
    }
    if trial.trial_days <= 0 || trial.trial_days > TRIAL_DAYS {
        return false;
    }
    let started = match chrono::DateTime::parse_from_rfc3339(&trial.started_at) {
        Ok(d) => d.with_timezone(&chrono::Utc),
        Err(_) => return false,
    };
    if started > chrono::Utc::now() + chrono::Duration::hours(24) {
        return false;
    }
    trial.sig == trial_sig(&trial.started_at, trial.trial_days, &trial.hw, &trial.last_seen)
}

/// Pre-hardening `trial.json` files had only `started_at` + `trial_days`.
/// Accept them (so honest mid-trial users keep their remaining days) only
/// when they look exactly like what the old code wrote and do not claim a
/// start in the future.
fn is_plausible_legacy(trial: &TrialData) -> bool {
    trial.sig.is_empty()
        && trial.hw.is_empty()
        && trial.last_seen.is_empty()
        && trial.trial_days == TRIAL_DAYS
        && chrono::DateTime::parse_from_rfc3339(&trial.started_at)
            .map(|d| d.with_timezone(&chrono::Utc) <= chrono::Utc::now() + chrono::Duration::hours(1))
            .unwrap_or(false)
}

pub fn load_or_init_trial() -> TrialData {
    let hw = get_hardware_id();
    let now = chrono::Utc::now();

    let mut candidates: Vec<TrialData> = Vec::new();
    let mut tampered = false;
    let mut intact_files = 0usize;

    for path in [trial_file_path().ok(), trial_marker_path().ok()]
        .into_iter()
        .flatten()
    {
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        match serde_json::from_str::<TrialData>(&content) {
            Ok(t) if trial_is_authentic(&t, &hw) => {
                intact_files += 1;
                candidates.push(t);
            }
            Ok(mut t) if is_plausible_legacy(&t) => {
                t.hw = hw.clone();
                t.last_seen = now.to_rfc3339();
                sign_trial(&mut t);
                candidates.push(t);
            }
            _ => tampered = true,
        }
    }

    // When both copies survive, the least generous (earliest start) wins.
    let mut trial = match candidates.into_iter().min_by_key(|t| {
        chrono::DateTime::parse_from_rfc3339(&t.started_at)
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or(now)
    }) {
        Some(t) => t,
        None if tampered => {
            // A record existed but failed verification: fail closed to an
            // already-expired trial instead of granting a fresh one.
            let mut t = TrialData {
                started_at: (now - chrono::Duration::days(TRIAL_DAYS + 1)).to_rfc3339(),
                trial_days: TRIAL_DAYS,
                hw: hw.clone(),
                last_seen: now.to_rfc3339(),
                sig: String::new(),
            };
            sign_trial(&mut t);
            t
        }
        None => new_trial(&hw),
    };

    // Advance the monotonic high-water mark (throttled to hourly so gated
    // IPC calls don't rewrite the files on every invocation).
    let should_advance = match chrono::DateTime::parse_from_rfc3339(&trial.last_seen) {
        Ok(seen) => now > seen.with_timezone(&chrono::Utc) + chrono::Duration::hours(1),
        Err(_) => true,
    };
    if should_advance {
        trial.last_seen = now.to_rfc3339();
        sign_trial(&mut trial);
    }
    if should_advance || tampered || intact_files < 2 {
        persist_trial(&trial);
    }
    trial
}

fn new_trial(hw: &str) -> TrialData {
    let now = chrono::Utc::now().to_rfc3339();
    let mut trial = TrialData {
        started_at: now.clone(),
        trial_days: TRIAL_DAYS,
        hw: hw.to_string(),
        last_seen: now,
        sig: String::new(),
    };
    sign_trial(&mut trial);
    trial
}

/// Best-effort write to both locations; a single surviving copy is enough
/// for the next load to restore the other.
fn persist_trial(trial: &TrialData) {
    let json = match serde_json::to_string_pretty(trial) {
        Ok(j) => j,
        Err(_) => return,
    };
    for path in [trial_file_path().ok(), trial_marker_path().ok()]
        .into_iter()
        .flatten()
    {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&path, &json).ok();
    }
}

pub fn trial_days_remaining(trial: &TrialData) -> i64 {
    let started = match chrono::DateTime::parse_from_rfc3339(&trial.started_at) {
        Ok(d) => d.with_timezone(&chrono::Utc),
        // Unparseable start = tampered record: fail closed, never fail open.
        Err(_) => return 0,
    };

    // If the clock was rolled back past the last time the app ran, count
    // from the high-water mark instead of the (rewound) wall clock.
    let mut now = chrono::Utc::now();
    if let Ok(seen) = chrono::DateTime::parse_from_rfc3339(&trial.last_seen) {
        let seen = seen.with_timezone(&chrono::Utc);
        if seen > now {
            now = seen;
        }
    }

    let expires = started + chrono::Duration::days(trial.trial_days);
    (expires - now).num_days().max(0)
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

/// Simple key activation (BL-TYPE-CODE format).
/// Only available in debug builds; release builds require a signed license.
#[cfg(debug_assertions)]
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

    let features = feature_gate::available_features_for_type(&license_type);

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

        // Signature check (always enforced in release builds).
        // In debug builds the simple-key activation flow writes a sentinel
        // signature `"dev-mode-no-signature"` — accept it so that `BL-PRO-XXX`
        // activations remain usable during local development.
        #[cfg(debug_assertions)]
        let skip_sig_check = license.signature == "dev-mode-no-signature";
        #[cfg(not(debug_assertions))]
        let skip_sig_check = false;

        if !skip_sig_check && !verify_signature(&license.payload, &license.signature) {
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
            features: feature_gate::available_features_for_type(&LicenseType::Professional),
            message: format!("Trial: {} days remaining", days),
        }
    } else {
        // Trial expired → fall back to Community (Free) tier, not zero features
        LicenseStatus {
            is_valid: true,
            license_type: LicenseType::Free,
            days_remaining: None,
            licensee: String::new(),
            email: String::new(),
            features: feature_gate::available_features_for_type(&LicenseType::Free),
            message: "Trial expired. Community features are still available.".into(),
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

    fn make_trial(started_at: String, trial_days: i64) -> TrialData {
        let mut t = TrialData {
            started_at,
            trial_days,
            hw: get_hardware_id(),
            last_seen: chrono::Utc::now().to_rfc3339(),
            sig: String::new(),
        };
        sign_trial(&mut t);
        t
    }

    #[test]
    fn test_trial_days() {
        let trial = make_trial(chrono::Utc::now().to_rfc3339(), 7);
        let days = trial_days_remaining(&trial);
        assert!(days >= 6 && days <= 7, "expected 6-7 days remaining, got {}", days);
    }

    #[test]
    fn test_signed_trial_is_authentic() {
        let trial = make_trial(chrono::Utc::now().to_rfc3339(), 7);
        assert!(trial_is_authentic(&trial, &get_hardware_id()));
    }

    #[test]
    fn test_edited_fields_break_authenticity() {
        let hw = get_hardware_id();
        let base = make_trial(chrono::Utc::now().to_rfc3339(), 7);

        // Editing the start date without re-signing
        let mut edited = base.clone();
        edited.started_at = (chrono::Utc::now() + chrono::Duration::days(300)).to_rfc3339();
        assert!(!trial_is_authentic(&edited, &hw));

        // Inflating the duration — even re-signed, >TRIAL_DAYS is rejected
        let mut inflated = base.clone();
        inflated.trial_days = 999_999;
        sign_trial(&mut inflated);
        assert!(!trial_is_authentic(&inflated, &hw));

        // Record copied from a different machine
        let mut foreign = base.clone();
        foreign.hw = "BL-0000000000000000".into();
        sign_trial(&mut foreign);
        assert!(!trial_is_authentic(&foreign, &hw));

        // v1-style record with no tag at all
        let bare = TrialData {
            started_at: chrono::Utc::now().to_rfc3339(),
            trial_days: 7,
            hw: String::new(),
            last_seen: String::new(),
            sig: String::new(),
        };
        assert!(!trial_is_authentic(&bare, &hw));
    }

    #[test]
    fn test_future_start_rejected() {
        let hw = get_hardware_id();
        let trial = make_trial((chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339(), 7);
        assert!(!trial_is_authentic(&trial, &hw));
    }

    #[test]
    fn test_unparseable_start_fails_closed() {
        // Pre-hardening this fell back to `now` → a never-expiring trial.
        let mut trial = make_trial("not-a-date".into(), 7);
        sign_trial(&mut trial);
        assert_eq!(trial_days_remaining(&trial), 0);
    }

    #[test]
    fn test_clock_rollback_does_not_extend() {
        // last_seen far beyond the wall clock simulates a rolled-back clock:
        // remaining days count from the high-water mark, not from `now`.
        let mut trial = make_trial(chrono::Utc::now().to_rfc3339(), 7);
        trial.last_seen = (chrono::Utc::now() + chrono::Duration::days(20)).to_rfc3339();
        sign_trial(&mut trial);
        assert_eq!(trial_days_remaining(&trial), 0);
    }

    #[test]
    fn test_legacy_plausibility() {
        let legacy = TrialData {
            started_at: (chrono::Utc::now() - chrono::Duration::days(3)).to_rfc3339(),
            trial_days: 7,
            hw: String::new(),
            last_seen: String::new(),
            sig: String::new(),
        };
        assert!(is_plausible_legacy(&legacy));

        // Forged legacy with a future start is not migrated
        let mut forged = legacy.clone();
        forged.started_at = (chrono::Utc::now() + chrono::Duration::days(300)).to_rfc3339();
        assert!(!is_plausible_legacy(&forged));

        // Forged legacy with inflated duration is not migrated
        let mut inflated = legacy.clone();
        inflated.trial_days = 9_999;
        assert!(!is_plausible_legacy(&inflated));
    }

    #[test]
    fn test_hex_roundtrip() {
        let data = b"hello world";
        let encoded = hex::encode(data);
        let decoded = hex::decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }
}
