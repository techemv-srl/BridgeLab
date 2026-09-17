# BridgeLab CLI

The BridgeLab validators from the command line — for CI pipelines, batch
screening and scripts.

It is a thin front-end over the desktop app's library: the **same** HL7 v2
parser and validator (version-aware, from MSH-12), the **same** FHIR checks
and profile engine (the built-in FHIR R4 core, plus any package you install),
the **same** plugin packs and PHI anonymiser. What the app reports, the CLI
reports.

**Edition.** The CLI reads no licence and is built without the `pro`
feature: it behaves as the **Community** edition, by construction. That
means the Community cap on active plugin packs applies, and packages beyond
the built-in core are whatever is already in the app's package directory (or
the directory you point it at).

## Installation

Prebuilt binaries are attached to every
[release](https://github.com/techemv-srl/BridgeLab/releases) as
`bridgelab-cli-<target>` (Windows, macOS Intel and Apple Silicon, Linux).

From source (needs Rust; no Tauri or WebKit — the desktop shell is not
compiled):

```bash
cd tools/bridgelab-cli
cargo build --release          # → target/release/bridgelab-cli
```

## Commands

### `validate` — HL7 v2 or FHIR, detected per file

```bash
bridgelab-cli validate message.hl7
bridgelab-cli validate "./messages/*.hl7" bundle.json
bridgelab-cli validate message.hl7 --format json
bridgelab-cli validate "**/*.hl7" --format junit > results.xml
```

- HL7 v2: structure, MSH, required fields, lengths and data types against
  the catalogue of the version in MSH-12, plus any active plugin rules.
- FHIR (JSON or XML): the built-in checks on the root **and every Bundle
  entry and contained resource**; Bundle rules (fullUrl, identity,
  references that must resolve); conformance against the built-in R4 core
  and installed packages, including each resource's own `meta.profile`;
  primitive formats; plugin FHIRPath rules. A resource type no package
  defines is reported as *not checked*, never as clean.

Options: `--fhir-packages DIR` reads distilled packages from `DIR` instead of
the app's config directory (the R4 core is always included); `--no-plugins`
ignores plugin packs; `--strict` (default) exits 1 on errors.

Exit codes: 0 = clean, 1 = errors found or a file failed to parse, 2 = usage.

### `info` — message metadata

```bash
bridgelab-cli info "./*.hl7" bundle.json
bridgelab-cli info message.hl7 --json
```

Kind (hl7/fhir), message type or resource type, version, segment or entry
count, size.

### `anonymize` — mask PHI fields (HL7 v2)

```bash
bridgelab-cli anonymize patient.hl7
bridgelab-cli anonymize patient.hl7 --output patient-safe.hl7
```

The app's 21 built-in PHI fields across PID/NK1/IN1/GT1, plus the extra
fields of active plugin packs (`--no-plugins` to ignore them). Structure is
preserved, so the output still parses and validates.

### `to-json` — HL7 v2 to a structured JSON document

```bash
bridgelab-cli to-json message.hl7 --output message.json
```

### `batch` — a directory of messages

```bash
bridgelab-cli batch ./messages --extension hl7
bridgelab-cli batch ./fhir --extension json --json > batch-report.json
```

Exit 1 if any file has errors.

## CI/CD integration

### GitHub Actions

```yaml
- name: Validate HL7 and FHIR fixtures
  run: |
    curl -sSL -o bridgelab-cli https://github.com/techemv-srl/BridgeLab/releases/latest/download/bridgelab-cli-x86_64-unknown-linux-gnu
    chmod +x bridgelab-cli
    ./bridgelab-cli validate "test/fixtures/**/*.hl7" "test/fixtures/**/*.json" --format junit > junit.xml

- uses: mikepenz/action-junit-report@v4
  if: always()
  with:
    report_paths: junit.xml
```

### Pre-commit hook

```bash
#!/bin/sh
files=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(hl7|json)$')
[ -n "$files" ] && bridgelab-cli validate $files
```

### Air-gapped sites

Copy the distilled packages the app produced (`<config>/BridgeLab/fhir-packages/`)
next to your fixtures and point the CLI at them with `--fhir-packages`. Nothing
is ever downloaded.

## What the CLI does not do

MLLP/HTTP transport, the listener, licence activation, templates and every
interactive feature stay in the desktop app.
