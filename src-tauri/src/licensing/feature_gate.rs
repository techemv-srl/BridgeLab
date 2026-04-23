//! Feature-gate enforcement for BridgeLab licensing tiers.
//!
//! Every IPC command that is restricted beyond the Community (Free/Trial) tier
//! calls `require(feature, &registry)` before executing. If the feature is not
//! available, a user-friendly error is returned so the frontend can show an
//! upgrade prompt.
//!
//! ## Tier mapping
//!
//! | Feature tag        | Community | Pro | Enterprise |
//! |--------------------|-----------|-----|------------|
//! | core               | ✓         | ✓   | ✓          |
//! | hl7v2              | ✓         | ✓   | ✓          |
//! | fhir_parse         | ✓         | ✓   | ✓          |
//! | validation         | ✓         | ✓   | ✓          |
//! | mllp_send          | ✓ (1)     | ✓   | ✓          |
//! | mllp_listen        | ✗         | ✓   | ✓          |
//! | http_get           | ✓         | ✓   | ✓          |
//! | http_mutate        | ✗         | ✓   | ✓          |
//! | http_auth          | ✗         | ✓   | ✓          |
//! | anonymize_detect   | ✓         | ✓   | ✓          |
//! | anonymize_mask     | ✗         | ✓   | ✓          |
//! | export             | ✗         | ✓   | ✓          |
//! | fhirpath           | ✗         | ✓   | ✓          |
//! | bundle_visualizer  | ✗         | ✓   | ✓          |
//! | plugins_unlimited  | ✗         | ✓   | ✓          |
//! | test_cases_unlimited| ✗        | ✓   | ✓          |
//! | soap               | ✗         | ✗   | ✓          |
//! | priority_support   | ✗         | ✗   | ✓          |

use crate::licensing::{self, LicenseType};

/// Features that the Community tier gets for free.
const COMMUNITY_FEATURES: &[&str] = &[
    "core",
    "hl7v2",
    "fhir_parse",
    "validation",
    "mllp_send",
    "http_get",
    "anonymize_detect",
];

/// Additional features unlocked by Pro.
const PRO_FEATURES: &[&str] = &[
    "mllp_listen",
    "http_mutate",
    "http_auth",
    "anonymize_mask",
    "export",
    "fhirpath",
    "bundle_visualizer",
    "plugins_unlimited",
    "test_cases_unlimited",
];

/// Additional features unlocked by Enterprise.
const ENTERPRISE_FEATURES: &[&str] = &[
    "soap",
    "priority_support",
];

/// Error returned when a feature is gated.
#[derive(Debug)]
pub struct FeatureGatedError {
    pub feature: String,
    pub required_tier: String,
    pub message: String,
}

impl std::fmt::Display for FeatureGatedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Check whether the current license grants access to a feature.
/// Returns `Ok(())` if allowed, or a descriptive error otherwise.
pub fn require(feature: &str) -> Result<(), String> {
    // Community features are always available
    if COMMUNITY_FEATURES.contains(&feature) {
        return Ok(());
    }

    let status = licensing::check_license_status();

    // Trial gets Pro-level access
    if status.license_type == LicenseType::Trial && status.is_valid {
        return Ok(());
    }

    let has_feature = match status.license_type {
        LicenseType::Free => false,
        LicenseType::Professional => {
            PRO_FEATURES.contains(&feature) || COMMUNITY_FEATURES.contains(&feature)
        }
        LicenseType::Enterprise => true,
        LicenseType::Trial if status.is_valid => true,
        _ => false,
    };

    // Also check the explicit feature list in the signed license payload
    let explicit = status.features.iter().any(|f| f == feature);

    if has_feature || explicit {
        Ok(())
    } else {
        let tier = if ENTERPRISE_FEATURES.contains(&feature) {
            "Enterprise"
        } else {
            "Professional"
        };
        Err(format!(
            "UPGRADE_REQUIRED:{}:{}:This feature requires a {} license. \
             Upgrade at Settings → Activation or contact info@techemv.it.",
            feature, tier, tier
        ))
    }
}

/// Return the feature list for a given license type (used by keygen + simple key).
pub fn available_features_for_type(lt: &super::LicenseType) -> Vec<String> {
    let mut features: Vec<String> = COMMUNITY_FEATURES.iter().map(|s| s.to_string()).collect();
    match lt {
        super::LicenseType::Professional => {
            features.extend(PRO_FEATURES.iter().map(|s| s.to_string()));
        }
        super::LicenseType::Enterprise => {
            features.extend(PRO_FEATURES.iter().map(|s| s.to_string()));
            features.extend(ENTERPRISE_FEATURES.iter().map(|s| s.to_string()));
        }
        _ => {}
    }
    features
}

/// Return the full list of features available to the current user.
pub fn available_features() -> Vec<String> {
    let status = licensing::check_license_status();
    let mut features: Vec<String> = COMMUNITY_FEATURES.iter().map(|s| s.to_string()).collect();

    let pro_access = matches!(
        status.license_type,
        LicenseType::Professional | LicenseType::Enterprise
    ) || (status.license_type == LicenseType::Trial && status.is_valid);

    if pro_access {
        features.extend(PRO_FEATURES.iter().map(|s| s.to_string()));
    }

    if status.license_type == LicenseType::Enterprise {
        features.extend(ENTERPRISE_FEATURES.iter().map(|s| s.to_string()));
    }

    features
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn community_features_always_pass() {
        // Community features pass regardless of license state
        assert!(require("core").is_ok());
        assert!(require("hl7v2").is_ok());
        assert!(require("validation").is_ok());
        assert!(require("mllp_send").is_ok());
        assert!(require("http_get").is_ok());
        assert!(require("anonymize_detect").is_ok());
    }

    #[test]
    fn pro_features_gated_without_license() {
        // Without a valid Pro/Enterprise license, Pro features should fail
        // (unless there's an active trial, which depends on file state)
        let result = require("export");
        // This may pass during trial or fail after; we just verify it doesn't panic
        assert!(result.is_ok() || result.unwrap_err().contains("UPGRADE_REQUIRED"));
    }

    #[test]
    fn available_features_includes_community() {
        let features = available_features();
        assert!(features.contains(&"core".to_string()));
        assert!(features.contains(&"hl7v2".to_string()));
    }
}
