# Changelog

All notable user-facing changes to BridgeLab. Dates are UTC.

## [0.2.0] — 2026-04-29

### Added
- **XSD schema export** (`Tools → Export message schema as XSD…`). Generates standards-compliant XSD files for HL7 v2.xml message types, ready to drop into Astraia / BizTalk / XMLSpy and other XML-based integration engines. Free tier: ADT^A01, ADT^A40, ORM^O01, ORU^R01 in v2.5. Pro tier: full message catalogue (planned for incremental shipment via the new `hl7-schema-importer` tool).
- **HL7 schema importer** (`tools/hl7-schema-importer/`): build-time tool that ingests HL7 v2.x schema definitions from external sources (hl7-dictionary today, HAPI / official v2.xml XSDs / CSV tables planned). Round-trips and validates BridgeLab JSON payloads.
- **Per-machine Windows install option**: the installer now offers Current user (no UAC, `%LOCALAPPDATA%`) or All users (UAC prompt, `%PROGRAMFILES%`). Required for shared workstations and Windows Server scenarios.
- **Branded NSIS wizard**: header banner + sidebar BMPs generated from the brand icon, regenerable via `scripts/gen_nsis_banners.py`.
- README troubleshooting section covering WebView2 install failures and the new per-user / per-machine choice.
- Contact section in the README pointing to `info@techemv.it` and `www.techemv.it`.

### Changed
- **Offline WebView2 runtime** baked into the Windows installer (~150 MB) instead of the previous online bootstrapper (~13 MB). Fixes installer aborts on Windows Server 2022 / corporate desktops behind firewalls or with IE Enhanced Security Configuration enabled (error `WININET_E_CONNECTION_RESET / 0x800072EFE`).
- Auto-updater endpoint moved from the private dev repo to `techemv-srl/BridgeLab` so updates are served from the public release feed.
- Installer copyright string now includes the contact email; bundle homepage points to `www.techemv.it` instead of the private repo URL.
- Schema-data architecture migrated from hand-coded Rust to JSON loaded via `include_str!`. Same on-the-wire output, but the importer tool can now refresh the dataset without touching Rust code.

### Fixed
- **MLLP graceful close**: `send()` and `receive_one()` now read until the MLLP terminator (FS CR) and call `shutdown()` before dropping the stream. Prevents the peer (HAPI / Mirth / similar) from logging `Connection reset by peer` immediately after sending its ACK. Includes a 1 MiB cap on response buffer growth to defend against misbehaving peers that stream without a terminator.
- **XSD export error semantics**: an unknown `message_code` now returns `Message 'X' not found` regardless of license tier, instead of the previous `UPGRADE_REQUIRED` for Free users — which masked input bugs and made client error handling license-dependent for the same invalid request.
- **`test_trial_days`**: pre-existing red test on `main` updated to assert the 7-day trial introduced in 3c6b28d (was still asserting 30 days).

### UI polish
- XSD export dialog: proper modal styling for the three footer buttons (Copia / Salva con nome… / Chiudi). Previously rendered as flat text without borders/padding.
- View menu: "Mostra campi dello standard", "Struttura Messaggio" and "Ispettore Campo" now show a ✓ glyph reflecting their on/off state.

### Documentation
- New "Schema Export (XSD)" section in the in-app manual (EN + IT) covering the generator output, free/Pro split, and licensing stance ("derivative work for interoperability"; HL7-copyrighted material is *not* redistributed).
- Landing page (`docs/site/`): new feature card for XSD export and a new "XSD export" column in the comparison table.
- ROADMAP: XSD export marked delivered in Q2 2026; follow-up items track the full v2.5 catalogue and the v2.3/2.4/2.6/2.7/2.8 expansions.
- INTERNAL.md sync block: PowerShell pre-flight as the canonical form (with bash kept as an "equivalent" for Linux/macOS dev boxes).

## [0.1.0] — 2026-04-23

Initial public-facing release on `techemv-srl/BridgeLab`.

- HL7 v2.x parser (SIMD), FHIR JSON/XML parsing.
- Smart truncation for 5-10 MB messages with base64 payloads.
- MLLP client + listener (Pro), HTTP client (GET community / mutate auth Pro).
- 21 PHI-field anonymization across PID/NK1/IN1/GT1.
- FHIR Bundle visualizer + FHIRPath evaluator (Pro).
- 5-language UI (EN, IT, FR, ES, DE).
- Ed25519-signed offline license verification + 7-day trial.
- Plugin packs (declarative JSON validation + anonymization).
- Cross-platform installers (Windows NSIS + MSI, macOS DMG, Linux .deb / .rpm / .AppImage).
- File association for `.hl7`.
