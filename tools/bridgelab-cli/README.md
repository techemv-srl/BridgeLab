# BridgeLab CLI

Headless HL7 validation and tooling for CI/CD pipelines.

## Installation

```bash
cd tools/bridgelab-cli
cargo build --release
```

The binary is produced at `target/release/bridgelab-cli`.

## Commands

### `validate` - Validate HL7 files

```bash
# Single file
bridgelab-cli validate message.hl7

# Multiple files with glob
bridgelab-cli validate "./messages/*.hl7"

# JSON output for automation
bridgelab-cli validate message.hl7 --format json

# JUnit XML for CI integration (GitHub Actions, Jenkins)
bridgelab-cli validate "**/*.hl7" --format junit > results.xml
```

Exit codes: 0 = all valid, 1 = errors found, 2 = usage error.

### `info` - Show message metadata

```bash
# Table output
bridgelab-cli info "./*.hl7"

# JSON output
bridgelab-cli info message.hl7 --json
```

Displays: message type, HL7 version, segment count, file size.

### `anonymize` - Mask PHI fields

```bash
# Print to stdout
bridgelab-cli anonymize patient.hl7

# Save to file
bridgelab-cli anonymize patient.hl7 --output patient-safe.hl7
```

Masks: PID-3/4/5/7/11/13/19, NK1-2/4/5, IN1-16/36, GT1-3/5/6/12.

### `to-json` - Convert HL7 to structured JSON

```bash
bridgelab-cli to-json message.hl7 --output message.json
```

Useful for downstream processing or documentation.

### `batch` - Validate directory of messages

```bash
bridgelab-cli batch ./messages --extension hl7

# JSON summary for CI
bridgelab-cli batch ./messages --json > batch-report.json
```

## CI/CD Integration Examples

### GitHub Actions

```yaml
- name: Install BridgeLab CLI
  run: cargo install --git https://github.com/1warpengine/HL7_editor bridgelab-cli

- name: Validate HL7 messages
  run: bridgelab-cli validate "test/fixtures/*.hl7" --format junit > junit.xml

- name: Publish results
  uses: mikepenz/action-junit-report@v4
  if: always()
  with:
    report_paths: 'junit.xml'
```

### GitLab CI

```yaml
hl7-validation:
  image: rust:latest
  script:
    - cargo install --git https://gitlab.com/yourorg/bridgelab-cli
    - bridgelab-cli batch ./hl7-fixtures --json > report.json
  artifacts:
    paths: [report.json]
```

### Pre-commit hook

```bash
#!/bin/sh
# .git/hooks/pre-commit
files=$(git diff --cached --name-only --diff-filter=ACM | grep '\.hl7$')
if [ -n "$files" ]; then
    bridgelab-cli validate $files --format text
fi
```

## Features

- Fast SIMD-accelerated HL7 parser (memchr)
- Structural + MSH validation (STRUCT-00X, MSH-00X rules)
- PHI anonymization (21 known field definitions)
- JSON + JUnit XML + text output formats
- Glob pattern support
- Batch processing with summary statistics
- Zero network dependencies - fully offline

## Limitations vs desktop BridgeLab

The CLI is a lightweight subset of the full desktop app. It does NOT include:
- FHIR parsing / FHIRPath evaluation
- MLLP/HTTP transport
- License management
- Message templates
- Interactive features

For those, use the desktop BridgeLab or the GUI's scripting hooks.
