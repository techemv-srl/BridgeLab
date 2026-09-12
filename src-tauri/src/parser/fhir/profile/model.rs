//! Simplified StructureDefinition model.
//!
//! A FHIR StructureDefinition carries far more than a validator needs: the
//! differential, mappings, publisher metadata, narrative. Installing a
//! package distils each definition into the shape below and stores that,
//! so loading a package at startup reads a few megabytes of the parts that
//! matter rather than tens of megabytes of the parts that do not.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One StructureDefinition, reduced to what validation uses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Canonical URL — how `meta.profile` and `type.profile` refer to it.
    pub url: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    /// "resource", "complex-type", "primitive-type" or "logical".
    pub kind: String,
    /// The resource or data type this constrains, e.g. "Patient".
    #[serde(rename = "type")]
    pub type_name: String,
    /// Canonical URL of the definition this one derives from.
    #[serde(default)]
    pub base: Option<String>,
    /// "specialization" for a base definition, "constraint" for a profile.
    #[serde(default)]
    pub derivation: String,
    pub elements: Vec<ProfileElement>,
}

impl Profile {
    /// True for a profile that constrains another rather than defining a
    /// resource outright — a US Core Patient, say.
    pub fn is_constraint(&self) -> bool {
        self.derivation == "constraint"
    }

    /// Elements whose path is a direct child of `parent`.
    pub fn children_of(&self, parent: &str) -> Vec<&ProfileElement> {
        let depth = parent.matches('.').count() + 1;
        self.elements
            .iter()
            .filter(|e| {
                e.path.starts_with(parent)
                    && e.path.len() > parent.len()
                    && e.path.as_bytes()[parent.len()] == b'.'
                    && e.path.matches('.').count() == depth
            })
            .collect()
    }
}

/// One element definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileElement {
    /// Dotted path, e.g. `Patient.name.given`. Choice elements keep their
    /// `[x]` suffix.
    pub path: String,
    #[serde(default)]
    pub min: u32,
    /// `None` means unbounded (`*`).
    #[serde(default)]
    pub max: Option<u32>,
    /// Allowed type codes. A choice element lists several.
    #[serde(default)]
    pub types: Vec<String>,
    /// Value the element is fixed to.
    #[serde(default)]
    pub fixed: Option<Value>,
    /// Pattern the element must match (a subset of its content).
    #[serde(default)]
    pub pattern: Option<Value>,
    /// Short description, used in messages.
    #[serde(default)]
    pub short: String,
    /// True when the element is a slice of another (`Patient.identifier:mrn`).
    #[serde(default)]
    pub slice_name: Option<String>,
}

impl ProfileElement {
    /// The element name as it appears in the JSON encoding, without the
    /// `[x]` suffix.
    pub fn base_name(&self) -> &str {
        let name = self.path.rsplit('.').next().unwrap_or(&self.path);
        name.strip_suffix("[x]").unwrap_or(name)
    }

    pub fn is_choice(&self) -> bool {
        self.path.ends_with("[x]")
    }

    /// The JSON key a choice element takes for `type_code`, e.g.
    /// `valueQuantity` for `value[x]` and `Quantity`.
    pub fn choice_key(&self, type_code: &str) -> String {
        let mut chars = type_code.chars();
        let head = chars.next().map(|c| c.to_ascii_uppercase());
        match head {
            Some(h) => format!("{}{}{}", self.base_name(), h, chars.as_str()),
            None => self.base_name().to_string(),
        }
    }

    pub fn cardinality(&self) -> String {
        match self.max {
            Some(max) => format!("{}..{}", self.min, max),
            None => format!("{}..*", self.min),
        }
    }
}

/// Everything loaded from one installed package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilePackage {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub fhir_version: String,
    pub profiles: Vec<Profile>,
}

/// Profiles from every installed package, indexed for lookup.
#[derive(Debug, Default)]
pub struct ProfileIndex {
    by_url: BTreeMap<String, Profile>,
    /// Base (non-constraint) definition per type name, e.g. "Patient".
    base_by_type: BTreeMap<String, String>,
    packages: Vec<PackageSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageSummary {
    pub name: String,
    pub version: String,
    pub title: String,
    pub fhir_version: String,
    pub profile_count: usize,
}

impl ProfileIndex {
    pub fn add_package(&mut self, package: ProfilePackage) {
        for profile in &package.profiles {
            // The base definition of a type is the one that specialises it;
            // constraints on top are found through meta.profile instead.
            if !profile.is_constraint() && !profile.type_name.is_empty() {
                self.base_by_type
                    .entry(profile.type_name.clone())
                    .or_insert_with(|| profile.url.clone());
            }
            self.by_url.insert(profile.url.clone(), profile.clone());
        }
        self.packages.push(PackageSummary {
            name: package.name,
            version: package.version,
            title: package.title,
            fhir_version: package.fhir_version,
            profile_count: package.profiles.len(),
        });
    }

