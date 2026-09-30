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
/// Largest single file read from a package, and the most read in total.
const MAX_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;
/// Most JSON values (objects, arrays and list items) a StructureDefinition
/// may hold before it is parsed into a tree. Parsed, each small value costs
/// tens of bytes, so a few MB of `[0,0,…]` or `[{},{},…]` would take
/// gigabytes; the largest core definition has about 12 000.
const MAX_JSON_ITEMS: usize = 1_000_000;
/// Largest `fixed[x]` / `pattern[x]` value kept from a definition. Kept
/// values stay in memory for as long as the package is installed; real
/// ones are a code or a small CodeableConcept.
const MAX_FIXED_BYTES: usize = 64 * 1024;

/// Just the `resourceType` of a JSON resource. Every other field is
/// skipped without being built, so a file that is not a
/// StructureDefinition costs no memory beyond its text.
#[derive(serde::Deserialize)]
struct Head {
    #[serde(rename = "resourceType")]
    resource_type: Option<String>,
}

/// An upper bound on the values a JSON text holds (commas and brackets,
/// including any inside strings).
fn json_items(text: &str) -> usize {
    text.bytes().filter(|b| matches!(b, b'{' | b'[' | b',')).count()
}

pub fn read_package(tgz: &Path) -> Result<ProfilePackage, String> {
    let file = fs::File::open(tgz).map_err(|e| format!("Could not open {}: {}", tgz.display(), e))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    let mut name = String::new();
    let mut total_read: u64 = 0;
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

        // Checked for every entry, read or skipped: moving past an entry
        // still decompresses it. A resource file is at most a few MB; a
        // larger entry (a decompression bomb, 3 GB of spaces in a 3 MB
        // archive) or too much in total refuses the package.
        if entry.size() > MAX_ENTRY_BYTES {
            return Err(format!(
                "The package holds a file of {} MB ({}); files over {} MB are not accepted",
                entry.size() >> 20,
                path.display(),
                MAX_ENTRY_BYTES >> 20
            ));
        }
        total_read += entry.size();
        if total_read > MAX_TOTAL_BYTES {
            return Err(format!(
                "The package unpacks to more than {} MB of resources; not installed",
                MAX_TOTAL_BYTES >> 20
            ));
        }

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
        if std::io::Read::take(&mut entry, MAX_ENTRY_BYTES).read_to_string(&mut text).is_err() {
            continue; // binary or non-UTF-8 payload: not a resource we read
        }
        // Only package.json and StructureDefinitions are read; everything
        // else is recognised without building it.
        if file_name != "package.json" {
            match serde_json::from_str::<Head>(&text) {
                Ok(Head { resource_type: Some(rt) }) if rt == "StructureDefinition" => {}
                _ => continue,
            }
            if json_items(&text) > MAX_JSON_ITEMS {
                return Err(format!(
                    "{} holds more than {} JSON values, far more than a StructureDefinition needs; not installed",
                    path.display(),
                    MAX_JSON_ITEMS
                ));
            }
        } else if json_items(&text) > MAX_JSON_ITEMS {
            return Err("package.json is far larger than package metadata; not installed".into());
        }
        let Ok(json) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        drop(text);

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
            let oversized = profile.elements.iter().find(|e| {
                [&e.fixed, &e.pattern].into_iter().flatten().any(|v| {
                    serde_json::to_string(v).map_or(true, |t| t.len() > MAX_FIXED_BYTES)
                })
            });
            if let Some(e) = oversized {
                return Err(format!(
                    "{} fixes {} to a value over {} KB; not installed",
                    profile.url,
                    e.path,
                    MAX_FIXED_BYTES >> 10
                ));
            }
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
/// The distilled `hl7.fhir.r4.core` the binary carries, so base R4
/// conformance needs no download and no installation. Produced by
/// `cargo run --example distil-fhir-package`; ~200 KB compressed, the same
/// index an installation of the `.tgz` would write.
const BUILTIN_CORE: &[u8] = include_bytes!("../../../../resources/fhir/hl7.fhir.r4.core-4.0.1.json.gz");

/// The built-in core package, decoded. `None` only if the embedded bytes
/// fail to decode, which a build with a corrupt resource would show at
/// once in the tests.
pub fn builtin() -> Option<ProfilePackage> {
    let decoder = flate2::read::GzDecoder::new(BUILTIN_CORE);
    serde_json::from_reader(decoder).ok()
}

pub fn load_installed() -> Vec<ProfilePackage> {
    match packages_root() {
        Some(root) => load_installed_from(&root),
        None => vec![],
    }
}

/// The distilled packages under `root` — the app's config directory by
/// default, any directory for a CI checkout or an air-gapped provisioning
/// folder.
pub fn load_installed_from(root: &Path) -> Vec<ProfilePackage> {
    scan_installed_from(root).map(|scan| scan.packages).unwrap_or_default()
}

/// What reading a packages directory found: the packages, and every
/// `.json` file that could not be read as one (path, reason).
pub struct InstalledScan {
    pub packages: Vec<ProfilePackage>,
    pub skipped: Vec<(PathBuf, String)>,
}

/// As [`load_installed_from`], but reports what went wrong instead of
/// silently returning less: `Err` when the directory cannot be read at
/// all, and each file that is not a distilled package in `skipped`. The
/// CLI uses it so a mistyped `--fhir-packages` never looks like a clean
/// run against the core alone.
pub fn scan_installed_from(root: &Path) -> Result<InstalledScan, String> {
    let entries = fs::read_dir(root).map_err(|e| format!("{}: {}", root.display(), e))?;

    let mut packages = Vec::new();
    let mut skipped = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        match fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<ProfilePackage>(&text) {
                Ok(pkg) => packages.push(pkg),
                Err(e) => skipped.push((path, format!("not a distilled FHIR package: {}", e))),
            },
            Err(e) => skipped.push((path, e.to_string())),
        }
    }
    packages.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
    Ok(InstalledScan { packages, skipped })
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
    fn value_bombs_are_refused_or_skipped_without_being_built() {
        let tmp = std::env::temp_dir().join(format!("bl-pkg-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).unwrap();
        let meta = ("package.json", json!({"name": "bomb", "version": "1.0"}));
        let good = ("StructureDefinition-P.json", sd("http://acme.org/StructureDefinition/P", "Patient"));

        // Not a StructureDefinition: skipped however many values it holds.
        let array = ("StructureDefinition-bomb.json", Value::Array(vec![json!(0); MAX_JSON_ITEMS + 10]));
        let tgz = make_tgz(&tmp, &[meta.clone(), good.clone(), array]);
        assert_eq!(read_package(&tgz).unwrap().profiles.len(), 1);

        // A StructureDefinition that would expand to a huge tree.
        let mut big = sd("http://acme.org/StructureDefinition/Big", "Big");
        big["extra"] = Value::Array(vec![json!({}); MAX_JSON_ITEMS + 10]);
        let tgz = make_tgz(&tmp, &[meta.clone(), ("StructureDefinition-Big.json", big)]);
        assert!(read_package(&tgz).unwrap_err().contains("JSON values"));

        // A pattern value that would stay in memory for good.
        let mut fixed = sd("http://acme.org/StructureDefinition/F", "Patient");
        fixed["snapshot"]["element"][1]["patternString"] = json!("x".repeat(MAX_FIXED_BYTES + 1));
        let tgz = make_tgz(&tmp, &[meta, ("StructureDefinition-F.json", fixed)]);
        assert!(read_package(&tgz).unwrap_err().contains("KB"));
        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn stored_names_are_filesystem_safe() {
        assert_eq!(stored_name("hl7.fhir.r4.core", "4.0.1"), "hl7.fhir.r4.core#4.0.1.json");
        assert_eq!(stored_name("a/b", "1:0"), "a_b#1_0.json");
    }

    #[test]
    fn the_built_in_core_decodes_and_carries_the_base_definitions() {
        let core = builtin().expect("embedded core decodes");
        assert_eq!(core.name, "hl7.fhir.r4.core");
        assert_eq!(core.version, "4.0.1");
        assert_eq!(core.fhir_version, "4.0.1");
        assert!(core.profiles.len() > 500, "{} profiles", core.profiles.len());
        let mut index = super::super::model::ProfileIndex::default();
        index.add_builtin(core);
        for t in ["Patient", "Observation", "Bundle", "MessageHeader", "Quantity", "Reference"] {
            assert!(index.base_for_type(t).is_some(), "{t} missing from the built-in core");
        }
        assert!(index.packages()[0].builtin);
    }

    #[test]
    fn an_oversized_entry_refuses_the_package_even_outside_package_dir() {
        let tmp = std::env::temp_dir().join(format!("bl-pkg-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("bomb.tgz");
        let encoder = flate2::write::GzEncoder::new(fs::File::create(&path).unwrap(), flate2::Compression::fast());
        let mut builder = tar::Builder::new(encoder);
        let size = MAX_ENTRY_BYTES + 1;
        let mut header = tar::Header::new_gnu();
        header.set_size(size);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "examples/huge.txt", std::io::Read::take(std::io::repeat(b' '), size))
            .unwrap();
        builder.into_inner().unwrap().finish().unwrap();
        let err = read_package(&path).unwrap_err();
        assert!(err.contains("not accepted"), "{err}");
        let _ = fs::remove_dir_all(&tmp);
    }
}

