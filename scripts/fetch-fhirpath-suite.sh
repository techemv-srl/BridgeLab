#!/usr/bin/env bash
#
# Fetch the official HL7 FHIRPath R4 test suite for the conformance harness
# in src-tauri/tests/fhirpath_suite.rs.
#
# The suite is not vendored into this repository: it is third-party material
# maintained by HL7, and BridgeLab's own FHIRPath tests (which do ship) are
# the ones CI depends on. This script downloads it into a gitignored
# directory so the harness can run it on demand:
#
#     ./scripts/fetch-fhirpath-suite.sh
#     BL_FHIRPATH_SUITE=.fhirpath-suite cargo test --manifest-path src-tauri/Cargo.toml \
#         --test fhirpath_suite -- --nocapture
#
# Sources:
#   tests    https://github.com/FHIR/fhir-test-cases (r4/fhirpath)
#   examples https://hl7.org/fhir/R4/ — the FHIR specification examples,
#            and fhir-test-cases (r5) for inputs published only there
#
set -euo pipefail

DEST="${1:-.fhirpath-suite}"
TESTS_URL="https://raw.githubusercontent.com/FHIR/fhir-test-cases/master/r4/fhirpath/tests-fhir-r4.xml"
SPEC_BASE="https://hl7.org/fhir/R4"
TEST_CASES_BASE="https://raw.githubusercontent.com/FHIR/fhir-test-cases/master/r5"

# Input resources the suite references, in the JSON encoding this engine
# evaluates. Ones the specification does not publish as JSON are skipped;
# the harness reports the tests it could not run because of that.
EXAMPLES=(
    patient-example
    observation-example
    questionnaire-example
    valueset-example-expansion
    explanationofbenefit-example
)

# Inputs that exist only in the test-case repository, as XML; the harness
# converts them the way the app converts an XML resource.
XML_INPUTS=(
    parameters-example-decimals
)

mkdir -p "$DEST/input"

echo "Fetching the test definitions…"
curl -fsSL "$TESTS_URL" -o "$DEST/tests-fhir-r4.xml"

for name in "${EXAMPLES[@]}"; do
    printf 'Fetching %s… ' "$name"
    if curl -fsSL "$SPEC_BASE/$name.json" -o "$DEST/input/$name.json"; then
        echo "ok"
    else
        echo "unavailable (tests using it will be skipped)"
        rm -f "$DEST/input/$name.json"
    fi
done

for name in "${XML_INPUTS[@]}"; do
    printf 'Fetching %s… ' "$name"
    if curl -fsSL "$TEST_CASES_BASE/$name.xml" -o "$DEST/input/$name.xml"; then
        echo "ok"
    else
        echo "unavailable (tests using it will be skipped)"
        rm -f "$DEST/input/$name.xml"
    fi
done

cases=$(grep -c '<test ' "$DEST/tests-fhir-r4.xml" || true)
echo
echo "Downloaded $cases test cases to $DEST/"
echo "Run them with:"
echo "  BL_FHIRPATH_SUITE=$DEST cargo test --manifest-path src-tauri/Cargo.toml --test fhirpath_suite -- --nocapture"
