# Changelog

All notable user-facing changes to BridgeLab. Dates are UTC.

## [1.7.0] — 2026-09-22

### Security
- `quick-xml` 0.36 → 0.41 (RUSTSEC-2026-0194/0195: quadratic attribute
  checking and unbounded namespace allocation on crafted XML — the FHIR
  XML parser and the SOAP client read untrusted documents), `rustls`
  0.23.38 → 0.23.45 (RUSTSEC-2026-0285), and `plist` 1.8 → 1.10 so the
  copy Tauri pulls in moves off the vulnerable `quick-xml` too. The
  `bridgelab-cli` lockfile follows. `cargo audit` reports no advisories
  on either.

### Added
- **Every coded value now says what it means.** The 394 HL7 value
  tables (about 5,000 codes) ship with the app, imported from the same
  MIT-licensed source as the message catalogue, and each version's
  catalogue records which table every coded field and component draws
  from — 16 hand-written tables before, mapped to 14 fields. The tree
  shows the meaning next to the value (`M — Male`, `ADT — ADT message`,
  `F — Final results`), including components (MSH-9.2 from the
  event-type table, PID-3.5 from the identifier-type table); hovering a
  field in the editor explains its code, and auto-completion in any
  coded field offers the whole table instead of the three fields it used
  to know. The Field Inspector lists the table for the selected field
  *or component* and now distinguishes an HL7-defined table (`ID`
  fields: a value outside it is non-standard, warned) from a
  user-defined one (`IS` fields: suggestions, never warned) — the old
  "value not in table" warning fired on user-defined tables too.
- **Field metadata for every standard segment, per version.** The
  hover, auto-completion, Field Inspector and validation used to read a
  hand-written list of 15 segments that was the same for every HL7
  version; they now read the full catalogue of the version the message
  declares, so RXA-5 has a name, PID-8 in a v2.1 message has a v2.1
  definition, and the built-in validation checks required fields and
  lengths for every segment the standard defines, not just those 15.
  Expect a few more findings on messages that were "clean" before: an
  ORU without OBX-11, a PV1-16 holding free text.
- **ACK filters with counts** on the listener console and the send
  history: AA / AE / AR / no ACK / errors (failed), each chip showing
  how many rows match, so "AE 12" out of 300 stands out before anyone
  scrolls. An MLLP send now records the ACK code the receiver answered
  with (MSA-1) and shows it as a badge on its history row — a send that
  reached the peer and got an `AE` back is "OK" at the transport level
  and a rejection at the application level, and the row now tells the
  two apart.

### Changed
- **What Community gets of plugins is now stated everywhere the same
  way.** The README, the landing page, `docs/PLUGINS.md` and the manual
  (five languages) all say it: every pack kind — HL7 v2 rules, FHIR
  rules, PHI fields — and every check type runs in every tier; the only
  cap is 3 packs active at once in Community (10 saved test cases),
  unlimited in Pro. "Plugins (limited)" and "Plugin packs (basic)" read as
  if the rules themselves were cut down. The manual's plugin chapter also
  gained the `fhir/` folder it had been missing.
- **A release now ships 12 assets instead of 19, about 450 MB instead of
  1.4 GB.** The five per-language MSIs are one `en-US` MSI — the
  installer language only affects the installer's own dialogs, the app
  is multilingual either way, and the NSIS `setup.exe` already carries a
  language selector; the MSI stays for managed deployment. The two
  macOS `.app.tar.gz` updater bundles are gone: nothing consumed them —
  release artifacts are not signed, there is no `latest.json`, and
  *Help → Check for updates* has always fallen back to comparing against
  the latest GitHub release, which it still does. The AppImage no longer
  bundles GStreamer, which BridgeLab never used: 78 MB instead of 91 —
  the rest is the WebKitGTK and GTK stack it carries to run on any distro.

## [1.6.0] — 2026-09-17

### Fixed
- **A Bundle was validated as a Bundle and nothing else.** The built-in
  FHIR checks ran on the root resource only, so a message Bundle whose
  Observation had no `status` reported two notes about the Bundle itself
  and ✗ 0 ⚠ 0 — the opposite of what was true. Every entry and every
  `contained` resource is now checked as the resource it is, reported
  under its path (`entry[3].resource.status`). With a profile package
  installed, each entry is also validated against its own
  `meta.profile`, not just its base type.
