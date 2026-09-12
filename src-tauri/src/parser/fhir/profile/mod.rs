//! FHIR profile validation.
//!
//! Conformance against a StructureDefinition, driven by FHIR NPM packages
//! the user installs. The official HL7 validator is a Java tool; requiring
//! a JRE would undo the point of an offline desktop app, so the checks it
//! performs structurally are implemented here in Rust instead.
//!
//! - [`package`] reads a `.tgz` and distils its StructureDefinitions.
//! - [`model`] is the reduced definition the validator works from.
//! - [`validate`] walks a resource against a profile.
//!
//! Terminology is deliberately out of scope: a `required` binding needs the
//! ValueSet expanded, which means shipping the terminology packages or
//! calling a server. Bindings are left unchecked rather than half-checked.

pub mod model;
pub mod package;
pub mod validate;

use std::sync::RwLock;

use serde::Serialize;
use serde_json::Value;

use crate::parser::fhir::FhirValidationIssue;
use model::ProfileIndex;

/// Installed packages, loaded once and shared across commands.
pub struct ProfileRegistry {
    index: RwLock<ProfileIndex>,
}

/// What the frontend shows in the package manager.
#[derive(Debug, Clone, Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub title: String,
    pub fhir_version: String,
    pub profile_count: usize,
}

impl ProfileRegistry {
    pub fn new() -> Self {
        Self {
            index: RwLock::new(ProfileIndex::default()),
        }
    }

    /// Rebuild the index from what is installed on disk.
    pub fn reload(&self) -> Result<usize, String> {
        let mut index = ProfileIndex::default();
        for package in package::load_installed() {
            index.add_package(package);
        }
        let count = index.profile_count();
        *self.index.write().map_err(|e| e.to_string())? = index;
        Ok(count)
    }

    pub fn packages(&self) -> Vec<PackageInfo> {
        let Ok(index) = self.index.read() else {
            return vec![];
        };
        index
            .packages()
            .iter()
            .map(|p| PackageInfo {
                name: p.name.clone(),
                version: p.version.clone(),
                title: p.title.clone(),
                fhir_version: p.fhir_version.clone(),
                profile_count: p.profile_count,
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.index.read().map(|i| i.is_empty()).unwrap_or(true)
    }

    /// Validate against the base definition of the resource's type plus any
    /// profile it declares. `None` when nothing applicable is installed.
    pub fn validate(&self, resource: &Value) -> Option<Vec<FhirValidationIssue>> {
        let index = self.index.read().ok()?;
        if index.is_empty() {
            return None;
        }
        validate::validate_declared(resource, &index)
    }
}

impl Default for ProfileRegistry {
    fn default() -> Self {
        Self::new()
    }
}
