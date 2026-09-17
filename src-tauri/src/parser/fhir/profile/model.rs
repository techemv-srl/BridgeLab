//! Simplified StructureDefinition model.
//!
//! A FHIR StructureDefinition carries far more than a validator needs: the
//! differential, mappings, publisher metadata, narrative. Installing a
//! package distils each definition into the shape below and stores that,
//! so loading a package at startup reads a few megabytes of the parts that
//! matter rather than tens of megabytes of the parts that do not.

use std::cmp::Ordering;
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
    /// Every installed version of a canonical, newest first, so an
    /// unversioned reference gets the newest and a pinned one (`url|1.2.0`)
    /// can be answered exactly.
    by_url: BTreeMap<String, Vec<Profile>>,
    /// Base (non-constraint) definition per type name, e.g. "Patient".
    base_by_type: BTreeMap<String, String>,
    packages: Vec<PackageSummary>,
}

/// What a canonical reference resolved to.
#[derive(Debug)]
pub enum Lookup<'a> {
    Found(&'a Profile),
    /// The canonical is installed, just not at the version it was pinned to.
    WrongVersion { installed: Vec<String> },
    Missing,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageSummary {
    pub name: String,
    pub version: String,
    pub title: String,
    pub fhir_version: String,
    pub profile_count: usize,
    /// Shipped inside the binary rather than installed by the user: always
    /// present, never removable.
    pub builtin: bool,
}

impl ProfileIndex {
    /// A package the user installed.
    pub fn add_package(&mut self, package: ProfilePackage) {
        self.add(package, false);
    }

    /// The package the binary carries. Added first, so a user-installed
    /// copy of the same version replaces its definitions and a newer one
    /// outranks them — the built-in copy is a floor, not a ceiling.
    pub fn add_builtin(&mut self, package: ProfilePackage) {
        self.add(package, true);
    }

    fn add(&mut self, package: ProfilePackage, builtin: bool) {
        let ProfilePackage {
            name,
            version,
            title,
            fhir_version,
            profiles,
        } = package;
        let profile_count = profiles.len();

        for profile in profiles {
            // The base definition of a type is the one that specialises it;
            // constraints on top are found through meta.profile instead.
            if !profile.is_constraint() && !profile.type_name.is_empty() {
                self.base_by_type
                    .entry(profile.type_name.clone())
                    .or_insert_with(|| profile.url.clone());
            }
            let versions = self.by_url.entry(profile.url.clone()).or_default();
            // The same version shipped by two packages is one definition;
            // the later install replaces the earlier copy rather than
            // sitting beside it.
            versions.retain(|p| p.version != profile.version);
            versions.push(profile);
            versions.sort_by(|a, b| compare_versions(&b.version, &a.version));
        }
        self.packages.push(PackageSummary {
            name,
            version,
            title,
            fhir_version,
            profile_count,
            builtin,
        });
    }

    /// Resolve a canonical reference, honouring a `|version` pin.
    ///
    /// An unversioned reference gets the newest installed version. A pinned
    /// one gets exactly that version or [`Lookup::WrongVersion`] — never a
    /// silent substitute, which would answer a different question than the
    /// one the resource asked.
    pub fn lookup(&self, canonical: &str) -> Lookup<'_> {
        let (url, pinned) = match canonical.split_once('|') {
            Some((url, version)) => (url, Some(version)),
            None => (canonical, None),
        };
        let Some(versions) = self.by_url.get(url) else {
            return Lookup::Missing;
        };
        match pinned {
            None => versions.first().map(Lookup::Found).unwrap_or(Lookup::Missing),
            Some(wanted) => versions
                .iter()
                .find(|p| p.version == wanted)
                .map(Lookup::Found)
                .unwrap_or_else(|| Lookup::WrongVersion {
                    installed: versions.iter().map(|p| p.version.clone()).collect(),
                }),
        }
    }

    /// [`lookup`](Self::lookup) as an `Option`, for callers that only need
    /// the profile when it resolves exactly.
    pub fn get(&self, canonical: &str) -> Option<&Profile> {
        match self.lookup(canonical) {
            Lookup::Found(profile) => Some(profile),
            _ => None,
        }
    }

    /// The base definition for a resource or data type name — the newest
    /// installed version of it.
    pub fn base_for_type(&self, type_name: &str) -> Option<&Profile> {
        self.base_by_type
            .get(type_name)
            .and_then(|url| self.by_url.get(url))
            .and_then(|versions| versions.first())
    }

    pub fn packages(&self) -> &[PackageSummary] {
        &self.packages
    }

    pub fn is_empty(&self) -> bool {
        self.by_url.is_empty()
    }

    /// Installed profiles, counting each version of a canonical.
    pub fn profile_count(&self) -> usize {
        self.by_url.values().map(Vec::len).sum()
    }
}

