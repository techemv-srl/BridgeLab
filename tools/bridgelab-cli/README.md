# BridgeLab CLI

The BridgeLab validators from the command line — for CI pipelines, batch
screening and scripts.

It is a thin front-end over the desktop app's library: the **same** HL7 v2
parser and validator (version-aware, from MSH-12), the **same** FHIR checks
and profile engine (the built-in FHIR R4 core, plus any package you install),
the **same** plugin packs and PHI anonymiser. What the app reports, the CLI
reports.

**Licence.** The CLI is free and reads no licence: batch validation, PHI
masking, FHIRPath and installed FHIR packages all work without one. Two
limits mirror the Community edition: the cap on active plugin packs, and
the XSD export, which offers the four common v2.5 messages — the full
catalogue is part of BridgeLab Pro. It is built without the `pro` feature.

**Plugin packs.** The CLI reads the app's plugins folder and the on/off
choices made in the app's Settings → Plugins. A pack that does not load, a
pack left inactive by the cap (at most 3 run, the ones that appeared first)
and a pack with warnings (a pack with no rules under its key, such as
`"rules"` instead of `"validation_rules"`, is one) are each named on
standard error, so a pack missing from a CI run is never silent; HL7 v2
reports also carry the app's `PLUGIN-CAP` info note when packs are over the
cap. The CLI only reads that folder: it creates no folder and records
nothing in the app's plugin state.

**Standard input.** Every command that reads one message accepts `-` as the
file name: `cat message.hl7 | bridgelab-cli validate -` (once per command:
standard input can be read only once).

**Paths and patterns.** `validate`, `info` and `test` take files and glob
patterns. A path that exists is read as it is, even when its name holds
glob characters (`msg[1].hl7`); a file matched by two patterns is read
once. Patterns ignore case on every OS: `"*.hl7"` also finds `ADT.HL7`,
and on Linux a file whose name is not valid UTF-8 is matched too. A path or
pattern that matches no file is never skipped: it is reported on standard
error and in the output (a `NOT FOUND` entry, a JSON row with
`parse_error`, a JUnit failure) and the command exits 1. So is a folder
given where a file is expected (use `batch` for a folder) and a folder a
pattern runs through that cannot be read (`INPUT ERROR`).

**Quoting.** Put patterns and arguments holding `^` in double quotes:
`"*.hl7"`, `"ORU^R01"`. Windows `cmd.exe` strips an unquoted `^` and keeps
single quotes as part of the argument, so `'*.hl7'` matches nothing there.

**Terminal output.** Text output never passes control characters from a
message or an ACK to the terminal: they are shown as `\xNN` (an escape
sequence in a field could otherwise clear the screen or draw a fake
result). JSON output escapes them as JSON does.

## Installation

