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
`"rules"` instead of `"validation_rules"`, a rule on a malformed segment
name or on field or component 0, a rule with an empty id or name, or a
pattern too large or too slow to compile) are each named on standard
error, so a pack missing from a CI run is never silent; HL7 v2 and FHIR
reports also carry the app's `PLUGIN-CAP` info note when packs with rules
for them are over the cap. A pack that holds no rules takes no slot. The
CLI only reads that folder: it creates no folder and records nothing in
the app's plugin state. Packs the app has not seen yet (all of them, on a
machine without the app) are ordered by when their files appeared in the
folder, so a pack added later never displaces one in use. If the app's
on/off choices (`.state.json`) cannot be read, standard error says so and
each pack runs as its own `"enabled"` says.

**Standard input.** Every command that reads one message accepts `-` as the
file name: `cat message.hl7 | bridgelab-cli validate -` (once per command:
standard input can be read only once).

**Paths and patterns.** `validate`, `info` and `test` take files and glob
patterns. A path that exists is read as it is, even when its name holds
glob characters (`msg[1].hl7`), and so is a named pipe or a process
substitution (`validate <(zcat msg.hl7.gz)`); a file matched by two
patterns is read once. Patterns ignore case on every OS, folder names
included: `"*.hl7"` also finds `ADT.HL7`, `"g2/sub/*.hl7"` finds
`G2/Sub/x.hl7`, and on Linux a file or folder whose name is not valid
UTF-8 is matched too, at any depth `"**/*.hl7"` or `"*/*.hl7"` reaches.
Hidden files and folders (names starting with a dot, such as the
`._name.hl7` files macOS leaves on shared drives, or `.git`) match only a
pattern that spells the dot (`".*.hl7"`, `".cache/*.hl7"`). A path or
pattern that matches no file is never skipped: it is reported on standard
error and in the output (a `NOT FOUND` entry, a JSON row with
`parse_error`, a JUnit failure) and the command exits 1. So is a folder
given where a file is expected (use `batch` for a folder) and a folder a
pattern runs through that cannot be read (`INPUT ERROR`).

**Quoting.** Put patterns and arguments holding `^` in double quotes:
`"*.hl7"`, `"ORU^R01"`. Windows `cmd.exe` strips an unquoted `^` and keeps
single quotes as part of the argument, so `'*.hl7'` matches nothing there.

**Reports and output files.** Write reports and messages to a file with
`--output` (`validate`, `test`, `batch`, `anonymize`, `to-json`, `xsd`)
rather than with `>`. Windows PowerShell 5.1 re-encodes what a program
writes: `>` saves it as UTF-16, so a JUnit report that declares UTF-8 or a
JSON report cannot be read by CI tools, and a message piped into the CLI
(`Get-Content x.hl7 | bridgelab-cli anonymize -`) loses its accented
letters before the CLI sees it. Give file names instead of pipes there;
`cmd.exe`, PowerShell 7.4 and later, and Unix shells pass the bytes
unchanged. `--output` writes exactly the bytes the command produces, in
UTF-8 for reports, atomically (see `anonymize`).

**Terminal output.** Text output never passes control characters to the
terminal, on standard output or standard error, whether they come from a
message, an ACK, a plugin pack, a file name or an argument: they are shown
as `\xNN` (an escape sequence in a field could otherwise clear the screen
or draw a fake result). JSON output escapes them as `\uNNNN`, DEL and
U+0080 to U+009F included, and JUnit output writes DEL and U+0080 to
U+009F as character references (`&#155;`); the other control characters,
which XML cannot hold, become U+FFFD.

## Installation

