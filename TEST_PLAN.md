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
| BL-SHORTCUT-07 | P1 | F5 parses current message | Press F5 | Tree updates with parsed message | |
| BL-SHORTCUT-08 | P1 | F6 runs validation | Press F6 | Validation panel appears with results | |
| BL-SHORTCUT-09 | P1 | Ctrl+J toggles validation panel | Press Ctrl+J | Panel shows/hides | |
| BL-SHORTCUT-10 | P1 | Ctrl+K toggles communication panel | Press Ctrl+K | Panel shows/hides | |
| BL-SHORTCUT-11 | P1 | Ctrl+P toggles FHIRPath panel | Press Ctrl+P | Panel shows/hides | |
| BL-SHORTCUT-12 | P1 | Ctrl+, opens settings | Press Ctrl+, | Settings modal opens | |

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
| BL-PERF-03 | P0 | Editor shows truncated preview | Open 5MB file | Base64 fields show `{...N bytes}` marker | |
| BL-PERF-04 | P0 | Editor remains responsive | Type/scroll in large message | No lag, 60fps feel | |
| BL-PERF-05 | P0 | Click `{...}` expands inline | Right-click > Expand Truncated Field | Field expands in editor | |
| BL-PERF-06 | P0 | Expand All works | Right-click > Expand All Truncated Fields | All fields expanded | |
| BL-PERF-07 | P0 | Collapse All re-truncates | Right-click > Collapse All Expanded Fields | Text returns to truncated | |
| BL-PERF-08 | P1 | Multiple truncated fields per segment | Load message with 2+ truncated in PID | Expand via context menu picks correct field | |
| BL-PERF-09 | P1 | Memory stays below 300MB | Load 10MB file, monitor RAM | Process memory <300MB | |
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
| BL-EDITOR-14 | P2 | Find/replace works | Ctrl+F | Monaco find widget opens | |

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

## 7. Tree View

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-TREE-01 | P0 | Tree shows segments | Parse message | Segments listed with position | |
| BL-TREE-02 | P0 | Expand segment shows fields | Click arrow on PID | Fields PID-1 to PID-30 shown | |
| BL-TREE-03 | P0 | Expand field shows components | Click arrow on field with components | Components 1-N shown | |
| BL-TREE-04 | P0 | Lazy loading works | Expand large segment | Children fetched on demand, no freeze | |
| BL-TREE-05 | P1 | Field names shown | Hover/inspect field | HL7 standard name displayed (e.g. "Patient Name") | |
| BL-TREE-06 | P1 | Truncated fields have `{...}` button | Load large field | Red `{...}` button visible | |
| BL-TREE-07 | P1 | Click `{...}` opens expansion | Click the button | Modal shows full field content | |
| BL-TREE-08 | P1 | Modal "Copy to Clipboard" works | In expansion modal, click Copy | Full content in clipboard | |
| BL-TREE-09 | P1 | Resize splitter works | Drag splitter between tree and editor | Panels resize, position saved | |
| BL-TREE-10 | P1 | Tree panel hideable | Ctrl+B | Panel hides, editor takes full width | |
| BL-TREE-11 | P2 | Badge shows field count | Check unexpanded segments | Count badge shown | |
| BL-TREE-12 | P2 | Click node navigates editor | Click segment in tree | Editor scrolls to corresponding line | |

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
| BL-TAB-08 | P1 | Opening same file twice | Open file already open | Focuses existing tab, doesn't duplicate | |

## 9. File Operations

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-FILE-01 | P0 | Open .hl7 file | File > Open File, select .hl7 | File loads in new tab | |
| BL-FILE-02 | P0 | Open .txt file | Open .txt with HL7 content | Parses correctly | |
| BL-FILE-03 | P0 | Open .json FHIR | Open FHIR resource .json | Detected as FHIR, tree shows resource | |
| BL-FILE-04 | P1 | Save overwrites file | Edit, Ctrl+S | Original file updated | |
| BL-FILE-05 | P1 | Save As to new file | Ctrl+Shift+S | New file created at chosen path | |
| BL-FILE-06 | P1 | Recent files list | Open file, reopen app | File in File > Recent Files | |
| BL-FILE-07 | P1 | Recent file click | Click entry in Recent | File opens | |
| BL-FILE-08 | P1 | Clear recent | File > Clear Recent | List emptied | |
| BL-FILE-09 | P2 | Drag & drop file | Drag .hl7 into window | File opens | |

