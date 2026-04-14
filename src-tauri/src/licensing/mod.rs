use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

/// Stored license data (persisted to disk).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseData {
    pub license_key: String,
    pub license_type: LicenseType,
    pub licensee: String,
    pub email: String,
    pub activated_at: String,
    pub expires_at: Option<String>,
    pub hardware_id: String,
    pub features: Vec<String>,
}

/// Trial tracking data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialData {
    pub started_at: String,
    pub trial_days: i64,
}

/// Get hardware fingerprint for license binding.
pub fn get_hardware_id() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();

    // Combine hostname + OS + username for a reasonably stable fingerprint
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

/// Get the license file path.
fn license_file_path() -> Result<PathBuf, String> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "Could not determine data directory".to_string())?;
    Ok(data_dir.join("BridgeLab").join("license.json"))
}

/// Get the trial file path.
fn trial_file_path() -> Result<PathBuf, String> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "Could not determine data directory".to_string())?;
    Ok(data_dir.join("BridgeLab").join("trial.json"))
}

/// Load stored license from disk.
pub fn load_license() -> Option<LicenseData> {
    let path = license_file_path().ok()?;
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Save license to disk.
pub fn save_license(license: &LicenseData) -> Result<(), String> {
    let path = license_file_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(license)
        .map_err(|e| format!("Serialize failed: {}", e))?;
    std::fs::write(path, json).map_err(|e| format!("Write failed: {}", e))
}

/// Remove license from disk.
pub fn remove_license() -> Result<(), String> {
    let path = license_file_path()?;
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Load or initialize trial data.
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

    // Initialize new trial
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

/// Calculate trial days remaining.
pub fn trial_days_remaining(trial: &TrialData) -> i64 {
    let started = chrono::DateTime::parse_from_rfc3339(&trial.started_at)
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    let expires = started + chrono::Duration::days(trial.trial_days);
    let remaining = expires - chrono::Utc::now();
    remaining.num_days().max(0)
}

/// Validate a license key (offline check).
/// Format: BL-{type}-{hash} where type is FREE/PRO/ENT
pub fn validate_key(key: &str, hardware_id: &str) -> Result<LicenseData, String> {
    let parts: Vec<&str> = key.split('-').collect();
    if parts.len() < 3 || parts[0] != "BL" {
        return Err("Invalid license key format".into());
    }

    let license_type = match parts[1] {
        "FREE" => LicenseType::Free,
        "PRO" => LicenseType::Professional,
        "ENT" => LicenseType::Enterprise,
        _ => return Err("Unknown license type".into()),
    };

    // In production, this would verify a cryptographic signature.
    // For now, accept keys matching format BL-{TYPE}-{any 8+ chars}
    let key_body = parts[2..].join("-");
    if key_body.len() < 8 {
        return Err("License key too short".into());
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
        LicenseType::Free => None, // Never expires
        _ => Some(
            (chrono::Utc::now() + chrono::Duration::days(365))
                .to_rfc3339(),
        ),
    };

    Ok(LicenseData {
        license_key: key.to_string(),
        license_type,
        licensee: String::new(),
        email: String::new(),
        activated_at: chrono::Utc::now().to_rfc3339(),
        expires_at,
        hardware_id: hardware_id.to_string(),
        features,
    })
}

/// Check the current license status.
pub fn check_license_status() -> LicenseStatus {
    let hardware_id = get_hardware_id();

    // Check stored license first
    if let Some(license) = load_license() {
        // Verify hardware
        if license.hardware_id != hardware_id {
            return LicenseStatus {
                is_valid: false,
                license_type: LicenseType::Expired,
                days_remaining: None,
                licensee: license.licensee,
                email: license.email,
                features: vec![],
                message: "License is bound to a different machine".into(),
            };
        }

        // Check expiration
        if let Some(ref expires) = license.expires_at {
            if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(expires) {
                let days = (exp.with_timezone(&chrono::Utc) - chrono::Utc::now()).num_days();
                if days < 0 {
                    return LicenseStatus {
                        is_valid: false,
                        license_type: LicenseType::Expired,
                        days_remaining: Some(0),
                        licensee: license.licensee,
                        email: license.email,
                        features: vec![],
                        message: "License has expired".into(),
                    };
                }
                return LicenseStatus {
                    is_valid: true,
                    license_type: license.license_type,
                    days_remaining: Some(days),
                    licensee: license.licensee,
                    email: license.email,
                    features: license.features,
                    message: format!("{} days remaining", days),
                };
            }
        }

        // No expiration (e.g., Free license)
        return LicenseStatus {
            is_valid: true,
            license_type: license.license_type,
            days_remaining: None,
            licensee: license.licensee,
            email: license.email,
            features: license.features,
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
    fn test_validate_key_free() {
        let hw = get_hardware_id();
        let license = validate_key("BL-FREE-ABCD1234EFGH", &hw).unwrap();
        assert_eq!(license.license_type, LicenseType::Free);
        assert!(license.features.contains(&"core".to_string()));
    }

    #[test]
    fn test_validate_key_pro() {
        let hw = get_hardware_id();
        let license = validate_key("BL-PRO-12345678ABCD", &hw).unwrap();
        assert_eq!(license.license_type, LicenseType::Professional);
        assert!(license.features.contains(&"fhir".to_string()));
    }

    #[test]
    fn test_validate_key_invalid() {
        let hw = get_hardware_id();
        assert!(validate_key("INVALID", &hw).is_err());
        assert!(validate_key("BL-FREE-short", &hw).is_err());
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
}