    pub fn get(&self, url: &str) -> Option<&Profile> {
        // A versioned canonical (`…|4.0.1`) refers to the same definition.
        self.by_url
            .get(url)
            .or_else(|| self.by_url.get(url.split('|').next().unwrap_or(url)))
    }

    /// The base definition for a resource or data type name.
    pub fn base_for_type(&self, type_name: &str) -> Option<&Profile> {
        self.base_by_type
            .get(type_name)
            .and_then(|url| self.by_url.get(url))
    }

    pub fn packages(&self) -> &[PackageSummary] {
        &self.packages
    }

    pub fn is_empty(&self) -> bool {
        self.by_url.is_empty()
    }

    pub fn profile_count(&self) -> usize {
        self.by_url.len()
    }
}

/// Distil a StructureDefinition into a [`Profile`].
///
/// Returns `None` for definitions without a snapshot: the differential
/// alone cannot be validated against without computing the snapshot, which
/// needs the whole ancestry and is what the package publisher already did.
pub fn from_structure_definition(sd: &Value) -> Option<Profile> {
    let url = sd.get("url")?.as_str()?.to_string();
    let snapshot = sd.get("snapshot")?.get("element")?.as_array()?;

    let elements = snapshot
        .iter()
        .filter_map(element_from_json)
        .collect::<Vec<_>>();
    if elements.is_empty() {
        return None;
    }

    Some(Profile {
        url,
        name: string_at(sd, "name"),
        version: string_at(sd, "version"),
        kind: string_at(sd, "kind"),
        type_name: string_at(sd, "type"),
        base: sd
            .get("baseDefinition")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        derivation: string_at(sd, "derivation"),
        elements,
    })
}

fn element_from_json(e: &Value) -> Option<ProfileElement> {
    let path = e.get("path")?.as_str()?.to_string();

    let max = match e.get("max").and_then(|v| v.as_str()) {
        Some("*") => None,
        Some(text) => text.parse::<u32>().ok(),
        None => None,
    };

    let types = e
        .get("type")
        .and_then(|v| v.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|t| t.get("code").and_then(|c| c.as_str()))
                // FHIRPath system types appear as full URLs on primitive
                // `value` elements; the short name is what we compare.
                .map(|code| code.rsplit('/').next().unwrap_or(code).to_string())
                .collect()
        })
        .unwrap_or_default();

    Some(ProfileElement {
        path,
        min: e.get("min").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        max,
        types,
        fixed: prefixed_value(e, "fixed"),
        pattern: prefixed_value(e, "pattern"),
        short: string_at(e, "short"),
        slice_name: e
            .get("sliceName")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

/// `fixed[x]` / `pattern[x]` carry the type in the key name.
fn prefixed_value(e: &Value, prefix: &str) -> Option<Value> {
    e.as_object()?.iter().find_map(|(k, v)| {
        (k.len() > prefix.len()
            && k.starts_with(prefix)
            && k[prefix.len()..].starts_with(|c: char| c.is_ascii_uppercase()))
        .then(|| v.clone())
    })
}

fn string_at(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn patient_sd() -> Value {
        json!({
            "resourceType": "StructureDefinition",
            "url": "http://hl7.org/fhir/StructureDefinition/Patient",
            "name": "Patient",
            "version": "4.0.1",
            "kind": "resource",
            "type": "Patient",
            "derivation": "specialization",
            "snapshot": { "element": [
                {"path": "Patient", "min": 0, "max": "*"},
                {"path": "Patient.id", "min": 0, "max": "1",
                 "type": [{"code": "http://hl7.org/fhirpath/System.String"}]},
                {"path": "Patient.identifier", "min": 0, "max": "*",
                 "type": [{"code": "Identifier"}]},
                {"path": "Patient.gender", "min": 0, "max": "1",
                 "type": [{"code": "code"}]},
                {"path": "Patient.deceased[x]", "min": 0, "max": "1",
                 "type": [{"code": "boolean"}, {"code": "dateTime"}]},
                {"path": "Patient.name", "min": 0, "max": "*",
                 "type": [{"code": "HumanName"}]}
            ]}
        })
    }

    #[test]
    fn distils_a_structure_definition() {
        let p = from_structure_definition(&patient_sd()).expect("profile");
        assert_eq!(p.url, "http://hl7.org/fhir/StructureDefinition/Patient");
        assert_eq!(p.type_name, "Patient");
        assert!(!p.is_constraint());
        assert_eq!(p.elements.len(), 6);
    }

    #[test]
    fn reads_cardinality_including_unbounded() {
        let p = from_structure_definition(&patient_sd()).unwrap();
        let identifier = p.elements.iter().find(|e| e.path == "Patient.identifier").unwrap();
        assert_eq!(identifier.min, 0);
        assert_eq!(identifier.max, None);
        assert_eq!(identifier.cardinality(), "0..*");

        let gender = p.elements.iter().find(|e| e.path == "Patient.gender").unwrap();
        assert_eq!(gender.max, Some(1));
        assert_eq!(gender.cardinality(), "0..1");
    }

    #[test]
    fn shortens_fhirpath_system_type_urls() {
        let p = from_structure_definition(&patient_sd()).unwrap();
        let id = p.elements.iter().find(|e| e.path == "Patient.id").unwrap();
        assert_eq!(id.types, vec!["System.String"]);
    }

    #[test]
    fn choice_elements_keep_every_type_and_build_their_json_keys() {
        let p = from_structure_definition(&patient_sd()).unwrap();
        let deceased = p.elements.iter().find(|e| e.is_choice()).unwrap();
        assert_eq!(deceased.base_name(), "deceased");
        assert_eq!(deceased.types, vec!["boolean", "dateTime"]);
        assert_eq!(deceased.choice_key("boolean"), "deceasedBoolean");
        assert_eq!(deceased.choice_key("dateTime"), "deceasedDateTime");
    }

    #[test]
    fn children_of_returns_only_direct_children() {
        let sd = json!({
            "url": "u", "name": "n", "kind": "resource", "type": "Patient",
            "snapshot": { "element": [
                {"path": "Patient"},
                {"path": "Patient.name", "type": [{"code": "HumanName"}]},
                {"path": "Patient.name.given", "type": [{"code": "string"}]},
                {"path": "Patient.name.family", "type": [{"code": "string"}]},
                {"path": "Patient.gender", "type": [{"code": "code"}]}
            ]}
        });
        let p = from_structure_definition(&sd).unwrap();
        let top: Vec<&str> = p.children_of("Patient").iter().map(|e| e.path.as_str()).collect();
        assert_eq!(top, vec!["Patient.name", "Patient.gender"]);
        let nested: Vec<&str> = p
            .children_of("Patient.name")
            .iter()
            .map(|e| e.path.as_str())
            .collect();
        assert_eq!(nested, vec!["Patient.name.given", "Patient.name.family"]);
    }

    #[test]
    fn reads_fixed_and_pattern_values() {
        let sd = json!({
            "url": "u", "name": "n", "kind": "resource", "type": "Observation",
            "snapshot": { "element": [
                {"path": "Observation"},
                {"path": "Observation.status", "fixedCode": "final"},
                {"path": "Observation.code", "patternCodeableConcept": {"coding": [{"code": "1234-5"}]}}
            ]}
        });
        let p = from_structure_definition(&sd).unwrap();
        assert_eq!(p.elements[1].fixed, Some(json!("final")));
        assert!(p.elements[2].pattern.is_some());
    }

    #[test]
    fn a_definition_without_a_snapshot_is_skipped() {
        let sd = json!({
            "url": "u", "name": "n", "kind": "resource", "type": "Patient",
            "differential": { "element": [{"path": "Patient.name", "min": 1}] }
        });
        assert!(from_structure_definition(&sd).is_none());
    }

    #[test]
    fn the_index_finds_profiles_by_canonical_url_with_or_without_a_version() {
        let mut index = ProfileIndex::default();
        index.add_package(ProfilePackage {
            name: "hl7.fhir.r4.core".into(),
            version: "4.0.1".into(),
            title: String::new(),
            fhir_version: "4.0.1".into(),
            profiles: vec![from_structure_definition(&patient_sd()).unwrap()],
        });

        assert!(index
            .get("http://hl7.org/fhir/StructureDefinition/Patient")
            .is_some());
        assert!(index
            .get("http://hl7.org/fhir/StructureDefinition/Patient|4.0.1")
            .is_some());
        assert!(index.base_for_type("Patient").is_some());
        assert!(index.base_for_type("Observation").is_none());
        assert_eq!(index.packages().len(), 1);
        assert_eq!(index.profile_count(), 1);
    }
}