## 10. Validation

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-VALID-01 | P0 | Valid message: no errors | Load `adt_a01_small.hl7`, F6 | 0 errors, maybe warnings | |
| BL-VALID-02 | P0 | Detect missing MSH | Load invalid file, F6 | Error STRUCT-001/002 reported | |
| BL-VALID-03 | P0 | Detect missing MSH-9 | Message without MSH-9 | Error MSH-001 or MSH-002 | |
| BL-VALID-04 | P1 | Detect missing MSH-10 | Message without MSH-10 | Warning MSH-003 | |
| BL-VALID-05 | P1 | Detect missing required field | PID without PID-3 or PID-5 | Error REQ-PID-3/5 | |
| BL-VALID-06 | P1 | Field length validation | Field exceeds max_length | Warning LEN-... | |
| BL-VALID-07 | P1 | Data type validation (SI) | Non-numeric in SI field | Warning TYPE-SI-... | |
| BL-VALID-08 | P1 | Validation panel shows results | F6 | Bottom panel shows issues list | |
| BL-VALID-09 | P1 | Filter by severity | Click error/warning/info badges | List filters to chosen severity | |
| BL-VALID-10 | P1 | Sort by severity/segment | Use sort dropdown | Issues reorder | |
| BL-VALID-11 | P2 | Click issue navigates to field | Click issue | Editor jumps to relevant location (when implemented) | |
| BL-VALID-12 | P1 | Close validation panel | Click X or Ctrl+J | Panel hides | |

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
| BL-FHIR-08 | P1 | Unchecked conformance is stated | Same, with no profile package installed | Info finding says conformance was **not** checked and points at Tools → FHIR profile packages | |

## 12. FHIR Bundle Visualizer

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-BUNDLE-01 | P0 | Open visualizer | Tools > FHIR Bundle Visualizer | Modal opens with 3-pane layout | |
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
| BL-MLLP-10 | P1 | Auto-ACK on receive | Send message to listener with auto-ack | Sender receives ACK | |
| BL-MLLP-11 | P1 | Received message opens in new tab | Listener receives | New tab with received content | |
| BL-MLLP-12 | P1 | Advanced settings toggle | Click "Advanced MLLP Settings" | Panel expands with extra fields | |
| BL-MLLP-13 | P2 | Custom framing chars | Change start/end chars | Used in MLLP frame | |

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

## 20. Export

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-EXP-01 | P1 | Export JSON | Tools > Export JSON | File downloads | |
| BL-EXP-02 | P1 | Export CSV | Tools > Export CSV | File downloads | |
| BL-EXP-03 | P1 | JSON structure correct | Open exported JSON | Contains message_type, version, segments | |
| BL-EXP-04 | P1 | CSV structure correct | Open CSV | Header: Segment,Position,Field,Value | |

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
| BL-I18N-07 | P1 | Status bar translated | Check segments/truncated labels | Translated | |

## 23. Settings

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-SET-01 | P0 | Open settings | Ctrl+, or Edit > Settings | Modal opens with 4 sections | |
| BL-SET-02 | P0 | Change font size | Set to 16, save | Editor font increases | |
| BL-SET-03 | P1 | Change font family | Pick different font | Editor font changes | |
| BL-SET-04 | P1 | Theme switcher inside settings | Display section, click Light | Theme updates after save | |
| BL-SET-05 | P1 | Language switcher | Display section, pick lang | UI updates after save | |
| BL-SET-06 | P1 | Truncation threshold | Change to 50 in Parser | New messages truncated at 50 chars | |
| BL-SET-07 | P1 | Settings persist | Close, reopen | Values retained | |
| BL-SET-08 | P2 | Cancel discards changes | Edit, click Cancel | No changes applied | |

## 24. Licensing

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-LIC-01 | P0 | Trial starts on first launch | Fresh install | 14 days trial active | |
| BL-LIC-02 | P0 | Trial banner shows days | Check top of window | Yellow banner with days remaining | |
| BL-LIC-03 | P1 | Banner urgent ≤3 days | Simulate ≤3 days remaining | Red, non-dismissible banner | |
| BL-LIC-04 | P0 | Open activation dialog | Click Upgrade | Dialog opens | |
| BL-LIC-05 | P0 | Activate Free license | Enter `BL-FREE-ABCD1234EFGH` | Free activated | |
| BL-LIC-06 | P0 | Activate Pro license | Enter `BL-PRO-12345678ABCD` | Pro activated | |
| BL-LIC-07 | P0 | Activate Enterprise | `BL-ENT-ENTERPRISEKEY` | Enterprise activated | |
| BL-LIC-08 | P1 | Invalid key rejected | Enter "INVALID" | Error shown | |
| BL-LIC-09 | P1 | Short key rejected | `BL-PRO-ab` | Error (too short) | |
| BL-LIC-10 | P1 | Hardware ID shown | Open activation | BL-XXXXXXXXXXXXXXXX visible | |
| BL-LIC-11 | P1 | Feature list correct | After Pro activation | Shows fhir, mllp, http, anonymize, export | |
| BL-LIC-12 | P1 | Deactivate works | Click Deactivate | Returns to trial | |
| BL-LIC-13 | P0 | License persists | Activate, close, reopen | Still active | |
| BL-LIC-14 | P1 | Signed Ed25519 key | Use bridgelab-keygen | Key activates, signature verified | |

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

