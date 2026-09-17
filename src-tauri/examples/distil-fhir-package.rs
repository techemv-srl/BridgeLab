//! Distil a FHIR NPM package into the gzipped index the app embeds.
//!
//!     cargo run --example distil-fhir-package -- \
//!         .fhir-packages/hl7.fhir.r4.core.tgz \
//!         resources/fhir/hl7.fhir.r4.core-4.0.1.json.gz
//!
//! The output is exactly what `Tools → FHIR profile packages…` would write
//! to the config directory for the same `.tgz`, compressed. Refresh it when
//! the bundled core package changes; `scripts/refresh-bundled-fhir-core.sh`
//! runs the whole sequence.

use std::io::Write;
use std::path::PathBuf;

use bridgelab_lib::parser::fhir::profile::package;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [tgz, out] = args.as_slice() else {
        eprintln!("usage: cargo run --example distil-fhir-package -- <package.tgz> <out.json.gz>");
        std::process::exit(2);
    };

    let package = match package::read_package(&PathBuf::from(tgz)) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };
    let json = serde_json::to_vec(&package).expect("serialise");

    let file = std::fs::File::create(out).unwrap_or_else(|e| {
        eprintln!("{}: {}", out, e);
        std::process::exit(1);
    });
    let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::best());
    encoder.write_all(&json).and_then(|_| encoder.finish().map(|_| ())).unwrap_or_else(|e| {
        eprintln!("{}: {}", out, e);
        std::process::exit(1);
    });

    println!(
        "{} {} — {} profiles, {} bytes distilled, written to {}",
        package.name,
        package.version,
        package.profiles.len(),
        json.len(),
        out
    );
}
