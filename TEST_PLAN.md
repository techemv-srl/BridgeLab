# BridgeLab Test Plan

**Version**: 0.1.0
**Last Updated**: April 2026 (r2)
**Status**: In progress - updated as features are completed

## Purpose

This document defines manual test cases to verify that all BridgeLab functionality works as expected before each release. Tests are grouped by feature area. Each test has a unique ID for tracking.

## Test Execution

- **Test ID format**: `BL-{AREA}-{NN}` (e.g., `BL-PARSER-01`)
- **Status codes**: \u2705 Pass | \u274C Fail | \u26A0 Partial | \u23F8 Skipped | \u2753 Blocked
- **Priority**: P0 (critical) | P1 (major) | P2 (normal) | P3 (minor)
- **Platforms**: Windows 11, macOS 14+, Linux (Ubuntu 22.04+)

### Excel export for tracking

Export the plan to a formatted Excel workbook:

```bash
pip install openpyxl
python scripts/test_plan_to_excel.py
# -> produces TEST_PLAN.xlsx with one sheet per section
```

The Excel has added columns (Tested By, Tested At, Notes) plus color-coded
Priority and Status cells. Share the `.xlsx` with the QA team for execution.

### Automated tests

Three GitHub Actions workflows run on every push:

- **ci.yml** - builds frontend and tests Rust core
- **feature-tests.yml** - CLI feature tests + Rust integration tests +
  license signing roundtrip
- **release.yml** - triggered by `v*` tags for cross-platform builds

See `.github/workflows/` for details.

## Pre-requisites

Before running tests:

- Build the app: `pnpm tauri build`
- Or run dev mode: `pnpm tauri dev`
- Have sample HL7 files ready (see `tests/fixtures/hl7/`)
- Have a FHIR bundle JSON ready (see `tests/fixtures/fhir/`)
- Clean state: remove `~/.local/share/BridgeLab/` (Linux) or equivalent to reset DB/license

## Test Fixtures Required

| File | Purpose |
|------|---------|
| `adt_a01_small.hl7` | Basic ADT^A01 (~500 bytes) |
| `oru_r01_with_base64.hl7` | ORU with base64 PDF section (~5MB) |
| `orm_o01_multisegment.hl7` | ORM with many OBX segments |
| `invalid_structure.hl7` | Missing MSH for validation tests |
| `bundle_patient.json` | Simple FHIR Bundle with Patient + Observation |
| `bundle_many_refs.json` | Bundle with 10+ entries and cross-references |
| `dangling_bundle.json` | Bundle with broken references |

---

## 1. Application Launch & UI Shell

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-APP-01 | P0 | App starts without errors | Launch app | Window opens, no error dialogs, DevTools console clean | |
| BL-APP-02 | P0 | Initial empty tab created | Launch app | "Untitled" tab visible, Monaco editor has focus | |
| BL-APP-03 | P0 | Trial banner shows on first run | Fresh install, launch | Yellow "Trial: 14 days remaining" banner at top | |
| BL-APP-04 | P1 | Window is resizable | Drag window corners | Window resizes, panels reflow correctly | |
| BL-APP-05 | P1 | Minimum window size respected | Try to resize below 900x600 | Window stops at 900x600 | |
| BL-APP-06 | P1 | App icon is the new bridge design | Check taskbar/dock | Bridge icon with HL7 badge, not placeholder | |
| BL-APP-07 | P0 | Manual opens with content on every OS | On Windows, macOS and Linux: press F1 (and Help → Manual) in English and in Italian; click a Contents entry; rebind a shortcut and reopen | A separate window shows the manual in the UI language, with the contents list and all sections; the entry scrolls to its section; the shortcuts table shows the new binding. Never a blank window | |

## 2. Menu Bar & Keyboard Shortcuts

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-MENU-01 | P0 | File menu opens on click | Click File | Dropdown opens with all entries | |
| BL-MENU-02 | P0 | File menu closes on outside click | Click File then click outside | Dropdown closes | |
| BL-MENU-03 | P1 | All 5 main menus present | Check menubar | File, Edit, View, Tools, Help visible | |
| BL-SHORTCUT-01 | P0 | Ctrl+O opens file dialog | Press Ctrl+O | Native file picker opens | |
| BL-SHORTCUT-02 | P0 | Ctrl+N opens template dialog | Press Ctrl+N | Template selection modal opens | |
| BL-SHORTCUT-03 | P1 | Ctrl+L opens Test Case Library | Press Ctrl+L | Library modal opens | |
| BL-SHORTCUT-04 | P1 | Ctrl+S triggers save | Have modified tab, press Ctrl+S | Save dialog or save to file path | |
| BL-SHORTCUT-05 | P1 | Ctrl+W closes active tab | Press Ctrl+W | Tab closes, next tab becomes active | |
| BL-SHORTCUT-06 | P1 | Ctrl+B toggles tree panel | Press Ctrl+B | Tree panel hides/shows | |
| BL-SHORTCUT-07 | P1 | Re-parse has no default key; F5 never reloads the app | Press F5; then bind Re-parse to F5 in Settings → Keyboard Shortcuts and press F5 | Nothing happens (no reload); after binding, the message is re-parsed | |
| BL-SHORTCUT-08 | P1 | F6 runs validation | Press F6 | Validation panel appears with results | |
| BL-SHORTCUT-09 | P1 | Ctrl+J toggles validation panel | Press Ctrl+J | Panel shows/hides | |
| BL-SHORTCUT-10 | P1 | Ctrl+K toggles communication panel | Press Ctrl+K | Panel shows/hides | |
| BL-SHORTCUT-11 | P1 | Ctrl+P toggles FHIRPath panel | Press Ctrl+P | Panel shows/hides | |
| BL-SHORTCUT-12 | P1 | Ctrl+, opens settings | Press Ctrl+, | Settings modal opens | |
| BL-SHORTCUT-13 | P1 | App shortcuts work inside the editor | Click in the editor, press Ctrl+L, then Ctrl+K | Test Case Library opens; Communication panel toggles | |
| BL-SHORTCUT-14 | P1 | Bare keys and F1 are refused | Settings → Keyboard Shortcuts, rebind Validate, press Q, then F1 | Warning shown, OK disabled; binding unchanged | |
| BL-SHORTCUT-15 | P1 | Capture works with the editor focused | Click in the editor, open Settings → Keyboard Shortcuts, rebind Validate, press Ctrl+F | Capture shows Ctrl+F; no Find widget opens in the editor | |
| BL-SHORTCUT-16 | P1 | Browser keys do not reload (Windows) | Press Ctrl+R and Ctrl+Shift+R with a message open | Nothing happens; tabs and panels stay | |

## 3. HL7 Parser (Core)

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-PARSER-01 | P0 | Parse basic ADT^A01 | Paste ADT^A01 sample | Tree shows MSH, EVN, PID, PV1 segments | |
| BL-PARSER-02 | P0 | Detect message type | Parse any message | Status bar shows correct message type (e.g. ADT^A01) | |
| BL-PARSER-03 | P0 | Detect HL7 version | Parse message with MSH-12=2.5 | Status bar shows "v2.5" | |
| BL-PARSER-04 | P0 | Auto-parse on paste | Paste HL7 text in editor | Tree updates within 500ms (debounced) | |
| BL-PARSER-05 | P0 | Custom delimiters recognized | Parse message with pipe delimiter variants | Delimiters parsed from MSH-1/2 correctly | |
| BL-PARSER-06 | P1 | CRLF line endings supported | Parse message with \\r\\n | Segments split correctly | |
| BL-PARSER-07 | P1 | LF line endings supported | Parse message with \\n only | Segments split correctly | |
| BL-PARSER-08 | P1 | CR line endings supported | Parse message with \\r only | Segments split correctly | |
| BL-PARSER-09 | P1 | Empty fields preserved | Parse `PID\|1\|\|\|MRN` | Empty fields shown in tree | |
| BL-PARSER-10 | P1 | Z-segment support | Parse with ZDS segment | Z-segment appears in tree | |
| BL-PARSER-11 | P0 | BOM stripped | Parse UTF-8 BOM prefixed file | Parses successfully | |
| BL-PARSER-12 | P2 | Invalid message rejected gracefully | Paste "garbage" | Error shown, no crash | |

## 4. Large Message Handling (Performance)

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-PERF-01 | P0 | Open 5MB file <2s | Open `oru_r01_with_base64.hl7` | Tree populated within 2 seconds | |
| BL-PERF-02 | P0 | Open 10MB file <3s | Open larger file | Tree populated within 3 seconds | |
| BL-PERF-03 | P0 | Editor shows folded fields | Open 5MB file | Base64 fields show as a chip such as `⟨Base64 · 4.7 MB #1⟩` | |
| BL-PERF-04 | P0 | Editor remains responsive | Type/scroll in a large message, and in one with 20,000 OBX segments (tree visible) | No lag; the tree appears within seconds and scrolls smoothly (only the rows in view are drawn) | |
| BL-PERF-05 | P0 | A folded field expands inline | Click the chip, or right-click > Expand Folded Field | Field expands in editor | |
| BL-PERF-06 | P0 | Expand All works | Right-click > Expand All Folded Fields | All fields expanded | |
| BL-PERF-07 | P0 | Fold All folds again | Right-click > Fold All Long Fields | Long fields show as chips again | |
| BL-PERF-08 | P1 | Multiple truncated fields per segment | Load message with 2+ truncated in PID | Expand via context menu picks correct field | |
| BL-PERF-09 | P1 | Backend memory stays below 300MB | Load 10MB file, monitor RAM of the `bridgelab` process and of the web view (WebKitWebProcess / msedgewebview2 / WebContent) | `bridgelab` <300MB; record the web view's figure (about 900 MB on Linux, about 1.2 GB for the whole app) | |
| BL-PERF-10 | P2 | Parser benchmark test passes | `cargo test test_large_message_performance` | Test passes | |

## 5. Editor (Monaco)

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-EDITOR-01 | P0 | Paste from notepad works | Copy from Notepad, paste in editor | Text appears in Monaco | |
| BL-EDITOR-02 | P0 | Paste triggers auto-parse | Paste valid HL7 | Tree populates after 500ms | |
| BL-EDITOR-03 | P0 | HL7 syntax highlighting | Load HL7 message | Segments colored purple, delimiters distinct | |
| BL-EDITOR-04 | P0 | Dark theme applies | Load message in dark mode | Background #1e1e2e, text light | |
| BL-EDITOR-05 | P0 | Light theme applies | Switch to light theme | Background light, text dark | |
| BL-EDITOR-06 | P1 | Context menu: Show in Tree (field precision) | Right-click inside a specific field (e.g. PID-5), click Show in Tree | Tree panel opens, segment expanded, field node `PID-5` selected & scrolled to | |
| BL-EDITOR-07 | P1 | Context menu: Copy Segment | Right-click, Copy Segment | Line copied to clipboard | |
| BL-EDITOR-08 | P1 | Context menu: Copy Full Message | Right-click, Copy Full | Original full message in clipboard | |
| BL-EDITOR-09 | P1 | Context menu: Copy Truncated | Right-click, Copy Truncated | Truncated version in clipboard | |
| BL-EDITOR-10 | P1 | Cursor position updates | Click in editor | Status bar shows Ln X, Col Y correctly | |
| BL-EDITOR-11 | P1 | Minimap visible by default | Open file | Minimap shown on right side | |
| BL-EDITOR-12 | P1 | Word wrap works | Toggle in settings | Lines wrap at viewport edge | |
| BL-EDITOR-13 | P2 | Undo/redo work | Type, Ctrl+Z, Ctrl+Y | Changes undo/redo correctly | |
| BL-EDITOR-14 | P2 | Find/replace works, folded fields included | Paste an ORU whose long NTE mentions `Smith` twice (folded) and PID-5 = `Smith^John`; Ctrl+H, find `Smith`, Replace All with `Jones`; save | The widget counts 3; the NTE chip opens; after Replace All no `Smith` is left in the editor or the saved file | |

