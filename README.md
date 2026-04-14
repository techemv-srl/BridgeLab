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
# Rust tests
cd src-tauri && cargo test

# Frontend check
pnpm check
pnpm build
```

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