- **Bundle rules the specification states are now checked**: every entry
  carries a fullUrl (a POST request excepted); a persistent fullUrl agrees
  with the resource's own type and id — while a `urn:uuid:` fullUrl is
  the resource's identity and needs no id, so a message Bundle written
  the NHS/IHE way stays clean; fullUrls are unique outside history
  bundles; and every Reference inside the Bundle resolves — to an entry's
  fullUrl, a `Type/id`, or a `#contained` — with a warning in message,
  document, transaction, batch and collection bundles and a note in a
  searchset, which may legitimately return part of a graph. A document
  may not reference outside itself.
- **`resolve()` did not follow `urn:uuid:` references.** FHIRPath's
  `resolve()` matched only `Type/id`, so in a message Bundle — where every
  reference is another entry's `urn:uuid:` fullUrl — it found nothing.
  An entry is now reachable by its fullUrl as written.
- **A missing profile hid every other FHIR finding.** When `meta.profile`
  named a profile that was not installed, the validator reported that and
  stopped — the base definition never ran, so a misspelled element in the
  same resource went unreported. The missing profile is still reported;
  the remaining checks now run alongside it.
- **A version pin in `meta.profile` was ignored.** `…/StructureDefinition/X|1.2.0`
  validated against whichever `X` was installed, and with two versions of
  a package installed the one that won was decided by load order. A pinned
  canonical now resolves to exactly that version; when it is not installed
  the report says so and names the versions that are, instead of quietly
  substituting one. An unpinned reference, and the base definition of a
  resource type, use the newest installed version.
- **Double-clicking a `.hl7` file did not open BridgeLab on Linux** —
  shipped in the 1.5.0 packages, recorded there.
- **A long FHIR path overlapped the message in the validation panel.**
  The location column was sized for `PID-5`; a Bundle finding's path
  painted over the text beside it. It now ellipsises past 280 px, with
  the full path in the tooltip. (Binaries rebuilt on the same tag.)

### Added
- **The FHIR R4 core is built into the binary.** Profile conformance
  used to need `hl7.fhir.r4.core` installed by hand before it did
  anything; the distilled index (about 200 KB compressed) now ships
  inside BridgeLab, so cardinality, element types, choice elements and
  unknown-element checks run against the base R4 definitions on a fresh
  install, offline, in every tier — nothing to download. The package
  manager lists it as built-in; a copy you install yourself replaces its
  definitions, a newer version outranks it, and implementation guides
  install on top as before (installing packages stays Pro). With the
  core always present, the hand-written Observation `status`/`code` and
  Patient `birthDate` checks were redundant with it and are gone — one
  defect, one finding; the gender value check stays, because terminology
  is the one thing the definitions do not carry.
- **Primitive values are checked for their lexical form** during profile
  conformance, with the specification's own patterns: `date`, `dateTime`
  (a time needs a time zone), `instant`, `time`, `code` (no stray
  whitespace), `id`, `oid`, `uuid`, URIs without whitespace, `positiveInt`
  and `unsignedInt` ranges. One check goes beyond the letter of the
  specification, on purpose: an endpoint — `MessageHeader.source.endpoint`,
  `destination.endpoint`, `Endpoint.address` — written without a scheme
  (`https//host`) is reported as a warning, because nothing can connect to
  it as written. It is confined to endpoints: the core package itself
  keeps bare type names in a `url`-typed extension, and those are fine.
- **`bridgelab-cli` is now the app's own validators, headless.** The old
  CLI was a separate, minimal reimplementation — structure and MSH checks
  only, nothing else — while the manual called it "the same validator".
  It is now a thin front-end over the library the desktop app is built
  on: the same version-aware HL7 v2 validation, the same FHIR checks with
  Bundle entries, references, the built-in R4 core and installed
  packages, the same plugin packs and PHI anonymiser. `validate` and
  `batch` detect HL7 v2 or FHIR per file; `--fhir-packages DIR` points at
  a provisioned package folder for CI and air-gapped sites. The CLI reads
  no licence and is built without the `pro` feature: it behaves as the
  Community edition, by construction. Binaries ship with every release as
  `bridgelab-cli-<target>`. Under the hood the library gained a `desktop`
  feature that holds everything needing a window, so the CLI compiles
  without Tauri or WebKit.