## 6. Auto-complete (HL7)

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-AUTO-01 | P1 | Segment suggestions at line start | Type "P" at new line | Suggestions include PID, PV1, PV2 | |
| BL-AUTO-02 | P1 | MSH-9 message type suggestions | After MSH...\|, position at field 9 | Suggestions include ADT^A01, ORU^R01, etc. | |
| BL-AUTO-03 | P1 | MSA-1 ACK codes | In MSA segment field 1 | AA, AE, AR suggested | |
| BL-AUTO-04 | P1 | PID-8 gender suggestions | In PID-8 | M, F, O, U, A suggested | |
| BL-AUTO-05 | P2 | Hover shows field info | Hover on any field | Tooltip with name, type, required flag | |
| BL-AUTO-06 | P1 | Hints follow MSH-12 | Open a message declaring `2.3` in MSH-12, hover a PID field | Field metadata comes from the 2.3 tables, not 2.5 | |
| BL-AUTO-07 | P1 | Hints for the oldest versions | MSH-12 = `2.1`, then `2.2`; hover a PID field | Tooltip still appears (regression: these lost hints entirely when they were added to the catalogue) | |
| BL-AUTO-08 | P2 | Unknown version falls back | MSH-12 = `2.9` or empty | Hints still shown, from the default tables | |
| BL-AUTO-09 | P1 | Any coded field completes from its table | In OBX-11, then ORC-1, then PV1-2 | Every value of table 0085 / 0119 / 0004 offered with its meaning; no hand-coded list involved | |
| BL-AUTO-10 | P1 | Hover explains the code | Hover PID-8 = `M`, then MSA-1 = `AE` | Tooltip ends with `M — Male (HL7 table 0001)` / `AE — …`; hover a segment the old 15-segment list never had (RXA-5) and the name still shows | |
| BL-AUTO-11 | P2 | Hover flags a value outside a closed table | MSA-1 = `XX` (ID), then PID-8 = `X` (IS) | MSA-1 tooltip says `XX is not in HL7 table 0008`; PID-8 says nothing about the table (user-defined) | |

## 7. Tree View

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-TREE-01 | P0 | Tree shows segments | Parse message | Segments listed with position | |
| BL-TREE-02 | P0 | Expand segment shows fields | Click arrow on PID | Fields PID-1 to PID-30 shown | |
| BL-TREE-03 | P0 | Expand field shows components | Click arrow on field with components; then on PID-3 = `111^^^H^MR~222^^^H^SS` | Components 1-N shown; PID-3 lists PID-3(1) and PID-3(2), each with its components | |
| BL-TREE-04 | P0 | Lazy loading works | Expand large segment | Children fetched on demand, no freeze | |
| BL-TREE-05 | P1 | Field names shown | Hover/inspect field | HL7 standard name displayed (e.g. "Patient Name") | |
| BL-TREE-06 | P1 | Truncated fields have `{...}` button | Load large field | Red `{...}` button visible | |
| BL-TREE-07 | P1 | Click `{...}` opens expansion | Click the button | Modal shows full field content | |
| BL-TREE-08 | P1 | Modal "Copy to Clipboard" works | In expansion modal, click Copy | Full content in clipboard | |
| BL-TREE-09 | P1 | Resize splitter works | Drag splitter between tree and editor | Panels resize, position saved | |
| BL-TREE-10 | P1 | Tree panel hideable | Ctrl+B | Panel hides, editor takes full width | |
| BL-TREE-11 | P2 | Badge shows field count | Check unexpanded segments | Count badge shown | |
| BL-TREE-12 | P2 | Click node navigates editor | Click segment in tree | Editor scrolls to corresponding line | |
| BL-TREE-13 | P1 | Coded values explained inline | Expand PID, MSH, PV1 of an ADT^A01 | Rows read `PID-8  M — Male`, `MSH-9  ADT^A01 — ADT message`, `MSH-11  P — Production`, `PV1-2  I — Inpatient`; free-text fields (PID-5) have no suffix | |
| BL-TREE-14 | P1 | Components explained from their own table | Expand MSH-9, then PID-3 | `MSH-9.2  A01 — ADT/ACK - Admit/visit notification`; `PID-3.5  MR — Medical record number`; `PID-3.1` has no suffix | |
| BL-TREE-15 | P2 | Unknown code has no suffix; first repetition rules | PID-8 = `Q`; OBX-8 = `H~A` | PID-8 shows just `Q`; OBX-8 reads `H~A — Above high normal` | |
| BL-TREE-16 | P1 | Segment grid opens on the most repeated segment | Open the sample *Lab results: complete blood count*; View → Segment Grid (Ctrl+Shift+G) | Bottom panel *Segment Grid* shows OBX (12): 12 rows, columns OBX-1…OBX-14 that hold values with their names (e.g. *Observation Value*), OBX-8 cells showing `L — Below low normal` / `H — …` | |
| BL-TREE-17 | P1 | From the tree, filter, navigate | Right-click the ORC segment in the tree → *Show all ORC in a table*; pick OBX again, filter `L`; click an OBX-5 cell | The grid switches to ORC; the filter keeps only rows with an L; the click selects that OBX-5 in the editor | |
| BL-TREE-18 | P2 | Follows edits; HL7 v2 only | Delete an OBX line and re-parse; open a FHIR resource | The grid shows 11 rows; with FHIR the panel is not offered | |
| BL-TREE-19 | P1 | Tree keeps its state across edits | Expand PID and select PID-5; edit PID-5 and wait for the auto-parse | PID stays expanded, PID-5 stays selected and the inspector shows the new value | |
| BL-TREE-20 | P1 | Keyboard navigation | Click a segment row, then use Down/Up, Right (expand), Left (collapse / go to parent), Home/End, PageDown | Selection moves row by row and the tree scrolls with it, past the rows drawn at first | |
| BL-TREE-21 | P1 | Insert segment at the standard position | ADT^A01 `MSH, EVN, PID, PV1, ROL`, Show Schema Fields, right-click greyed NK1 → Insert; then Ctrl+Z | NK1 lands between PID and PV1 (not after ROL); in a `#`-delimited message it is `NK1#…`; Ctrl+Z removes it and earlier edits stay undoable | |

## 8. Multi-Tab Support

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-TAB-01 | P0 | Open multiple files | File > Open 3 files sequentially | 3 tabs visible | |
| BL-TAB-02 | P0 | Tab switching preserves state | Switch between tabs | Content, cursor, tree restored per tab | |
| BL-TAB-03 | P0 | Close tab with X | Click X on tab | Tab closes, next one active | |
| BL-TAB-04 | P1 | Middle-click closes tab | Mouse middle button on tab | Tab closes | |
| BL-TAB-05 | P1 | Modified indicator | Edit without saving | Dot or asterisk shown on tab | |
| BL-TAB-06 | P1 | New tab button (+) | Click + | Empty tab created | |
| BL-TAB-07 | P1 | Tab context menu | Right-click tab | Shows Close, Close Others | |
| BL-TAB-08 | P1 | Opening same file twice | Open file already open; also through `./`, `../` and a symlinked folder | Focuses existing tab, doesn't duplicate | |

## 9. File Operations

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-FILE-01 | P0 | Open .hl7 file | File > Open File, select .hl7 | File loads in new tab | |
| BL-FILE-02 | P0 | Open .txt file | Open .txt with HL7 content | Parses correctly | |
| BL-FILE-02b | P1 | Lenient opening | Open files with a leading blank line, a UTF-16 (Notepad "Unicode") encoding, MLLP framing, an FHS/BHS batch header, and one starting `XYZ|` | The first four parse; the last opens as text with a dialog saying why it is not parsed | |
| BL-FILE-03 | P0 | Open .json FHIR | Open FHIR resource .json | Detected as FHIR, tree shows resource | |
| BL-FILE-04 | P1 | Save overwrites file | Edit, Ctrl+S | Original file updated | |
| BL-FILE-05 | P1 | Save As to new file | Ctrl+Shift+S | New file created at chosen path | |
| BL-FILE-06 | P1 | Recent files list | Open file, reopen app | File in File > Recent Files | |
| BL-FILE-07 | P1 | Recent file click | Click entry in Recent | File opens | |
| BL-FILE-08 | P1 | Clear recent | File > Clear Recent | List emptied | |
| BL-FILE-09 | P2 | Drag & drop file | Drag .hl7 into window | File opens | |
| BL-FILE-10 | P0 | Open and save a Latin-1 file in the app | Open `tests/fixtures/hl7/adt_a01_latin1.hl7`; edit PID-8; Save | Opens with accented names intact (no error); the saved file is still ISO-8859-1 (`file` reports ISO-8859 text) | |
| BL-FILE-11 | P0 | A read-only file is not replaced | Linux/macOS: `chmod 444 m.hl7`; Windows: Properties → Read-only. Open it, edit, Ctrl+S | An error names the file as read-only; the file on disk is unchanged and no `.bridgelab-tmp` file is left; Save As to another name works | |

## 10. Validation

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-VALID-01 | P0 | Valid message: no errors | Load `adt_a01_small.hl7`, F6 | 0 errors, maybe warnings | |
| BL-VALID-02 | P0 | Detect missing MSH | Load invalid file, F6 | Error STRUCT-001/002 reported | |
| BL-VALID-03 | P0 | Detect missing MSH-9 | Message without MSH-9 | Error MSH-001 or MSH-002 | |
| BL-VALID-04 | P1 | Detect missing MSH-10 | Message without MSH-10 | Error REQ-MSH-10, reported once (no MSH-003 warning beside it) | |
| BL-VALID-05 | P1 | Detect missing required field | PID without PID-3 or PID-5 | Error REQ-PID-3/5 | |
| BL-VALID-06 | P1 | Field length validation | Field exceeds max_length | Warning LEN-... | |
| BL-VALID-07 | P1 | Data type validation (SI) | Non-numeric in SI field | Warning TYPE-SI-... | |
| BL-VALID-08 | P1 | Validation panel shows results | F6 | Bottom panel shows issues list | |
| BL-VALID-09 | P1 | Filter by severity | Click error/warning/info badges | List filters to chosen severity | |
| BL-VALID-10 | P1 | Sort by severity/segment | Use sort dropdown | Issues reorder | |
| BL-VALID-11 | P2 | Click issue navigates to field | Press F6 on a message with blank lines between segments; click the PID-7 issue | The editor selects PID-7 and the tree selects PID-7; the row shows the segment number | |
| BL-VALID-12 | P2 | Report follows edits | Press F6, then delete an OBX line | With auto-parse on, the counts and issues update; with it off, the panel says the report is out of date | |
| BL-VALID-12 | P1 | Close validation panel | Click X or Ctrl+J | Panel hides | |
| BL-VALID-13 | P1 | Required fields for every standard segment | ORU^R01 with an OBX lacking OBX-11; VXU with RXA lacking RXA-5 | `REQ-OBX-11` and `REQ-RXA-5` errors (neither segment was in the old fixed list); `adt_a01_small.hl7` still validates with no issues | |
| BL-VALID-14 | P2 | Required fields follow the declared version | Same PID in a message declaring `2.1` and one declaring `2.5` | Findings differ where the standards differ; no crash on a v2.1 message with fields beyond its 20 | |
| BL-VALID-15 | P1 | Data types: dates, timestamps, numbers | Validate `PID-7 = 19801399`, `MSH-7 = 2024-01-01 12:00`, `EVN-2 = notadate`; an OBX with OBX-2 `NM` and OBX-5 `abc`, then OBX-5 `6.2` | Warnings TYPE-DTM-PID-7, TYPE-DTM-MSH-7, TYPE-DTM-EVN-2, TYPE-NM-OBX-5; `6.2` and `20240229` are accepted, `20230229` is not | |
| BL-VALID-16 | P2 | Second MSH reported | Paste two messages one after the other and validate | Warning STRUCT-004 on the second MSH | |
| BL-VALID-17 | P2 | Unknown MSH-12 version named | Validate a message declaring `2.8`, then `2.5` | `2.8`: info MSH-005 naming v2.5 as the catalogue used; `2.5`: no MSH-005 | |
| BL-VALID-18 | P2 | Lengths in characters | PID-5 of 132 accented characters (262 UTF-8 bytes) in a v2.5 message | No LEN-PID-5 warning (limit 250) | |

