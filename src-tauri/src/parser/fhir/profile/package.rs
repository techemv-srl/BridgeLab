//! FHIR NPM package handling.
//!
//! A FHIR package is a gzipped tar whose entries live under `package/`:
//! `package/package.json` for the metadata, and one JSON file per resource.
//! Only the StructureDefinitions matter here, and only the parts of those
//! that [`super::model`] keeps.
//!
//! Installing distils the package once and writes the result to
//! `<config>/BridgeLab/fhir-packages/<name>#<version>.json`. The core R4
//! package is ~50 MB unpacked and 4581 files; the distilled index is a
//! couple of megabytes in a single file, which is what makes loading it at
//! startup reasonable.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::model::{from_structure_definition, Profile, ProfilePackage};

/// Where installed packages live.
pub fn packages_root() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("BridgeLab").join("fhir-packages"))
}

fn ensure_root() -> Result<PathBuf, String> {
    let root = packages_root().ok_or("Could not determine the config directory")?;
    fs::create_dir_all(&root).map_err(|e| format!("Could not create {}: {}", root.display(), e))?;
    Ok(root)
}

/// Read a `.tgz` and distil the StructureDefinitions it contains.
pub fn read_package(tgz: &Path) -> Result<ProfilePackage, String> {
    let file = fs::File::open(tgz).map_err(|e| format!("Could not open {}: {}", tgz.display(), e))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    let mut name = String::new();
    let mut version = String::new();
    let mut title = String::new();
    let mut fhir_version = String::new();
    let mut profiles: Vec<Profile> = Vec::new();

    let entries = archive
        .entries()
        .map_err(|e| format!("{} is not a readable tar archive: {}", tgz.display(), e))?;

    for entry in entries {
        let mut entry = entry.map_err(|e| format!("Corrupt archive entry: {}", e))?;
        let path = entry
            .path()
            .map_err(|e| format!("Corrupt archive entry path: {}", e))?
            .to_path_buf();

        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !file_name.ends_with(".json") {
            continue;
        }

        // Package payloads live under `package/`; ignore anything else the
        // archive happens to carry (examples/, .index.json, …).
        let in_package = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            == Some("package");
        if !in_package {
            continue;
        }
        if file_name == ".index.json" {
            continue;
        }

        let mut text = String::new();
        if entry.read_to_string(&mut text).is_err() {
            continue; // binary or non-UTF-8 payload: not a resource we read
        }
        let Ok(json) = serde_json::from_str::<Value>(&text) else {
            continue;
        };

        if file_name == "package.json" {
            name = string_at(&json, "name");
            version = string_at(&json, "version");
            title = string_at(&json, "title");
            fhir_version = json
                .get("fhirVersions")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            continue;
        }

        if json.get("resourceType").and_then(|v| v.as_str()) != Some("StructureDefinition") {
            continue;
        }
        if let Some(profile) = from_structure_definition(&json) {
            profiles.push(profile);
        }
    }

    if name.is_empty() {
        return Err(format!(
            "{} has no package/package.json — it does not look like a FHIR package",
            tgz.display()
        ));
    }
    if profiles.is_empty() {
        return Err(format!(
            "{} contains no StructureDefinition with a snapshot. Packages published \
             without snapshots cannot be validated against.",
            name
        ));
    }

    Ok(ProfilePackage {
        name,
        version,
        title,
        fhir_version,
        profiles,
    })
}

/// File name a distilled package is stored under.
fn stored_name(name: &str, version: &str) -> String {
    // '#' separates name and version the way FHIR tooling writes it, and is
    // legal on every platform we ship to.
    let safe = |s: &str| s.replace(['/', '\\', ':'], "_");
    format!("{}#{}.json", safe(name), safe(version))
}

/// Install a `.tgz` into the packages directory, replacing any copy of the
/// same name and version.
pub fn install(tgz: &Path) -> Result<ProfilePackage, String> {
    let package = read_package(tgz)?;
    let root = ensure_root()?;
    let target = root.join(stored_name(&package.name, &package.version));

    let text = serde_json::to_string(&package)
        .map_err(|e| format!("Could not serialise the package index: {}", e))?;
    fs::write(&target, text)
        .map_err(|e| format!("Could not write {}: {}", target.display(), e))?;

    Ok(package)
}

/// Remove an installed package.
pub fn remove(name: &str, version: &str) -> Result<(), String> {
    let root = ensure_root()?;
    let target = root.join(stored_name(name, version));
    if !target.exists() {
        return Err(format!("{} {} is not installed", name, version));
    }
    fs::remove_file(&target).map_err(|e| format!("Could not remove {}: {}", target.display(), e))
}