- `cargo run --example validate-fhir -- <file>` validates a resource
  headlessly the way the app does, including conformance against the
  installed packages.

### Changed
- **Activation dialog describes the tiers by what they gate.** "Basic
  MLLP/HTTP" and "full HTTP" are replaced with the actual split: MLLP send
  and HTTP GET are Community; the listener, HTTP write methods and any
  `Authorization` header need Pro. The welcome card for XSD export no longer
  carries a blanket PRO badge — four v2.5 messages export in every tier —
  and the manual's tier table gained the XSD row it was missing.

## [1.5.0] — 2026-09-12

### Added
- **FHIR profile validation against installed StructureDefinitions.**
  Until now `meta.profile` was listed and left unchecked — the official
  HL7 validator is a Java tool, and requiring a JRE would undo the point
  of an offline desktop app. BridgeLab now reads FHIR NPM packages
  directly and validates against their StructureDefinitions in Rust.
  - **Tools → FHIR profile packages…** installs a `.tgz`
    (`hl7.fhir.r4.core` for the base definitions, then national or
    site-specific implementation guides). Installing distils the package
    once — the R4 core is 4581 files — so startup reads a compact index
    rather than the archive.
  - Every FHIR validation then also checks **cardinality** (a required
    element missing, a `0..1` element repeating), **element types**,
    **choice elements** (`value[x]` must appear as exactly one of
    `valueQuantity`, `valueString`, … — a plain `value` or two forms at
    once is reported), **fixed values and patterns**, and **unknown
    elements**, which catches a typo like `genderr` or an element
    belonging to a different resource type.
  - Profiles declared in `meta.profile` are applied automatically when
    the defining package is installed; when it is not, the validator says
    so instead of quietly reporting a clean result. When profiles did
    run, the report says that too.
  - Terminology stays out of scope: a `required` binding can only be
    checked by expanding the ValueSet, so bindings are left unchecked
    rather than half-checked.
  - Verified against hl7.fhir.r4.core 4.0.1: all **4578 resources in the
    package validate with 14 findings, every one of them genuine** — 13
    `SearchParameter` files that really do omit the required `base`, and
    one ValueSet declaring a profile the core package does not contain.
    `scripts/fetch-fhir-core-package.sh` sets the test up.
  - Installing a package requires a Professional license; packages
    already installed keep validating in every tier, so a lapsed trial
    never turns previously clean resources red.
- **Custom FHIR validation rules, with a builder.** The declarative plugin
  packs that already extend the HL7 v2 validator now cover FHIR too:
  `<config>/BridgeLab/plugins/fhir/*.json`. A rule is either a FHIRPath
  invariant that must hold (`identifier.exists()`, the way FHIR writes its
  own constraints) or a FHIRPath selector plus a check on the values it
  picks up — must be present, how many, matches a pattern, one of a list,
  contains text, length bounds. Rules run alongside the built-in checks on
  every FHIR validation, still with no code execution.
  - **Tools → FHIR validation rules…** (Pro) edits them in-app: it
    validates the FHIRPath before saving and can run a rule against the
    resource in the active tab, showing which values the selector actually
    matched. A rule scoped to a type the open resource is not says so
    rather than reporting a pass.
  - A rule scoped to `Patient` also fires for the Patients inside a
    Bundle, reported against `entry[n].resource.…`.
  - A rule whose FHIRPath fails to evaluate is reported as a finding
    instead of being silently skipped.
  - Reference pack: `examples/plugins/fhir/sample-fhir-rules.json`; schema
    in [docs/PLUGINS.md](docs/PLUGINS.md). Rules run in every tier under
    the existing plugin-pack cap — only the editor is Pro.
