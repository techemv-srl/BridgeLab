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

use std::cell::Cell;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::rc::Rc;

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
/// Largest GNU long-name or PAX header entry read. These describe the entry
/// after them (a path, a size) and are read whole; real ones hold a path
/// of a few hundred bytes. tar itself reads one whole before any check
/// here sees it, so a 3 MB package declaring a 3 GB long name took 6 GB.
const MAX_EXTENSION_BYTES: u64 = 64 * 1024;
/// Most bytes decompressed from the package in all: what its entries may
/// hold, and room for their headers and padding. Anything tar reads past
/// the checks here (headers, entries skipped) stays under it too.
const MAX_STREAM_BYTES: u64 = MAX_TOTAL_BYTES + 64 * 1024 * 1024;
/// Largest package file read. Real ones are a few MB (the R4 core unpacks
/// to about 50 MB); this is room for what MAX_TOTAL_BYTES of JSON
/// compresses to.
const MAX_PACKAGE_FILE_BYTES: u64 = 256 * 1024 * 1024;

/// A reader that gives at most `left` bytes and then fails, setting `over`
/// so the caller can say why rather than report a corrupt archive.
struct Capped<R> {
    inner: R,
    left: u64,
    over: Rc<Cell<bool>>,
}

impl<R: Read> Read for Capped<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.left == 0 {
            // At the cap: the stream may end exactly here, which is fine.
            let mut one = [0u8; 1];
            if self.inner.read(&mut one)? == 0 {
                return Ok(0);
            }
            self.over.set(true);
            return Err(std::io::Error::other("the package unpacks to more than its limit"));
        }
        let room = buf.len().min(usize::try_from(self.left).unwrap_or(usize::MAX));
        let n = self.inner.read(&mut buf[..room])?;
        self.left -= n as u64;
        Ok(n)
    }
}

/// Count an entry towards the most a package may hold in all.
fn add_to_total(total: &mut u64, size: u64) -> Result<(), String> {
    *total = total.saturating_add(size);
    if *total > MAX_TOTAL_BYTES {
        return Err(format!(
            "The package unpacks to more than {} MB of resources; not installed",
            MAX_TOTAL_BYTES >> 20
        ));
    }
    Ok(())
}

/// `3072 MB` or `100000 bytes`, for the errors.
fn size_in_words(bytes: u64) -> String {
    if bytes >= 1 << 20 {
        format!("{} MB", bytes >> 20)
    } else {
        format!("{} bytes", bytes)
    }
}

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
    use crate::utils::bounded_read::{read_regular_file, ReadRefusal};
    // Only a regular file: reading a named pipe waits for a writer that
    // never comes, and each install picked that way held a thread for good.
    let bytes = read_regular_file(tgz, MAX_PACKAGE_FILE_BYTES).map_err(|refusal| match refusal {
        ReadRefusal::NotRegular => format!(
            "{} is not a file (a folder, a device or a named pipe); pick the package's .tgz file",
            tgz.display()
        ),
        ReadRefusal::TooLarge { size } => format!(
            "{} is {} MB; a package file over {} MB is not accepted",
            tgz.display(),
            size >> 20,
            MAX_PACKAGE_FILE_BYTES >> 20
        ),
        ReadRefusal::Io(e) => format!("Could not open {}: {}", tgz.display(), e),
    })?;
    read_package_from(bytes.as_slice(), tgz, MAX_STREAM_BYTES)
}

