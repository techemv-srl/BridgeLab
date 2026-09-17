//! Profile validation against a real FHIR package.
//!
//! The package is third-party (CC0, but large) and is not vendored. Run
//! `./scripts/fetch-fhir-core-package.sh` to download it, then:
//!
//! ```text
//! BL_FHIR_PACKAGE=.fhir-packages/hl7.fhir.r4.core.tgz \
//!     cargo test --test fhir_profiles -- --nocapture
//! ```
//!
//! Without `BL_FHIR_PACKAGE` the test reports that it was skipped and
//! passes, so a normal `cargo test` run stays offline.
//!
//! What it asserts: the package parses, the core resource definitions are
//! all there, the specification's own example resources validate clean, and
//! deliberately broken copies of them are caught.

use std::fs;
use std::path::PathBuf;

use bridgelab_lib::parser::fhir::profile::model::{ProfileIndex, ProfilePackage};
use bridgelab_lib::parser::fhir::profile::{package, validate};
use serde_json::{json, Value};

fn load() -> Option<(ProfilePackage, ProfileIndex)> {
    let path = std::env::var("BL_FHIR_PACKAGE").ok()?;
    let pkg = package::read_package(&PathBuf::from(&path))
        .unwrap_or_else(|e| panic!("cannot read {}: {}", path, e));
    let mut index = ProfileIndex::default();
    index.add_package(pkg.clone());
    Some((pkg, index))
}

fn skipped() {
    eprintln!(
        "skipped: set BL_FHIR_PACKAGE to a FHIR package .tgz \
         (see scripts/fetch-fhir-core-package.sh) to run these"
    );
}

#[test]
fn the_core_package_yields_the_resource_definitions() {
    let Some((pkg, index)) = load() else {
        return skipped();
    };

    eprintln!(
        "{} {} — {} profiles distilled",
        pkg.name,
        pkg.version,
        pkg.profiles.len()
    );

    // A package this size should carry every resource people actually open.
    for type_name in [
        "Patient", "Observation", "Bundle", "Encounter", "Condition",
        "MedicationRequest", "DiagnosticReport", "Practitioner", "Organization",
        // …and the data types the validator recurses into.
        "HumanName", "Identifier", "CodeableConcept", "Coding", "Reference",
        "Period", "Quantity", "Meta",
    ] {
        assert!(
            index.base_for_type(type_name).is_some(),
            "{} is missing from {}",
            type_name,
            pkg.name
        );
    }
    assert!(pkg.profiles.len() > 500, "only {} profiles", pkg.profiles.len());
}

#[test]
fn specification_examples_validate_clean() {
    let Some((_, index)) = load() else {
        return skipped();
    };
    let Ok(dir) = std::env::var("BL_FHIR_EXAMPLES") else {
        eprintln!("skipped: set BL_FHIR_EXAMPLES to a directory of example resources");
        return;
    };

    let mut checked = 0usize;
    let mut with_findings: Vec<(String, Vec<String>)> = Vec::new();

    for entry in fs::read_dir(&dir).expect("examples directory").flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else { continue };
        let Ok(resource) = serde_json::from_str::<Value>(&text) else { continue };
        if resource.get("resourceType").is_none() {
            continue;
        }

        let Some(issues) = validate::validate_declared(&resource, &index) else {
            continue;
        };
        checked += 1;
        if !issues.is_empty() {
            with_findings.push((
                path.file_name().unwrap().to_string_lossy().to_string(),
                issues.iter().map(|i| format!("{} @ {}", i.message, i.path)).collect(),
            ));
        }
    }

    assert!(checked > 0, "no example resources were validated");
    if !with_findings.is_empty() {
        eprintln!("\n--- examples with findings ({}) ---", with_findings.len());
        for (name, issues) in with_findings.iter().take(30) {
            eprintln!("{}:", name);
            for i in issues.iter().take(5) {
                eprintln!("    {}", i);
            }
        }
    }
    let rate = with_findings.len() as f64 / checked as f64;
    eprintln!(
        "\nvalidated {} specification examples, {} produced findings ({:.2}%)",
        checked,
        with_findings.len(),
        rate * 100.0
    );

    // Resources published by HL7 conform to HL7's own definitions, so a
    // finding here is almost always a bug in this validator — that is the
    // point of running against the whole package.
    //
    // "Almost": on hl7.fhir.r4.core 4.0.1 a residue of 14 out of 4578 is
    // genuine, and was checked by hand. Thirteen
    // SearchParameter-*-extensions-*.json files really do omit the required
    // `base` element, and ValueSet-endpoint-payload-type declares a profile
    // that the core package does not contain — which the validator reports
    // rather than quietly skipping. A threshold rather than zero keeps the
    // test meaningful across package versions without allow-listing files
    // by name.
    assert!(
        rate < 0.01,
        "{:.2}% of specification examples produced findings — that is too many \
         to be the package's own non-conformance; check the validator",
        rate * 100.0
    );
}

