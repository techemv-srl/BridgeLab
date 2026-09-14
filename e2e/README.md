# Release acceptance checks

Drives the **real, installed** BridgeLab binary through
[tauri-driver](https://v2.tauri.app/develop/tests/webdriver/) and checks the
package it came from. No dev server, no mocks: what is exercised here is what
a user installs.

This is the gate a release goes through. Run it on the built packages before
tagging.

## Running it

```bash
# 1. build the Linux packages
pnpm tauri build --target x86_64-unknown-linux-gnu

# 2. install the .deb (the suite tests the installed application)
sudo dpkg -i src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/deb/BridgeLab_<version>_amd64.deb

# 3. run the checks
node e2e/run.mjs
```

`run.mjs` starts Xvfb and tauri-driver itself and stops them afterwards, so
the whole thing is one command. It exits non-zero if any check fails.

### Prerequisites

| Tool | Where from |
|---|---|
| `Xvfb` | `xvfb` package |
| `WebKitWebDriver` | `webkit2gtk-driver` package |
| `tauri-driver` | `cargo install tauri-driver` |

### Options

| Variable | Meaning |
|---|---|
| `BL_APP` | application to drive (default `/usr/bin/bridgelab`) |
| `BL_DISPLAY` | X display to use (default `:99`) |
| `BL_DRIVER_URL` | an already-running tauri-driver |

A suite can also be run on its own:

```bash
node e2e/suites/package.mjs path/to/BridgeLab_1.5.0_amd64.deb   # no display needed
node e2e/suites/app.mjs                                          # needs a running tauri-driver
```

## What it checks

**`suites/package.mjs`** — reads the `.deb` and, when installed, the system:
version agreement with `package.json`, runtime dependencies declared exactly
once, section/priority, binary and desktop entry and icons present, and the
shared-mime-info definition that makes the `.hl7` association work. That last
one exists because the desktop entry's `MimeType=` only says *BridgeLab can
open this*; without a MIME definition nothing maps `*.hl7` to the type and
double-clicking a message never reaches the app.

**`suites/app.mjs`** — the application itself:

- **Shell** — window, menu bar, first-run welcome screen.
- **HL7 v2** — paste, parse, tree, validation report.
- **Version catalogue** — every shipped version offered in the XSD export,
  v2.7.1 marked as an alias of v2.7, and the oldest catalogue exporting.
- **FHIRPath** — ten expressions covering three-valued logic, precedence,
  partial-precision dates, unit conversion, duration arithmetic, sorting,
  strings, filtering, `resolve()` across a Bundle, choice elements by base
  name, `trace()`, and a named error for an unknown function.
- **FHIR rules builder** — dialog, presets, both rule forms.
- **Profile validation** — with a FHIR package installed: the package is
  listed, a conforming resource comes back clean *and says so*, and a
  misspelled element, a wrong JSON type, a choice element written plainly, a
  missing required element and an uninstalled declared profile are each
  caught. With no package installed, it checks the app reports conformance as
  **not** checked rather than implying a clean result.

To exercise conformance itself, install a package first. The in-app route is
`Tools → FHIR profile packages…`, but that goes through a native file dialog
WebDriver cannot drive, so for an automated run install it headlessly — same
code, same distilled index, same location:

```bash
./scripts/fetch-fhir-core-package.sh
cargo run --manifest-path src-tauri/Cargo.toml \
    --example install-fhir-package -- .fhir-packages/hl7.fhir.r4.core.tgz
```

With the core package installed the app suite runs 36 checks instead of 30.

## Writing checks

A check records a pass or a failure and never aborts the run, so one broken
thing does not hide the rest. Prefer asserting on what a user can see, and
keep the failure message carrying the observed value — a check that only says
"failed" costs more time than it saves.

Two traps this suite already accounts for, both of which cost an afternoon:

- **Focus decides where a paste goes.** `AppShell` hands the event to Monaco
  when Monaco holds focus, so `paste()` in `lib.mjs` drops focus first.
  Without that, the document silently stays as it was and every later check
  reads the previous resource.
- **Zero tabs is the intended first-run state**, not a failure. Monaco mounts
  when a document exists, so load one before expecting an editor.
- **The shell is interactive before startup has settled.** The menu bar
  mounts first; the welcome card waits for session restore to finish, so
  there is a window with neither it nor an editor on screen. Wait for one of
  the two states, don't sample once — a check that samples passes or fails by
  how fast the machine is that day.