Prebuilt binaries are attached to every
[release](https://github.com/techemv-srl/BridgeLab/releases) as
`bridgelab-cli-<target>` (Windows, macOS Intel and Apple Silicon, Linux).
The Linux binary (`x86_64-unknown-linux-gnu`, built on Ubuntu 22.04) needs
glibc 2.34 or newer — Ubuntu 22.04, Debian 12, RHEL/Rocky/Alma 9 or later;
on an older system build it from source. Only the two most recent releases
keep their assets, so download from `latest` rather than pinning an old
version's URL.

From source (needs Rust; no Tauri or WebKit — the desktop shell is not
compiled):

```bash
cd tools/bridgelab-cli
cargo build --release          # → target/release/bridgelab-cli
```

## Commands

Input files may be UTF-8 (with or without a byte-order mark), UTF-16 with a
byte-order mark, or a legacy charset: a file that is not UTF-8 is read in
the charset its MSH-18 declares, or as Windows-1252 when MSH-18 is empty,
exactly as the app opens it. The table 0211 charsets read are ASCII, the
ISO 8859 parts (`8859/1` … `8859/9`, `8859/15`), `UNICODE UTF-8`, `BIG-5`,
`GB 18030-2000` and `KS X 1001` (as EUC-KR). A message that declares one
BridgeLab cannot decode (`CNS 11643-1992`, `ISO IR14`, `ISO IR87`,
`ISO IR159`, or UTF-16/UTF-32 without a byte-order mark) is refused with
the reason by `validate`, `info`, `batch`, `to-json`, `anonymize` and
`send` (exit 1): in a multi-byte charset a byte of a character can equal a
delimiter, so its fields cannot be trusted. A message that declares UTF-8
but is not valid UTF-8 is read as Windows-1252 with a `CHARSET` warning
(`validate`) or a note on standard error.

### `validate` — HL7 v2 or FHIR, detected per file

```bash
bridgelab-cli validate message.hl7
bridgelab-cli validate "./messages/*.hl7" bundle.json
bridgelab-cli validate message.hl7 --format json
bridgelab-cli validate "**/*.hl7" --format junit > results.xml
```

- HL7 v2: structure (a second MSH in one text is reported, STRUCT-004),
  MSH, required fields, lengths (in characters) and data types against the
  catalogue of the version in MSH-12, plus any active plugin rules. An
  MSH-12 version BridgeLab has no catalogue for (a future v2.8, a typo) is
  validated against the catalogue of its major.minor version, or v2.5, and
  an info note (MSH-005) names the one used.
- FHIR (JSON or XML): the built-in checks on the root **and every Bundle
  entry and contained resource**; Bundle rules (fullUrl, identity,
  references that must resolve); conformance against the built-in R4 core
  and installed packages, including each resource's own `meta.profile`;
  primitive formats; plugin FHIRPath rules. A resource type no package
  defines is reported as *not checked* (info, `PROFILE-NOT-CHECKED`), a
  declared profile that is not installed as `PROFILE-NOT-INSTALLED`
  (warning) — never as clean. JSON input that is not a FHIR resource
  (`package.json`) or does not parse is reported as such ("Not a FHIR
  resource…", "Invalid JSON … at line 3"), not as a broken HL7 message.

Options: `--fhir-packages DIR` reads distilled packages from `DIR` instead of
the app's config directory (the R4 core is always included). A `DIR` that
cannot be read, or holds no readable package, is named on standard error and
added to every FHIR result as a `FHIR-PACKAGES` warning; each file in it
that is not a distilled package is named on standard error. `--no-plugins`
ignores plugin packs; `--format text|json|junit` (any other value is a usage
error); `--strict` (default) exits 1 on errors, `--no-strict` (or
`--strict=false`) exits 0 anyway.

JUnit: one test case per file, named by its path as given (tabs and line
breaks in names and messages are written as character references). A file with
errors is a `<failure>` (the first error as its message, all of them in its
text); a file that parsed clean but was not fully checked — a
`PROFILE-NOT-CHECKED`, `PROFILE-NOT-INSTALLED` or `FHIR-PACKAGES` note — is
`<skipped>` with the reason, so CI shows it apart from a pass without
failing the build; a file that does not parse or a path that matches nothing
is a `<failure>` of class `PARSE` or `INPUT`.

Exit codes: 0 = clean, 1 = errors found, a file failed to read or parse, or
a path matched no file, 2 = usage (unknown option or value, `-` given twice).

### `info` — message metadata

```bash
bridgelab-cli info "./*.hl7" bundle.json
bridgelab-cli info message.hl7 --json
```

Kind (hl7/fhir), message type or resource type, version (MSH-12 for HL7;
for FHIR the `fhirVersion` a conformance resource declares, empty for other
resources — `meta.versionId` is not a FHIR version), segment or entry count,
size, and whether the built-in checks found errors (`CHECKS` in the table).
In `--json`, `parsed` says the file was read as HL7 v2 or FHIR and `valid`
that the built-in checks found no error; plugin packs and FHIR profiles
are applied by `validate`, which is the command to gate on. Long paths are
shortened from the start, so the file name stays visible. Exit 1 when a
path matches no file, 0 otherwise.

### `anonymize` — mask PHI fields (HL7 v2)

```bash
bridgelab-cli anonymize patient.hl7
bridgelab-cli anonymize patient.hl7 --output patient-safe.hl7
```

The app's 89 built-in PHI fields across PID/PV1/MRG/NK1/GT1/IN1/IN2, NTE
comments and free-text (TX/FT) OBX results, plus the extra
fields of active plugin packs (`--no-plugins` to ignore them). Structure is
preserved, so the output still parses and validates. Every segment ends
with CR, the last one included. `--output` and a pipe get the message in
the input's own charset, so MSH-18 stays true; on a terminal the text is
shown as UTF-8, one segment per line, whatever the charset (a Windows
console and a Linux terminal both expect it). A message in a charset
BridgeLab cannot decode is not anonymized (exit 1): its fields could be
split wrongly and leave a value such as the birth date in clear.

`--output` (also for `to-json` and `xsd`) is written atomically: the new
content goes to a temporary file that replaces the target only once it is
complete, so a failed write (a full disk) leaves an existing file — even
the input itself — untouched, and the error names the path. A read-only
target is refused on every OS (exit 1), never replaced.

### `to-json` — HL7 v2 to a structured JSON document

```bash
bridgelab-cli to-json message.hl7 --output message.json
```

### `batch` — a directory of messages

```bash
bridgelab-cli batch ./messages --extension hl7,txt,dat
bridgelab-cli batch ./fhir --extension json --json > batch-report.json
```

Walks the directory and its subdirectories, following symbolic links, in
file-name order. `-e`/`--extension` takes one or more comma-separated
extensions, any case, with or without the dot (`HL7`, `.hl7`); the default
is `hl7` only. `--json` gives each file's issues as `validate --format json`
does. `--fhir-packages DIR` and `--no-plugins` work as for `validate`. A
file or folder that cannot be read is a failed entry. A file with no errors
that was not fully checked (the notes `validate` shows as JUnit `skipped`)
counts as valid and is listed as *not checked* with the reason
(`not_checked` and `not_checked_files` in JSON). Exit 1 if any file has
errors, or if no file matched the extension (nothing checked is not a
pass); 2 if the directory does not exist.

The app's folder batch (Tools → Batch validation…) counts differently on the
same folder: it reads one folder only (no subdirectories), takes `.hl7`,
`.txt` and `.dat` files by default, checks HL7 v2 only (a FHIR file there
fails to parse) and counts an empty file as a parse error with no error
count, where the CLI counts it as one error. Pass `--extension
hl7,txt,dat` to scan the same files.

### `test` — run test case packs

```bash
bridgelab-cli test regression.bltests.json
bridgelab-cli test "packs/*.bltests.json" --format junit > tests.xml
```

Runs the packs exported from the Test Case Library (*Export…*, see
[`docs/TEST_CASE_PACKS.md`](../../docs/TEST_CASE_PACKS.md)): each case is
parsed and validated with the same engines as `validate`, then compared
with its expectations — the message type (only the components given are
compared: `ADT` matches any ADT event, `ADT^A01` matches `ADT^A01` and
`ADT^A01^ADT_A01` but not `ADT^A04`) and valid/invalid, a message that does not parse
counting as invalid. `expected_validation_result` is `valid` or `invalid`
(any case); any other value makes the pack unreadable. A failed case shows
its first errors, and why a FHIR resource was not fully checked. A case
that meets its expectations but whose FHIR resource was not fully checked
(as `validate` reports it) is *not checked* — `~` in text, `skipped` in
JSON and JUnit — never a pass. Output `text` (grouped by pack), `json` or
`junit` (one test suite per pack); `--fhir-packages` and `--no-plugins` as
for `validate`. Exit 1 if a case fails, or a pack cannot be read, matches
no file or has no test cases; not-checked cases alone exit 0.

### `fhirpath` — evaluate an expression

```bash
bridgelab-cli fhirpath "Patient.name.family" patient.json
cat bundle.json | bridgelab-cli fhirpath "Bundle.entry.count()"
bridgelab-cli fhirpath "Observation.value.ofType(Quantity)" obs.xml --json
```

The app's FHIRPath 2.0 engine, on a FHIR resource in JSON or XML (the
input defaults to standard input). Prints one value per line, or with
`--json` the whole result (values, count, `trace()` output). Exit 1 on a
syntax or evaluation error. An expression that starts with `-` would be
read as an option: put `--` before it (`bridgelab-cli fhirpath -- "-5 mod 3"
patient.json`).

### `send` — MLLP

```bash
bridgelab-cli send message.hl7 --host 10.0.0.5 --port 2575
cat message.hl7 | bridgelab-cli send - --host 10.0.0.5 --port 2575 --json
```

Frames the message (LF or CRLF line endings are converted to the CR HL7
uses), waits for the acknowledgement and prints it. A file holding several
messages is sent as one MLLP frame per message (each MSH starts one), each
acknowledged in turn; sending stops at the first message not accepted.
Batch envelope segments (FHS/BHS/BTS/FTS) are not sent, with a note. With
`--json` the result is that of the last message sent, with every message's
in `results`. An ACK whose MSA-2 names another message than the MSH-10
sent is not an acceptance, and a reply without an MSA acknowledgement
code is reported as not an acknowledgement. `--timeout` (seconds, 1 to
86400, default 10) covers connect, sending (a peer that stops reading for
that long ends the send) and response. `--encoding` sets the wire charset
(e.g. `ISO-8859-1`, `windows-1252`, `ASCII`); without it a UTF-8 file goes
out as UTF-8, a legacy one in its own charset, and a UTF-16 one in the
charset its MSH-18 declares, else UTF-8. A message with characters the
wire charset cannot hold is not sent: the characters are named and the
command exits 1 before anything goes out (as in the app). An unknown
`--encoding` is refused (exit 2). Exit 0 only when the ACK code is `AA` or
`CA`; an `AE`/`AR`, no ACK or a connection error exits 1 — so a script can
tell "delivered and accepted" from everything else.

### `xsd` — schema of a message structure

```bash
bridgelab-cli xsd ADT_A01 > ADT_A01.xsd
bridgelab-cli xsd "ORU^R01" --hl7-version 2.5 --output ORU_R01.xsd
```

The Community set: `ADT_A01`, `ADT_A40`, `ORM_O01`, `ORU_R01` in v2.5.
Other messages and versions of the catalogue exit 1 with a note — the full
catalogue (every structure of v2.1 to v2.7.1) is exported from the app with
Pro. A message structure or version that does not exist exits 2.

## CI/CD integration

### GitHub Actions

```yaml
- name: Validate HL7 and FHIR fixtures
  run: |
    curl -fsSL -o bridgelab-cli https://github.com/techemv-srl/BridgeLab/releases/latest/download/bridgelab-cli-x86_64-unknown-linux-gnu
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
# Validate the staged HL7 and FHIR files; commits without any pass.
# NUL-separated names: spaces, quotes and non-ASCII names are safe.
git diff --cached --name-only --diff-filter=ACM -z -- \
    ':(icase)*.hl7' ':(icase)*.fhir.json' ':(icase)*.fhir.xml' |
  xargs -0 -r bridgelab-cli validate --no-plugins --
```

Name your FHIR resources `*.fhir.json` / `*.fhir.xml` (or list your own
folders as pathspecs, e.g. `':(icase)fixtures/fhir/*.json'`; `:(icase)`
makes Git match `C.Hl7` as well, since pathspecs are case-sensitive) so `package.json` and
other JSON never reach the validator — if one does, it is reported as "Not a
FHIR resource", not as a broken HL7 message. `xargs -r` (GNU and BSD) runs
nothing when no file matches; any invalid file makes the hook exit non-zero
and blocks the commit.

### Air-gapped sites

Copy the distilled packages the app produced (`<config>/BridgeLab/fhir-packages/`)
next to your fixtures and point the CLI at them with `--fhir-packages`. Nothing
is ever downloaded.

## What the CLI does not do

The MLLP listener, the HTTP and SOAP clients, licence activation,
templates and every interactive feature stay in the desktop app.