/// Order two version strings, newest last: numeric segments compare as
/// numbers ("4.10.0" after "4.9.0"), other segments as text, and a release
/// ranks above its own pre-release ("4.0.1" after "4.0.1-snapshot").
fn compare_versions(a: &str, b: &str) -> Ordering {
    let split = |s: &str| {
        s.split(['.', '-'])
            .map(|seg| match seg.parse::<u64>() {
                Ok(n) => (Some(n), seg.to_string()),
                Err(_) => (None, seg.to_string()),
            })
            .collect::<Vec<_>>()
    };
    let (a, b) = (split(a), split(b));
    for i in 0..a.len().max(b.len()) {
        match (a.get(i), b.get(i)) {
            (Some(x), Some(y)) => {
                let ord = match (&x.0, &y.0) {
                    (Some(m), Some(n)) => m.cmp(n),
                    _ => x.1.cmp(&y.1),
                };
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            // "4.0.1" vs "4.0.1-snapshot": the extra segment is a
            // pre-release tag, so the shorter one is newer. "4.0" vs
            // "4.0.1": the extra segment is numeric, so the longer is.
            (None, Some(extra)) => {
                return if extra.0.is_some() { Ordering::Less } else { Ordering::Greater };
            }
            (Some(extra), None) => {
                return if extra.0.is_some() { Ordering::Greater } else { Ordering::Less };
            }
            (None, None) => break,
        }
    }
    Ordering::Equal
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

    fn package(name: &str, version: &str, sds: Vec<Value>) -> ProfilePackage {
        ProfilePackage {
            name: name.into(),
            version: version.into(),
            title: String::new(),
            fhir_version: "4.0.1".into(),
            profiles: sds.iter().filter_map(from_structure_definition).collect(),
        }
    }

    #[test]
    fn a_pinned_canonical_gets_exactly_that_version_and_an_unpinned_one_the_newest() {
        let mut old = patient_sd();
        old["version"] = json!("4.0.1");
        let mut new = patient_sd();
        new["version"] = json!("4.3.0");
        let mut index = ProfileIndex::default();
        // Installed older-first on purpose: order of installation must not
        // decide which version answers.
        index.add_package(package("hl7.fhir.r4.core", "4.0.1", vec![old]));
        index.add_package(package("hl7.fhir.r4b.core", "4.3.0", vec![new]));

        let url = "http://hl7.org/fhir/StructureDefinition/Patient";
        assert_eq!(index.get(url).unwrap().version, "4.3.0");
        assert_eq!(index.get(&format!("{url}|4.0.1")).unwrap().version, "4.0.1");
        assert_eq!(index.get(&format!("{url}|4.3.0")).unwrap().version, "4.3.0");
        assert_eq!(index.base_for_type("Patient").unwrap().version, "4.3.0");
        assert_eq!(index.profile_count(), 2);

        match index.lookup(&format!("{url}|5.0.0")) {
            Lookup::WrongVersion { installed } => {
                assert_eq!(installed, vec!["4.3.0".to_string(), "4.0.1".to_string()]);
            }
            other => panic!("expected WrongVersion, got {other:?}"),
        }
        assert!(matches!(
            index.lookup("http://acme.org/StructureDefinition/Nope|1.0"),
            Lookup::Missing
        ));
    }

    #[test]
    fn the_same_version_installed_twice_is_one_definition() {
        let mut index = ProfileIndex::default();
        index.add_package(package("a", "1.0", vec![patient_sd()]));
        index.add_package(package("b", "1.0", vec![patient_sd()]));
        assert_eq!(index.profile_count(), 1);
        assert_eq!(index.packages().len(), 2);
    }

    #[test]
    fn versions_order_numerically_with_pre_releases_below_their_release() {
        let mut versions = vec!["4.9.0", "4.10.0", "4.0.1-snapshot", "4.0.1", "4.0"];
        versions.sort_by(|a, b| compare_versions(b, a));
        assert_eq!(versions, vec!["4.10.0", "4.9.0", "4.0.1", "4.0.1-snapshot", "4.0"]);
    }

    #[test]
    fn a_built_in_package_is_a_floor_the_user_can_replace_or_outrank() {
        let mut index = ProfileIndex::default();
        index.add_builtin(package("hl7.fhir.r4.core", "4.0.1", vec![patient_sd()]));
        assert!(index.packages()[0].builtin);

        // Installing the same version again swaps the definitions in place.
        index.add_package(package("hl7.fhir.r4.core", "4.0.1", vec![patient_sd()]));
        assert_eq!(index.profile_count(), 1);
        assert_eq!(index.packages().len(), 2);
        assert!(!index.packages()[1].builtin);

        // A newer one outranks it for unpinned lookups.
        let mut newer = patient_sd();
        newer["version"] = json!("4.3.0");
        index.add_package(package("hl7.fhir.r4b.core", "4.3.0", vec![newer]));
        assert_eq!(index.base_for_type("Patient").unwrap().version, "4.3.0");
        assert_eq!(index.profile_count(), 2);
    }
}
