#!/usr/bin/env bash
#
# Refresh the FHIR R4 core index the binary carries.
#
# The app embeds a distilled hl7.fhir.r4.core so profile conformance works
# offline on a fresh install. This fetches the package (CC0, ~4.5 MB),
# distils it exactly as an in-app installation would, and writes the
# gzipped index under src-tauri/resources/fhir/. Run it when the bundled
# core version changes, then commit the result.
#
set -euo pipefail

cd "$(dirname "$0")/.."

VERSION="${1:-4.0.1}"
DEST=".fhir-packages"
TGZ="$DEST/hl7.fhir.r4.core.tgz"
OUT="src-tauri/resources/fhir/hl7.fhir.r4.core-${VERSION}.json.gz"

if [ ! -f "$TGZ" ]; then
    ./scripts/fetch-fhir-core-package.sh "$DEST"
fi

mkdir -p src-tauri/resources/fhir
cargo run --quiet --manifest-path src-tauri/Cargo.toml \
    --example distil-fhir-package -- "$TGZ" "$OUT"

echo
echo "If the version changed, update the include_bytes! path in"
echo "src-tauri/src/parser/fhir/profile/package.rs and remove the old file."
