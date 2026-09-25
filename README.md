# BridgeLab

**HL7 made simple** - A modern HL7/FHIR message editor for healthcare integration professionals.

![Made by TECHEMV SRL](https://img.shields.io/badge/Made%20by-TECHEMV%20SRL-blue)
![License](https://img.shields.io/badge/License-Open%20Core-green)
![Platforms](https://img.shields.io/badge/Platforms-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)

Free and open-source core (MIT), free for commercial use. **Pro** adds the MLLP
listener, batch validation, PHI masking and the full XSD catalogue - EUR 249 per
seat per year, EUR 199 for the first year on licenses bought by 31 December 2026.
[Download or buy](https://techemv-srl.github.io/BridgeLab/)

## Links

- **Website**: <https://techemv-srl.github.io/BridgeLab/> - downloads, pricing
  and FAQ. Source in [docs/site/](docs/site/), deployed to GitHub Pages via
  [`.github/workflows/pages.yml`](.github/workflows/pages.yml); preview locally
  with `python3 -m http.server --directory docs/site 4173`.
- **Changelog**: [CHANGELOG.md](CHANGELOG.md)
- **Plugin docs**: [docs/PLUGINS.md](docs/PLUGINS.md)
- **Test plan**: [TEST_PLAN.md](TEST_PLAN.md)
- **Roadmap**: [ROADMAP.md](ROADMAP.md)

## Features

- **HL7 v2.x parser** - SIMD-accelerated streaming parser, handles 5-10MB messages with base64 sections smoothly
- **HL7 versions** - Schema catalogue for **v2.1, v2.2, v2.3, v2.3.1, v2.4, v2.5, v2.5.1, v2.6, v2.7 and v2.7.1**
  — ten selectable versions, 1,965 message definitions. The version is read from MSH-12, so field names,
  data types and lengths come from the catalogue the message itself declares; v2.7.1 is a technical correction
  of v2.7 and shares its definitions, so it is offered as an alias. HL7 v2.8+ is not covered:
  [hl7-dictionary](https://github.com/Ensighten/hl7-dictionary), the MIT-licensed source behind these, stops at v2.7
- **FHIR support** - Parse and validate JSON/XML FHIR resources (Patient, Observation, Bundle, ...), with profile validation against the built-in FHIR R4 core and any installed FHIR NPM package (cardinality, element types, choice elements, fixed values, unknown elements, primitive formats, Bundle references) — offline, nothing to download
- **FHIRPath 2.0** - Full operator set and ~70 functions, verified against the official HL7 FHIRPath test suite
- **Smart truncation** - Large fields auto-truncated to `{...N bytes}`, expandable inline or all at once
- **Validation** - Structural, field-level, data-type validation with 5 rule categories
- **MLLP transport** - Client & server with custom framing, auto-ACK, encoding selection
- **HTTP client** - GET/POST/PUT/DELETE/PATCH with Basic/Bearer auth, headers, timeout
- **Anonymization** - 21 known PHI field definitions across PID/NK1/IN1/GT1
- **Export** - JSON, CSV, structured representations
- **XSD schema export** - Generate HL7 v2.xml compatible XSD files for any supported message type (4 common messages in v2.5 free, full catalogue in Pro). Useful for integration engines that accept hand-authored schemas (Astraia, BizTalk, XMLSpy…).
- **5 Languages** - English, Italian, French, Spanish, German
- **Licensing** - Ed25519-signed offline license verification, hardware binding, 14-day trial
- **Field Inspector** - Side panel showing HL7 standard metadata (name, type, required, max length, description) for the selected tree node
- **HL7 value tables** - All 394 HL7 tables (about 5,000 codes) ship with the app, mapped per version to every coded field and component: the tree shows what a code means next to the value (`M — Male`), the editor explains it on hover and completes it, the inspector lists the whole table and tells an HL7-defined table (a value outside it is non-standard) from a user-defined one (suggestions)
- **ACK filters** - The listener console and the send history filter by outcome (AA / AE / AR / no ACK / failed) with live counts; MLLP sends record the ACK code they got back
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

# Export the in-app English manual as a Word document
node scripts/manual-to-docx.mjs        # -> docs/BridgeLab-User-Manual-EN.docx (gitignored)

# FHIRPath conformance against the official HL7 suite (third-party, not vendored)
./scripts/fetch-fhirpath-suite.sh
BL_FHIRPATH_SUITE=.fhirpath-suite cargo test --manifest-path src-tauri/Cargo.toml \
    --test fhirpath_suite -- --nocapture

# FHIR profile validation against the HL7 R4 core package's 4578 examples
./scripts/fetch-fhir-core-package.sh
BL_FHIR_PACKAGE=.fhir-packages/hl7.fhir.r4.core.tgz \
BL_FHIR_EXAMPLES=.fhir-packages/examples \
    cargo test --manifest-path src-tauri/Cargo.toml --test fhir_profiles -- --nocapture

# Validate one resource headlessly, the way the app does (built-in core + installed packages)
cargo run --manifest-path src-tauri/Cargo.toml --example validate-fhir -- resource.json
```

The FHIR R4 core (`hl7.fhir.r4.core` 4.0.1, CC0) ships **inside the binary**
as a ~200 KB distilled index — `src-tauri/resources/fhir/` — so profile
conformance works on a fresh install with nothing to download. To refresh it
after a core update: `./scripts/refresh-bundled-fhir-core.sh`.

### Release acceptance

Before tagging a release, run exactly what CI runs — `cargo check
--all-targets` and `cargo test --all` in `src-tauri/`, `pnpm check`,
`pnpm build` — not just `cargo test --lib`: the integration tests under
`src-tauri/tests/` compile against the same dependencies, and a lib-only
run once shipped a release whose test job failed on the public mirror.
Then build the packages, install one, and run the acceptance suite
against the **installed** application:

```bash
pnpm tauri build --target x86_64-unknown-linux-gnu
sudo dpkg -i src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/deb/BridgeLab_<version>_amd64.deb

# so profile conformance is exercised rather than skipped
./scripts/fetch-fhir-core-package.sh
cargo run --manifest-path src-tauri/Cargo.toml \
    --example install-fhir-package -- .fhir-packages/hl7.fhir.r4.core.tgz

pnpm e2e
```

It drives the real binary through tauri-driver (starting Xvfb and the driver
itself) and checks the package too — version agreement, dependencies, the
`.hl7` MIME association — then the shell, HL7 parsing and validation, the
version catalogue, the FHIRPath engine, the FHIR rules builder and profile
validation. Exits non-zero on any failure. See [`e2e/README.md`](e2e/README.md).

Full manual test catalogue lives in [`TEST_PLAN.md`](TEST_PLAN.md) (~300 cases
organized by feature area). CI automates the automatable slice:

- [`ci.yml`](.github/workflows/ci.yml) - `cargo check`/`cargo test --all`,
  `svelte-check` (0 errors), `pnpm build`
- [`feature-tests.yml`](.github/workflows/feature-tests.yml) - CLI feature
  tests, HL7 fixtures (parser/info/validate/anonymize/batch/JUnit), FHIR
  fixture integrity, schema-lookup Rust tests, license keygen roundtrip

More checks belong to every release, none of them automatable:

- **The comparison table on the landing page** (`docs/site/index.html`,
  section *Why a new HL7 editor*) is comparative advertising under
  Directive 2006/114/EC (D.Lgs. 145/2007 in Italy): it may state only
  material, **verifiable** facts — never a judgement of someone else's
  software — with the source and the date. At each release, re-check
  every cell on the vendor's own site (FHIR, platforms, XSD export, list
  price, latest release), write "Not advertised" where the site does not
  mention a feature and "Quote on request" where no price is published,
  and update the date in the footnote. A cell that cannot be verified is
  removed, not guessed.
- **Dependency advisories**: `cargo audit` on `src-tauri/`,
  `tools/bridgelab-cli/` and `tools/hl7-schema-importer/`, `pnpm audit`
  at the root. Fix what has a fix; note in the CHANGELOG what does not
  (today `glib` 0.18 and `rand` 0.7, transitive through Tauri).
- **The release note ends with the company footer**, after a horizontal
  rule, exactly as below. The release workflow's placeholder body already
  carries it; keep it when you paste the real note over the draft.

  ```markdown
  ---

  *BridgeLab is built by TECHEMV SRL — [www.techemv.it](https://www.techemv.it) · info@techemv.it*
  ```

## Plugin packs (declarative rules)

BridgeLab accepts user-supplied **JSON plugin packs** that extend the
built-in validator and PHI anonymizer **without running any code**.

```
<config_dir>/BridgeLab/plugins/
├── validation/    *.json  - extra HL7 v2 rules (not_empty, regex,
│                            one_of, max_length, min_length, contains)
├── fhir/          *.json  - extra FHIR rules: a FHIRPath invariant, or a
│                            FHIRPath selector plus a check (including
│                            cardinality). Built in-app via
│                            Tools → FHIR validation rules…
└── anonymization/ *.json  - extra PHI fields merged with the built-in list
```

Manage packs from **Settings → Plugins**: list, toggle on/off (persisted),
Reload, "Open plugins folder". See [docs/PLUGINS.md](docs/PLUGINS.md) for
the full schema and examples; ready-to-copy reference packs live under
[`examples/plugins/`](examples/plugins).

**Tiers.** The whole mechanism is in every tier — all three pack kinds,
every check type, live reload, per-pack toggles. The only difference is
how many packs can be **active at the same time**: up to **3** in
Community, unlimited in Pro and Enterprise (the same split applies to
saved test cases: 10 in Community). Nothing is ever locked or deleted:
packs enabled beyond the cap show an "inactive" badge and start
contributing rules again as soon as a slot frees up. The in-app editor
that writes FHIR packs (*Tools → FHIR validation rules…*) is Pro; a FHIR
pack written by hand runs in Community like any other.

Scripted plugins (sandboxed JS, WASM) are on the roadmap as layers on top
of this declarative baseline.

## Resource usage

BridgeLab does **not** require memory tuning - the Rust backend uses zero-copy
parsing plus on-demand field truncation, and peak RAM stays below ~300 MB even
on 10 MB messages. If you want to trade display fidelity for IPC size on
unusually large files, adjust **Settings → Parser → Truncation threshold**.

## Installer options

Per-platform installer configuration lives in [`src-tauri/tauri.conf.json`](src-tauri/tauri.conf.json).

- **Windows NSIS**: shows the license page (the root `LICENSE`: MIT, plus BUSL-1.1 for the `pro/` directories), a language selector (EN/IT/FR/ES/DE),
  installs to `%LOCALAPPDATA%\Programs\BridgeLab` by default (current user), LZMA compression
- **Windows MSI (WiX)**: one `en-US` package for managed deployment (GPO, Intune, silent
  install). The installer language only affects the installer's own dialogs — the app is
  multilingual either way — so the per-language MSIs were dropped
- **macOS DMG**: presents a drag-to-Applications layout with the app + Applications icons
- **Linux .deb**: declares `libwebkit2gtk-4.1-0` + `libgtk-3-0` dependencies, `utils` section
- **Linux AppImage**: distro-agnostic; does not bundle GStreamer (BridgeLab plays no media).
  Most of its ~78 MB is the WebKitGTK and GTK stack it has to carry to run anywhere
- **Linux .rpm**: declares `webkit2gtk4.1` + `gtk3` dependencies
- **File association**: `.hl7` is registered so double-clicking a file opens BridgeLab.
  On Linux the `.deb`/`.rpm` also ship a shared-mime-info definition
  ([`src-tauri/linux/bridgelab-hl7.xml`](src-tauri/linux/bridgelab-hl7.xml)) declaring
  `application/hl7-v2` with a `*.hl7` glob and an `MSH|` magic rule — without it the
  desktop entry's `MimeType=` claim has no type to match and the association never fires

The root `LICENSE` file is referenced from the bundle `licenseFile` field
and included in the installer payload. It states the split: MIT for the
repository, Business Source License 1.1 for `src-tauri/src/pro/` and
`src/lib/pro/` (official builds may be run in every edition, with the
features unlocked only where the edition includes them; any other
production use needs an active Enterprise subscription; each version
turns MIT four years after publication). The bundle's
`license` field, which ends up in the Linux package metadata, says
`MIT AND BUSL-1.1` accordingly.

## Building a Release

Releases are built via GitHub Actions and require maintainer access.
Binaries are published to [GitHub Releases](https://github.com/TECHEMV-SRL/BridgeLab/releases).

## License Keys

BridgeLab uses Ed25519-signed license keys with hardware binding, verified
locally — there is no license server dependency at runtime.

**Online activation (default)** — buy a license and receive an activation
code (`BL-PRO-XXXX-XXXX-XXXX`) by e-mail. Paste it in *Settings → License →
Activate*: the app exchanges it once over HTTPS for a signed key bound to
this machine, then everything works offline. Each seat allows a limited
number of activations; use *Deactivate* to free the seat before moving to
another machine.

Licenses can be bought from the website; for invoices, purchase orders or
multi-seat quotes, write to info@techemv.it.

**Offline keys** — for air-gapped sites, send your Hardware ID (shown in
the activation dialog) to **info@techemv.it** and receive a signed key by
e-mail. No network access is ever required with this flow.

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
│   └── bridgelab-cli/        # Validation + anonymization CLI
└── .github/workflows/        # CI/CD pipelines
```

## Troubleshooting installs

**Windows Server 2022 / corporate desktops — WebView2 install fails with code `-2147012866`**

The error `0x800072EFE` (`WININET_E_CONNECTION_RESET`) means the
WebView2 bootstrapper could not download the runtime from Microsoft's
CDN — typically because the machine has no internet, sits behind a
firewall that blocks `msedge.api.cdp.microsoft.com`, or has Internet
Explorer Enhanced Security Configuration enabled (default on Windows
Server).

Two ways to fix:

1. **Use the offline installer** — from v0.2.0 onwards the standard
   BridgeLab installer ships with the WebView2 runtime embedded
   (~150 MB instead of ~13 MB). Download the latest from
   [Releases](https://github.com/techemv-srl/BridgeLab/releases).
2. **Install WebView2 manually first**, then run any older BridgeLab
   installer:
   <https://developer.microsoft.com/microsoft-edge/webview2/> — pick
   the **Evergreen Standalone Installer** for x64. Once installed
   system-wide, BridgeLab installers (online or offline) will skip
   the WebView2 step entirely.

**Per-user vs per-machine install (Windows)**

The installer asks where to install:

- **Current user only** — installs under
  `%LOCALAPPDATA%\Programs\BridgeLab`, no admin elevation required.
- **All users on this machine** — installs under
  `%PROGRAMFILES%\BridgeLab`, prompts for admin (UAC). Useful on
  shared workstations and Windows Server.

The choice is remembered for in-place upgrades.

## Contact

**TECHEMV SRL**
- Email: [info@techemv.it](mailto:info@techemv.it)
- Web: [www.techemv.it](https://www.techemv.it)

Licenses are bought from the [website](https://techemv-srl.github.io/BridgeLab/).
For invoices, purchase orders, multi-seat quotes, enterprise inquiries,
integration support and bug reports outside GitHub Issues, reach us at the
email above.

## License

Open Core — the core is MIT-licensed and free for commercial use.
Paid-tier feature implementations live in two clearly marked
directories — `src-tauri/src/pro/` and `src/lib/pro/` — licensed under
the Business Source License 1.1 (production use requires an active
subscription; each converts to MIT four years after publication). See
the root `LICENSE` file for the exact carve-out; everything published
before those directories existed remains MIT. Building the Rust backend
with `--no-default-features --features desktop` produces a Community-only
binary that compiles without the BUSL directories; `--no-default-features`
alone builds the headless core library, which is what `bridgelab-cli`
sits on — no Tauri, no WebKit, no BUSL code.

Copyright (c) 2026 TECHEMV SRL
