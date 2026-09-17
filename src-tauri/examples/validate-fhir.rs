//! Validate a FHIR resource from the command line, the way the app does.
//!
//!     cargo run --example validate-fhir -- path/to/resource.json
//!
//! Runs the built-in checks — on the root and on every Bundle entry and
//! contained resource — then conformance against the StructureDefinitions
//! of the packages installed in the app's config directory, when any apply.
//! No licence is consulted: this is the Community behaviour, which is also
//! what the `bridgelab-cli` front-end will do.

use std::process::exit;

use bridgelab_lib::parser::fhir::{self, profile::ProfileRegistry};

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: cargo run --example validate-fhir -- <resource.json|xml>");
        exit(2);
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}: {}", path, e);
            exit(2);
        }
    };

    let resource = if content.trim_start().starts_with('<') {
        fhir::parse_fhir_xml(&content)
    } else {
        fhir::parse_fhir_json(&content)
    };
    let resource = match resource {
        Ok(r) => r,
        Err(e) => {
            eprintln!("cannot parse {}: {}", path, e);
            exit(1);
        }
    };

    let mut issues = fhir::validate_fhir_json(&resource);

    let registry = ProfileRegistry::new();
    let packages = registry.reload().unwrap_or(0);
    let mut profiles_applied = false;
    if let Some(json) = &resource.json_value {
        if let Some(found) = registry.validate(json) {
            profiles_applied = true;
            issues.extend(found);
        }
    }

    println!(
        "{} — {} ({} profile{} installed, conformance {})",
        path,
        resource.resource_type,
        packages,
        if packages == 1 { "" } else { "s" },
        if profiles_applied { "checked" } else { "not checked" }
    );
    for issue in &issues {
        println!("  {:<7} {}  @ {}", issue.severity, issue.message, issue.path);
    }
    let errors = issues.iter().filter(|i| i.severity == "error").count();
    let warnings = issues.iter().filter(|i| i.severity == "warning").count();
    println!("{} error(s), {} warning(s), {} issue(s) in total", errors, warnings, issues.len());
    exit(if errors > 0 { 1 } else { 0 });
}