## 26. Updater

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-UPD-01 | P2 | Check for updates | Help > Check for Updates | Shows "latest version" or update available | |
| BL-UPD-02 | P2 | No update dialog | If no update | Alert "You are running the latest version" | |
| BL-UPD-03 | P2 | Update download | If update available | Downloads and prompts restart | |

## 27. Tree ↔ Editor Navigation

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-NAV-01 | P0 | Editor → Tree (segment) | Right-click on the segment name (e.g. "PID"), Show in Tree | Tree opens, `seg{N}` node selected, scrolled to view | |
| BL-NAV-02 | P0 | Editor → Tree (field) | Right-click inside PID-5 content, Show in Tree | Tree opens, segment expanded, `seg{N}.f5` field node selected | |
| BL-NAV-03 | P0 | Editor → Tree (MSH-1 separator) | Right-click on the first `\|` in MSH, Show in Tree | Tree selects MSH field at position 1 (Field Separator) | |
| BL-NAV-04 | P0 | Editor → Tree (MSH-2 encoding chars) | Right-click on `^~\&`, Show in Tree | Tree selects MSH-2 node | |
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

## 29. Schema-aware Tree

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-SCHEMA-01 | P1 | Toggle from View menu | View → Show Schema Fields | Setting toggles on/off | |
| BL-SCHEMA-02 | P1 | Expanding segment injects placeholders | Enable the flag, expand PID in a minimal message | All PID-1..PID-20 slots shown; missing positions rendered dim/italic | |
| BL-SCHEMA-03 | P1 | Placeholders dimmed | Inspect placeholder rows | Opacity ~0.5, italic, trailing ` ·` marker | |
| BL-SCHEMA-04 | P1 | Real fields unaffected | Compare populated vs missing fields in same segment | Real fields full opacity, placeholders dim | |
| BL-SCHEMA-05 | P1 | Inspector still works on placeholders | Click a placeholder | Inspector shows schema info (no current value section) | |
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
| BL-SESSION-05 | P1 | Toggle off in Settings | Uncheck "Restore open tabs on startup", save, close, reopen | Fresh single Untitled tab, previous session cleared on next save | |
| BL-SESSION-06 | P1 | Debounced autosave | Type rapidly | Only one save IPC ~800ms after the last keystroke | |
| BL-SESSION-07 | P1 | Cursor position persists | Move cursor, close, reopen | Cursor returns to same line/column | |
| BL-SESSION-08 | P1 | File path association | Open a .hl7 file, close, reopen | Tab reopens with filePath intact; Ctrl+S saves back to same file | |
| BL-SESSION-09 | P2 | Large message session | Session with 10 MB file | Restore completes in <3s | |
| BL-SESSION-10 | P2 | Empty session fallback | Fresh DB | One empty Untitled tab is created | |

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
| BL-PKG-05 | P1 | macOS DMG opens with background | Mount `.dmg` | Window shows app icon + Applications shortcut laid out | |
| BL-PKG-06 | P1 | Linux .deb lists correct deps | `dpkg -I *.deb` | `Depends:` includes libwebkit2gtk-4.1-0, libgtk-3-0 | |
| BL-PKG-07 | P1 | Linux .deb section utils | `dpkg -I *.deb` | `Section: utils`, `Priority: optional` | |
| BL-PKG-08 | P1 | AppImage bundles media framework | Run AppImage offline | GStreamer-dependent features work | |
| BL-PKG-09 | P2 | File association `.hl7` | Install, double-click .hl7 | Opens in BridgeLab | |
| BL-PKG-10 | P2 | About dialog version matches installer | Launch installed build | About shows 0.1.0 (or current) | |

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
| BL-TEL-01 | P0 | Telemetry off = zero requests | Default install, use the app, watch the endpoint | No telemetry request ever sent | |
| BL-TEL-02 | P0 | Telemetry on = one POST per 24 h | Enable in Settings → Privacy, restart twice same day | Exactly one POST; `telemetry_last_sent` updated | |
| BL-TEL-03 | P1 | Send now | Enable, press "Send now" | POST fires; inline confirmation text | |
| BL-TEL-04 | P0 | Preview matches payload | Open "Show what is sent", compare with captured POST body | Identical JSON (timestamps aside) | |
| BL-TEL-05 | P0 | No PII in payload | Inspect captured payload | No hostname, username, file names, message content; installation_id is a random UUID | |
| BL-TEL-06 | P1 | Revoked notice | Mock telemetry response with `revoked: true` | Dismissible banner appears; local license NOT deleted; dismiss clears it | |