- **The FHIRPath evaluator now implements the FHIRPath 2.0 language.** It
  was a path walker that understood navigation, `[n]`,
  `where(field = 'value')` and a handful of functions; anything else
  failed. It is now a real tokenizer, parser and evaluator with the
  specification's full operator set and precedence, three-valued boolean
  logic, partial-precision date/time literals, quantities with unit
  conversion, `$this`/`$index`/`$total`, the `%resource` / `%ucum` /
  `%ext-…` constants, and around seventy functions — including `sort()`,
  `repeat()`, `aggregate()`, `iif()`, `ofType()`, the string and math
  libraries, `extension()`, `hasValue()` and `resolve()`, which follows a
  Reference to a contained or bundled resource.
  - Comparing values of different precision now returns the empty
    collection instead of a guess: `@2015-02-04 = @2015-02` is neither
    true nor false.
  - `trace('label')` passes its input through and shows what it captured
    under the result, so a long path can be inspected halfway along.
  - Errors name the problem — an unknown function, a missing bracket, an
    operator given a collection where it needs one value — instead of
    silently returning nothing.
  - Verified against the **official HL7 FHIRPath test suite**: 836 of the
    922 runnable cases pass. `scripts/fetch-fhirpath-suite.sh` downloads
    it and `cargo test --test fhirpath_suite` runs it. The remaining gap
    is `lowBoundary()`/`highBoundary()` (exact decimal arithmetic),
    compound units, and cases that need the StructureDefinitions loaded.
- **Three more HL7 versions in the schema catalogue: v2.1, v2.2 and
  v2.5.1.** v2.5.1 is the baseline most US interfaces are written
  against, and v2.1/v2.2 cover legacy feeds still in production. The
  catalogue now spans **10 selectable versions and 2,320 message
  structures** — XSD export, the schema-aware tree and the Field
  Inspector all read from them.

### Fixed
- **HL7 v2.7.1 exported v2.7 data under its own name.** The shipped
  v2.7.1 payload was a byte-identical copy of the v2.7 one. v2.7.1 is a
  technical-correction release of v2.7 and genuinely carries the same
  message definitions, so it is now declared an alias: it is marked
  `(= v2.7)` in the version dropdown, exports from the v2.7 catalogue,
  and no longer embeds a duplicate 2 MB payload in the binary.
- **Editor autocomplete and hover always described fields against HL7
  v2.5**, whatever version the open message declared. Both now read
  MSH-12 and look the field up in the matching catalogue, so a v2.3 or
  v2.7 message gets that version's field names, data types and lengths.
- **Double-clicking a `.hl7` file did not open BridgeLab on Linux.** The
  desktop entry claimed `application/hl7-v2`, but nothing on the system
  defined that type, so no file ever matched it. The `.deb` and `.rpm`
  now ship a shared-mime-info definition registering the type with a
  `*.hl7` glob and an `MSH|` content rule — the latter so a message saved
  without an extension is still recognised. The MIME database is rebuilt
  by the package manager on install.

### Changed
- **Windows installer artwork.** The NSIS sidebar and header images still
  carried the pre-1.0 bridge mark; they are regenerated from the unified
  About-style mark used by the app icon, the About dialog and the website.
  The app's web favicon (`static/favicon.*`) is aligned to the same mark.
- **Windows uninstaller icon.** The uninstaller window showed the generic
  NSIS icon in its title bar because no uninstaller icon was configured;
  it now uses the BridgeLab icon and the same header image as the
  installer.

## [1.4.1] — 2026-09-10

### Fixed
- **Tools → Compare messages… did nothing.** The client bundle was
  picking up Svelte's *server* lifecycle stubs for components importing
  from `svelte` (the Vite 5-era `@sveltejs/vite-plugin-svelte` 4 dropped
  the `browser` resolve condition under Vite 6), so the diff dialog threw
  on open instead of rendering. Upgraded the plugin to the Vite 6 line;
  the compare dialog now opens with the side-by-side diff of two tabs.
- **Field Inspector stuck on "Generating…" with FHIR documents.** The
  inspector kept the node selected in a previous HL7 tab (e.g. "MSH (0)")
  when switching to a FHIR tab, and waited forever for an HL7 v2 schema
  lookup that never runs for FHIR elements. The selection is now cleared
  on every tab switch, FHIR elements show their path with a clear note
  instead of a spinner, and the panel's loading label no longer borrows
  the XSD export's "Generating…" text.