#[test]
fn deliberate_breakages_are_caught() {
    let Some((_, index)) = load() else {
        return skipped();
    };

    let cases: Vec<(&str, Value, &str)> = vec![
        (
            "misspelled element",
            json!({"resourceType": "Patient", "genderr": "female"}),
            "not defined",
        ),
        (
            "choice element written as a plain one",
            json!({"resourceType": "Observation", "status": "final",
                   "code": {"text": "x"}, "value": {"value": 1}}),
            "not defined",
        ),
        (
            "wrong JSON type",
            json!({"resourceType": "Patient", "active": "yes"}),
            "must be a boolean",
        ),
        (
            "repeating a 0..1 element",
            json!({"resourceType": "Patient", "gender": ["female", "male"]}),
            "allows 0..1 but found 2",
        ),
        (
            "required element missing",
            json!({"resourceType": "Observation"}),
            "is required",
        ),
        (
            "two forms of the same choice",
            json!({"resourceType": "Patient",
                   "deceasedBoolean": true, "deceasedDateTime": "2020-01-01"}),
            "only one of its forms",
        ),
    ];

    for (name, resource, expected) in cases {
        let issues = validate::validate_declared(&resource, &index)
            .unwrap_or_else(|| panic!("{}: no profile applied", name));
        assert!(
            issues.iter().any(|i| i.message.contains(expected)),
            "{}: expected a finding containing {:?}, got {:?}",
            name,
            expected,
            issues.iter().map(|i| &i.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn a_declared_profile_that_is_not_installed_is_reported() {
    let Some((_, index)) = load() else {
        return skipped();
    };
    let resource = json!({
        "resourceType": "Patient",
        "meta": {"profile": ["http://hl7.org/fhir/us/core/StructureDefinition/us-core-patient"]}
    });
    let issues = validate::validate_declared(&resource, &index).expect("base applies");
    assert!(
        issues.iter().any(|i| i.message.contains("not installed")),
        "{:?}",
        issues
    );
}

/// The install path the app uses: read a `.tgz`, write the distilled index,
/// load it back through the registry, and validate with it.
#[test]
fn install_and_reload_round_trips_through_the_registry() {
    let Some(_) = std::env::var("BL_FHIR_PACKAGE").ok() else {
        return skipped();
    };
    // Point the config dir at a scratch location so the test never touches
    // a real installation.
    let scratch = std::env::temp_dir().join(format!("bl-fhir-{}", std::process::id()));
    fs::create_dir_all(&scratch).unwrap();
    // SAFETY: single-threaded test; the variable is read by dirs::config_dir
    // on the next call only.
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", &scratch);
    }

    let tgz = std::env::var("BL_FHIR_PACKAGE").unwrap();
    let installed = package::install(&PathBuf::from(&tgz)).expect("install");
    assert!(installed.profiles.len() > 500);

    let registry = bridgelab_lib::parser::fhir::profile::ProfileRegistry::new();
    let loaded = registry.reload().expect("reload");
    // The binary carries the same package: the installed copy replaces its
    // definitions one for one, so the count is unchanged, and the package
    // manager lists both — the built-in floor and the installed copy.
    assert_eq!(loaded, installed.profiles.len(), "every profile survives the round trip");
    assert!(!registry.is_empty());
    let packages = registry.packages();
    assert_eq!(packages.len(), 2);
    assert!(packages.iter().any(|p| p.builtin) && packages.iter().any(|p| !p.builtin));

    // And it validates through the same entry point the command uses.
    let broken = json!({"resourceType": "Patient", "genderr": "female"});
    let issues = registry.validate(&broken).expect("Patient profile applies");
    assert!(issues.iter().any(|i| i.message.contains("genderr")));

    package::remove(&installed.name, &installed.version).expect("remove");
    assert!(package::load_installed().is_empty());

    fs::remove_dir_all(&scratch).ok();
}