## Test Matrix by Platform

Run full suite on each:

| Platform | Version | Status | Last Tested | Notes |
|----------|---------|--------|-------------|-------|
| Windows 11 | - | | | |
| macOS Apple Silicon | 14+ | | | |
| macOS Intel | 13+ | | | |
| Ubuntu 22.04 | - | | | |
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
| BL-PROF-01 | P0 | Manager opens | Tools → FHIR profile packages… | Dialog opens, empty state explains where to get a package | |
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
| BL-PROF-14 | P1 | Remove a package | Click Remove | Row disappears; F6 no longer reports profile findings | |
| BL-PROF-15 | P1 | Install gated in Community | Community tier, try to install | Upgrade prompt; already-installed packages keep validating | |
| BL-PROF-16 | P2 | Bad archive rejected | Install a `.tgz` that is not a FHIR package | Clear error; existing packages unaffected | |
| BL-PROF-17 | P2 | Whole-package regression | `BL_FHIR_PACKAGE=… BL_FHIR_EXAMPLES=… cargo test --test fhir_profiles` | Findings rate under the recorded threshold | |

## 38. HL7 Version Catalogue

| ID | Priority | Description | Steps | Expected Result | Status |
|----|----------|-------------|-------|-----------------|--------|
| BL-VER-01 | P0 | Ten versions offered | Tools → Export message schema as XSD… | Dropdown lists 2.1, 2.2, 2.3, 2.3.1, 2.4, 2.5, 2.5.1, 2.6, 2.7, 2.7.1 | |
| BL-VER-02 | P0 | v2.7.1 labelled as an alias | Look at the dropdown entry | Shown as `HL7 v2.7.1 (= v2.7)` | |
| BL-VER-03 | P0 | Export works for a new version | Pick v2.5.1, choose a message, preview | XSD generated without error | |
| BL-VER-04 | P1 | Oldest versions are small but real | Pick v2.1 | Message list is short (~39) and exports cleanly | |
| BL-VER-05 | P1 | Tree follows MSH-12 | Open messages declaring 2.3 and 2.6 with schema-aware tree on | Placeholder rows differ per version | |
| BL-VER-06 | P2 | Counts agree across the UI | Welcome card, manual, landing FAQ | All say ten versions / 2,320 structures | |

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

- **CLI feature tests** (BL-CLI-01..12) - validate, JSON, JUnit, info, anonymize, to-json, batch
- **Rust core tests** - `cd src-tauri && cargo test --all` (unit + integration, 319 tests)
- **Schema lookup** (BL-INSP-05 partial) - `get_segment_info` / `get_field_info` for MSH/PID/PV1
- **Parser fixtures** (BL-PARSER-01/02/03, BL-PERF-03) - smoke over `tests/fixtures/hl7/` via CLI
- **MLLP roundtrip** (BL-MLLP-04/05/09/10) - in-process listener with auto-ACK verified both from the
  client side (ACK received) and the server side (decoded message), plus connection-refused path
- **HTTP roundtrip** (BL-HTTP-01/02/04/06) - in-process HTTP/1.1 server exercises GET and POST
  with custom headers, body echo and connection-refused error reporting
- **Keygen roundtrip** (BL-LIC-14) - generate keypair, sign a license, verify signature
- **Frontend check** - `pnpm check` (svelte-check) runs with 0 errors threshold
- **Frontend build** - `pnpm build` succeeds
- **Frontend unit tests** - `pnpm test` (vitest: stores, MSH-12 version detection)

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
uses zero-copy parsing + field truncation so peak RAM stays well under 300 MB
even on 10 MB messages (BL-PERF-09). Monaco's virtual scrolling keeps the
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

