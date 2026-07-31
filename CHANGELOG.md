# Changelog

All notable user-facing changes to BridgeLab. Dates are UTC.

## [0.3.0] — 2026-07-31

### Added
- **HL7 value tables in the Field Inspector**. Selecting a coded field (PID-8 Administrative Sex, PV1-2 Patient Class, MSA-1 Acknowledgment Code, ORC-1 Order Control, OBX-11 Observation Result Status, MSH-9/11, PID-16/24/30, PV1-4, ORC-5, OBR-25, AL1-2, DG1-6) now shows the **allowed values with their meanings**, highlights the value currently in the message and warns when the current value is not in the table — non-standard codes surface at a glance before the receiving system rejects them. Deliberately partial tables (0076 Message Type) never produce false warnings.
- **Runnable test cases**. The Test Case Library gains "Expected message type" and "Expected validation" (valid/invalid) fields plus **Run check** per case and **Run all** on the filtered list: each case's content is parsed and validated for real (HL7 v2 or FHIR, auto-detected) and compared against its expectations. Pass/fail badge per row, failure detail in the case view, passed/total summary in the toolbar — the library is now a regression-test tool for interface changes, not just a snippet store.

## [0.2.5] — 2026-07-31

### Added
- **Welcome screen**: with no restored session the app now opens on an onboarding card — Open file (Ctrl+O), New from template (Ctrl+N), Test case library (Ctrl+L), User manual (F1), blank tab — with live shortcut hints and the recent-files list. Pasting an HL7 message anywhere creates the first tab.
- **Live status bar**: clickable validation summary (✖ errors / ⚠ warnings opens the panel, scoped to the active tab), modified-file dot, truncation badge click expands all truncated fields, "not parsed" hint when idle.
- **Keyboard-shortcuts cheat-sheet**: Help → Keyboard Shortcuts opens Settings directly on the Shortcuts section; the manual's shortcut table is now generated from the live bindings, so customizations show up and it can never drift again.
- **Manual (EN + IT)**: new sections for tree search, message compare, the listener console (including the 32 MB retained-content budget), character encoding; the connection-profiles section rewritten to match the shipped UI.

### Fixed
- **Editor settings now actually apply**: font size/family, word wrap (incl. "At Column"), minimap, line numbers, tab size and whitespace rendering were saved but never read — Monaco hardcoded everything. They now load at startup and apply live when Settings closes.
- **Rebinding a shortcut no longer triggers it**: pressing Ctrl+O to assign it used to also open the file picker.
- **Monaco robustness**: an initialization failure now shows an error panel with a Retry button instead of a permanently blank editor; FHIR tabs get JSON/XML syntax highlighting; the right-click menu follows language changes without a restart.
- **Validation panel**: fixed a possible crash with duplicate issues on the same rule/field, a filter dead-end when the last issue of the selected severity was fixed, and missing tooltips on long messages.
- **Dialog polish**: template picker and test-case library gained loading states, visible errors with Retry, Esc/Enter handling, autofocus, an unsaved-changes confirm on Cancel and a category auto-complete; the field inspector gained the position row, a copy button and visible errors; templates and test cases are now fully translated in all 5 languages (their translations existed but were never wired).

## [0.2.4] — 2026-06-10

### Added
- **Search inside the message** (message tree panel). New sticky search bar: case-insensitive matching on segment type (`PID`), schema field name (`Patient Name`) and field value — including fields in segments the tree hasn't expanded yet. Enter / Shift+Enter cycle matches, Esc clears, Ctrl/Cmd+F focuses the box when the tree has focus. Match-kind badges (`=` value, `Aa` schema name, `§` segment); clicking a result expands the segment, selects the field and scrolls it into view. HL7 v2 tabs only (FHIR resources live in a separate store).
- **Compare messages** (`Tools → Compare messages…`). Side-by-side read-only Monaco diff of any two open tabs with HL7 syntax highlighting: pick left/right, swap-sides button, Esc closes. Warns when fewer than two tabs are open.
- **Listener console** (Communication → MLLP). Rolling live log (newest first, 200 entries) of everything the MLLP listener receives: local time, peer address, payload bytes, ACK badge (green `AA` / red `AE`-`AR` / `—` when auto-ACK off), encoding used, first-line snippet. Click a row to re-open that message in a tab. Listener errors appear inline as red rows. New "Open received messages in a new tab" toggle (default on) lets you disable tab-spam during high-volume tests and cherry-pick from the console instead. Full message contents retained for click-to-open are capped at a 32 MiB rolling budget so unattended sessions can't exhaust renderer memory.

