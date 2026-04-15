# BridgeLab

**HL7 made simple** - A modern HL7/FHIR message editor for healthcare integration professionals.

![Made by TECHEMV SRL](https://img.shields.io/badge/Made%20by-TECHEMV%20SRL-blue)
![License](https://img.shields.io/badge/License-Open%20Core-green)
![Platforms](https://img.shields.io/badge/Platforms-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)

## Features

- **HL7 v2.x parser** - SIMD-accelerated streaming parser, handles 5-10MB messages with base64 sections smoothly
- **FHIR support** - Parse and validate JSON/XML FHIR resources (Patient, Observation, Bundle, ...)
- **Smart truncation** - Large fields auto-truncated to `{...N bytes}`, expandable inline or all at once
- **Validation** - Structural, field-level, data-type validation with 5 rule categories
- **MLLP transport** - Client & server with custom framing, auto-ACK, encoding selection
- **HTTP client** - GET/POST/PUT/DELETE/PATCH with Basic/Bearer auth, headers, timeout
- **Anonymization** - 21 known PHI field definitions across PID/NK1/IN1/GT1
- **Export** - JSON, CSV, structured representations
- **5 Languages** - English, Italian, French, Spanish, German
- **Licensing** - Ed25519-signed offline license verification, hardware binding, 30-day trial
- **Field Inspector** - Side panel showing HL7 standard metadata (name, type, required, max length, description) for the selected tree node
- **Schema-aware Tree** - Optional view that injects placeholder rows for every field defined by the standard so you see what _could_ be populated
- **Precise Editor ↔ Tree navigation** - Right-click a field in the editor to highlight it in the tree, or right-click a tree node to select the matching range in Monaco

## Stack

- **Frontend**: Svelte 5 + TypeScript + Monaco Editor
- **Backend**: Rust + Tauri 2
- **Database**: SQLite (rusqlite)
- **Transport**: tokio (MLLP), reqwest (HTTP)
- **Licensing**: Ed25519 signatures + hardware fingerprint

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 20+
- [pnpm](https://pnpm.io/) 10+
- Platform-specific Tauri dependencies ([setup guide](https://tauri.app/start/prerequisites/))

### Running

```bash
pnpm install
pnpm tauri dev
```

### Testing

```bash
# Rust tests (parser, validation, anonymization, licensing, ...)
cd src-tauri && cargo test

# Frontend check & build
pnpm check
pnpm build

# Generate the QA Excel workbook from TEST_PLAN.md
pip install openpyxl
python scripts/test_plan_to_excel.py   # -> TEST_PLAN.xlsx (gitignored)
```

Full manual test catalogue lives in [`TEST_PLAN.md`](TEST_PLAN.md) (~300 cases
organized by feature area). CI automates the automatable slice:

- [`ci.yml`](.github/workflows/ci.yml) - `cargo check`/`cargo test --all`,
  `svelte-check` (0 errors), `pnpm build`
- [`feature-tests.yml`](.github/workflows/feature-tests.yml) - CLI feature
  tests, HL7 fixtures (parser/info/validate/anonymize/batch/JUnit), FHIR
  fixture integrity, schema-lookup Rust tests, license keygen roundtrip

## Resource usage

BridgeLab does **not** require memory tuning - the Rust backend uses zero-copy
parsing plus on-demand field truncation, and peak RAM stays below ~300 MB even
on 10 MB messages. If you want to trade display fidelity for IPC size on
unusually large files, adjust **Settings → Parser → Truncation threshold**.

## Installer options

Per-platform installer configuration lives in [`src-tauri/tauri.conf.json`](src-tauri/tauri.conf.json).

- **Windows NSIS**: shows the MIT license page, a language selector (EN/IT/FR/ES/DE),
  installs to `%LOCALAPPDATA%\Programs\BridgeLab` by default (current user), LZMA compression
- **Windows MSI (WiX)**: multi-language (en-US/it-IT/fr-FR/es-ES/de-DE)
- **macOS DMG**: presents a drag-to-Applications layout with the app + Applications icons
- **Linux .deb**: declares `libwebkit2gtk-4.1-0` + `libgtk-3-0` dependencies, `utils` section
- **Linux AppImage**: bundles the media framework so GStreamer-dependent features work offline
- **Linux .rpm**: declares `webkit2gtk4.1` + `gtk3` dependencies
- **File association**: `.hl7` is registered so double-clicking a file opens BridgeLab

The MIT `LICENSE` file at the repo root is referenced from the bundle
`licenseFile` field and included in the installer payload.

## Building a Release

Create a signed tag to trigger the release workflow:

```bash
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions will build installers for Windows (.msi), macOS (.dmg), and Linux (.AppImage, .deb).

## License Keys (for vendors)

Use the CLI tool in `tools/bridgelab-keygen/` to generate signed license keys.
See [tools/bridgelab-keygen/README.md](tools/bridgelab-keygen/README.md) for details.

Simple workflow:

```bash
cd tools/bridgelab-keygen
cargo run --release -- generate-keypair
cargo run --release -- generate --license-type pro --licensee "Acme Corp" --email admin@acme.com --days 365
```

## Project Structure

```
BridgeLab/
├── src/                      # Svelte 5 frontend
│   ├── lib/components/       # UI components
│   ├── lib/ipc/              # Tauri command wrappers
│   ├── lib/i18n/             # Translations (EN, IT, FR, ES, DE)
│   └── lib/stores/           # Svelte 5 runes stores
├── src-tauri/                # Rust backend
│   └── src/
│       ├── parser/           # HL7 v2.x + FHIR parsers
│       ├── validation/       # Validation engine
│       ├── communication/    # MLLP, HTTP clients
│       ├── anonymization/    # PHI detection & masking
│       ├── licensing/        # Ed25519 license verification
│       └── commands/         # IPC command handlers
├── tools/
│   └── bridgelab-keygen/     # License generator CLI
└── .github/workflows/        # CI/CD pipelines
```

## License

Open Core - Free for non-commercial use.
Paid tiers: Professional & Enterprise.

Copyright (c) 2026 TECHEMV SRL - All rights reserved.