- **FHIR files could not be opened from disk.** File → Open, a file
  passed on the command line / "open with", and the recent-files list all
  went through the HL7 parser, so a FHIR JSON or XML resource failed with
  "Message does not start with MSH" even though the open dialog offers a
  "FHIR Resources" filter. Files are now routed by content to the right
  parser.
- **Trial and license days rounded down.** A freshly started 14-day trial
  showed "13 days remaining" and a license expiring later today showed
  "0 days"; remaining days are now rounded up, and a license is treated
  as expired at its actual expiry instant instead of up to a day later.
- **FHIR tree nodes did not expand.** Clicking a container node of a FHIR
  resource (e.g. `name [2 items]`) asked the HL7 backend for its children
  and silently failed with "Message not found"; the tree now uses the
  FHIR command for FHIR documents.
- **FHIRPath results hidden.** In the default bottom-panel height the
  result list ended up in a 36-pixel box below the summary line, so the
  evaluated values were not visible without scrolling that box. Recent
  expressions now share the examples row, the results area keeps a
  usable minimum height and the panel scrolls as a whole.
- **Monaco JSON worker.** FHIR JSON tabs handed the generic editor worker
  to the JSON language service, which logged "undefined is not an object
  (evaluating 'require.toUrl')" and left JSON validation/formatting off.
  The JSON worker is now bundled and used for JSON models.

## [1.4.0] — 2026-09-01

### Added
- **SOAP client (Enterprise)**. New SOAP tab in the Communication panel
  for SOAP 1.1/1.2 endpoints (IHE-style middlewares, regional gateways,
  legacy hospital web services): envelope building with a default or
  custom template (`{payload}` placeholder), optional WS-Security
  UsernameToken and WS-Addressing headers, correct per-version content
  types, and response parsing with Body extraction and SOAP Fault
  decoding for both versions. Connection profiles and request history
  cover SOAP like MLLP and HTTP. WSDL import is planned as a follow-up.

### Changed
- **Dual-license layout**. The repository now carves out two directories
  (`src-tauri/src/pro/`, `src/lib/pro/`) under the Business Source
  License 1.1 for paid-tier feature implementations; everything else
  remains MIT, and all code published before this change stays MIT.
  Building with `--no-default-features` produces a Community-only binary
  without the BUSL directories.

## [1.3.1] — 2026-08-21

### Added
- **One-click license renewal pickup**. For licenses activated online, the
  License dialog gains an **Update license** button: it re-activates with
  your stored code (the seat is reused, never consumed twice) and shows the
  new expiry immediately — no more deactivate-and-re-paste. On top of that,
  within 14 days of expiry (or past it) the app silently picks a renewed
  expiry up by itself at startup, at most once a day, with every network
  failure ignored — fully offline installations notice nothing.

## [1.3.0] — 2026-08-20

### Added
- **Online activation codes**. Buy a license, receive a short code
  (`BL-PRO-XXXX-XXXX-XXXX`) by e-mail and paste it in the activation dialog:
  the app exchanges it once over HTTPS for a signed license bound to your
  machine — after that everything works offline, exactly like before.
  Offline signed keys remain available for air-gapped sites, and existing
  keys keep working untouched. *Deactivate* now also frees the seat on the
  license server (best-effort — local removal always succeeds).
- **Opt-in usage statistics (default OFF)** with a new
  **Settings → Privacy** section: a toggle, a "Show what is sent" preview of
  the exact JSON payload, and a "Send now" button. When enabled, BridgeLab
  automatically sends — at most once a day — usage counters plus app
  version, OS, license tier, a random installation ID and (for
  online-activated licenses) the activation code. The data is pseudonymous:
  no message content, file names, host names or personal data are ever
  sent, and nothing is sent at all while the toggle is off.
- **Pro trial extended to 14 days** (was 7) — a more realistic evaluation
  window for hospital integration teams. Extensions to 30 days remain
  available on request via info@techemv.it.