/// [`read_package`] on the gzipped bytes of `compressed`, decompressing at
/// most `stream_limit` bytes. `tgz` names it in the errors.
fn read_package_from(compressed: impl Read, tgz: &Path, stream_limit: u64) -> Result<ProfilePackage, String> {
    let over = Rc::new(Cell::new(false));
    let decoder = Capped {
        inner: flate2::read::GzDecoder::new(compressed),
        left: stream_limit,
        over: over.clone(),
    };
    let mut archive = tar::Archive::new(decoder);
    let too_much = || format!("The package unpacks to more than {} MB; not installed", stream_limit >> 20);

    let mut name = String::new();
    let mut total_read: u64 = 0;
    let mut version = String::new();
    let mut title = String::new();
    let mut fhir_version = String::new();
    let mut profiles: Vec<Profile> = Vec::new();

    // Raw entries: tar does not read the long-name and PAX entries itself,
    // so each one's size is checked here before a byte of it is read.
    let entries = archive
        .entries()
        .map_err(|e| format!("{} is not a readable tar archive: {}", tgz.display(), e))?
        .raw(true);

    // What a long-name or PAX entry says about the entry that follows it.
    let mut long_name: Option<Vec<u8>> = None;
    let mut pax: Option<Vec<u8>> = None;

    for entry in entries {
        let mut entry = entry.map_err(|e| if over.get() { too_much() } else { format!("Corrupt archive entry: {}", e) })?;
        let size = entry.size();

        // A long name or PAX header is read whole, so its size is checked
        // before any of it is.
        let kind = entry.header().entry_type();
        let describes_next = if kind.is_gnu_longname() {
            Some((&mut long_name, "long file name"))
        } else if kind.is_pax_local_extensions() {
            Some((&mut pax, "PAX header"))
        } else {
            None
        };
        if describes_next.is_some() || kind.is_gnu_longlink() || kind.is_pax_global_extensions() {
            if size > MAX_EXTENSION_BYTES {
                return Err(format!(
                    "The package holds a {} entry of {}; one over {} KB is not accepted",
                    if kind.is_gnu_longname() || kind.is_gnu_longlink() { "long name" } else { "PAX header" },
                    size_in_words(size),
                    MAX_EXTENSION_BYTES >> 10
                ));
            }
            add_to_total(&mut total_read, size)?;
            let mut data = Vec::new();
            (&mut entry)
                .take(MAX_EXTENSION_BYTES)
                .read_to_end(&mut data)
                .map_err(|e| if over.get() { too_much() } else { format!("Corrupt archive entry: {}", e) })?;
            // Link names and global headers say nothing a package needs.
            if let Some((slot, what)) = describes_next {
                if slot.replace(data).is_some() {
                    return Err(format!("Corrupt archive: two {} entries describe the same file", what));
                }
            }
            continue;
        }

        // The path a long-name or PAX entry gave this one, else its own.
        let pax_values = pax.take();
        let pax_value = |key: &str| {
            let pax = pax_values.as_deref()?;
            tar::PaxExtensions::new(pax)
                .filter_map(Result::ok)
                .find(|ext| ext.key() == Ok(key))
                .map(|ext| ext.value_bytes().to_vec())
        };
        // tar lays the archive out by the header's size; a PAX size that
        // says otherwise would make this reading differ from other tools'.
        if let Some(pax_size) = pax_value("size") {
            if std::str::from_utf8(&pax_size).ok().and_then(|s| s.trim().parse::<u64>().ok()) != Some(size) {
                return Err("Corrupt archive: a PAX header gives a file another size than its own header".into());
            }
        }
        let mut path_bytes = match long_name.take().or_else(|| pax_value("path")) {
            Some(bytes) => bytes,
            None => entry.path_bytes().into_owned(),
        };
        while path_bytes.last() == Some(&0) {
            path_bytes.pop();
        }

        // Checked for every entry, read or skipped: moving past an entry
        // still decompresses it. A resource file is at most a few MB; a
        // larger entry (a decompression bomb, 3 GB of spaces in a 3 MB
        // archive) or too much in total refuses the package.
        if size > MAX_ENTRY_BYTES {
            return Err(format!(
                "The package holds a file of {} MB ({}); files over {} MB are not accepted",
                size >> 20,
                String::from_utf8_lossy(&path_bytes),
                MAX_ENTRY_BYTES >> 20
            ));
        }
        add_to_total(&mut total_read, size)?;

        // A name that is not UTF-8 is not one a package file has.
        let Ok(path) = String::from_utf8(path_bytes).map(PathBuf::from) else {
            continue;
        };

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
            if over.get() {
                return Err(too_much());
            }
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
    remove_from(&ensure_root()?, name, version)
}

fn remove_from(root: &Path, name: &str, version: &str) -> Result<(), String> {
    let target = root.join(stored_name(name, version));
    // Already gone (deleted by hand while the app was open): there is
    // nothing left to remove, and the reload that follows takes it off the
    // list. Refusing left it listed with a "not installed" error.
    if !target.exists() {
        return Ok(());
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

/// What reading a packages directory found: the packages, and every other
/// file in it, with the reason it was not read (path, reason).
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
        // Folders, and hidden files such as .DS_Store, are not packages
        // anyone put there.
        let hidden = entry.file_name().to_string_lossy().starts_with('.');
        if hidden || path.is_dir() {
            continue;
        }
        // Any case: `PROFILE.JSON` from a Windows share is a package too.
        let extension = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
        match extension.as_deref() {
            Some("json") => {}
            // Every other file is named, not passed over: a `.tgz` copied
            // here as it was downloaded looked like a package the run used.
            Some("tgz") => {
                skipped.push((
                    path,
                    "a FHIR package archive (.tgz), not a distilled package: install it in the app \
                     (Tools → FHIR profile packages…) and use the .json file it writes"
                        .into(),
                ));
                continue;
            }
            _ => {
                skipped.push((path, "not a distilled FHIR package (.json)".into()));
                continue;
            }
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

    /// A packages directory given to the CLI: a `.JSON` in capitals was
    /// passed over, and so was a `.tgz` copied there as downloaded, both
    /// without a word, as if the run had used them.
    #[test]
    fn a_packages_directory_reads_any_case_and_names_every_other_file() {
        let tmp = std::env::temp_dir().join(format!("bl-pkg-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(tmp.join("sub")).unwrap();
        let tgz = make_tgz(
            &tmp,
            &[
                ("package.json", json!({"name": "acme.fhir.profiles", "version": "1.0.0", "fhirVersions": ["4.0.1"]})),
                ("StructureDefinition-Patient.json", sd("http://acme.org/StructureDefinition/Patient", "Patient")),
            ],
        );
        let distilled = serde_json::to_string(&read_package(&tgz).unwrap()).unwrap();
        fs::write(tmp.join("ACME.JSON"), distilled).unwrap();
        fs::write(tmp.join("notes.txt"), "where the packages came from").unwrap();
        fs::write(tmp.join(".DS_Store"), [0u8; 8]).unwrap();

        let scan = scan_installed_from(&tmp).unwrap();
        assert_eq!(scan.packages.len(), 1, "ACME.JSON is read");
        let mut skipped: Vec<(String, String)> = scan
            .skipped
            .iter()
            .map(|(p, why)| (p.file_name().unwrap().to_string_lossy().into_owned(), why.clone()))
            .collect();
        skipped.sort();
        let names: Vec<&str> = skipped.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["notes.txt", "test-package.tgz"], "hidden files and folders are not named");
        assert!(skipped[1].1.contains("FHIR package archive (.tgz)"), "{}", skipped[1].1);
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

    /// Removing a package whose file was deleted by hand succeeds: there
    /// is nothing left to remove, and the reload after it unlists it.
    #[test]
    fn removing_a_package_already_gone_succeeds() {
        let root = std::env::temp_dir().join(format!("bl-pkg-rm-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join(stored_name("acme.ig", "1.0.0"));
        std::fs::write(&file, b"{}").unwrap();
        assert!(remove_from(&root, "acme.ig", "1.0.0").is_ok());
        assert!(!file.exists());
        assert!(remove_from(&root, "acme.ig", "1.0.0").is_ok(), "already gone");
        std::fs::remove_dir_all(&root).ok();
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

    /// The header of a GNU long-name (`L`) or PAX (`x`) entry declaring
    /// `size` bytes.
    fn extension_header(kind: u8, size: u64) -> Vec<u8> {
        let mut header = tar::Header::new_gnu();
        header.set_path(if kind == b'x' { "PaxHeaders/x" } else { "././@LongLink" }).unwrap();
        header.set_size(size);
        header.set_mode(0o644);
        header.set_entry_type(tar::EntryType::new(kind));
        header.set_cksum();
        header.as_bytes().to_vec()
    }

    /// Gives `head` and then zeros, as many as are asked for, counting what
    /// was taken. Past `limit` it fails, so a reader that went on into the
    /// declared size fails rather than filling the memory.
    struct Endless {
        head: std::io::Cursor<Vec<u8>>,
        taken: Rc<Cell<u64>>,
        limit: u64,
    }

    impl Read for Endless {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.taken.get() >= self.limit {
                return Err(std::io::Error::other("read on into the declared size"));
            }
            let n = match self.head.read(buf)? {
                0 => {
                    let n = buf.len().min((self.limit - self.taken.get()) as usize);
                    buf[..n].fill(0);
                    n
                }
                n => n,
            };
            self.taken.set(self.taken.get() + n as u64);
            Ok(n)
        }
    }

    /// A long-name or PAX entry declaring gigabytes is refused from its
    /// header, before tar reads any of it. tar reads such an entry whole
    /// before handing over the entry it describes: a 3 MB package of
    /// zeros declaring a 3 GB long name took 6 GB of memory, and a 512 MB
    /// PAX header installed after taking 750 MB.
    #[test]
    fn long_name_and_pax_entries_declaring_gigabytes_are_refused_unread() {
        for (kind, size, what) in [
            (b'L', 3u64 << 30, "long name entry of 3072 MB"),
            (b'x', 512 << 20, "PAX header entry of 512 MB"),
            (b'L', MAX_EXTENSION_BYTES + 1, "long name entry of 65537 bytes"),
        ] {
            let tar_stream = Endless {
                head: std::io::Cursor::new(extension_header(kind, size)),
                taken: Rc::new(Cell::new(0)),
                limit: 256 << 20,
            };
            let gz = flate2::read::GzEncoder::new(tar_stream, flate2::Compression::fast());
            let started = std::time::Instant::now();
            // With 1 MB to decompress at most, reading the entry would stop
            // with another error: this one comes from its header alone.
            let err = read_package_from(gz, Path::new("bomb.tgz"), 1 << 20).unwrap_err();
            assert!(err.contains(what) && err.contains("over 64 KB is not accepted"), "{err}");
            assert!(started.elapsed() < std::time::Duration::from_secs(5), "{:?}", started.elapsed());
        }
    }

    /// The same from a file: a small package whose long name declares 3 GB.
    #[test]
    fn a_package_file_with_a_huge_long_name_is_refused_at_once() {
        let tmp = std::env::temp_dir().join(format!("bl-pkg-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("L3g.tgz");
        let mut gz = flate2::write::GzEncoder::new(fs::File::create(&path).unwrap(), flate2::Compression::fast());
        std::io::Write::write_all(&mut gz, &extension_header(b'L', 3 << 30)).unwrap();
        std::io::Write::write_all(&mut gz, &vec![0u8; 1 << 20]).unwrap();
        gz.finish().unwrap();
        let err = read_package(&path).unwrap_err();
        assert!(err.contains("long name entry of 3072 MB"), "{err}");
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Long names written either way still name the files they describe:
    /// GNU tar's `L` entries, and the PAX `path` that npm writes.
    #[test]
    fn long_file_names_still_reach_their_files() {
        let tmp = std::env::temp_dir().join(format!("bl-pkg-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("long.tgz");
        let gz = flate2::write::GzEncoder::new(fs::File::create(&path).unwrap(), flate2::Compression::fast());
        let mut builder = tar::Builder::new(gz);
        let add = |builder: &mut tar::Builder<_>, name: &str, content: &Value| {
            let text = serde_json::to_vec(content).unwrap();
            let mut header = tar::Header::new_gnu();
            header.set_size(text.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            // Over 100 bytes: the builder writes a GNU long-name entry.
            builder.append_data(&mut header, name, text.as_slice()).unwrap();
        };
        add(&mut builder, "package/package.json", &json!({"name": "long.names", "version": "1.0"}));
        let gnu_name = format!("package/StructureDefinition-{}.json", "g".repeat(120));
        add(&mut builder, &gnu_name, &sd("http://acme.org/StructureDefinition/G", "Patient"));

        // A PAX header giving the next entry its path, as npm writes one.
        let pax_name = format!("package/StructureDefinition-{}.json", "p".repeat(120));
        let record = |key: &str, value: &str| {
            let body = format!(" {key}={value}\n");
            let mut len = body.len() + 1;
            while format!("{len}{body}").len() != len {
                len += 1;
            }
            format!("{len}{body}")
        };
        let text = serde_json::to_vec(&sd("http://acme.org/StructureDefinition/P", "Observation")).unwrap();
        let pax_body = format!("{}{}", record("path", &pax_name), record("size", &text.len().to_string()));
        let mut pax = tar::Header::new_ustar();
        pax.set_path("PaxHeaders/p.json").unwrap();
        pax.set_size(pax_body.len() as u64);
        pax.set_entry_type(tar::EntryType::XHeader);
        pax.set_mode(0o644);
        pax.set_cksum();
        builder.append(&pax, pax_body.as_bytes()).unwrap();
        let mut header = tar::Header::new_ustar();
        header.set_path("package/truncated-name").unwrap();
        header.set_size(text.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append(&header, text.as_slice()).unwrap();
        builder.into_inner().unwrap().finish().unwrap();

        let pkg = read_package(&path).unwrap();
        let mut urls: Vec<&str> = pkg.profiles.iter().map(|p| p.url.as_str()).collect();
        urls.sort();
        assert_eq!(urls, ["http://acme.org/StructureDefinition/G", "http://acme.org/StructureDefinition/P"]);
        let _ = fs::remove_dir_all(&tmp);
    }

    /// No more than the limit is decompressed in all, whatever tar reads
    /// past the checks on each entry.
    #[test]
    fn decompression_stops_at_the_stream_limit() {
        let tmp = std::env::temp_dir().join(format!("bl-pkg-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).unwrap();
        let tgz = make_tgz(
            &tmp,
            &[
                ("package.json", json!({"name": "capped", "version": "1.0"})),
                ("StructureDefinition-P.json", sd("http://acme.org/StructureDefinition/P", "Patient")),
            ],
        );
        let open = || fs::File::open(&tgz).unwrap();
        assert_eq!(read_package_from(open(), &tgz, MAX_STREAM_BYTES).unwrap().profiles.len(), 1);
        let err = read_package_from(open(), &tgz, 1024).unwrap_err();
        assert!(err.contains("unpacks to more than"), "{err}");

        // The reader itself: up to the limit, and an error one byte past it.
        let over = Rc::new(Cell::new(false));
        let mut exact = Capped { inner: &b"abcdef"[..], left: 6, over: over.clone() };
        let mut all = Vec::new();
        assert_eq!(exact.read_to_end(&mut all).unwrap(), 6);
        assert!(!over.get());
        let mut short = Capped { inner: &b"abcdef"[..], left: 5, over: over.clone() };
        assert!(short.read_to_end(&mut Vec::new()).is_err());
        assert!(over.get());
        let _ = fs::remove_dir_all(&tmp);
    }

    /// A PAX size that disagrees with the entry's own header is refused:
    /// tools reading by one or by the other would see different files.
    #[test]
    fn a_pax_size_that_disagrees_with_the_header_is_refused() {
        let tmp = std::env::temp_dir().join(format!("bl-pkg-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("pax-size.tgz");
        let gz = flate2::write::GzEncoder::new(fs::File::create(&path).unwrap(), flate2::Compression::fast());
        let mut builder = tar::Builder::new(gz);
        let body = "12 size=999\n";
        let mut pax = tar::Header::new_ustar();
        pax.set_path("PaxHeaders/x").unwrap();
        pax.set_size(body.len() as u64);
        pax.set_entry_type(tar::EntryType::XHeader);
        pax.set_cksum();
        builder.append(&pax, body.as_bytes()).unwrap();
        let mut header = tar::Header::new_ustar();
        header.set_path("package/package.json").unwrap();
        header.set_size(2);
        header.set_cksum();
        builder.append(&header, &b"{}"[..]).unwrap();
        builder.into_inner().unwrap().finish().unwrap();
        let err = read_package(&path).unwrap_err();
        assert!(err.contains("another size"), "{err}");
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Only a regular file of sensible size is read as a package. A named
    /// pipe waited for a writer forever; a device would be read until the
    /// memory ran out.
    #[cfg(unix)]
    #[test]
    fn a_named_pipe_or_a_huge_file_is_not_read_as_a_package() {
        let tmp = std::env::temp_dir().join(format!("bl-pkg-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).unwrap();
        let fifo = tmp.join("pipe.tgz");
        let c = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
        // On a thread, so a read that blocks fails the test instead of
        // hanging it.
        let (tx, rx) = std::sync::mpsc::channel();
        let (pipe, dir) = (fifo.clone(), tmp.clone());
        std::thread::spawn(move || {
            tx.send((install(&pipe).map(|_| ()), read_package(&dir).map(|_| ()))).ok();
        });
        let (from_pipe, from_dir) = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("reading a named pipe as a package never ended");
        let err = from_pipe.unwrap_err();
        assert!(err.contains("is not a file (a folder, a device or a named pipe)"), "{err}");
        assert!(from_dir.unwrap_err().contains("is not a file"));
        for dev in ["/dev/zero", "/dev/urandom"] {
            if Path::new(dev).exists() {
                assert!(read_package(Path::new(dev)).unwrap_err().contains("is not a file"), "{dev}");
            }
        }

        // Sparse: refused from its size, nothing read.
        let big = tmp.join("big.tgz");
        fs::File::create(&big).unwrap().set_len(MAX_PACKAGE_FILE_BYTES + 1).unwrap();
        let err = read_package(&big).unwrap_err();
        assert!(err.contains("a package file over 256 MB is not accepted"), "{err}");
        let _ = fs::remove_dir_all(&tmp);
    }
}