/// Every installed package, as distilled on disk.
///
/// A file that fails to parse is skipped rather than failing the load: one
/// bad package should not take the others with it.
pub fn load_installed() -> Vec<ProfilePackage> {
    let Some(root) = packages_root() else {
        return vec![];
    };
    let Ok(entries) = fs::read_dir(&root) else {
        return vec![];
    };

    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(text) = fs::read_to_string(&path) {
            if let Ok(pkg) = serde_json::from_str::<ProfilePackage>(&text) {
                out.push(pkg);
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
    out
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
    use std::io::Write;

    /// Build a minimal FHIR package archive in memory.
    fn make_tgz(dir: &Path, files: &[(&str, Value)]) -> PathBuf {
        let path = dir.join("test-package.tgz");
        let file = fs::File::create(&path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        {
            let mut builder = tar::Builder::new(encoder);
            for (name, content) in files {
                let text = serde_json::to_vec(content).unwrap();
                let mut header = tar::Header::new_gnu();
                header.set_size(text.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                builder
                    .append_data(&mut header, format!("package/{}", name), text.as_slice())
                    .unwrap();
            }
            let mut encoder = builder.into_inner().unwrap();
            encoder.flush().unwrap();
            encoder.finish().unwrap();
        }
        path
    }

    fn sd(url: &str, type_name: &str) -> Value {
        json!({
            "resourceType": "StructureDefinition",
            "url": url,
            "name": type_name,
            "kind": "resource",
            "type": type_name,
            "derivation": "specialization",
            "snapshot": { "element": [
                {"path": type_name, "min": 0, "max": "*"},
                {"path": format!("{}.id", type_name), "min": 0, "max": "1"}
            ]}
        })
    }

    #[test]
    fn reads_a_package_and_keeps_only_structure_definitions() {
        let tmp = std::env::temp_dir().join(format!("bl-pkg-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).unwrap();

        let tgz = make_tgz(
            &tmp,
            &[
                ("package.json", json!({
                    "name": "acme.fhir.profiles",
                    "version": "1.2.3",
                    "title": "ACME profiles",
                    "fhirVersions": ["4.0.1"]
                })),
                ("StructureDefinition-Patient.json",
                 sd("http://acme.org/StructureDefinition/Patient", "Patient")),
                // Not a StructureDefinition: must be ignored.
                ("ValueSet-genders.json",
                 json!({"resourceType": "ValueSet", "url": "http://acme.org/vs"})),
                // A StructureDefinition without a snapshot: also ignored.
                ("StructureDefinition-NoSnapshot.json", json!({
                    "resourceType": "StructureDefinition",
                    "url": "http://acme.org/StructureDefinition/NoSnapshot",
                    "name": "NoSnapshot", "kind": "resource", "type": "NoSnapshot",
                    "differential": {"element": [{"path": "NoSnapshot"}]}
                })),
            ],
        );

        let pkg = read_package(&tgz).expect("package should read");
        assert_eq!(pkg.name, "acme.fhir.profiles");
        assert_eq!(pkg.version, "1.2.3");
        assert_eq!(pkg.title, "ACME profiles");
        assert_eq!(pkg.fhir_version, "4.0.1");
        assert_eq!(pkg.profiles.len(), 1, "only the Patient definition");
        assert_eq!(pkg.profiles[0].type_name, "Patient");

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn rejects_an_archive_without_package_metadata() {
        let tmp = std::env::temp_dir().join(format!("bl-pkg-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).unwrap();
        let tgz = make_tgz(
            &tmp,
            &[("StructureDefinition-Patient.json",
               sd("http://acme.org/StructureDefinition/Patient", "Patient"))],
        );
        let err = read_package(&tgz).unwrap_err();
        assert!(err.contains("package.json"), "{}", err);
        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn rejects_a_package_with_no_usable_definitions() {
        let tmp = std::env::temp_dir().join(format!("bl-pkg-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).unwrap();
        let tgz = make_tgz(
            &tmp,
            &[("package.json", json!({"name": "empty.pkg", "version": "1.0"}))],
        );
        let err = read_package(&tgz).unwrap_err();
        assert!(err.contains("no StructureDefinition"), "{}", err);
        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn stored_names_are_filesystem_safe() {
        assert_eq!(stored_name("hl7.fhir.r4.core", "4.0.1"), "hl7.fhir.r4.core#4.0.1.json");
        assert_eq!(stored_name("a/b", "1:0"), "a_b#1_0.json");
    }
}