## [1.2.0] — 2026-08-03

### Added
- **Insert missing segments from the structure tree**. In the full standard-structure view, right-click a grayed expected-segment row → *Insert segment*: the segment's skeleton is added to the message at its standard position, with separators up to its last required field, and the message re-parses immediately.
- **Discover BridgeLab cards** on the welcome screen: four compact cards open the distinguishing features directly — test-message generator, anonymization, MLLP listener, XSD export — with PRO badges on the licensed ones.

### Fixed
- **Help → Check for updates now works**. It compares the installed version with the latest release and, with your confirmation, takes you to the download (in-app download and restart will activate automatically once release signing is enabled). The old menu entry silently did nothing — external links never opened from inside the app, which also silenced the trial banner's "Compare plans" link; both open in your browser now.

## [1.1.0] — 2026-08-03

### Added
- **Full standard-structure view**. With `View → Show standard fields` on, the Message Structure tree now also shows the segments the standard defines for the message type but that are absent from the message — grayed ghost rows at their standard position, annotated with group path, required/optional, repeating and choice status. Ghost segments expand into their complete field list, and composite fields expand into components (e.g. OBX-16 → XCN.1 ID Number, XCN.2 Family Name). Field placeholders inside present segments are now correct for every shipped HL7 version and expand into components too.
- **Open with / double-click integration**. BridgeLab registers the `.hl7` extension: double-clicking a file opens it in the running instance as a new tab (single-instance — no more second window), or starts the app with the file open. Multiple files dropped or opened land as tabs in order.
- **Native drag & drop**. Dropping files onto the window now works — anywhere, including the welcome screen. (The desktop webview never received browser-style drops; the app now listens to the native drop event with real file paths.)

### Fixed
- **File-open failures are visible**. Errors while opening a file used to be written into the active tab — from the welcome screen (no tabs) they were invisible, making a failing open look like a dead button. Failures now surface in an error dialog with the underlying reason, everywhere.
- **Bottom panels no longer stack off-screen**. Validation, Communication and FHIRPath now share one resizable container as tabs: always fully visible at any height, the Communication panel keeps its listener console when in a background tab, and opening a panel from menu/shortcut/status bar brings its tab to the front.
- **First edit after opening a file re-parses correctly** (the auto-parse mute set by file open could swallow the first user edit).

## [1.0.0] — 2026-08-02

### Added
- **Batch anonymization** (`Tools → Batch anonymize…`, Pro). Mask a whole folder of `.hl7`/`.txt`/`.dat` files in one pass with the same pipeline as the interactive dialog (built-in PHI catalogue + active plugin rules), written as copies into an output folder. Originals are never touched: the tool refuses to overwrite any selected source, and same-named inputs from different folders get numeric suffixes. Per-file masked-PHI counts and errors; same 5000-file / 10 MB caps as batch validation.
- **Bundle reference graph**. The FHIR Bundle visualizer gains a List / Graph toggle: entries become nodes colored by resource type, references become directed arrows, clicking a node drives the detail pane. Available up to 150 entries.
- **FHIR templates**. `File → New from template` includes a FHIR category: minimal Patient, blood-pressure Observation with components, and a transaction Bundle wired via `urn:uuid` references (real RFC 4122 UUIDs).
- **ACK generator**. The Communication panel builds an ACK for the current message: pick AA/AE/AR, the control ID is read from MSH-10 (honoring a custom MSH-1 separator), the result opens in a new tab. Messages without MSH-10 are refused instead of producing an uncorrelatable ACK.
- **User manual in five languages**. The in-app manual (F1) is fully translated into French, Spanish and German alongside English and Italian — previously those languages showed English content — including localized UI mockups. All five manuals document every feature through 1.0.
- **`meta.profile` awareness**. FHIR validation lists declared canonical profile URIs (all URI schemes accepted) as info findings and flags malformed declarations; profile conformance itself is intentionally not claimed.

### Changed
- Trial banner: new "Compare plans" link; dismissal now persists in the preferences profile (migrating any previous browser-storage value).