## [0.2.3] — 2026-05-04

### Added
- **User-selectable encoding for MLLP Send and Receive**. Both sides now expose a dropdown with the major encodings used in HL7 deployments — `UTF-8`, `ISO-8859-1` (Latin-1), `ISO-8859-2` (Central EU), `ISO-8859-15` (Latin-1 + €), `windows-1252`, `windows-1250`, `windows-1251`, `ASCII`. The listener decodes the inbound payload **and re-encodes the auto-ACK with the same charset**, so the peer doesn't see mojibake on its side. Send and Receive can be set independently. UI labels are localized in en/it/de/fr/es.

### Fixed
- **MLLP listener now accepts ISO-8859-1 payloads** (and other non-UTF-8 charsets) without the spurious `"could not unframe MLLP payload"` error that was firing whenever the upstream system used Latin-1 encoding.

## [0.2.2] — 2026-05-01

### Added
- **Persistent MLLP listener** with Start / Stop. Replaces the previous fire-and-forget single-shot `Listen for incoming` button: now binds the port and keeps accepting connections until you click Stop. Each incoming message opens in a new tab labelled `Inbox HH:MM:SS` so the message you were editing isn't overwritten. Status pill shows `Listening on {addr}:{port} · {N} received`.
- **Listener settings** (Settings collapsible): bind address (default `0.0.0.0`, switch to `127.0.0.1` to restrict to localhost), ACK code dropdown (AA / AE / AR for testing how the upstream system handles each ack class), per-connection read timeout, auto-ACK toggle.
- **📋 Incolla button** in the Activation Licenza dialog. Click reads the license key directly from the clipboard via the Clipboard API, bypassing Monaco — necessary because Monaco's global keyboard listeners intercept Ctrl+V even when the textarea is focused, so the keyboard paste landed in the active message tab instead of the dialog.

### Fixed
- **macOS-Intel build no longer hangs** on the release workflow. `runs-on: macos-13` was deprecated by GitHub on 2024-12-04 and the runner image was removed on 2025-12-01 — jobs pinned to macos-13 sat in `Queued` indefinitely. Cross-compile to `x86_64-apple-darwin` from a `macos-14` Apple Silicon runner instead. Output is still a normal x86_64 `.dmg` for Intel Macs.
- **Listener "ACK code" dropdown clipped** the localised options ("AA (accept)" rendered as "AA (accet"). Widened to fit the longest label.
- **License paste lands in the dialog regardless of editor state** (see Added — the new explicit Paste button replaces the unreliable focus-shuffling).
- **Activation modal focus** correctly steals from Monaco on mount via `blur()` + `requestAnimationFrame(focus())`, so Ctrl+V works at least when no Monaco tab is open. Clipboard button is the bullet-proof path for all other states.

### Chore
- Removed dead CSS rule `.intro kbd` in `ShortcutsEditor.svelte` — `svelte-check` warning count dropped from 20 to 19. No functional change.

## [0.2.1] — 2026-04-30

### Fixed
- **Trial-expired banner now dismissable**: when the 7-day trial elapsed and the licence transitioned to `free` (community fallback, fully usable), the red banner stayed up and could not be closed. The × button is now shown for `free` and `trial` (non-urgent) states, and the dismissal persists across restarts (scoped to the current `license_type`, so the banner reappears on real state transitions). Only `expired` (a real Pro/Enterprise licence that lapsed) and `trial` with ≤3 days remaining stay non-dismissable.
- **"Disattiva Licenza" no longer shown for Free users**: the button surfaced in the Activation modal even when the user had no licence to deactivate, doing nothing on click. Now it only appears for `professional` and `enterprise`. Free / Trial / Expired show the activation form instead.

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
