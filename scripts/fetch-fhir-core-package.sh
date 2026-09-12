#!/usr/bin/env bash
#
# Fetch the HL7 FHIR R4 core package for the profile-validation tests in
# src-tauri/tests/fhir_profiles.rs.
#
# The package is CC0 but ~4.5 MB compressed and 4581 files unpacked, so it
# is not vendored. Download it into a gitignored directory and point the
# tests at it:
#
#     ./scripts/fetch-fhir-core-package.sh
#     BL_FHIR_PACKAGE=.fhir-packages/hl7.fhir.r4.core.tgz \
#     BL_FHIR_EXAMPLES=.fhir-packages/examples \
#         cargo test --manifest-path src-tauri/Cargo.toml --test fhir_profiles -- --nocapture
#
# The same .tgz is what the app installs through
# Tools → FHIR profile packages…
#
set -euo pipefail

DEST="${1:-.fhir-packages}"
PACKAGE_URL="https://packages2.fhir.org/packages/hl7.fhir.r4.core/4.0.1"
TGZ="$DEST/hl7.fhir.r4.core.tgz"

mkdir -p "$DEST"

echo "Fetching hl7.fhir.r4.core 4.0.1…"
curl -fsSL "$PACKAGE_URL" -o "$TGZ"

# The example resources the tests validate live inside the same archive.
echo "Unpacking the example resources…"
rm -rf "$DEST/examples"
mkdir -p "$DEST/examples"
tar xzf "$TGZ" -C "$DEST/examples" --strip-components=1
rm -f "$DEST/examples/package.json" "$DEST/examples/.index.json"

count=$(find "$DEST/examples" -name '*.json' | wc -l | tr -d ' ')
echo
echo "Package:  $TGZ"
echo "Examples: $DEST/examples ($count resources)"
echo
echo "Run the tests with:"
echo "  BL_FHIR_PACKAGE=$TGZ BL_FHIR_EXAMPLES=$DEST/examples \\"
echo "      cargo test --manifest-path src-tauri/Cargo.toml --test fhir_profiles -- --nocapture"