### Fixed
- Release builds compile warning-free (three Windows build warnings eliminated).
- Removed the dead single-shot MLLP receive command and 52 unused translation keys.
- Batch anonymization hardening from review: destinations are checked against all selected sources and output names are reserved globally — no scenario can silently overwrite an original or a previous result.

## [0.6.0] — 2026-08-02

### Added
- **Six more HL7 versions in the XSD export**: complete catalogues for **v2.3, 2.3.1, 2.4, 2.6, 2.7 and 2.7.1** join the existing 2.5 — **1,964 message structures** across the seven supported versions, generated from the MIT-licensed hl7-dictionary with referential-integrity validation. Pick the version in the export dialog; v2.5 remains the free tier (ADT^A01, ADT^A40, ORM^O01, ORU^R01), the other versions are Pro.

### Changed
- **Community plan limits are now enforced**: the free tier keeps up to **3 active validation/anonymization plugins** and **10 saved test cases**. Nothing is ever locked or deleted — test cases saved beyond the limit (e.g. during a trial) stay visible, editable and runnable; only *new* saves and plugin activations beyond the cap ask for an upgrade. Plugins over the limit show an explanatory badge in Settings → Plugins, and disabling one frees the slot immediately. Trial and Pro/Enterprise remain unlimited.

### Fixed
- **Licensing tiers hardened**: a valid trial could in principle pass Enterprise-only feature gates (none are used by shipped features yet, so no practical impact); trials now get exactly the Pro feature set, matching the documented tiers. The whole tier matrix is covered by regression tests.
- **About dialog version**: Help → About showed a hardcoded "Version 0.1.0" regardless of the installed release; it now displays the real application version.
- **Exported XSDs now compile under strict schema processors**: a few HL7 structures repeat a segment across an optional-only window (e.g. DFT^P03's two ROL positions), which violates XSD's Unique Particle Attribution rule and made standard compilers reject the schema. Affected sequences are now emitted as an annotated unordered choice that accepts every valid message; group names containing `/` (v2.7) are sanitized to valid XML names. All 1,964 exportable schemas verified against a strict external compiler.

## [0.5.0] — 2026-07-31

### Added
- **Full HL7 v2.5 message catalogue**. The XSD export now covers the entire v2.5 standard — **248 message structures, 149 segments, 78 composite data types** (was 4/32/49) — imported from the MIT-licensed hl7-dictionary via the new official converter in `tools/hl7-schema-importer`. Nested groups and choice blocks (e.g. ORM order detail) are fully represented. Free tier unchanged (ADT^A01, ADT^A40, ORM^O01, ORU^R01); the remaining 244 messages are the Pro tier's catalogue.
- **FHIR XML validation**. XML resources are now converted to the canonical JSON model (value attributes, repeated-element arrays, Bundle resource containers, primitive extensions, XHTML narrative handling) and validated with the same rule set as JSON — closing the "JSON only" limitation noted in v0.4.0. Bundle analysis and FHIRPath work on XML resources too.

## [0.4.0] — 2026-07-31

### Added
- **Batch validation** (`Tools → Batch validation…`, Pro). Validate a whole folder (or a picked set) of `.hl7`/`.txt`/`.dat` files in one pass: one row per file with message type, version, segment count and error/warning totals — active plugin rules included, same pipeline as interactive validation. Only-failures filter, passed/failed summary, spreadsheet-safe CSV export, click a row to open the file in the editor. Files are processed in memory; caps at 5000 files and 10 MB per file.
- **Test-message generator** (`Tools → Generate test messages…`). Creates syntactically valid ADT^A01/A08, ORU^R01, ORM^O01 (or mixed) messages with plausible synthetic patient data — names, birth dates, MRNs, addresses and lab panels with reference ranges, including a realistic share of flagged abnormal results. No real PHI. Seeded runs are reproducible; open the results in tabs or save them to a folder as numbered `.hl7` files.
- **Real FHIR validation (JSON)**. `Validate` (F6) on a FHIR JSON resource now runs the actual FHIR rule set (resourceType, id, per-resource field checks) and lists the findings in the validation panel with their JSON paths — replacing the previous "parsed successfully" stub. FHIR XML validation is planned; XML resources currently parse and display but are not rule-checked.

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