Prebuilt binaries are attached to every
[release](https://github.com/techemv-srl/BridgeLab/releases) as
`bridgelab-cli-<target>` (macOS Intel and Apple Silicon, Linux) and
`bridgelab-cli-<target>.exe` (Windows).
The Linux binary (`x86_64-unknown-linux-gnu`, built on Ubuntu 22.04) needs
glibc 2.34 or newer — Ubuntu 22.04, Debian 12, RHEL/Rocky/Alma 9 or later;
on an older system build it from source. Only the two most recent releases
keep their assets, so download from `latest` rather than pinning an old
version's URL.

**macOS.** The CLI binary is not signed or notarized. A copy downloaded
with a browser carries the quarantine attribute, and macOS refuses to run
it ("cannot be opened because the developer cannot be verified" or "Apple
could not verify…"). Make it executable and remove the attribute once:

```bash
chmod +x bridgelab-cli-aarch64-apple-darwin
xattr -d com.apple.quarantine bridgelab-cli-aarch64-apple-darwin
```

(`x86_64-apple-darwin` on an Intel Mac.) Alternatively run it once, then
choose *Allow Anyway* in System Settings → Privacy & Security. A copy
fetched with `curl`, as in a CI job, has no quarantine attribute and runs
as it is.

`bridgelab-cli --version` prints the version and the build code
(`bridgelab-cli 2.1.0 (build 20261120.1ea84f3)`, the date and the source
revision), the same code the app's About shows, so a CI log says exactly
which build ran.

From source (needs Rust; no Tauri or WebKit — the desktop shell is not
compiled):

```bash
cd tools/bridgelab-cli
cargo build --release          # → target/release/bridgelab-cli
```

### Docker

Every release is also published as a container image,
`ghcr.io/techemv-srl/bridgelab-cli`, tagged with its version and `latest`
(Linux x86-64). The command and its options follow the image name; mount
the files to check on `/work`, the image's working directory:

```bash
docker run --rm -v "$PWD:/work" ghcr.io/techemv-srl/bridgelab-cli validate "messages/*.hl7"
docker run --rm -v "$PWD:/work" ghcr.io/techemv-srl/bridgelab-cli validate "**/*.hl7" --format junit > results.xml
docker run --rm -i ghcr.io/techemv-srl/bridgelab-cli anonymize - < message.hl7 > message-safe.hl7
```

The CLI runs as an unprivileged user (uid 10001). Results written to
standard output, as above, land on the host as usual; to write a file
inside the mount with `--output`, run as your own user:
`docker run --rm --user "$(id -u):$(id -g)" -v "$PWD:/work" …`.

Plugin packs and FHIR packages are not in the image. The CLI reads plugin
packs from `/config/BridgeLab/plugins` (its configuration folder in the
image), so mount your plugins folder there, the one with `validation/`,
`fhir/` and `anonymization/` inside; FHIR packages are a directory you pass
with `--fhir-packages`:

```bash
docker run --rm -v "$PWD:/work" \
  -v "$PWD/plugins:/config/BridgeLab/plugins:ro" \
  -v "$PWD/fhir-packages:/packages:ro" \
  ghcr.io/techemv-srl/bridgelab-cli validate "fixtures/**/*" --fhir-packages /packages
```

To build the image yourself, from the repository root (the build context
holds no Git history, so the build code `--version` prints is passed in;
without it the image says `build unknown`):

```bash
docker build -f tools/bridgelab-cli/Dockerfile \
  --build-arg BRIDGELAB_BUILD="$(git log -1 --format=%cd --date=format:%Y%m%d).$(git rev-parse --short=7 HEAD)" \
  -t bridgelab-cli .
```

## Commands

`bridgelab-cli --help` lists the commands; `bridgelab-cli <command> --help`
shows a command's options and ends with examples like the ones below.

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
bridgelab-cli validate "**/*.hl7" --format junit --output results.xml
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
  primitive formats; the JSON array shape (an element that can repeat is
  an array even with one item, `"name": [{…}]`, and one that cannot is
  never an array, `"gender": "male"`; new in 2.1, so a file that passed
  2.0 can now fail); plugin FHIRPath rules, under the budget described
  in [docs/PLUGINS.md](../../docs/PLUGINS.md). A resource type no package
  defines is reported as *not checked* (info, `PROFILE-NOT-CHECKED`), a
  declared profile that is not installed as `PROFILE-NOT-INSTALLED`
  (warning) — never as clean. JSON input that is not a FHIR resource
  (`package.json`) or does not parse is reported as such ("Not a FHIR
  resource…", "Invalid JSON … at line 3"), not as a broken HL7 message.

Options: `--fhir-packages DIR` reads distilled packages (`.json`, in any
case) from `DIR` instead of the app's config directory (the R4 core is
always included). A `DIR` that cannot be read, or holds no readable
package, is named on standard error and added to every FHIR result as a
`FHIR-PACKAGES` warning; each other file in it (a `.tgz` package as
downloaded, which the app installs and distils, or any file that is not a
distilled package) is named on standard error. `--no-plugins`
ignores plugin packs; `--format text|json|junit` (any other value is a usage
error); `--output FILE` writes the report to `FILE` instead of standard
output; `--strict` (default) exits 1 on errors, `--no-strict` (or
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
split wrongly and leave a value such as the birth date in clear. A FHIR
resource is refused with a message that names it (exit 1).

`--output` (also for `to-json`, `xsd`, and the reports of `validate`,
`test` and `batch`) is written atomically: the new
content goes to a temporary file that replaces the target only once it is
complete, so a failed write (a full disk) leaves an existing file — even
the input itself — untouched, and the error names the path. A read-only
target is refused on every OS (exit 1), never replaced. The temporary file
is readable by its owner only, and the replaced file keeps its
permissions, ACL and extended attributes.

### `to-json` — HL7 v2 to a structured JSON document

```bash
bridgelab-cli to-json message.hl7 --output message.json
```

A file that holds several messages (each MSH starts one) becomes a JSON
array with one document per message, each with the type and version of
its own MSH, and a note on standard error; a single message is one
document. A FHIR resource is refused with a message that names it.

### `batch` — a directory of messages

```bash
bridgelab-cli batch ./messages --extension hl7,txt,dat
bridgelab-cli batch ./fhir --extension json --json --output batch-report.json
```

Walks the directory and its subdirectories, following symbolic links, in
file-name order; hidden files and folders inside it (`._name.hl7`, `.git`)
are passed over. `--output FILE` writes the summary to `FILE`.
`-e`/`--extension` takes one or more comma-separated
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
bridgelab-cli test "packs/*.bltests.json" --format junit --output tests.xml
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
`junit` (one test suite per pack), to standard output or to `--output
FILE`; `--fhir-packages` and `--no-plugins` as
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
syntax or evaluation error. One evaluation nests at most 64 levels, builds
at most 16 MB per string and about 512 MB in all, and does at most 50
million steps in 60 seconds; past a limit it ends with an error that names
it ("The expression takes too long to evaluate…"), exit 1. An expression
that starts with `-` would be
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
Batch envelope segments (FHS/BHS/BTS/FTS) are not sent, with a note. Input
that is not HL7 v2 (plain text, a FHIR resource, a part before an MSH that
does not parse) is refused before anything is sent (exit 1): it has no
MSH-10 an ACK could be matched to. With
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
bridgelab-cli xsd ADT_A01 --output ADT_A01.xsd
bridgelab-cli xsd "ORU^R01" --hl7-version 2.5 --output ORU_R01.xsd
```

The message is a structure (`ADT_A01`), a type (`ADT^A01`) or a whole
MSH-9: `ADT^A04^ADT_A01` exports its structure, `ADT_A01`.
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
    ./bridgelab-cli validate "test/fixtures/**/*.hl7" "test/fixtures/**/*.json" --format junit --output junit.xml

- uses: mikepenz/action-junit-report@v4
  if: always()
  with:
    report_paths: junit.xml
```

### GitLab CI (container image)

The image's entry point is the CLI, so a job that runs shell commands in
it sets an empty one:

```yaml
validate-hl7:
  image:
    name: ghcr.io/techemv-srl/bridgelab-cli:latest
    entrypoint: [""]
  script:
    - bridgelab-cli validate "fixtures/**/*.hl7" --format junit --output junit.xml
  artifacts:
    when: always
    reports:
      junit: junit.xml
```

### Pre-commit hook

```bash
#!/bin/sh
# Validate the HL7 and FHIR files as they are staged: the content the
# commit will hold, not the working-tree copy, which may differ (a file
# staged, then edited). Commits without any such file pass.
# NUL-separated names: spaces, quotes and non-ASCII names are safe.
staged=$(mktemp -d) || exit 1
trap 'rm -rf "$staged"' EXIT
git diff --cached --name-only --diff-filter=ACM -z -- \
    ':(icase)*.hl7' ':(icase)*.fhir.json' ':(icase)*.fhir.xml' |
  git checkout-index -z --stdin --prefix="$staged/"
cd "$staged" || exit 1
find . -type f -print0 | xargs -0 -r bridgelab-cli validate --no-plugins --
```

`git checkout-index` writes the staged copy of each file into a temporary
folder, under its own path, and the report names them by that path
(`./msgs/adt.hl7`).

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
