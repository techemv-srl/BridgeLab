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
//! | plugins_unlimited  | ✗ (3)     | ✓   | ✓          |
//! | test_cases_unlimited| ✗ (3)    | ✓   | ✓          |
//! | xsd_export_community| ✓ (2)    | ✓   | ✓          |
//! | xsd_export_full    | ✗         | ✓   | ✓          |
//! | batch_validate     | ✗         | ✓   | ✓          |
//! | soap               | ✗         | ✗   | ✓          |
//! | priority_support   | ✗         | ✗   | ✓          |
//!
//! (1) `mllp_send` is fully available in the Community tier.
//! (2) `xsd_export_community` covers the 4 common messages (ADT^A01,
//!     ADT^A40, ORM^O01, ORU^R01) in v2.5 only. All other messages or
//!     versions require `xsd_export_full` (Pro).
//! (3) Community keeps up to [`COMMUNITY_MAX_ACTIVE_PLUGINS`] plugin packs
//!     active and [`COMMUNITY_MAX_TEST_CASES`] saved test cases. Existing
//!     data is never locked or deleted when a trial ends — the caps only
//!     block *new* activations/saves beyond the limit.

use crate::licensing::{self, LicenseStatus, LicenseType};

/// Features that the Community tier gets for free.
const COMMUNITY_FEATURES: &[&str] = &[
    "core",
    "hl7v2",
    "fhir_parse",
    "validation",
    "mllp_send",
    "http_get",
    "anonymize_detect",
    "xsd_export_community",
];

/// Additional features unlocked by Pro.
const PRO_FEATURES: &[&str] = &[
    "batch_validate",
    "mllp_listen",
    "http_mutate",
    "http_auth",
    "anonymize_mask",
    "export",
    "fhirpath",
    "bundle_visualizer",
    "plugins_unlimited",
    "test_cases_unlimited",
    "xsd_export_full",
];

/// Additional features unlocked by Enterprise.
const ENTERPRISE_FEATURES: &[&str] = &[
    "soap",
    "priority_support",
];

/// Community cap on simultaneously active plugin packs.
pub const COMMUNITY_MAX_ACTIVE_PLUGINS: usize = 3;
/// Community cap on saved test cases.
pub const COMMUNITY_MAX_TEST_CASES: usize = 10;

/// Cap on active plugin packs for the current license, `None` = unlimited.
pub fn active_plugin_limit() -> Option<usize> {
    if require("plugins_unlimited").is_ok() {
        None
    } else {
        Some(COMMUNITY_MAX_ACTIVE_PLUGINS)
    }
}

/// Cap on saved test cases for the current license, `None` = unlimited.
pub fn test_case_limit() -> Option<usize> {
    if require("test_cases_unlimited").is_ok() {
        None
    } else {
        Some(COMMUNITY_MAX_TEST_CASES)
    }
}

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
    // Community features are always available, no need to read license state
    if COMMUNITY_FEATURES.contains(&feature) {
        return Ok(());
    }
    check_feature(feature, &licensing::check_license_status())
}