## 11. FHIR Support

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-FHIR-01 | P0 | FHIR JSON auto-detected | Paste `{"resourceType":"Patient",...}` | Format "FHIR JSON" in status bar | |
| BL-FHIR-02 | P0 | FHIR tree view | Parse FHIR Patient | Tree shows resourceType, id, name, etc. | |
| BL-FHIR-03 | P1 | Expand array in tree | Click arrow on name array | Shows [0], [1] entries | |
| BL-FHIR-04 | P1 | FHIR XML detection | Paste FHIR XML | Format "FHIR XML" detected | |
| BL-FHIR-05 | P1 | FHIR validation | Load Patient, F6 | Validation report (error on bad gender, etc.) | |
| BL-FHIR-06 | P1 | FHIR Bundle analysis | Load bundle, Tools > Bundle Visualizer | Modal opens with entries | |
| BL-FHIR-07 | P1 | meta.profile surfaced | Load a resource declaring `meta.profile`, F6 | Info finding lists the canonical URL | |
| BL-FHIR-08 | P1 | Unchecked conformance is stated | Load a resource whose `resourceType` no package defines (e.g. a typo), F6 | Info finding says conformance was **not** checked for that type and points at Tools → FHIR profile packages | |
| BL-FHIR-09 | P0 | Bundle entries are validated | Load a `message` Bundle whose entry Observation has no `status`, F6 | Error "Observation.status is required" at `Bundle.entry[n].resource.status` from the built-in core; status bar counts it | |
| BL-FHIR-10 | P0 | urn:uuid entries need no id | Same Bundle: `urn:uuid:` fullUrls, no `id` anywhere | No finding about ids on entries (only the root's info) | |
| BL-FHIR-11 | P1 | Persistent fullUrl agrees with id | Entry `fullUrl` `http://…/Patient/1` with resource `id` `2` | Warning at `entry[n].fullUrl` ("disagrees") | |
| BL-FHIR-12 | P1 | Dangling reference in a message Bundle | `DiagnosticReport.result.reference` → a urn no entry carries | Warning at `entry[n].resource.result[0].reference` ("does not resolve") | |
| BL-FHIR-13 | P1 | Contained resources are validated | Observation with a `contained` Patient whose `gender` is `yes` | Error at `contained[0].gender`; the built-in core also reports the contained resource's structure | |
| BL-FHIR-14 | P1 | Entry profile honoured | Package installed; entry declares a profile requiring `gender`, none given | Error at `Bundle.entry[n].resource.gender` | |
| BL-FHIR-15 | P1 | Endpoint without scheme | Package installed; `MessageHeader.destination.endpoint` = `https//host/x` | Warning "no scheme" on that element; a `url` elsewhere written relatively is not reported | |
| BL-FHIR-16 | P2 | Primitive lexical checks | Package installed; `birthDate` `2018-13-40`, a `dateTime` without time zone | Errors naming the type and the expected form | |
| BL-FHIR-17 | P1 | resolve() over urn:uuid | Message Bundle, FHIRPath `Bundle.entry.resource.ofType(Observation).subject.resolve().name.family` | The Patient's family name | |
| BL-FHIR-XML-01 | P0 | FHIR XML typed like JSON | Validate `<Patient><active value="true"/><multipleBirthInteger value="2"/>…`; FHIRPath `Patient.active = true`; an Observation with `valueQuantity/value = 6.30`: `Observation.value.value > 6.2` and `Observation.value.ofType(Quantity)` | No type errors; `true`; `true` and one Quantity — the same answers as the JSON encoding | |
| BL-FHIR-XML-02 | P1 | XML meta.profile applied | A Patient in XML with one `<meta><profile value="…"/></meta>` naming an installed profile that requires birthDate, without birthDate; also as a test case expected valid | The profile's findings appear exactly as for the JSON encoding; the test case fails (no false pass) | |

## 12. FHIR Bundle Visualizer

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-BUNDLE-01 | P0 | Open visualizer | Tools > FHIR Bundle Visualizer | Modal opens with the entry list and the detail pane (details, references out and in, raw JSON) | |
| BL-BUNDLE-02 | P0 | List shows all entries | Load bundle with 10 entries | 10 entries shown | |
| BL-BUNDLE-03 | P0 | Click entry shows details | Click any entry | Right pane shows resource JSON | |
| BL-BUNDLE-04 | P0 | Outgoing references clickable | Entry has refs | References listed, click navigates | |
| BL-BUNDLE-05 | P0 | Incoming references | Select referenced entry | Shows "Referenced by" list | |
| BL-BUNDLE-06 | P0 | Dangling reference highlighted | Load bundle with broken ref | Reference shown with "dangling" badge | |
| BL-BUNDLE-07 | P1 | Search filter works | Type in search | List narrows to matching entries | |
| BL-BUNDLE-08 | P1 | Type filter works | Select type from dropdown | List shows only that resource type | |
| BL-BUNDLE-09 | P1 | Resource type counts | Check header | Shows bundle type, entry count, dangling refs | |
| BL-BUNDLE-10 | P1 | Patient display name | Load Patient with name | Display name shown correctly | |
| BL-BUNDLE-11 | P1 | Observation display | Load Observation | Code text displayed | |
| BL-BUNDLE-12 | P2 | Rejects non-Bundle | Try on single Patient | Error message shown | |

## 13. FHIRPath Evaluator

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-FP-01 | P0 | Panel opens | Ctrl+P | FHIRPath panel opens at bottom | |
| BL-FP-02 | P0 | Simple path works | `Patient.gender` | Returns gender value | |
| BL-FP-03 | P0 | Array flatten | `Patient.name.family` | All family names listed | |
| BL-FP-04 | P0 | Array index | `Patient.name[0].family` | First family name | |
| BL-FP-05 | P0 | count() function | `Bundle.entry.count()` | Returns number | |
| BL-FP-06 | P0 | first() / last() | `Patient.name.first().family` | First result only | |
| BL-FP-07 | P0 | where() filter | `Bundle.entry.where(resource.resourceType = 'Patient')` | Filtered entries | |
| BL-FP-08 | P1 | select() | `Bundle.entry.select(resource.resourceType)` | List of types | |
| BL-FP-09 | P1 | distinct() | `Bundle.entry.select(resource.resourceType).distinct()` | Unique types | |
| BL-FP-10 | P1 | Invalid expression error | `Patient.name[` | Error names the problem, not a silent empty result | |
| BL-FP-11 | P1 | Example chips work | Click an example | Expression run | |
| BL-FP-12 | P2 | History persists | Run 3 queries | History chips show | |
| BL-FP-13 | P0 | Boolean logic is three-valued | `true and {}` then `false and {}` | Empty collection, then `false` | |
| BL-FP-14 | P0 | Operator precedence | `1 + 2 * 3` | `7` | |
| BL-FP-15 | P0 | Choice element by base name | `Observation.value.unit` on a valueQuantity | Unit returned (no need to write `valueQuantity`) | |
| BL-FP-16 | P1 | Partial-precision dates | `@2015-02-04 = @2015-02` | Empty collection (indeterminate), not true/false | |
| BL-FP-17 | P1 | Quantity unit conversion | `4 'g' = 4000 'mg'` | `true` | |
| BL-FP-18 | P1 | Date duration arithmetic | `Patient.birthDate + 18 years` | Date 18 years later, same precision | |
| BL-FP-19 | P1 | resolve() follows a Reference | On a Bundle: `Bundle.entry.resource.ofType(Observation).subject.resolve().id` | Referenced Patient id | |
| BL-FP-20 | P1 | sort() orders results | `(3 \| 1 \| 2).sort()` then `sort(-$this)` | Ascending, then descending | |
| BL-FP-21 | P1 | trace() shows captures | `Patient.name.trace('names').count()` | Count returned **and** a "names" block listed under the result | |
| BL-FP-22 | P1 | Unknown function reported | `Patient.nosuchfn()` | "Unknown function" error | |
| BL-FP-23 | P2 | Singleton misuse reported | `Patient.name.given > 1` | Error mentioning a single value | |
| BL-FP-24 | P2 | Conformance suite | `./scripts/fetch-fhirpath-suite.sh` then `BL_FHIRPATH_SUITE=… cargo test --test fhirpath_suite` | Pass rate at or above the recorded baseline | |

## 14. Communication - MLLP

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-MLLP-01 | P0 | Open communication panel | Ctrl+K | Panel opens at bottom | |
| BL-MLLP-02 | P0 | MLLP tab shows current message | Have message, open panel | Shows tab name and byte count | |
| BL-MLLP-03 | P0 | Send without message disabled | No message loaded | Send button disabled | |
| BL-MLLP-04 | P0 | Send to localhost | Start receiver on 2575, click Send | Connection attempt, success/fail | |
| BL-MLLP-05 | P0 | ACK detection | Server sends MSA\|AA | Result labeled "ACK (Accept)" | |
| BL-MLLP-06 | P0 | NACK AE detection | Server sends MSA\|AE | Result labeled "NACK (Application Error)" | |
| BL-MLLP-07 | P0 | NACK AR detection | Server sends MSA\|AR | Result labeled "NACK (Application Reject)" | |
| BL-MLLP-08 | P1 | Connection timeout | Non-existent host | Error "Connection timed out" after timeout | |
| BL-MLLP-09 | P1 | Listen for incoming | Click Listen on port 2576 | Waits for connection | |
| BL-MLLP-15 | P0 | Listener is local by default | Fresh start, Listen with the defaults; from another machine `nc <host> 2576`; then click *Accept connections from other machines*, restart the listener, repeat | Default bind `127.0.0.1` with the "only this computer" hint; the remote connection is refused. After the switch the field reads `0.0.0.0`, the network warning shows, and the remote connection is accepted | |
| BL-MLLP-10 | P1 | Auto-ACK on receive | Send message to listener with auto-ack | Sender receives ACK | |
| BL-MLLP-11 | P1 | Received message opens in new tab | Listener receives | New tab with received content | |
| BL-MLLP-12 | P1 | Advanced settings toggle | Click "Advanced MLLP Settings" | Panel expands with extra fields | |
| BL-MLLP-13 | P2 | Custom framing chars | Change start/end chars | Used in MLLP frame | |
| BL-MLLP-14 | P1 | Console filters by outcome | Listener with ACK code AA receives 2 messages; switch to AE, receive 1; turn auto-ACK off, receive 1; send garbage bytes | Chips read `All 5 · AA 2 · AE 1 · AR 0 · No ACK 1 · Errors 1`; each chip narrows the rows to that outcome; `Clear` resets counts to 0 | |

## 15. Communication - HTTP

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-HTTP-01 | P0 | GET request works | Set URL to public API, method GET, Send | Response shown | |
| BL-HTTP-02 | P0 | POST request with body | POST to test endpoint | Body sent, response received | |
| BL-HTTP-03 | P0 | Response status shown | After request | Status code + text displayed | |
| BL-HTTP-04 | P1 | Custom headers sent | Add "Content-Type: application/fhir+json" | Header in request | |
| BL-HTTP-05 | P1 | Response headers expandable | Click "Response Headers" | List of headers shown | |
| BL-HTTP-06 | P1 | Timeout respected | Short timeout, slow server | Fails after timeout | |
| BL-HTTP-07 | P1 | Basic auth | Enable Basic Auth, set user/pass | Authorization header sent | |
| BL-HTTP-08 | P1 | Bearer token | Enable Bearer, set token | Authorization: Bearer sent | |
| BL-HTTP-09 | P1 | Body fallback to active tab | Empty body, message in tab | Tab content sent as body | |

## 16. History

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-HIST-01 | P1 | MLLP send logged | Send MLLP message | Entry appears in History tab | |
| BL-HIST-02 | P1 | HTTP request logged | Send HTTP request | Entry appears in History | |
| BL-HIST-03 | P1 | Click entry shows detail | Click in history | Detail panel shows all fields | |
| BL-HIST-04 | P1 | Clear history | Click Clear All | List empties | |
| BL-HIST-05 | P2 | Persists across restart | Close, reopen app | History still present | |
| BL-HIST-06 | P1 | MLLP send records its ACK code | Send to a listener answering AA, then to one answering AE (Listen tab, ACK code = AE) | Rows show a green `AA` / red `AE` badge after the OK status; detail says `OK · ACK AE`; an HTTP row has no badge | |
| BL-HIST-07 | P1 | Filter chips with counts | With the rows above, click `AE`, then `Failed`, then `All` | Chips read e.g. `All 3 · AA 1 · AE 1 · AR 0 · No ACK 0 · Failed 1`; `AE` shows the one row; a chip whose count is 0 is disabled; `All` restores the list | |
| BL-HIST-08 | P2 | Upgrade keeps old rows | Open a database written before this version | History still lists; old rows have no ACK badge and fall under `No ACK` only when they are MLLP OK rows | |

## 17. Anonymization

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-ANON-01 | P0 | Open dialog | Tools > Anonymize | Modal opens with PHI list | |
| BL-ANON-02 | P0 | Detect PID-5 (name) | Message with PID-5 set | Name flagged HIGH sensitivity | |
| BL-ANON-03 | P0 | Detect PID-7 (DOB) | Has birthdate | Flagged HIGH | |
| BL-ANON-04 | P0 | Detect PID-19 (SSN) | Has SSN | Flagged HIGH | |
| BL-ANON-05 | P1 | High sensitivity: REDACTED | Anonymize | Text fields become REDACTED | |
| BL-ANON-06 | P1 | High sensitivity: 0s for numeric | Anonymize SSN | Numeric becomes 000000000 | |
| BL-ANON-07 | P1 | Medium sensitivity: X*** | Anonymize NK1-2 | First char kept, rest *** | |
| BL-ANON-08 | P1 | Anonymized opens in new tab | Click Open in New Tab | New tab with anonymized content | |
| BL-ANON-09 | P1 | Copy to Clipboard works | Click Copy | Clipboard has anonymized version | |
| BL-ANON-10 | P2 | Structure preserved | Compare original vs anonymized | Same segments, same positions | |
| BL-ANON-11 | P0 | Every PHI field of the covered segments is masked | Message with PID-23/29, NK1-5/6/16/30..33/37, GT1-2/7/8/16..19, IN1-18/19/44/49, IN2-2/63, PV1-19 and an `OBX\|1\|TX` free-text result, all filled; Tools > Anonymize > Anonymize; search the output for each original value | Every field is listed in the dialog; none of the values is in the output; a numeric OBX (`NM`) result and the insurer's name are unchanged | |
| BL-ANON-12 | P1 | No false "no PHI" | Message whose only PHI is IN2-2 and GT1-8 | The dialog lists both fields; a message with no PHI at all says the fields BridgeLab checks hold none and that free text elsewhere is not checked | |
| BL-ANON-13 | P1 | v2.3 identifiers stay valid | *Lab order* sample (v2.3) → Anonymize → F6 | PID-3 reads `000…^^^HOSP^MR`-style (authority and type kept); no length error | |
| BL-ANON-14 | P0 | Batch anonymize never replaces a file | A/patient.hl7 (patient NEWPAT) and B/patient.hl7 (another message); Tools → Batch anonymize…, input A/patient.hl7, output folder B; then on Linux/macOS put a symlink and a hard link to a selected source in the output folder and run again | The row says a file of that name already exists; B/patient.hl7 and the sources are unchanged; the output column shows no `\\?\` prefix on Windows | |
| BL-ANON-15 | P1 | Generator "Save all" never replaces a file | `echo OLD > gen/adt_a01_001.hl7`; generate 3 ADT^A01; *Save all to folder…* → gen | "Saved 2 of 3 files"; adt_a01_001.hl7 listed as not saved and still reads OLD; a seed of `-1`, `1.5` or `1e20` gives the whole-number message | |
| BL-ANON-17 | P0 | Multi-byte names | Paste a message with PID-5 `李小龍^Jan` and PID-11 city `Västerås`; Tools → Anonymize → Anonymize; then set PID-7 to `1990051年` and press F6; open a Big5 file declaring `BIG-5` | The masked message opens in a new tab and the app keeps running; F6 reports a PID-7 type error; the Big5 file shows the name correctly, with no charset warning | |
| BL-ANON-16 | P0 | Tools work on the editor text | Settings → Parser: auto-parse off. Paste message A (ALPHA^ANNA), select all and type message B (BRAVO^BORIS); Tools → Anonymize; then Export JSON | The dialog lists BRAVO^BORIS and the output is message B masked; the export is message B; text that does not parse says so instead of using the old parse | |

## 18. Templates

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-TMPL-01 | P0 | Open template dialog | Ctrl+N | Modal opens with template list | |
| BL-TMPL-02 | P0 | Search templates | Type "adt" | List filters to ADT templates | |
| BL-TMPL-03 | P0 | Preview on select | Click template | Preview shown in right pane | |
| BL-TMPL-04 | P0 | Create from template | Click Create Message | New tab with template content | |
| BL-TMPL-05 | P1 | Double-click creates | Double-click template | Same as Create button | |
| BL-TMPL-06 | P1 | Timestamps populated | Create ADT^A01 | MSH-7 has current time, MSH-10 unique | |
| BL-TMPL-07 | P1 | Categories shown | Browse list | Templates grouped by category | |
| BL-TMPL-08 | P1 | Sample messages open from the welcome screen and File menu | Welcome screen → *Sample messages*; File → *Sample Messages…* | The same dialog: 13 samples grouped by category, a version chip per version (2.3, 2.5, 2.5.1) plus *All versions*, a preview, the note that data are fictional | |
| BL-TMPL-09 | P0 | A sample opens parsed and clean | Pick *Lab results: complete blood count*, *Open in new tab*, press F6 | A new tab named `ORU^R01 v2.5.1` with the message parsed (12 OBX in the tree) and validation reporting no errors or warnings; the same for any other sample | |
| BL-TMPL-10 | P1 | Templates validate and are stamped in local time | With TZ Europe/Rome, create SIU^S12 and MDM^T02 from templates, F6; create two ADT^A01 in the same second | No errors; MSH-7 is the local time with `+0200`/`+0100`; the two MSH-10 differ | |
| BL-TMPL-10 | P2 | Filters | Type "merge"; pick the 2.3 chip | "merge" leaves the ADT^A40 sample; 2.3 leaves the ORM and the metabolic panel; *Open* is disabled when the selected sample is filtered out | |

## 19. Test Case Library

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-TCLIB-01 | P0 | Open library | Ctrl+L | Modal opens | |
| BL-TCLIB-02 | P0 | Save current message | Have message loaded, click Save Current | Form populated, save button enabled | |
| BL-TCLIB-03 | P0 | Save new test case | Fill form, click Save | Test case appears in list | |
| BL-TCLIB-04 | P0 | Load test case | Select case, click Load in Editor | Opens in new tab | |
| BL-TCLIB-05 | P1 | Edit test case | Select, click Edit | Form opens with current values | |
| BL-TCLIB-06 | P1 | Delete test case | Click Delete, confirm | Removed from list | |
| BL-TCLIB-07 | P1 | Search filter | Type in search | List filters by name/description/tags | |
| BL-TCLIB-08 | P1 | Category grouping | Save cases with different categories | Grouped in list | |
| BL-TCLIB-09 | P1 | Tags shown as chips | Save case with tags | Chips visible | |
| BL-TCLIB-10 | P2 | Persist across restart | Save, close, reopen | Cases still present | |
| BL-TCLIB-11 | P0 | Export a pack | Library with an HL7 v2 case holding a patient name and a FHIR case; *Export…* with no search, save | The export panel lists the HL7 v2 case with PID-5 among its fields and the FHIR case as "not checked"; the file is `format: bridgelab-test-cases`, `format_version: 1`, with both cases and their ids; with a search active only the matching cases are exported | |
| BL-TCLIB-12 | P1 | Export with PHI masked (Pro) | Same library under Pro (or trial): tick *Mask personal data*, export; then under Community | Pro: the file's HL7 v2 content has the PHI masked, the library still shows the original; the result names how many cases were masked. Community: the box is disabled with a PRO badge | |
| BL-TCLIB-13 | P0 | Import preview and conflicts | Export a pack, edit one case, delete another, import the pack | Preview lists: the deleted one as *New*, the untouched ones as *Already in the library*, the edited one as *Differs* with a choice; *Keep mine* leaves it, *Replace* restores the exported version (creation date kept), *Keep both* adds "… (imported)"; the summary counts match what is written | |
| BL-TCLIB-14 | P1 | Import respects the Community cap | Community with 9 cases; import a pack with 3 new cases | The preview warns that there is room for 1; *Import* is disabled; nothing is written. Under Pro the same import adds all 3 | |
| BL-TCLIB-15 | P2 | Foreign or future files | Import a plugin pack JSON, a non-JSON file, and a pack with `format_version: 2` | Clear error each time ("Not a BridgeLab test case pack", "Not a JSON file", "made by a newer BridgeLab"); the library is unchanged | |
| BL-TCLIB-16 | P2 | Search, dates and re-import | Search for a control ID that is only in a case's message text; with TZ Europe/Rome note a case's *Updated* time; import a hand-written pack with `"category": ""` twice | The case is found; *Updated* shows the local time of the save; the second import lists the case as already in the library, not *Differs* | |
| BL-TCLIB-17 | P2 | Pack export keeps its extension | *Export…*, type `cases` with no extension | The file is `cases.bltests.json` | |

## 20. Export

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-EXP-01 | P1 | Export JSON | Tools > Export JSON | A save dialog proposes `<tab>.json`; the file is written where chosen; typing a name without extension (Linux) saves `name.json`, asking first if that file exists | |
| BL-EXP-02 | P1 | Export CSV | Tools > Export CSV | A save dialog proposes `<tab>.csv`; the file is written where chosen; a name typed without extension gets `.csv` | |
| BL-EXP-03 | P1 | JSON structure correct | Open exported JSON (and `bridgelab-cli to-json`) | Contains message_type, version, segments; each segment's fields in message order (MSH-2 before MSH-10) | |
| BL-EXP-04 | P1 | CSV structure correct | Open CSV in Excel (Windows, Italian or German regional format) with PID-5 `Müller^José` and an OBX-5 of `-2.3` | Header: Segment,Position,Field,Value; accents shown correctly (UTF-8 BOM); `-2.3` is a number, a value starting with `=` gets a leading `'`. With a `;` list separator Excel may still put rows in one column: use Data → From Text/CSV | |

## 21. Theme & Appearance

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-THEME-01 | P0 | Dark theme default | Fresh install | Dark colors | |
| BL-THEME-02 | P0 | Switch to light | View > Theme > Light | Light colors applied | |
| BL-THEME-03 | P0 | Monaco respects theme | Switch theme | Editor background matches | |
| BL-THEME-04 | P1 | Theme persists | Close and reopen | Previous theme restored | |
| BL-THEME-05 | P1 | Settings modal theme consistency | Open Settings in both themes | Colors consistent | |
| BL-THEME-06 | P2 | All panels themed | Open all panels | All use theme colors | |

## 22. Internationalization (i18n)

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-I18N-01 | P0 | Switch to Italian | View > Language > Italiano | Menu, dialogs, tooltips in IT | |
| BL-I18N-02 | P1 | Switch to French | Select Français | UI in French | |
| BL-I18N-03 | P1 | Switch to Spanish | Select Español | UI in Spanish | |
| BL-I18N-04 | P1 | Switch to German | Select Deutsch | UI in German | |
| BL-I18N-05 | P0 | Language persists | Restart app | Previous language loaded | |
| BL-I18N-06 | P1 | About dialog translated | Open About in each language | Copyright and description translated | |
| BL-I18N-07 | P1 | Status bar translated | Check the segments and folded-fields labels | Translated | |

## 23. Settings

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-SET-01 | P0 | Open settings | Ctrl+, or Edit > Settings | Modal opens with 8 sections: Editor, Display, Keyboard Shortcuts, Parser, Performance, Plugins, Privacy, License Activation | |
| BL-SET-02 | P0 | Change font size | Set to 16, save | Editor font increases | |
| BL-SET-03 | P1 | Change font family | Pick different font | Editor font changes | |
| BL-SET-04 | P1 | Theme switcher inside settings | Display section, click Light | Theme updates after save | |
| BL-SET-05 | P1 | Language switcher | Display section, pick lang | UI updates after save | |
| BL-SET-06 | P1 | Fold threshold | Change *Fold fields longer than* to 50 in Parser; paste a message with a 120-character OBX-5 | OBX-5 shows as a fold chip and the status bar says 1 folded; the text and a saved file keep all 120 characters | |
| BL-SET-07 | P1 | Settings persist | Close, reopen | Values retained | |
| BL-SET-08 | P2 | Cancel discards changes | Edit, click Cancel | No changes applied | |

## 24. Licensing

Cases marked **debug build only** use simple `BL-FREE/PRO/ENT-…` keys, which
are compiled only into debug builds (`pnpm tauri dev`); a release build
accepts only signed licenses. Cases marked **maintainer only** need a license
signed with the private keygen, which is not part of the public repository.
None of these cases is covered by CI.

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-LIC-01 | P0 | Trial starts on first launch | Fresh install | 14 days trial active | |
| BL-LIC-02 | P0 | Trial banner shows days | Check top of window | Yellow banner with days remaining | |
| BL-LIC-03 | P1 | Banner urgent ≤3 days | Simulate ≤3 days remaining | Red, non-dismissible banner | |
| BL-LIC-04 | P0 | Open activation dialog | Click *Activate* on the trial banner | Dialog opens | |
| BL-LIC-05 | P0 | Activate Free license (**debug build only**) | Enter `BL-FREE-ABCD1234EFGH` | Free activated | |
| BL-LIC-06 | P0 | Activate Pro license (**debug build only**) | Enter `BL-PRO-12345678ABCD` | Pro activated | |
| BL-LIC-07 | P0 | Activate Enterprise (**debug build only**) | `BL-ENT-ENTERPRISEKEY` | Enterprise activated | |
| BL-LIC-08 | P1 | Invalid key rejected | Enter "INVALID" | Error shown | |
| BL-LIC-09 | P1 | Short key rejected (**debug build only**) | `BL-PRO-ab` | Error (too short) | |
| BL-LIC-10 | P1 | Hardware ID shown | Open activation | BL-XXXXXXXXXXXXXXXX visible | |
| BL-LIC-11 | P1 | Feature list correct | After Pro activation | Shows fhir, mllp, http, anonymize, export | |
| BL-LIC-12 | P1 | Deactivate works | Click Deactivate | Returns to trial | |
| BL-LIC-13 | P0 | License persists | Activate, close, reopen | Still active | |
| BL-LIC-14 | P1 | Signed Ed25519 key (**maintainer only**) | Activate a key signed with the maintainers' keygen | Key activates, signature verified | |
| BL-LIC-15 | P1 | Buy links open the pricing page | Help → Buy a License…; activation dialog → *See prices & buy* on Pro and on Enterprise; trial banner → *Compare plans*; trigger a Pro-only feature on Community → *See prices* | Each opens the browser at `…/BridgeLab/?utm_source=app&utm_medium=…#pricing` (medium: `menu`, `activation`, `trial_banner` or `upgrade_prompt`), scrolled to the pricing cards; *Close* on the prompt opens nothing | |

## 25. bridgelab-cli

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-CLI-01 | P0 | Validate valid file | `bridgelab-cli validate good.hl7` | OK, exit 0 | |
| BL-CLI-02 | P0 | Validate invalid file | Invalid file, strict mode | Errors listed, exit 1 | |
| BL-CLI-03 | P0 | JSON output | `--format json` | Valid JSON on stdout | |
| BL-CLI-04 | P0 | JUnit XML output | `--format junit` | Valid XML on stdout | |
| BL-CLI-05 | P1 | Glob pattern | `"*.hl7"` | All files processed | |
| BL-CLI-06 | P1 | Info command | `info file.hl7` | Metadata table shown | |
| BL-CLI-07 | P1 | Info --json | `info file.hl7 --json` | Structured JSON | |
| BL-CLI-08 | P1 | Anonymize to stdout | `anonymize file.hl7` | Anonymized HL7 printed | |
| BL-CLI-09 | P1 | Anonymize to file | `anonymize file.hl7 -o out.hl7` | File written | |
| BL-CLI-10 | P1 | to-json converts | `to-json file.hl7` | Structured JSON on stdout | |
| BL-CLI-11 | P1 | Batch directory | `batch ./messages` | Summary printed | |
| BL-CLI-12 | P1 | Batch --json | `batch ./dir --json` | JSON summary | |
| BL-CLI-13 | P2 | Help text | `bridgelab-cli --help` | All commands listed | |
| BL-CLI-14 | P0 | FHIR file validated | `bridgelab-cli validate tests/fixtures/fhir/bundle_patient.json` | Report kind FHIR; entries validated; exit 0 | |
| BL-CLI-15 | P0 | Same findings as the app | Validate the same HL7 and FHIR files in the app (F6) and in the CLI | Identical findings and counts | |
| BL-CLI-16 | P1 | Packages directory override | `validate x.json --fhir-packages <dir with a distilled IG>` | The IG's profiles are applied; built-in R4 core still present | |
| BL-CLI-17 | P1 | Free, no licence read | Machine with a Pro licence activated, and one without; run `validate`, `batch`, `anonymize`, `fhirpath` | Same results on both; the Community plugin cap applies on both; no licence file is read | |
| BL-CLI-18 | P1 | Release asset | Download `bridgelab-cli-<target>` from a release | Runs; `--version` prints the CLI version | |
| BL-CLI-19 | P1 | Standard input | `cat x.hl7 \| bridgelab-cli validate -`, same for `info -`, `to-json -`, `anonymize -` | Same output as with the file name; the report names the input `-` | |
| BL-CLI-20 | P1 | `fhirpath` | `fhirpath 'Bundle.entry.count()' bundle.json`; `--json`; a broken expression | One value per line; the JSON result with count and trace; the broken expression prints the parser error and exits 1 | |
| BL-CLI-21 | P1 | `xsd` Community set | `xsd "ADT^A01"`; `xsd SIU_S12`; `xsd FOO_X01` (in Windows `cmd.exe` too: an unquoted `^` is dropped there) | ADT_A01 is a well-formed XSD; SIU_S12 exits 1 naming the Community set and Pro; the unknown code exits 2 | |
| BL-CLI-22 | P0 | `send` over MLLP | Against a listener answering AA, then AE; then a closed port | AA: ACK printed, exit 0. AE: ACK printed, exit 1. Closed port: "Connection failed", exit 1. A file with LF line endings arrives with CR segment separators | |
| BL-CLI-23 | P0 | `test` runs packs | Export a pack from the app; `test pack.bltests.json`, then `--format junit`; edit one expectation so it fails | Every case listed ✓/✗ with the reason; valid JUnit with one suite per pack; exit 1 when a case fails | |
| BL-CLI-24 | P0 | Legacy charset files | `tests/fixtures/hl7/adt_a01_latin1.hl7` (ISO-8859-1, MSH-18 `8859/1`): `validate`, `to-json`, `anonymize -o out.hl7`, `send` to a listener; `send --encoding FOO` | Valid; PID-5 reads `Müller^Jörg`; out.hl7 is ISO-8859-1 with PV1-3 `Lettò 2` intact; the listener receives Latin-1 bytes and the ACK is AA; the unknown encoding exits 2 | |
| BL-CLI-25 | P0 | Nothing silently skipped | `validate good.hl7 missing.hl7`; `test pack.bltests.json missing.bltests.json`; `batch` on a folder with `BAD.HL7`, with `--extension .hl7`, and on an empty folder | The missing path is reported (NOT FOUND / pack error) and exit 1; BAD.HL7 is found; the empty folder exits 1 | |
| BL-CLI-26 | P1 | Not checked is not a pass | `validate x.json --fhir-packages /nonexistent --format junit` | Warning on stderr and a FHIR-PACKAGES warning in the result; the JUnit test case is `skipped` with the reason | |
| BL-CLI-27 | P1 | Multi-message send | A file with two messages, to a listener answering AA; then an ACK whose MSA-2 names another message | Two frames, each acknowledged, exit 0; the mismatched ACK exits 1 | |
| BL-CLI-29 | P0 | Multi-byte names never crash | An ADT^A01 with PID-5 `李小龍^Jan`, PID-7 `19900515` and PID-11 city `Västerås`; run `anonymize`, then set PID-7 to `1990051年` and run `validate --format junit`; repeat with Lindström | `anonymize` exits 0 with the name and city masked; `validate` exits 1 with a PID-7 type error and a complete JUnit report; never exit 101 | |
| BL-CLI-30 | P0 | East Asian charsets | The same message with MSH-18 `BIG-5`, encoded in Big5 (陳四明 in PID-5), then GB 18030 and EUC-KR (`GB 18030-2000`, `KS X 1001`); `to-json`, `anonymize -o anon.hl7`; then MSH-18 `CNS 11643-1992` | PID-5 reads 陳四明^Jan and PID-7 is the birth date; anon.hl7 is in Big5 with the birth date masked; the CNS file is refused with the reason, exit 1 | |
| BL-CLI-31 | P0 | Send refuses what the charset cannot hold; UTF-16 input | `send utf.hl7 --encoding ASCII` with Müller/€ in PID-5, to a listener; then a UTF-16LE file with a BOM, no `--encoding` | The first exits 1 naming the characters and the listener receives nothing; the UTF-16 file arrives as UTF-8 (no BOM) and its AA exits 0 | |
| BL-CLI-32 | P1 | Anonymize on a terminal | `anonymize tests/fixtures/hl7/adt_a01_latin1.hl7` in cmd.exe and in a Linux terminal, then with `-o out.hl7` | The terminal shows "Lettò" correctly, one segment per line, exit 0; out.hl7 is Latin-1 | |
| BL-CLI-28 | P1 | Patterns ignore case; read-only output refused | `g/a.hl7`, `g/B.HL7`, `g/c.Hl7`: `validate "g/*.hl7" --no-plugins`; `chmod 444 golden.json` then `to-json in.hl7 --output golden.json` | All three files are validated; the `to-json` exits 1 naming the file as read-only and golden.json is unchanged | |

## 26. Updater

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-UPD-01 | P2 | Check for updates | Help > Check for Updates | Shows "latest version" or update available | |
| BL-UPD-02 | P2 | No update dialog | If no update | Alert "You are running the latest version" | |
| BL-UPD-03 | P2 | Update available | Run an older build; Help > Check for Updates | Dialog names the newer version and opens the GitHub release page on confirm. (In-app download and restart need signed artifacts and a `latest.json`, which the release pipeline does not produce.) | |
| BL-UPD-04 | P1 | Startup check announces a newer release | Run a build older than the latest GitHub release with a fresh profile; answer *Yes, check* to the first-start question | Accent-coloured banner "BridgeLab X is available (you have Y)" with Download / Skip this version / ×; no dialog. Download opens the release page | |
| BL-UPD-05 | P1 | Once a day, skip, and off switch | Restart within 24 h; then press Skip and advance the clock a day; then untick Settings → Privacy → Check for new versions at startup | No second request within 24 h; a skipped version is not announced again (a later one is); with the box unticked no request is made at all (check with a proxy log) | |
| BL-UPD-06 | P1 | Silent offline | Start with the network down, or behind a proxy that blocks api.github.com | No banner, no error, no delay in startup | |
| BL-UPD-07 | P1 | Windows installer asks once | Fresh Windows machine: run the NSIS setup in each installer language; answer No; start the app. Re-run the setup over it | The setup asks (Yes preselected) in its own language; with No, Settings → Privacy shows the box unticked and no request goes out; re-running the setup does not ask again; `setup.exe /S` never asks | |
| BL-UPD-08 | P1 | Machine policy locks it off | Set `BRIDGELAB_DISABLE_UPDATE_CHECK=1`, start; unset it, create `policy.json` with `{"disable_update_check": true}` in the platform folder, start | No request in either case; the Settings box is unticked, disabled, and the hint names the environment variable or the file path; a malformed policy file changes nothing | |
| BL-UPD-09 | P1 | First-start question where the installer did not ask | Fresh profile from the AppImage/deb/dmg/MSI (no `installer.json`): start; wait ~10 s. Close with ×, restart. Answer *No*, restart. Fresh Windows NSIS install answered Yes/No: start | Banner asks (no request before answering); × asks again at next start; *No* stores the choice (Settings box unticked) and no request is made; *Yes, check* checks immediately. After an NSIS install that asked, no question appears | |

## 27. Tree ↔ Editor Navigation

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-NAV-01 | P0 | Editor → Tree (segment) | Right-click on the segment name (e.g. "PID"), Show Segment in Tree; repeat with blank lines between segments | Tree opens, `seg{N}` node selected, scrolled to view; blank lines do not shift it | |
| BL-NAV-02 | P0 | Editor → Tree (field) | Right-click inside `Doe` of PID-5, Show Segment in Tree; then inside `SEND` of MSH-3 | Tree opens, segment expanded, PID-5.1 selected; then MSH-3 | |
| BL-NAV-03 | P0 | Editor → Tree (MSH-1 separator) | Right-click on the first `\|` in MSH, Show Segment in Tree | Tree selects MSH field at position 1 (Field Separator) | |
| BL-NAV-04 | P0 | Editor → Tree (MSH-2 encoding chars) | Right-click on `^~\&`, Show Segment in Tree | Tree selects MSH-2 node | |
| BL-NAV-05 | P0 | Tree → Editor (segment) | Right-click segment node in tree, Show in Editor | Monaco reveals the segment line, cursor at column 1 | |
| BL-NAV-06 | P0 | Tree → Editor (field) | Right-click field node (e.g. PID-5) in tree, Show in Editor | Monaco reveals the line, cursor at field start, field text selected | |
| BL-NAV-07 | P1 | Tree → Editor (component) | Right-click component node (e.g. PID-5.1), Show in Editor | Selection narrows to the component within the field | |
| BL-NAV-08 | P1 | Same-target re-trigger | Show in Editor, click elsewhere, Show in Editor on same node | Selection re-applies (stamp forces effect to re-run) | |
| BL-NAV-09 | P1 | Navigation localized | Switch to IT, right-click on tree node | Menu shows "Mostra nell'Editor" | |
| BL-NAV-10 | P2 | Placeholder suppresses Show in Editor | Enable Schema Fields, right-click a placeholder field | "Show in Editor" entry is hidden (no physical position) | |

## 28. Field Inspector

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-INSP-01 | P0 | Panel visible by default | Load a message, look at tree panel bottom half | Inspector shown with "Select a node..." placeholder | |
| BL-INSP-02 | P0 | Toggle from View menu | View → Field Inspector | Inspector shows/hides | |
| BL-INSP-03 | P0 | Toggle from tree header ⓘ button | Click the ⓘ in the tree panel header | Inspector shows/hides | |
| BL-INSP-04 | P0 | Segment selection | Click a segment node (e.g. PID) | Inspector shows segment code, name, description, field count | |
| BL-INSP-05 | P0 | Field selection with schema | Click PID-5 | Inspector shows position "PID-5", name "Patient Name", data type "XPN", required Yes, max length 250, description | |
| BL-INSP-06 | P1 | Required flag highlighted | Click a required field (e.g. MSH-9) | "Required: Yes" rendered with emphasized color | |
| BL-INSP-07 | P1 | Repeating flag shown | Click PID-3 (Patient Identifier List) | "Repeating: Yes" | |
| BL-INSP-08 | P1 | Current value displayed | Select a populated field | Value box shows the text, length reported | |
| BL-INSP-09 | P1 | Truncated badge & View Full | Select a truncated base64 field | Red "truncated" badge + "View full value" button visible | |
| BL-INSP-10 | P1 | View Full opens modal | Click "View full value" | Expanded field modal appears with full content | |
| BL-INSP-11 | P1 | Z-segment schema unknown | Click a ZDS field node | Inspector shows "Not in HL7 standard (Z-segment or custom)" | |
| BL-INSP-12 | P1 | Inspector translates | Switch to FR/IT/ES/DE | All inspector labels localized | |
| BL-INSP-13 | P2 | No selection fallback | Deselect / reload | Shows placeholder text | |
| BL-INSP-14 | P1 | Closed table: allowed values + warning | Select MSA-1 (ID, table 0008), value `AA`; then edit it to `XX` | Header "Allowed values · HL7 0008"; `AA` row highlighted; with `XX` the "not in the HL7 table" warning appears | |
| BL-INSP-15 | P1 | Open table: suggested values, never a warning | Select PID-8 (IS, table 0001), value `X` | Header "Suggested values (user-defined table) · HL7 0001"; no warning; the six standard codes listed | |
| BL-INSP-16 | P1 | Component has its own table | Expand MSH-9, select MSH-9.2 (`A01`) | Table 0003 Event type listed, `A01` highlighted; select MSH-9.1 → table 0076 | |
| BL-INSP-17 | P2 | User-defined table without standard values | Select IN1-2 (table 0072) | No table section; the rest of the metadata shown | |
| BL-INSP-18 | P1 | Any standard segment has metadata | Select RXA-5 in a VXU, then a field of a v2.1 message | Name, type and length shown (the old fixed list had no RXA); v2.1 metadata differs from v2.5 where the standard does (PID has 20 fields in v2.1) | |

## 29. Schema-aware Tree

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-SCHEMA-01 | P1 | Toggle from View menu | View → Show Schema Fields | Setting toggles on/off | |
| BL-SCHEMA-02 | P1 | Expanding segment injects placeholders | Enable the flag, expand PID in a minimal message | All PID-1..PID-20 slots shown; missing positions rendered dim/italic | |
| BL-SCHEMA-03 | P1 | Placeholders dimmed | Inspect placeholder rows | Opacity ~0.5, italic, trailing ` ·` marker | |
| BL-SCHEMA-04 | P1 | Real fields unaffected | Compare populated vs missing fields in same segment | Real fields full opacity, placeholders dim | |
| BL-SCHEMA-05 | P1 | Inspector still works on placeholders | Click a placeholder, and OBX-5 of a greyed OBX | Inspector shows schema info and "Not in this message" (no current value) | |
| BL-SCHEMA-06 | P1 | Show in Editor hidden on placeholders | Right-click a placeholder | Context menu has no "Show in Editor" entry | |
| BL-SCHEMA-07 | P1 | Toggling re-initializes tree | Expand PID, toggle flag twice | Placeholders appear/disappear consistently | |
| BL-SCHEMA-08 | P1 | Sort by field position | Expand PID with flag on | Fields ordered by numeric position (1, 2, 3, ... 20) | |
| BL-SCHEMA-09 | P2 | Flag localized in menu | Switch language, open View menu | "Show Schema Fields" translated | |
| BL-SCHEMA-10 | P2 | Unknown segment falls back | Expand a Z-segment with flag on | Only actual fields shown (no schema to merge) | |

## 30. Monaco Hover / Overflow

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-HOVER-01 | P0 | Hover visible near top of editor | Hover on an MSH field with the cursor near the top | Tooltip renders completely (below the line), not clipped by the editor frame | |
| BL-HOVER-02 | P1 | Hover near right edge | Hover on a field near the right margin | Tooltip flows outside the editor bounds via `fixedOverflowWidgets` | |
| BL-HOVER-03 | P1 | Hover delay consistent | Hover and wait 300ms | Tooltip appears after delay, stays sticky | |
| BL-HOVER-04 | P2 | Hover content reflects schema | Hover on a known field | Shows HL7 field name / type / required metadata | |

## 31. Session Persistence (Notepad++-style)

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-SESSION-01 | P0 | Tabs restored after relaunch | Open 2 files + 1 pasted untitled tab, close app, reopen | All 3 tabs re-appear with same content, labels, active tab | |
| BL-SESSION-02 | P0 | Unsaved edits survive close | Type changes in Untitled, close app, reopen | Typed text is back, `isModified` flag still set | |
| BL-SESSION-03 | P0 | Active tab preserved | Switch to 2nd tab, close, reopen | 2nd tab is focused on startup | |
| BL-SESSION-04 | P0 | Tree/inspector rehydrate | After restore | Auto-parse runs on each restored tab so tree populates | |
| BL-SESSION-05 | P1 | Toggle off in Settings | Uncheck "Restore open tabs on startup", save, close, reopen | The welcome screen (no tabs); the saved tabs were deleted when the box was unticked (`session_tabs` is empty) | |
| BL-SESSION-06 | P1 | Debounced autosave | Type rapidly | Only one save IPC ~800ms after the last keystroke | |
| BL-SESSION-07 | P1 | Cursor position persists | Move cursor, close, reopen | Cursor returns to same line/column | |
| BL-SESSION-08 | P1 | File path association | Open a .hl7 file, close, reopen | Tab reopens with filePath intact; Ctrl+S saves back to same file | |
| BL-SESSION-09 | P2 | Large message session | Session with 10 MB file | Restore completes in <3s | |
| BL-SESSION-10 | P2 | Empty session fallback | Fresh DB | The welcome screen shows, with no tab | |
| BL-SESSION-11 | P0 | macOS: Cmd+Q goes through the close prompt | macOS only. Untick *Restore open tabs on startup*, paste a message, press Cmd+Q; then BridgeLab → Quit BridgeLab; then with restore on, type a character, press Cmd+Q within 0.5 s and relaunch | Cmd+Q and the menu item both ask about the unsaved tab (Cancel keeps the app open, Discard quits); with restore on the last character is back after relaunch; with the manual open, Quit closes it too | |
| BL-SESSION-12 | P1 | macOS: shortcuts shown with ⌘ | macOS only. Settings → Shortcuts; F1 → Keyboard Shortcuts | Bindings read ⌘O, ⌘⇧S…; the editor rows read Replace ⌘⌥F, Redo ⌘⇧Z, Go to Line ⌃G, and those keys do that in the editor | |

## 32. Plugin Packs (declarative)

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-PLUG-01 | P0 | Plugins dir auto-created | Open Settings → Plugins on fresh install | `validation/`, `fhir/` and `anonymization/` subdirs exist under `<config>/BridgeLab/plugins` | |
| BL-PLUG-02 | P0 | Open plugins folder button | Click "Open plugins folder" | OS file manager reveals the plugins directory | |
| BL-PLUG-03 | P0 | Drop validation pack, reload | Copy `examples/plugins/validation/sample-validation.json` into plugins dir, click Reload | Pack appears in list with rule_count=4, kind=validation | |
| BL-PLUG-04 | P0 | Custom rule fires on F6 | Load a PID without PID-3 populated, F6 | Validation panel includes `SAMPLE-PID-001` warning | |
| BL-PLUG-05 | P0 | Regex rule with component | Load a PID with lowercase family name, F6 | `SAMPLE-PID-002` info issue appears | |
| BL-PLUG-06 | P0 | one_of rule | Load a PV1 with `Q` as patient class, F6 | `SAMPLE-PV1-001` error appears | |
| BL-PLUG-07 | P1 | max_length rule | Load a PID with SSN 20 chars long, F6 | `SAMPLE-PID-003` warning appears | |
| BL-PLUG-08 | P0 | PHI plugin extends anonymizer | Drop `sample-eu-phi.json`, reload, load message with PID-25 set, run Anonymize | PID-25 is redacted; no double-mask on built-in fields | |
| BL-PLUG-09 | P1 | Toggle plugin off | Uncheck enable for `sample-validation`, F6 | Plugin rule ids no longer appear in the report | |
| BL-PLUG-10 | P1 | Toggle persists | Close app, reopen, open Settings → Plugins | Plugin remains disabled | |
| BL-PLUG-11 | P1 | Bad JSON surfaces error | Drop a malformed .json, Reload | Plugin shows with red error text, toggle disabled, registry unaffected | |
| BL-PLUG-12 | P1 | Severities mapped | All three severities in one file | Report counts (error/warning/info) increment correctly | |
| BL-PLUG-13 | P2 | Reload during session | Edit pack on disk, click Reload | Live rules update without restart | |
| BL-PLUG-14 | P2 | Plugins folder path shown | Check Settings → Plugins | Absolute path printed next to the toolbar | |

## 33. Packaging & Installer

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-PKG-01 | P1 | Windows NSIS installer launches | Run `BridgeLab_<ver>_x64-setup.exe` | Language selector, welcome screen, license page shown | |
| BL-PKG-02 | P1 | NSIS license page shows MIT text | Progress through installer | LICENSE file contents rendered | |
| BL-PKG-03 | P1 | NSIS install mode: current user | Default flow | App installed under `%LOCALAPPDATA%\Programs\BridgeLab` | |
| BL-PKG-04 | P1 | NSIS language selector offers 5 langs | Language combo | English, Italian, French, Spanish, German | |
| BL-PKG-05 | P1 | macOS DMG layout | Mount `.dmg` | Window shows the app icon and the Applications shortcut side by side (no background image) | |
| BL-PKG-06 | P1 | Linux .deb lists correct deps | `dpkg -I *.deb` | `Depends:` includes libwebkit2gtk-4.1-0, libgtk-3-0 | |
| BL-PKG-07 | P1 | Linux .deb section utils | `dpkg -I *.deb` | `Section: utils`, `Priority: optional` | |
| BL-PKG-08 | P1 | AppImage runs without GStreamer bundled | Run the AppImage on a distro without GStreamer, offline | App starts, parses and validates; the file is about 78 MB | |
| BL-PKG-09 | P2 | File association `.hl7` | Install, double-click .hl7 | Opens in BridgeLab | |
| BL-PKG-10 | P2 | About dialog version matches installer | Launch installed build | About shows 0.1.0 (or current) | |
| BL-PKG-11 | P1 | Windows uninstall with "Delete the application data" | Install, activate a license, use the app (history, test case, a plugin), then uninstall ticking *Delete the application data*; then uninstall once more without the box on a second install; then run the setup over an existing install (update) | Ticked: `%APPDATA%\BridgeLab` keeps only `license.json` and `trial.json`, everything else gone; `%APPDATA%\BridgeLab\.bl-state.json` and `%LOCALAPPDATA%\BridgeLab\.bl-state.json` still there; reinstall is licensed and the setup asks the update question again. Unticked or updating: nothing under `%APPDATA%\BridgeLab` is touched | |
| BL-PKG-12 | P1 | Windows: deleting one folder does not restart the trial | Fresh trial, use it a day (or set the clock), quit; delete `%LOCALAPPDATA%\BridgeLab`, start; then quit, delete `%APPDATA%\BridgeLab\.bl-state.json` only, start. Also upgrade a 1.8.1 install mid-trial | The trial keeps its start date each time (days left unchanged); after the upgrade the days left are the same as before | |
| BL-PKG-13 | P1 | Windows: hardware ID without reg.exe | Note Settings → License → Hardware ID; enable the policy *Prevent access to registry editing tools* (silent mode blocked), `gpupdate /force`, restart the app | The Hardware ID is unchanged and an activated licence stays valid | |

## 34. Regression / Bug Verification

Tests for bugs fixed in previous releases, run to prevent regressions.

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-REG-01 | P0 | Monaco no onDestroy crash | Open files, switch tabs many times | No TypeError in console | |
| BL-REG-02 | P0 | Paste works on first tab | Fresh start, paste in empty editor | Text appears and parses | |
| BL-REG-03 | P0 | Expand doesn't trigger on click | Click near `{...}` | Expansion only via right-click | |
| BL-REG-04 | P0 | Multi-truncated field selection | Segment with 2 truncated, expand near 2nd | Correct field expanded | |
| BL-REG-05 | P0 | i18n reactive | Change language | All UI updates immediately | |
| BL-REG-06 | P0 | Settings modal visible | Open Settings | Modal appears centered, not cut off | |
| BL-REG-07 | P0 | HTTP Send button not cut | Open HTTP panel, scroll down | Send button visible above status bar | |
| BL-REG-08 | P0 | Typing does not reset cursor | Type additional characters mid-line in a parseable message | Cursor stays where typed, characters persist after the 500ms auto-parse fires | |
| BL-REG-09 | P0 | Auto-parse preserves content | Paste, wait >500ms, continue typing | `tab.content` not overwritten by parser's truncated_text | |
| BL-REG-10 | P1 | Show in Tree passes field position | Right-click inside a field (not on segment name) | Tree highlights the exact field, not just the segment | |

---

## 35. Online Activation & Telemetry (1.3.0)

For network cases, point the app at a controllable endpoint with
`BRIDGELAB_LICENSE_SERVER=http://127.0.0.1:<port>/api/v1` (a tiny local mock
server, or an unroutable port to simulate "network down") and observe
requests server-side or with a packet capture.

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-OA-01 | P0 | Online activation happy path | Fresh install, paste valid `BL-PRO-…` code, Activate | License active (Pro features); `license.json` contains `activation_code` and `activated_at` | |
| BL-OA-02 | P0 | Wrong code | Paste well-formed but unknown code | Server message shown under the input; app state unchanged | |
| BL-OA-03 | P1 | Revoked code | Activate with revoked code | Server's revocation message shown verbatim | |
| BL-OA-04 | P0 | No seats left | Activate same code on a 3rd machine | Clear NO_SEATS message; after Deactivate on machine 1, machine 3 activates | |
| BL-OA-05 | P0 | Network down | Disable network, paste valid code | Error mentions checking connection / offline keys; app stays usable (trial/free) | |
| BL-OA-06 | P0 | Legacy Base64 key still works | Paste a signed offline key | Activates via the offline path, no server call | |
| BL-OA-07 | P0 | 1.2.0 license.json untouched | Start with a pre-1.3 `license.json` | Loads and validates unchanged; no new fields added on re-save | |
| BL-OA-08 | P0 | Deactivate with network down | Deactivate an online-activated license offline | Local license removed regardless; no error blocks the flow | |
| BL-OA-09 | P1 | Debug simple key unaffected | (debug build) `BL-PRO-ABCD1234EFGH` | Falls through to the simple-key path, not the online path | |
| BL-OA-10 | P1 | Weekly re-check | Online license expiring in > 14 days, last check 8 days ago; start the app with the server reachable | One activation request; license unchanged or updated; no prompt | |
| BL-OA-11 | P0 | Revoked code at re-check | Revoke the code server-side (or refund), make the last check older than 7 days, start the app | License set aside as `license.revoked.json`, notice banner shows the server message, status is Community without a restart | |
| BL-OA-12 | P0 | Server unreachable at re-check | Same as above with the network off (or server HTTP 500 / proxy page) | License and tier unchanged, no error shown | |
| BL-TEL-01 | P0 | Telemetry off = zero requests | Default install, use the app, watch the endpoint | No telemetry request ever sent | |
| BL-TEL-02 | P0 | Telemetry on = one POST per 24 h | Enable in Settings → Privacy, restart twice same day | Exactly one POST; `telemetry_last_sent` updated | |
| BL-TEL-03 | P1 | Send now | Enable, press "Send now" | POST fires; inline confirmation text | |
| BL-TEL-04 | P0 | Preview matches payload | Open "Show what is sent", compare with captured POST body | Identical JSON (timestamps aside) | |
| BL-TEL-05 | P0 | No PII in payload | Inspect captured payload | No hostname, username, file names, message content; installation_id is a random UUID | |
| BL-TEL-06 | P1 | Revoked notice | Mock telemetry response with `revoked: true` | Dismissible banner appears; local license NOT deleted by telemetry; the next start re-checks the code with the license server (BL-OA-11); dismiss clears it | |
| BL-TEL-07 | P0 | Machine policy forces telemetry off | Enable telemetry. Set `BRIDGELAB_DISABLE_TELEMETRY=1`, start and wait past the 10-min loop; unset it, put `{"disable_telemetry": true}` in the platform `policy.json`, start | No telemetry request in either case, also with *Send now* unavailable; the Settings box is unticked, disabled and the hint names the variable or the file; `disable_update_check` alone leaves telemetry untouched, and vice versa | |

## Test Matrix by Platform

Run full suite on each:

| Platform | Version | Status | Last Tested | Notes |
|----------|---------|--------|-------------|-------|
| Windows 11 | - | | | |
| Windows 10 | 22H2 | | | Oldest Windows the site names |
| macOS Apple Silicon | 14+ | | | |
| macOS Intel | 13+ | | | |
| macOS 11 or 12 | Safari 16.4+ | | | Oldest macOS the DMG installs on (Tailwind 4 needs Safari 16.4) |
| Ubuntu 22.04 | - | | | |
| Debian 12 | - | | | Named on the site and in the CLI README |
| RHEL / Rocky / Alma 9 | - | | | Named on the site and in the CLI README |
| Fedora 39+ | - | | | |

## 36. FHIR Validation Rules (builder)

Rules live in `<config>/BridgeLab/plugins/fhir/*.json`; the builder owns
`user-rules.json`. Reference pack: `examples/plugins/fhir/sample-fhir-rules.json`.

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-FRULE-01 | P0 | Editor opens | Tools → FHIR validation rules… | Dialog opens, lists existing rules and the pack path | |
| BL-FRULE-02 | P0 | Preset creates a usable rule | Click "Must have identifier", Save | Rule saved; pack file exists on disk | |
| BL-FRULE-03 | P0 | Invariant fires | Save `identifier.exists()` on Patient, open a Patient without one, F6 | Error finding with the rule's message | |
| BL-FRULE-04 | P0 | Invariant passes | Same rule, Patient **with** an identifier | No finding | |
| BL-FRULE-05 | P0 | Selector + check fires | `telecom.where(system='phone').value` + regex, open a Patient with an odd phone, F6 | Finding naming the offending value | |
| BL-FRULE-06 | P0 | Cardinality check | `name` + cardinality min 1, Patient with no name | Finding says "expected at least 1, found 0" | |
| BL-FRULE-07 | P0 | Bad FHIRPath rejected on save | Type `identifier.exists(` and Save | Save refused with a parse error; nothing written to disk | |
| BL-FRULE-08 | P0 | Test on open resource | Load a Patient, edit a rule, click Test | Pass/fail shown **and** the values the selector matched | |
| BL-FRULE-09 | P1 | Test says when a rule does not apply | Rule scoped to Observation, Patient open, Test | "Did not apply" — not a pass | |
| BL-FRULE-10 | P1 | Rule reaches Bundle entries | Rule scoped to Patient, open a Bundle with a non-conforming Patient, F6 | Finding path starts `entry[n].resource.` | |
| BL-FRULE-11 | P1 | Broken rule is reported | Hand-edit the pack to use `nosuchfn()`, Reload, F6 | Finding says the rule failed to evaluate (not silently skipped) | |
| BL-FRULE-12 | P1 | Severity respected | One rule per severity | Error/warning/info counts increment correctly | |
| BL-FRULE-13 | P1 | Hand-written pack loads | Copy the sample pack into `plugins/fhir/`, Reload | Appears in Settings → Plugins with kind `fhir` | |
| BL-FRULE-14 | P2 | Delete a rule | Select a rule, Delete, Save | Rule gone from the file after reload | |
| BL-FRULE-15 | P2 | Editor gated in Community | Community tier, open the editor and Save | Upgrade prompt; existing rules still run | |

## 37. FHIR Profile Packages & Conformance

Packages live in `<config>/BridgeLab/fhir-packages/`. Get
`hl7.fhir.r4.core` from packages.fhir.org, or run
`./scripts/fetch-fhir-core-package.sh`.

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-PROF-01 | P0 | Manager opens | Tools → FHIR profile packages… | Dialog opens and lists the built-in R4 core (not removable) and any installed packages | |
| BL-PROF-02 | P0 | Install core package | Install `hl7.fhir.r4.core` 4.0.1 `.tgz` | Row appears: name, version, FHIR 4.0.1, ~653 profiles | |
| BL-PROF-03 | P0 | Survives restart | Restart the app, reopen the manager | Package still listed; no re-read of the archive | |
| BL-PROF-04 | P0 | Conforming resource stays clean | Load a spec example Patient, F6 | No profile findings; report states profiles were applied | |
| BL-PROF-05 | P0 | Unknown element caught | Change `gender` to `genderr`, F6 | Error: not defined by Patient | |
| BL-PROF-06 | P0 | Wrong JSON type caught | `"active": "yes"`, F6 | Error: must be a boolean | |
| BL-PROF-07 | P0 | Cardinality caught | `"gender": ["male","female"]`, F6 | Error: allows 0..1 but found 2 | |
| BL-PROF-08 | P0 | Required element caught | Observation with no `status`/`code`, F6 | Error naming each required element | |
| BL-PROF-09 | P0 | Choice element caught | Write `value` instead of `valueQuantity` on an Observation, F6 | Error: not defined (the plain form is not legal) | |
| BL-PROF-10 | P1 | Two choice forms at once | Both `deceasedBoolean` and `deceasedDateTime`, F6 | Error: only one of its forms | |
| BL-PROF-11 | P1 | Declared profile applied | Install a package defining a profile, declare it in `meta.profile`, F6 | That profile's constraints are enforced on top of the base | |
| BL-PROF-12 | P1 | Missing profile reported | Declare a profile no installed package defines, F6 | Warning: declared but not installed | |
| BL-PROF-13 | P1 | Nested type checked | Break a field inside `name` (e.g. add `nosuchfield`), F6 | Finding path points at `Patient.name[0].nosuchfield` | |
| BL-PROF-14 | P1 | Remove a package | Click Remove on an installed IG | Row disappears; its profiles are no longer applied. The built-in core row has no Remove button | |
| BL-PROF-15 | P1 | Install gated in Community | Community tier, try to install | Upgrade prompt; already-installed packages keep validating | |
| BL-PROF-16 | P2 | Bad archive rejected | Install a `.tgz` that is not a FHIR package | Clear error; existing packages unaffected | |
| BL-PROF-17 | P2 | Whole-package regression | `BL_FHIR_PACKAGE=… BL_FHIR_EXAMPLES=… cargo test --test fhir_profiles` | Findings rate under the recorded threshold | |
| BL-PROF-18 | P0 | Built-in core listed and applied | Fresh install, Tools → FHIR profile packages… | `hl7.fhir.r4.core 4.0.1` listed as built-in, 653 profiles; F6 on a Patient with `genderr` reports the unknown element with no package installed | |
| BL-PROF-19 | P1 | Installed copy supersedes the built-in | Install `hl7.fhir.r4.core` 4.0.1 by hand | Two rows (built-in + installed), profile count unchanged; remove the installed copy and the built-in keeps validating | |

## 38. HL7 Version Catalogue

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-VER-01 | P0 | Ten versions offered | Tools → Export message schema as XSD… | Dropdown lists 2.1, 2.2, 2.3, 2.3.1, 2.4, 2.5, 2.5.1, 2.6, 2.7, 2.7.1 | |
| BL-VER-02 | P0 | v2.7.1 labelled as an alias | Look at the dropdown entry | Shown as `HL7 v2.7.1 (= v2.7)` | |
| BL-VER-03 | P0 | Export works for a new version | Pick v2.5.1, choose a message, preview | XSD generated without error | |
| BL-VER-04 | P1 | Oldest versions are small but real | Pick v2.1 | Message list is short (~39) and exports cleanly | |
| BL-VER-05 | P1 | Tree follows MSH-12 | Open messages declaring 2.3 and 2.6 with schema-aware tree on | Placeholder rows differ per version | |
| BL-VER-06 | P2 | Counts agree across the UI | Welcome card, manual, landing FAQ, README | All say ten versions / 2,320 **selectable** message structures; the README also gives 1,965 distinct definitions | |

## 39. Release Checklist

Run before every tag; none of these is a feature test, all three have bitten a release.

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-REL-01 | P0 | Local checks equal CI | In `src-tauri/`: `cargo check --all-targets`, `cargo test --all`; root: `pnpm check`, `pnpm build` | All clean — the integration tests under `src-tauri/tests/` compile too (a lib-only run let 1.7.0 ship with a red test job on the mirror) | |
| BL-REL-02 | P0 | Comparison table verified | Open every vendor site linked in the footnote of the landing's *Why a new HL7 editor* table; re-check FHIR, platforms, XSD export, list price, latest release per row | Every cell matches the vendor's site today; "Not advertised" / "Quote on request" where the site is silent; footnote date updated; no judgement words ("slow", "struggles", "minimal") anywhere in the table — Directive 2006/114/EC | |
| BL-REL-03 | P1 | Dependency advisories | `cargo audit` in `src-tauri/`, `tools/bridgelab-cli/`, `tools/hl7-schema-importer/`; `pnpm audit` at the root | No fixable advisory left; unfixable ones (transitive through Tauri) named in the CHANGELOG | |
| BL-REL-04 | P1 | Release assets | After the release workflow on the mirror: count the assets | 12 assets, no `.app.tar.gz`, one `en-US` MSI, CLI binary per platform | |
| BL-REL-05 | P2 | Release note footer | Open the published release on the mirror | The note ends with a horizontal rule and *BridgeLab is built by TECHEMV SRL · info@techemv.it*, the company name linked to techemvee.eu/en/ | |

## Test Execution Log

Log of test runs; append new sessions at bottom.

### 2026-04-14 (v0.1.0 baseline)
- **Tester**: TBD
- **Platform**: TBD
- **Status**: Initial test plan created, execution pending

---

## Notes & Known Issues

Add observations during testing here:

- (none yet)

## Automated Tests

Separately from this manual plan, the following automated tests run on every commit (see `.github/workflows/feature-tests.yml`):

- **CLI feature tests** (BL-CLI-01..12, 19..27) - validate, JSON, JUnit, info, anonymize, to-json, batch, standard input, fhirpath, xsd, MLLP send (against a one-shot ACK server, `tests/tools/mllp_ack_server.py`), test packs
- **Rust core tests** - `cd src-tauri && cargo test --all` (unit + integration, 335 tests)
- **Schema lookup** (BL-INSP-05 partial) - `get_segment_info` / `get_field_info` for MSH/PID/PV1
- **Parser fixtures** (BL-PARSER-01/02/03, BL-PERF-03) - smoke over `tests/fixtures/hl7/` via CLI
- **MLLP roundtrip** (BL-MLLP-04/05/09/10) - in-process listener with auto-ACK verified both from the
  client side (ACK received) and the server side (decoded message), plus connection-refused path
- **HTTP roundtrip** (BL-HTTP-01/02/04/06) - in-process HTTP/1.1 server exercises GET and POST
  with custom headers, body echo and connection-refused error reporting
- **Frontend check** - `pnpm check` (svelte-check) runs with 0 errors threshold
- **Frontend build** - `pnpm build` succeeds
- **Frontend unit tests** - `pnpm test` (vitest: stores, MSH-12 version detection)

Before every release, `pnpm e2e` runs the **acceptance suite** against the
installed desktop application (see `e2e/README.md`). It drives the real
binary through tauri-driver and covers, automatically, a large part of what
this document describes by hand: the package itself (BL-PKG), the shell,
HL7 parsing and validation, the version catalogue (BL-VER), the FHIRPath
engine (BL-FP), the FHIR rules builder (BL-FRULE) and profile validation
(BL-PROF). A case covered there does not need manual repetition; the manual
plan stays authoritative for everything the suite cannot reach — native file
dialogs, installers on Windows and macOS, MLLP and HTTP against real
endpoints, and anything requiring human judgement about appearance.

Two conformance suites run against third-party corpora that are **not
vendored**; both skip cleanly when the data is absent, so a plain
`cargo test` stays offline:

- **FHIRPath** (BL-FP-24) - `./scripts/fetch-fhirpath-suite.sh`, then
  `BL_FHIRPATH_SUITE=.fhirpath-suite cargo test --test fhirpath_suite`.
  Runs the official HL7 FHIRPath suite and fails if the pass rate drops
  below the baseline recorded in the harness.
- **FHIR profiles** (BL-PROF-17) - `./scripts/fetch-fhir-core-package.sh`,
  then `BL_FHIR_PACKAGE=… BL_FHIR_EXAMPLES=… cargo test --test fhir_profiles`.
  Validates every resource in the HL7 R4 core package; a findings rate
  above the threshold means a validator bug, since HL7's own resources
  conform to HL7's own definitions.

### Memory / performance tuning

**BridgeLab does not require manual memory configuration.** The Rust backend
uses zero-copy parsing + field truncation so its peak RAM stays under 300 MB
even on 10 MB messages (BL-PERF-09); the web view that draws the window takes
more (about 900 MB on Linux for a 10 MB message). Monaco's virtual scrolling keeps the
editor light. The only tunable knob lives in _Settings → Parser → Truncation
threshold_ and only affects how much text is sent across IPC - not total
memory consumption.

## How to Add Tests

When adding a new feature:

1. Add tests to the appropriate section (or create new `## N. Feature` section)
2. Use next available ID in format `BL-{AREA}-{NN}`
3. Include: Priority, clear steps, expected result
4. Commit this file along with the feature

Keep this document in sync with the product.

