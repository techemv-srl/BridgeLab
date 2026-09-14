//! Install a FHIR NPM package the way the app does, without the app.
//!
//!     cargo run --example install-fhir-package -- .fhir-packages/hl7.fhir.r4.core.tgz
//!
//! `Tools → FHIR profile packages…` picks a `.tgz` through a native file
//! dialog, which a WebDriver session cannot drive. This calls the same
//! `package::install` and writes the same distilled index to the same place,
//! so the release acceptance suite can exercise profile conformance instead
//! of skipping it.

use std::path::PathBuf;

use bridgelab_lib::parser::fhir::profile::package;

fn main() {
    let Some(arg) = std::env::args().nth(1) else {
        eprintln!("usage: cargo run --example install-fhir-package -- <package.tgz>");
        std::process::exit(2);
    };

    match package::install(&PathBuf::from(&arg)) {
        Ok(pkg) => {
            let root = package::packages_root()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<unknown>".into());
            println!(
                "installed {} {} ({} profiles) into {}",
                pkg.name,
                pkg.version,
                pkg.profiles.len(),
                root
            );
        }
        Err(e) => {
            eprintln!("install failed: {e}");
            std::process::exit(1);
        }
    }
}