/// Pure tier check against a given license status (separated from `require`
/// so tests can exercise every tier without touching on-disk license state).
fn check_feature(feature: &str, status: &LicenseStatus) -> Result<(), String> {
    if COMMUNITY_FEATURES.contains(&feature) {
        return Ok(());
    }

    let has_feature = match status.license_type {
        // A valid trial gets Pro-level access — not Enterprise
        LicenseType::Trial => status.is_valid && PRO_FEATURES.contains(&feature),
        LicenseType::Professional => status.is_valid && PRO_FEATURES.contains(&feature),
        LicenseType::Enterprise => status.is_valid,
        LicenseType::Free | LicenseType::Expired => false,
    };

    // Also check the explicit feature list in the signed license payload
    let explicit = status.is_valid && status.features.iter().any(|f| f == feature);

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

    fn status(lt: LicenseType, is_valid: bool) -> LicenseStatus {
        LicenseStatus {
            is_valid,
            license_type: lt,
            days_remaining: None,
            licensee: String::new(),
            email: String::new(),
            features: vec![],
            message: String::new(),
        }
    }

    #[test]
    fn community_features_always_pass() {
        // Community features pass regardless of license state
        assert!(require("core").is_ok());
        assert!(require("hl7v2").is_ok());
        assert!(require("validation").is_ok());
        assert!(require("mllp_send").is_ok());
        assert!(require("http_get").is_ok());
        assert!(require("anonymize_detect").is_ok());
        assert!(require("xsd_export_community").is_ok());

        // ...and even against an explicitly invalid status
        for f in COMMUNITY_FEATURES {
            assert!(check_feature(f, &status(LicenseType::Expired, false)).is_ok());
        }
    }

    #[test]
    fn free_tier_denies_pro_and_enterprise() {
        let free = status(LicenseType::Free, true);
        for f in PRO_FEATURES.iter().chain(ENTERPRISE_FEATURES) {
            let err = check_feature(f, &free).unwrap_err();
            assert!(err.contains("UPGRADE_REQUIRED"), "{f} should be gated for Free");
        }
    }

    #[test]
    fn valid_trial_gets_pro_but_not_enterprise() {
        let trial = status(LicenseType::Trial, true);
        for f in PRO_FEATURES {
            assert!(check_feature(f, &trial).is_ok(), "{f} should be open in trial");
        }
        for f in ENTERPRISE_FEATURES {
            let err = check_feature(f, &trial).unwrap_err();
            assert!(err.contains(":Enterprise:"), "{f} must stay Enterprise-only in trial");
        }
    }

    #[test]
    fn invalid_trial_denies_pro() {
        let trial = status(LicenseType::Trial, false);
        for f in PRO_FEATURES {
            assert!(check_feature(f, &trial).is_err(), "{f} must be gated when trial invalid");
        }
    }

    #[test]
    fn professional_gets_pro_but_not_enterprise() {
        let pro = status(LicenseType::Professional, true);
        for f in PRO_FEATURES {
            assert!(check_feature(f, &pro).is_ok(), "{f} should be open for Pro");
        }
        for f in ENTERPRISE_FEATURES {
            let err = check_feature(f, &pro).unwrap_err();
            assert!(err.contains(":Enterprise:"), "{f} must stay Enterprise-only for Pro");
        }
    }

    #[test]
    fn enterprise_gets_everything() {
        let ent = status(LicenseType::Enterprise, true);
        for f in COMMUNITY_FEATURES.iter().chain(PRO_FEATURES).chain(ENTERPRISE_FEATURES) {
            assert!(check_feature(f, &ent).is_ok(), "{f} should be open for Enterprise");
        }
    }

    #[test]
    fn expired_license_denies_pro() {
        let expired = status(LicenseType::Expired, false);
        for f in PRO_FEATURES {
            assert!(check_feature(f, &expired).is_err(), "{f} must be gated when expired");
        }
    }

    #[test]
    fn explicit_payload_feature_grants_access() {
        // A signed license can grant individual features beyond its tier
        let mut custom = status(LicenseType::Free, true);
        custom.features = vec!["export".into()];
        assert!(check_feature("export", &custom).is_ok());
        assert!(check_feature("fhirpath", &custom).is_err());

        // ...but only while the license is valid
        custom.is_valid = false;
        assert!(check_feature("export", &custom).is_err());
    }

    #[test]
    fn upgrade_error_names_the_correct_tier() {
        let free = status(LicenseType::Free, true);
        let pro_err = check_feature("export", &free).unwrap_err();
        assert!(pro_err.starts_with("UPGRADE_REQUIRED:export:Professional:"));
        let ent_err = check_feature("soap", &free).unwrap_err();
        assert!(ent_err.starts_with("UPGRADE_REQUIRED:soap:Enterprise:"));
    }

    #[test]
    fn tier_arrays_are_disjoint() {
        for f in PRO_FEATURES {
            assert!(!COMMUNITY_FEATURES.contains(f), "{f} duplicated in community");
        }
        for f in ENTERPRISE_FEATURES {
            assert!(!COMMUNITY_FEATURES.contains(f), "{f} duplicated in community");
            assert!(!PRO_FEATURES.contains(f), "{f} duplicated in pro");
        }
    }

    #[test]
    fn features_for_type_match_tier_arrays() {
        let free = available_features_for_type(&LicenseType::Free);
        assert_eq!(free.len(), COMMUNITY_FEATURES.len());

        let pro = available_features_for_type(&LicenseType::Professional);
        assert_eq!(pro.len(), COMMUNITY_FEATURES.len() + PRO_FEATURES.len());
        assert!(pro.contains(&"batch_validate".to_string()));
        assert!(!pro.contains(&"soap".to_string()));

        let ent = available_features_for_type(&LicenseType::Enterprise);
        assert_eq!(
            ent.len(),
            COMMUNITY_FEATURES.len() + PRO_FEATURES.len() + ENTERPRISE_FEATURES.len()
        );
        assert!(ent.contains(&"soap".to_string()));
    }

    #[test]
    fn available_features_includes_community() {
        let features = available_features();
        assert!(features.contains(&"core".to_string()));
        assert!(features.contains(&"hl7v2".to_string()));
    }
}
