# Changelog

All notable user-facing changes to BridgeLab. Dates are UTC.

## [Unreleased]

## [2.0.0] — 2026-09-30

A summary of the fixes, grouped by area, is in [docs/releases/2.0.0.md](docs/releases/2.0.0.md).

### Added
- **More editor settings.** Settings → Editor now also sets where word
  suggestions come from (this message, every open message, or none), the
  highlighting of other occurrences of the word under the cursor,
  clickable links and sticky scroll. The manual describes the editor
  settings.
- **Share test cases as a pack.** The Test Case Library has *Export…*
  and *Import…*: the cases in view (all, or those matching the search)
  go into one `.bltests.json` file to send to a colleague or keep in
  Git, and come back with a preview that says, before anything is
  written, which cases are new, already there (skipped) or different
  from yours (keep yours, replace, or keep both). Cases keep their id,
  so re-importing a pack only brings in what changed. Before an export
  BridgeLab lists the HL7 v2 cases whose PHI fields hold data and the
  FHIR cases it cannot check field by field; with Pro the HL7 v2
  messages can be masked in the exported file only. Every edition; an
  import respects the Community limit of 10 test cases, all or
  nothing. The format is documented in
  [`docs/TEST_CASE_PACKS.md`](docs/TEST_CASE_PACKS.md).
- **CLI: four new commands and standard input.** `bridgelab-cli test`
  runs test case packs and reports JUnit for CI; `fhirpath` evaluates an
  expression on a FHIR resource; `send` delivers a message over MLLP and
  exits non-zero unless the ACK is AA/CA; `xsd` exports the schema of the
  Community set of messages. Every command that reads one message accepts
  `-` for standard input.

- **Sample messages.** *File → Sample Messages* (and a button on the
  welcome screen) opens thirteen complete, realistic HL7 v2 messages —
  ADT admission, registration, update, discharge and merge; ORU results
  with a twelve-result blood count and a metabolic panel; ORM orders;
  SIU, MDM, DFT, VXU and an ACK — in v2.3, v2.5 and v2.5.1, with a
  version filter and a preview, each opening in a new tab. Unlike the
  templates they are finished messages; every one validates clean, and
  patients and data are fictional.
- **Segment grid.** *View → Segment Grid* (Ctrl+Shift+G), or *Show all
  OBX in a table* on a segment in the tree, opens a bottom panel with
  every occurrence of one segment type as a table — one row per
  occurrence, one column per populated field, named after the message's
  HL7 version, coded values with their meaning. A row filter narrows it,
  and clicking a cell selects that field in the editor. Every edition.

### Fixed
- **Sample messages in your language.** The names, descriptions and
  categories of the sample library stayed in English in the Italian,
  French, Spanish and German interface; they are now translated (a
  sample the translations do not know keeps its English text), and the
  search matches both.
- **Anonymize and Validate no longer crash on names such as Lindström.**
  A value whose 8th byte fell inside a multi-byte character — 李小龍,
  田中太郎, 김민준, Lindström, Østergård, a city of Västerås — stopped the
  timestamp check half way through a character: the app closed and the
  CLI exited 101 (a JUnit report was left empty, a batch stopped). Such a
  value is now simply not a date.
- **BIG-5, GB 18030-2000 and KS X 1001 are decoded.** These table 0211
  names were read as Windows-1252 without a word; in Big5 a byte of a
  character can be `|`, so fields shifted and `anonymize` left the birth
  date in clear. They now decode as Big5, GB18030 and EUC-KR in the app
  and the CLI. A charset BridgeLab cannot decode (CNS 11643-1992, the
  ISO 2022 Japanese sets, UTF-16/32 without a byte-order mark) is
  reported: the app warns when opening the file, batch validation and
  batch anonymisation refuse it, and the CLI exits 1 with the reason. A
  message that declares UTF-8 but is not UTF-8 is reported too, and is
  written back as UTF-8. Latin-1 and ASCII aliases such as `cp819`,
  `iso-ir-100` and `ansi_x3.4-1968` are encoded as Latin-1 and 7-bit
  ASCII, not Windows-1252.
- **`bridgelab-cli send`**: a message with characters the wire charset
  cannot hold is refused before anything is sent (exit 1), as in the
  app; it went out with `?` in their place and exited 0. A UTF-16 file is
  sent in the charset its MSH-18 declares, else UTF-8 (it went out as a
  UTF-16 frame and the AA was not recognised), in the app too.
  `--timeout` must be 1 to 86400 (0 timed out at once) and its help names
  the send phase; a reply with no MSA is reported as not an ACK.
- **`bridgelab-cli anonymize` on a terminal** prints UTF-8, one segment
  per line: a Latin-1 file failed on a Windows console and showed U+FFFD
  on Linux. Pipes and `--output` keep the source charset.
- **CLI reports.** Control characters from a message or an ACK are shown
  as `\xNN` in text output instead of reaching the terminal; JUnit keeps
  tabs and line breaks in attributes; HL7 v2 reports carry the
  `PLUGIN-CAP` note the app shows; `test` and `batch` report a FHIR
  resource that was not fully checked as *not checked* (JUnit
  `skipped`), not as a pass; a folder argument points to `batch`, XML
  that is not FHIR says so, a file name that is not UTF-8 is matched and
  an unreadable folder in a pattern is reported; long paths in `info`
  keep their file name.
- **Plugin packs.** A pack with no rules under its key (`"rules"` for
  `"validation_rules"`) loaded as an empty pack in silence; it now shows
  a warning naming the unknown key. A pattern that does not compile keeps
  its reason ("unclosed character class"), not just "regex parse error".
  The CLI no longer creates folders or records packs in the app's plugin
  state.
- **Masked test case packs** can fail plugin rules that check a PHI
  field's format; the export option, its done message and
  TEST_CASE_PACKS.md say so.
- **Interface text.** Settings and the status bar speak of folded fields
  ("Parser & Folding"), the View menu labels the panel toggle
  "Validation", and tooltips, placeholders and errors left in English
  are translated.
- **A refunded or revoked license now ends as the refund policy says.**
  A license activated online was checked with the server only in its last
  14 days, and a revoked code kept Pro until the license expired (never,
  for a license without an expiry). The app now re-checks an online-
  activated license at start-up once a week (once a day near expiry, as
  before); when the server answers that the code was revoked, the license
  is set aside, the server's message is shown in the notice banner and
  the app runs as Community. An unreachable server, or any other answer,
  changes nothing. Offline keys are not affected.
- **Lighter and faster with many and large messages.** Every parse kept a
  full copy of the message in memory for the rest of the session (each
  auto-parse while typing, too), and every tree, grid or validation lookup
  copied the whole message; parses nobody shows are now released and the
  rest are shared. The session no longer rewrites every tab's text on each
  autosave (a large unedited file is read back from disk at restore), and a
  restored session loads only the active tab at start; the others load when
  first shown. With ten messages open, three of them 2-5 MB, the app uses
  about half the memory it did, and a restored session opens in under 3 s.
- **Each tab keeps its own undo and redo history.** Switching tabs replaced
  the editor text and wiped the history; each tab now keeps its own editor
  model, with its caret, scroll position and history, and switching back
  costs no re-highlighting.
- **FHIRPath types of choice elements.** `ofType()`, `is` and `as` on a
  choice element go by the type in its name, so
  `Observation.effective.ofType(dateTime)` finds an `effectiveDateTime`
  that holds only a day (it was taken for a date and dropped), and
  `value.ofType(string)` no longer matches a `valueDateTime`. Rules written
  this way no longer report valid resources, and SearchParameter-style
  expressions such as `Condition.onset.as(dateTime)` work.
- **FHIRPath `resolve()`.** A reference resolves only to a resource of the
  type it names (`Practitioner/123` no longer finds `Patient/123`), an
  absolute URL only to the entry with that exact fullUrl, and
  `Patient/1/_history/2` to Patient/1. `#id` finds the resource contained
  in the Bundle entry the reference sits in. A custom FHIR rule on a Bundle
  entry resolves references to the other entries, as the FHIRPath panel
  does on the whole Bundle, instead of reporting them as missing. Resolving
  every reference of a 3000-entry Bundle takes a moment instead of two
  minutes, and the FHIRPath panel no longer freezes the window while it
  evaluates.
- **FHIR XML with any spelling of the namespace.** A resource whose root
  declares `xmlns='http://hl7.org/fhir'` (single quotes) or
  `xmlns = "…"` is recognised as FHIR XML whatever its type, instead of
  being read as a broken HL7 message. FHIR JSON and XML saved as UTF-16
  (Notepad's "Unicode", PowerShell 5.1 redirects) open as FHIR, in the app
  and the CLI.
- **Stricter FHIR primitives.** Dates that do not exist (2019-02-29,
  2020-02-30T10:00:00Z) are errors, a resource id is checked as an id (at
  most 64 letters, digits, `-` and `.`), and empty strings, arrays and
  objects, which FHIR JSON does not allow, are reported instead of passing.
- **Extensions on primitive values.** FHIRPath reads a primitive's
  extensions and id from its `_name` sibling, so
  `Patient.birthDate.extension(url)` finds a birth time in JSON and in
  XML. The validator checks those siblings too: an extension without a
  url, an unknown element or a value that is not an object is reported
  instead of passing.
- **FHIRPath panel and Bundle visualizer in your language.** Their labels,
  buttons, counts and messages follow the language setting (they were
  English only), and the visualizer shows real arrows instead of a literal
  `\u2192`. The manual describes the visualizer's two panes and the
  FHIRPath panel's *Recent* chips as they are.
- **Bundle visualizer references.** A `#id` reference to a resource the
  entry contains is no longer listed as dangling (only one to an id that is
  not contained is), and `Patient/1/_history/2` or a relative reference to
  an entry whose fullUrl ends in it links to that entry.
- **An R5 package no longer changes how R4 resources are checked.**
  Installing a package with R5 definitions of the core resources made them
  the base for every resource, so valid R4 resources got R5 errors. The
  base stays the newest definition of the built-in core's release (R4, or
  R4B when installed); the R5 ones answer only when asked for by version.
  The manual now says that validation follows R4, and its *FHIR
  validation* paragraph no longer claims that declared profiles are not
  checked or that `Patient.identifier` is required.
- **Out-of-date parse results are marked.** When an edit leaves the text
  unparseable (a trailing comma, say), the tree, the status bar and the
  FHIRPath panel kept answering from the previous version without a word;
  the status bar and the FHIRPath panel now say the text does not parse and
  that what they show comes from the last version that did.
- **FHIRPath integers.** Integer `+ - * div mod` are exact, and a result
  beyond FHIRPath's 32-bit Integer (`2147483647 + 1`) is empty, as the
  specification says, instead of a number rounded through floating point
  or stuck at the largest value (which the app and the CLI also printed
  differently).
- **FHIRPath dates.** Adding years to 29 February gives 28 February
  (`@2016-02-29 + 1 year` was the impossible 2017-02-29), as adding months
  already did. A date literal that does not exist (`@2015-02-30`,
  `@2015-13`) is an error, and `'2015-02-30'.convertsToDate()` is false.
- **HL7 editing: large messages, navigation, find, insert.** The tree
  draws only the rows on screen, so a message or log with thousands of
  segments no longer freezes the window; arrow keys move through it, and
  an edit keeps what was expanded and selected (the Field Inspector shows
  the new value). A repeating field lists each repetition. Clicking a
  validation issue selects the segment or field in the editor and the
  tree; the report follows later edits, or says it is out of date. Find
  and Replace All open the folded fields that match, so no match inside a
  folded note is missed or left behind, and a chip is never rewritten.
  Blank lines between segments no longer shift Show in Editor, grid
  clicks or Show Segment in Tree, which also picks the right MSH field
  and goes down to the component. Copy Segment copies the full line, not
  the chip. Insert segment lands at its place in the standard structure
  (not after a later ROL or NTE), uses the message's own separators and
  can be undone. An edit followed quickly by a tab switch is still
  parsed, and a message with another field separator is parsed as you
  type.
- **Opening HL7 files.** Blank lines, spaces, a byte-order mark or MLLP
  framing before MSH, UTF-16 files (saved back as UTF-16) and FHS/BHS
  batch files open and parse; any other file that does not parse opens as
  text with the reason. A segment code such as `PIDX` is reported instead
  of being read as `PID`. The same file reached through a symlinked folder
  opens once.
- **Validation and tree details.** The length rule applies to each
  repetition. The tree, the grid and editor hover describe an unknown
  version like the validator does (2.7.2 as 2.7). The grid's ellipsis
  marks only values it cut. The Field Inspector shows a component's own
  definition, a segment's field count and, for rows the message lacks,
  the schema without a made-up value.
- **Save.** A save keeps the file's owner and group; a hard-linked file,
  a file whose owner cannot be kept, or a writable file in a read-only
  folder is written in place, and a symlink to a missing file creates
  that file instead of being replaced.
- **Session and memory.** Turning session restore off deletes the saved
  tabs. An open file's text is no longer held twice. The documented
  memory figure is the backend's; the whole app, web view included, uses
  about 1.2 GB on Linux with a 10 MB message open. On macOS the menus
  show ⌘ shortcuts.
- **Opening files: macOS Finder, relative paths, missing files.** On
  macOS a double-click or *Open With* in Finder now opens the file (it
  only brought the app forward). A relative path given on the command
  line, or by a second launch, is resolved in the folder it was typed
  in, not BridgeLab's; a file that does not exist is reported instead
  of being ignored. The same file reached through `..` or, on Windows,
  a different letter case no longer opens a second tab. A Recent Files
  entry whose file is gone can be removed from the message, and the
  list keeps the last 30 files.
- **Files changed by another program.** When you come back to the
  window, or open the file again, BridgeLab says which open files were
  changed or deleted on disk and offers to load the new version. Save
  asks before overwriting a newer file or recreating a deleted one. At
  launch, a restored tab without unsaved edits shows its file as it is
  now.
- **Keyboard.** App shortcuts work with the cursor in the editor (Ctrl+L
  and Ctrl+K did nothing there). The shortcut editor refuses keys
  without Ctrl or Alt and the reserved F1, captures Ctrl+F and Ctrl+G
  even when opened from the editor, and lists the editor's own keys
  read-only. Ctrl+R no longer reloads the window on Windows. The
  editor keeps each tab's cursor position across tab switches and
  restarts.
- **Edit menu and Tools menu.** Undo, Redo, Cut, Copy and Paste in the
  Edit menu act on the editor or text field you were in (Copy used to
  empty the clipboard). Export JSON / CSV asks where to save. Tools
  commands on a tab with no parsed message, or on the wrong kind of
  document, say so instead of doing nothing or showing "Message not
  found".
- **Settings.** Auto-parse on/off and its delay, smooth scrolling and
  bracket pair colours now take effect and are kept; the unused *Max
  open messages* setting is gone. Out-of-range numbers are brought into
  range on save. Escape closes Settings and About.
- **Smaller things.** About no longer says "free for non-commercial
  use" (the core is MIT, free for commercial use too). The FHIR packages
  and rules dialogs show "Close" instead of a raw key; Word Wrap,
  Render Whitespace, tab names, licence types and a few more labels are
  translated; the status bar says "1 segment", and "elements" for FHIR
  resources; Expand All no longer marks the tab modified. The Sample
  Messages buttons stay inside the dialog at the minimum window size,
  and a few light-theme colours have more contrast. The manual's
  shortcut table and Session settings location are corrected.
- **Crafted input can no longer crash the app.** FHIRPath `decode('hex')`
  on text with accented letters aborted BridgeLab, and a FHIR XML document
  nested thousands of elements deep overflowed the stack; the first now
  returns nothing and the second is refused with a clear error (more than
  128 levels, the limit FHIR JSON already had). A FHIRPath expression
  nested more than 64 levels deep (thousands of parentheses, or a sum of
  thousands of terms) is refused with an error instead of closing the app,
  in the FHIRPath panel, the CLI and custom FHIR rules; a rule pack whose
  FHIRPath does not parse, or whose pattern does not compile, is listed
  with a warning in Settings → Plugins when it loads.
- **MLLP listener: every message on a connection, nothing half-received.**
  The listener took one message per connection and hung up (and two
  messages in one packet became one); it now serves each framed message in
  turn until the sender closes or goes idle. A message over 10 MiB was cut
  and answered AA; it is now refused with AR and an error in the console,
  and a frame that is not HL7 is answered AR instead of AA.
- **MLLP: charsets, line endings and ACKs as the manual says.** The
  listener and the sender default to a new *Auto* encoding: received
  messages are decoded in the charset MSH-18 declares, else UTF-8, else
  Latin-1 (a Latin-1 message came in as "M�ller" and was answered
  AA), and a message goes out in the charset its MSH-18 declares, else
  its file's, else UTF-8 (a file saying 8859/1 was sent as UTF-8). A
  message with characters the charset cannot hold is not sent. Segments
  go on the wire ending with CR, as from the CLI, also in HL7 v2 bodies
  sent over HTTP or SOAP (LF or CR LF from the editor went out as they
  were). ACKs, the listener's and Generate ACK's, mirror the message: its
  separators, sender and receiver swapped, trigger event, processing ID,
  version and charset, and an MSH-10 of their own (they were `P`, `2.5`
  and shared one MSH-10 per second). The header is found after a
  byte-order mark or blank lines and read with any field separator, and
  the result label reads MSA-1, commit-mode codes included.
- **MLLP listener: Stop closes open connections**, which went on being
  received and acknowledged until they went idle; a message just over
  10 MiB whose end came in the same read is refused like a longer one.
- **History as documented.** Messages received by the listener are
  logged; each entry shows the target (host:port or URL), the size, and
  the full request and response (cut past 256 KB); the newest 100 are
  kept and older ones deleted, and the list shows all of them. An HTTP
  4xx/5xx answer is logged with its code instead of "FAILED". Passwords
  and API keys stored in the history by older versions are removed at
  the first start.
- **HTTP: credentials of any kind need Pro.** Any header whose name
  suggests a key, token, secret, signature or session (such as
  `Ocp-Apim-Subscription-Key` or `X-Goog-Api-Key`), an HTTP auth scheme
  in any header, and keys or tokens in the query string (`access_token`,
  `api_key`, `sig`…) now count, not just eight header names; their values
  are redacted in the history. GET and DELETE no longer send the active
  message as a body unless one is typed, a response is read in the
  charset its Content-Type declares, and a URL with user:password is no
  longer reported as redirected.
- **SOAP: templates keep WS-Security and WS-Addressing**, which a custom
  envelope template silently dropped; they go into its Header, and a
  template with no SOAP Envelope and Body is refused. The CR segment ends
  of an HL7 payload are written as `&#13;`, so XML parsing no longer
  turns them into LF.
- **More PHI is masked.** PID-2, PID-21, the merge segment's prior
  identifiers and name (MRG-1/2/3/4/7) and NTE comments join the built-in
  catalogue.
- **Cleared data is really gone.** Clear saved session, Clear history and
  deleting a connection profile left the text readable in the database
  file; deleted rows are now overwritten, the write-ahead log is emptied,
  and an existing database is compacted once. On Linux the database is
  readable by its owner only.
- **HTTP and SOAP requests stay where you send them.** Redirects are
  followed only on the same server (or from http to https); a redirect to
  another server is shown with its target instead of re-sending the message
  and its API key there, and the result names the URL that answered. The
  SOAP client follows no redirects (a 307 re-posted the WS-Security
  password). Responses over 50 MB are cut and say so.
- **Smaller hardening.** CSV export neutralises cells starting with
  `= + - @`, like the batch export. A FHIR package with a file over 64 MB, or
  more than 1 GB unpacked, is refused as soon as the archive says so, without
  holding it in memory, and installing runs off the window's thread. Only
  StructureDefinitions are parsed; one whose JSON would expand to millions of
  values, or that fixes an element to a value over 64 KB, refuses the
  package, so a small crafted package can no longer take gigabytes of
  memory during the install or at every launch after it. The manual window gets only the permissions it
  needs. The .deb names TECHEMV SRL as maintainer and ships the licence.
- **Renaming the computer no longer costs the trial or the license.** The
  hardware ID was a hash of the host name and the user name: one start under
  another name expired the trial for good, and a paid license showed
  "License expired". The ID now comes from the operating system's machine
  identifier (Windows MachineGuid, macOS IOPlatformUUID, Linux machine-id),
  hashed with SHA-256. Licenses, offline keys and trials bound to the old ID
  keep working; renewals and Deactivate use the ID the seat was taken with,
  so no second activation is used up. A trial bound to another ID keeps its
  start date instead of expiring, and a clock that was ahead on the first
  start no longer expires it. On Windows the license and trial files move
  from the roaming to the local AppData, so a roaming profile no longer
  carries one PC's license to another.
- **A license that no longer applies can be removed.** Deactivate is offered
  for an expired license, one bound to another computer or one that no
  longer verifies, and the banner says which it is (and can be closed):
  Community keeps working. The trial banner says "1 day", not "1 days".
- **Links to the e-mail address no longer blank the window on Linux.** Every
  `mailto:` and web link, in the app and in the manual, opens in the mail
  client or browser.
- **Licensing and privacy details.** A machine `policy.json` with a
  byte-order mark (as PowerShell writes it) or `"true"` as a string is now
  honoured. Activation, renewal and usage statistics trust the operating
  system's certificates too, so they work behind a company's TLS-inspecting
  proxy, and a proxy or server error page is reported as such instead of
  "no internet access". Community HTTP requests can no longer carry
  credentials in the URL or in API-key or cookie headers, and credentials
  are removed from the request history. Importing a test-case pack that only
  replaces existing cases works over the Community cap. The update check
  toggle is unticked until the first-run question is answered, and Help →
  Check for Updates makes only the one documented request. The manual,
  privacy page, README, SECURITY.md and TEST_PLAN now match the app
  (licence-server contacts, telemetry fields, edition tables, banner
  buttons).
- **FHIRPath on the wrong kind of tab.** The FHIRPath panel opens only on
  a FHIR tab (it said "Message not found: …" on an HL7 v2 message) and
  says why otherwise. `conformsTo()` says it is not implemented rather
  than that profile packages are not loaded, and the CLI README shows how
  to pass an expression that starts with `-`.
- **A custom FHIR rule's id is shown.** Findings from FHIR rule packs carry
  the rule's `rule_id` in the validation panel (instead of "FHIR") and in
  the CLI's text and JSON output (instead of an empty string), so CI can
  filter by it.
- **The FHIR rules builder keeps what it did not write.** Save keeps the
  keys it does not use (`$schema`, an owner, a comment or ticket on a rule),
  and refuses, instead of overwriting, when `user-rules.json` changed on
  disk while the builder was open. Two rules with the same id and a
  pattern that does not compile are refused before they are saved. When
  the pack is switched off in Settings → Plugins, or left out by the
  Community limit, the builder says so (Test used to say "Rule fires" while
  F6 ran nothing).
- **The FHIR rules builder no longer wipes a rule file it cannot read.**
  When `plugins/fhir/user-rules.json` had a typo or a byte-order mark, the
  first Save replaced it with the builder's rules alone: hand-written rules,
  name and id were lost. Save now refuses and leaves the file alone; a
  byte-order mark is accepted. Closing the builder with unsaved rules asks
  first, and *Test* on a Bundle lists the values it matched in the entries.
- **Plugin packs: switched off means off everywhere.** The on/off choice is
  saved in the plugins folder, so the CLI honours it too, and it belongs to
  one pack: switching off a validation pack no longer switches off a PHI
  pack that shares its id (which left those fields unmasked). Two packs in
  one folder with the same id: the second is refused, listed with the
  reason, and none of its rules run (it used to run anyway, and Settings →
  Plugins showed no pack at all), and a newly added pack no longer displaces one in use. A
  refused fourth pack shows as off, with a message naming the limit, and a
  validation report says when packs you switched on were left out by the
  cap.
- **Plugin validation rules check what they say.** A repeating field is
  checked one repetition at a time, components follow the message's own
  separator, lengths count characters, and a pattern that does not compile
  is reported on every run instead of passing silently. Patterns are
  compiled once per run and validation no longer runs on the window's
  thread: 20 rules on a 5000-OBX message took 13 s and froze the app.
- **Plugin PHI masking is exact.** A PHI field on MSH masked the next field
  and field 0 replaced the segment name; repetitions were merged into one
  value and a `$` component separator was ignored. A field named by two
  packs is masked once at the stronger level, `phi_rules` in a pack outside
  `anonymization/` are applied instead of dropped, a misspelt sensitivity
  masks fully, and long numbers keep their length.
- **Plugin loading is robust and says what it skipped.** Packs with a
  byte-order mark or a `.JSON` extension load; a named pipe or an oversized
  file in the folder is reported instead of hanging startup. Settings lists
  each pack's warnings (unknown severity or sensitivity, broken pattern,
  unmaskable field), and the CLI names on standard error the packs it did
  not load or left inactive. The CLI reads PHI sensitivities as the app does.
- **A paste right after an edit no longer shows the previous message's
  tree.** The parse of the edit, running half a second later, overwrote the
  pasted message's result, and could even land in another tab after a tab
  switch. A background parse now updates only its own tab, and only while
  the tab still holds the text it parsed.
- **Pasting into a search box or a form field no longer replaces the
  open message.** A paste anywhere outside the editor went to a
  window-level fallback that overwrote the whole tab (tree search, MLLP
  host, licence key, FHIRPath expression), with no undo. Fields now get
  their own paste; pasting outside any field fills an empty tab or opens
  a new one, never over an existing message.
- **Closing a tab with unsaved changes asks first** (Ctrl+W, ×, middle
  click, Close Others, Close All). Without session restore, closing the
  window asks too.
- **The session keeps the last edits and survives a slow start.** Closing
  the window within a second of typing lost those keystrokes; the session
  is now written when the window closes. On a slow start the autosave of
  the still-empty tab set could delete the saved session before it was
  restored.
- **Restored tabs show their own message.** After a restart the tree,
  status bar and tools of the active tab could show another tab's message.
- **A failed save no longer destroys the file.** Saving truncated the file
  and then wrote it, so a full disk left a cut-off message; files are now
  written to a temporary file and swapped in only when complete (Save,
  Save As, test-case pack export, FHIR rule packs).
- **Saving a message with a long field no longer destroys it.** The
  editor held the truncated view of a message (`{...5144 bytes}` in place
  of a base64 attachment), and Save, Save As and the saved session wrote
  that view to disk: a 5 KB report became 233 bytes. The editor now
  always holds the whole message. Long values are *folded* on screen
  instead: a base64 attachment, a long note or a JSON `data` string shows
  as a compact chip such as `⟨Base64 · 5.1 KB⟩`, separators and the other
  components still visible. Click a chip (or Alt+Enter next to it) to
  expand it, *Fold This Field* / *Fold All Long Fields* to fold again, and
  the status-bar badge expands them all. A chip moves as one unit, copying
  copies the full content, and the fold threshold in Settings → Parser now
  takes effect.
- **Saving keeps the segment terminators.** A file with CR between
  segments came back with CRLF after any edit, and lost its final CR even
  without one; the tab also showed as modified right after opening. The
  editor now gives the text back with the line ends the file had.
- **Insert segment from the tree goes where it belongs again.** Right-
  clicking a greyed standard segment and choosing *Insert* put its
  skeleton on line 1, above MSH, which broke the message; it now lands
  after the segment that precedes it in the standard structure.
- **F1 opens the manual even when the editor has focus.** Monaco took F1
  for its command palette. The palette is still there, on
  Ctrl+Shift+P (Cmd+Shift+P on macOS) and in the editor's context menu.
- **An anonymized message validates clean.** Dates were masked to
  `00000000`, which the new date check reports; they are now masked to
  1900-01-01 with the original precision.
- **ISO-8859-1 is real Latin-1.** It was handled as Windows-1252, so `€`
  and typographic quotes went out as control bytes in a message that
  declares `8859/1`; they are now written as `?`, like any character the
  charset cannot hold. Choose windows-1252 to keep them.
- **Files in ISO-8859-1, Windows-1252 and the other legacy charsets.**
  The app refused them, and the CLI read them as garbage: `anonymize`
  and `to-json` turned whole segments into `<invalid utf8>` and still
  exited 0, and `send` corrupted accented characters and reported the
  ACK as a success. A file that is not UTF-8 is now read in the charset
  its MSH-18 declares, or as Windows-1252 when MSH-18 is empty, in the
  app (open, batch validation, batch anonymisation) and in every CLI
  command. Save, `anonymize` and batch anonymisation write it back in the
  same charset; `send` without `--encoding` transmits in it. A UTF-8
  byte-order mark no longer makes the CLI reject a message. The table
  0211 charsets read are ASCII, the ISO 8859 parts, UTF-8, BIG-5,
  GB 18030-2000 and KS X 1001; see below for the ones that cannot be
  decoded.
- **Sending in a legacy charset never writes `&#NNN;` any more.** A
  character the chosen encoding cannot hold was sent as an HTML-style
  reference that corrupted the field; such a message is now not sent at
  all, in the app and in `bridgelab-cli send`, and the characters are
  named (Save still writes them as `?`). *ASCII* now means 7-bit ASCII. `bridgelab-cli send` refuses an unknown `--encoding`
  instead of silently sending UTF-8.
- **Data types are checked as the manual always said.** Only SI and an
  8-digit DT were checked, so a birth date of `19801399`, a timestamp
  of `2024-01-01 12:00` or a numeric result of `abc` validated clean.
  Numbers (NM), dates (DT), timestamps (TS/DTM) and times (TM) are now
  checked for format and against the calendar and the clock, OBX-5 as
  the type OBX-2 declares, each repetition on its own. They are warnings,
  so no valid message changes verdict. A second MSH in the same text,
  which the manual also listed, is now reported too.
- **FHIR XML behaves like the same resource in JSON.** XML primitives
  were read as strings, so a valid Patient with `<active value="true"/>`
  failed validation ("must be a boolean"), `Patient.active = true` was
  false, a Quantity value could not be compared, and the
  `Observation.value.ofType(Quantity)` example in the CLI README returned
  nothing. A single `<profile>` in `meta` was not read as the list it is,
  so the profile it named was never applied, and an XML test case could
  pass against the profile it violated. XML is now typed from the FHIR R4
  definitions the app carries: booleans, integers and decimals as in
  JSON, and every repeating element as a list — in the app, the CLI and
  test packs alike.
- **The CLI no longer passes a CI run it did not check.** A path or glob
  that matches no file is reported and exits 1 in `validate`, `info` and
  `test`, even beside files that do match (a file named with glob
  characters is read as it is, a file matched twice is read once, `-`
  twice is a usage error). `batch` matches extensions in any case and
  with or without the dot (`HL7`, `.hl7`, `hl7,txt`), follows symbolic
  links, lists files in name order, reports unreadable entries, gives each
  file's issues in `--json`, and exits 1 when no file was checked. An
  empty test pack, or an `expected_validation_result` other than
  `valid`/`invalid`, is an error instead of a pass. A `--fhir-packages`
  folder that is missing or holds no readable package is named on stderr
  and in every FHIR result; in JUnit, resources whose profiles were not
  (all) checked are `skipped` with the reason instead of passing, test
  cases are named by their full path, and control characters no longer
  break the XML. `--format` and `--strict` take only valid values
  (`--no-strict` exists), and a closed pipe (`| head`) ends the CLI
  quietly instead of a panic.
- **Expected type `ADT^A01` matches `ADT^A01^ADT_A01`.** In the Test Case
  Library and in `bridgelab-cli test` only the components the expectation
  gives are compared, so the message structure v2.3.1+ adds as MSH-9.3 no
  longer fails a case; an expected result is read in any case.
- **HL7 parsing is linear on CR-only and LF-only files.** Each segment end
  was searched separately for CR and for LF, scanning to the end of the
  file for the one it did not use: a 17 MB message took almost two minutes instead of
  a second.
- **Field lengths count characters**, not UTF-8 bytes, so accented names
  are no longer reported over the limit. A missing MSH-10 or MSH-12 is
  reported once (as the required-field error), and an MSH-12 version
  BridgeLab has no catalogue for gets an info note (MSH-005) naming the
  version it was validated against.
- **MLLP send no longer hangs on a peer that stops reading.** The response
  timeout now bounds the write too (as "no progress for that long"), in
  the app and the CLI. `bridgelab-cli send` sends a file of several
  messages as one frame per message, each acknowledged, and does not take
  an ACK whose MSA-2 names another message as an acceptance.
- **CLI output files are written atomically** (`anonymize`, `to-json`,
  `xsd -o`): a failed write keeps the existing file, the input included,
  and the error names the path. Anonymized output ends with a CR and shows
  one segment per line on a terminal.
- **JSON that is not a FHIR resource says so.** `package.json` or a FHIR
  file with a syntax error is reported as "Not a FHIR resource" or
  "Invalid JSON … at line 1 column 37" instead of "Message does not start
  with MSH", in the app and the CLI. The FHIR "version" shown for a
  resource is no longer its `meta.versionId`, and a resource whose only
  profile findings are "declared but not installed" is no longer labelled
  as checked.
- **Documentation**: the pre-commit hook in the CLI README handles commits
  without HL7/FHIR files, file names with spaces or accents, and non-FHIR
  JSON; the Linux requirements are stated as they are (the app needs glibc
  2.35+, the CLI glibc 2.34+; the site said 2.31+); `send` exits 0 on AA
  or CA, as the manual and site now say.

- **Anonymization masks every personal field of the segments it covers.**
  The catalogue missed the insured's and guarantor's dates of birth,
  the insured's address, the IN2 SSN, the next of kin's business phone and
  contact person, the patient's birth place and death date, the visit
  number and more: they stayed in the output while the dialog said
  "Anonymized", and a message whose only PHI sat there was reported as
  having none. It now has 89 fields after the HL7 v2.5 definitions of PID,
  PV1 (visit numbers), MRG, NK1, GT1, IN1 and IN2, plus NTE comments and
  free-text (TX/FT) OBX results; the dialog, Batch anonymize, the CLI and
  "Mask personal data" in a test pack export share it. An identifier keeps
  its assigning authority and type (`000000^^^HOSP^MR`), so masked v2.3
  messages no longer fail the PID-3 length check, and a date or
  timestamp with fractional seconds or a UTC offset is masked to a valid
  1900-01-01 of the same shape instead of `REDACTED`. "No PHI" now says it
  covers the fields BridgeLab checks, not free text elsewhere, and is no
  longer shown in green. Low masking counts characters, not bytes (`Zoë`
  no longer gets a stray `...`). The manual's sensitivity examples, the
  README, the site and the CLI README give the real catalogue.

- **Batch anonymize and "Save all to folder" never replace a file.** A
  file of the same name already in the output folder was overwritten
  without a word, and a symlink or hard link there let the masked copy
  land on a selected original. Both now create each file exclusively: a
  name already taken is reported on its row (Batch anonymize) or listed
  as not saved (the generator), and the file is left as it is. Batch
  anonymize shows output paths without the Windows `\\?\` prefix.
- **Test-message generator**: the visit number is in PV1-19 (it was in
  PV1-18, which gave a length warning from the tenth message on), and a
  seed that is not a whole number from 0 to 2^53−1 (`-1`, `1.5`, `1e20`)
  gets a clear message instead of a raw error.

- **macOS: Cmd+Q asks about unsaved tabs and saves the session.** The
  standard Quit item (Cmd+Q) ended the app without the window's close
  request, so unsaved tabs were dropped without asking when session
  restore was off, and the last keystrokes were lost when it was on.
  *BridgeLab → Quit BridgeLab* is now the app's own item: it closes the
  window the way its close button does (prompt, final session save) and
  quits once the window has closed. Quitting from the Dock or by logging
  out still ends the app directly; the session autosave (under a second
  behind) covers it when restore is on.

- **A read-only file is not replaced on Linux and macOS.** Since saves
  became atomic, Save, test pack export, FHIR rule packs and every CLI
  `--output` replaced a write-protected file (`chmod 444`) without a
  word, while Windows refused. Every OS now refuses with an error that
  names the file as read-only, and the file stays as it was.
- **CLI patterns ignore case.** `validate "g/*.hl7"` skipped `B.HL7` and
  `c.Hl7` and still exited 0; patterns now match names in any case on
  every OS, as the app and `batch --extension` do.

- **Windows: deleting one folder no longer restarts the Pro trial.** Both
  copies of the trial record had ended up in `%LOCALAPPDATA%\BridgeLab`;
  the second copy is now kept in `%APPDATA%\BridgeLab` (and the
  uninstaller's data purge keeps it). The old copy is still read and
  kept, so a running trial is neither restarted nor shortened by the
  upgrade.
- **Windows: the hardware ID is read through the registry API.** It came
  from the output of `reg.exe`, which the "Prevent access to registry
  editing tools" policy blocks; the licence then read as bound to another
  computer. The ID itself is the same as before.

- **macOS: shortcuts are shown with ⌘.** Settings → Shortcuts and the
  manual's shortcut table said "Ctrl+O" while the menus said ⌘O, and
  listed the editor's Replace as Ctrl+H (⌘H hides the app), Redo as
  Ctrl+Y and Go to Line as Ctrl+G (⌘G is Find Next). They now show ⌘, ⌥
  and ⇧, and the editor's real macOS keys: ⌘⌥F, ⌘⇧Z and ⌃G.

- **Anonymize, Export JSON/CSV and Copy Truncated work on the text in
  the editor.** With auto-parse off they used the last parse, so after
  replacing the message the dialog listed, and masked, the previous
  patient. The text is now parsed first; text that does not parse, or is
  no longer an HL7 message, says so, and switching tabs meanwhile cancels
  the command instead of applying it to the other tab. *Compare messages* loads restored tabs that were not shown yet (one
  side of the diff was empty), and FHIRPath on an HL7 v2 tab no longer
  talks about the Bundle Visualizer.

- **Manual: the XSD licensing note matches the files.** It said every
  generated XSD carries a header acknowledging HL7®; the files have had
  no comment header since they were aligned with the namespace-less
  layout Astraia expects. The note now says so.

- **Exports.** CSV files start with a UTF-8 byte order mark, so Excel
  shows `Müller`, not `MÃ¼ller` (Tools → Export CSV and the batch
  validation report); a negative number such as `-2.3` stays a number
  instead of getting the formula guard `'`. Export JSON and the CLI's
  `to-json` list fields in message order (MSH-2 before MSH-10). On Linux
  a file name typed without an extension in a save dialog gets it (`.json`,
  `.csv`, `.xsd`, `.bltests.json`), asking first when that file exists.
- **Templates**: MSH-7 and the other timestamps are local time with the
  UTC offset (they were UTC with no offset, hours off), MSH-10 is unique
  per message rather than per second, the SIU^S12 and MDM^T02
  templates validate without errors (SCH-6/16/20 filled, TXA-12/17 in
  their fields), and the template list shows names, descriptions and
  categories in the UI language.
- **Test Case Library**: the search also looks in the message text and
  the expectations, as the manual says; *Updated* is shown in local time
  (it was off by the UTC offset); and a hand-written pack with an empty
  category is no longer "Differs" on every re-import.

- **Smaller fixes.** Tools → Export message schema as XSD opens on a free
  message for Community users instead of an upgrade error. Dropping a
  folder on the window says it is a folder, not "File not found". App
  commands answer the main window only (the manual window could call
  them). The manual no longer calls Copy Truncated Message "safe for
  email" (it copies patient data as it is), and says that only the
  Windows `.exe` setup asks about update checks, not the `.msi`.
- **Documentation**: macOS 11 or later with Safari 16.4 is required (the
  interface needs Safari 16.4; the DMG said 10.15), the AppImage has no
  `.hl7` file association, the privacy page and FAQ say where the local
  database keeps session tabs, the communication history and test cases
  (on Windows in the roaming AppData), and the test plan covers Windows 10
  and macOS 11/12.

### Changed
- **Every package carries the Business Source License.** The installers'
  licence page (Windows NSIS and MSI) now shows the MIT licence followed
  by the BUSL 1.1 text of the `pro/` directories, with its Additional Use
  Grant; every package installs the same text as `LICENSE.txt` next to
  the app, and the `.deb` and `.rpm` also put it in
  `/usr/share/doc/bridgelab/` (`LICENSE-pro`, `LICENSE-pro-ui`). Before,
  only the MIT licence and a pointer to the source repository were
  shipped. The installer page shows it one paragraph per line, so the
  text fits the box instead of breaking every line twice. The BUSL grant
  now names the site's *Terms of Sale & EULA* instead of "Terms of
  Service".
- **`bridgelab-cli info`** reports `parsed` (read as HL7 v2 or FHIR)
  apart from `valid`, which now means no error from the built-in checks;
  `valid: true` used to mean only that the file parsed.
- **Documentation matches the product.** The privacy page and FAQ say the
  history keeps full requests and responses (latest 100, up to 256 KB
  each, listener messages included) and list the landing page's requests
  to GitHub and FastSpring and the order attribution tag. The landing page,
  ROADMAP and manual no longer describe truncated editor text, live plugin
  reload, a right-click Field Inspector, Z-segment packs for the
  inspector or a team test case library "coming soon". The XSD is
  described as the v2.xml layout without its namespace. The README's
  licence section says what the BUSL directories hold (the SOAP client)
  and that a build without them is not a Community-only build. The manual
  in all five languages names menus and settings as the interface does.
- **The CLI is documented as free.** It never read a licence, and batch
  validation, PHI masking and installed FHIR packages already worked in
  it; the docs said it "behaves as the Community edition". They now say
  what it is: free, with two Community limits kept on purpose — active
  plugin packs, and the XSD export (full catalogue in Pro).

## [1.8.1] — 2026-09-25

### Added
- **IT can force the usage statistics off for a whole machine**, like
  the update check: `BRIDGELAB_DISABLE_TELEMETRY=1`, or
  `"disable_telemetry": true` in the machine `policy.json`
  (`%ProgramData%\BridgeLab\`, `/Library/Application Support/BridgeLab/`,
  `/etc/bridgelab/`). Nothing is sent then, whatever the user ticked, and
  Settings → Privacy shows the box locked with the reason.

### Changed
- **The MLLP listener listens on this computer only by default**
  (`127.0.0.1` instead of `0.0.0.0`). Starting a listener no longer opens
  a port to the whole network; to receive a feed from another machine,
  choose *Accept connections from other machines* (or type `0.0.0.0` or
  an interface address) under *Bind*. A hint under the field says which
  of the two is in effect.

### Fixed
- **The manual opens with its content on every platform.** *Help →
  Manual* (F1) loaded the manual in its own window from a temporary
  in-memory address created by the main window; on Linux that window
  opened empty. The manual is now a page of the app itself, loaded the
  same way on Windows, macOS and Linux, with the same contents,
  shortcuts table and language.

### Security
- **A content security policy for the app window.** Until now nothing but
  the code itself kept the webview from contacting any host. The policy
  allows only the app's own scripts, styles, fonts and workers, and
  network requests only to the app's backend and `api.github.com` (the
  update check); anything else is refused by the webview. Licensing and
  the opt-in statistics run in the native backend and are unaffected.

## [1.8.0] — 2026-09-24

### Added
- **Buying a license from inside the app.** Until now the app only said
  "contact info@techemv.it" — the Pro and Enterprise cards in the
  activation dialog read *Contact us*, and the "requires a Professional
  license" prompts pointed at the activation dialog. The online store
  was reachable only from the trial banner's *Compare plans*, and that
  landed on the top of the page. Now *Help → Buy a License…*, a *See
  prices & buy* button on the Pro and Enterprise cards, a *See prices*
  button on every upgrade prompt, and *Compare plans* all open the
  pricing section of the website, where the checkout runs. Each link
  carries `utm_source=app` and the entry point as `utm_medium`, which the
  landing passes on to the order, so sales can see where a purchase
  started. E-mail stays for invoices, purchase orders and quotes.
- **New versions are announced at startup.** Once a day, a few seconds
  after launch, BridgeLab asks GitHub for the latest release and, when it
  is newer, shows a banner with *Download* (the release page), *Skip this
  version* and a close button — no dialog, nothing installed
  automatically. The request is an anonymous GET to `api.github.com`
  with nothing about the user, the machine or the files; offline it is
  skipped silently. Nothing is requested before the user decides: the
  Windows installer asks during setup, and every other install asks once
  in a banner at first start (*Yes, check* / *No*; closing it asks again
  next time). The answer lives in *Settings → Privacy → Check for new
  versions at startup*; the privacy policy and the manual describe it. *Help → Check for Updates* shares the same code.
- **Choosing about the update check before it ever runs, and turning it
  off for a whole site.** The Windows installer asks, the first time,
  whether BridgeLab may check for new versions (*Yes* is the default;
  silent installs are not interrupted); the answer becomes the Settings
  preference. Administrators turn the check off for every user of a
  machine with `BRIDGELAB_DISABLE_UPDATE_CHECK=1` or a `policy.json`
  (`{"disable_update_check": true}`) in `%ProgramData%\BridgeLab\`,
  `/Library/Application Support/BridgeLab/` or `/etc/bridgelab/`; the
  Settings checkbox then shows as locked, naming where the policy came
  from.

### Changed
- The privacy policy now states that a license activated online repeats
  its activation request in the background, at most once a day, from 14
  days before expiry, to pick up a renewal — it said only that the
  software never needs to contact the server again.

### Fixed
- **"Delete the application data" on Windows uninstall left BridgeLab's
  data behind.** The option removed only the WebView2 profile
  (`%APPDATA%\com.bridgelab.app`); the database — preferences,
  communication history with message previews, test cases, open tabs —
  plugins, FHIR packages and the installer's answer stayed in
  `%APPDATA%\BridgeLab`. The uninstaller now removes that folder too when
  the box is ticked (never during an update), keeping only
  `license.json`, so a reinstall stays licensed (free the seat of an
  online activation with *Deactivate* first if the machine is leaving),
  and the trial files, so uninstalling does not restart the trial.
- **The macOS `.app.tar.gz` updater bundles were still uploaded with
  1.7.0**, although `createUpdaterArtifacts` is off in `tauri.conf.json`.
  The release workflow now repeats the setting on the `tauri build`
  command line, tells the action not to build a `latest.json`, and no
  longer hands the updater signing key to the build — without a key no
  signed updater bundle can be produced, so a regression fails the job
  instead of quietly re-adding the assets.

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
  macOS `.app.tar.gz` updater bundles were meant to go: nothing consumes
  them — release artifacts are not signed, there is no `latest.json`, and
  *Help → Check for updates* has always fallen back to comparing against
  the latest GitHub release, which it still does. (They still appeared on
  the 1.7.0 release; see *Unreleased*.) The AppImage no longer
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
  - Verified against hl7.fhir.r4.core 4.0.1: its **4578 conformance
    resources** (definitions, value sets, code systems and search
    parameters; the package has no clinical examples) **validate with 15
    files reported, every one genuine** — 10 `SearchParameter` files that
    really do omit the required `base`, 4 ValueSets declaring a profile the
    core package does not contain, and `CodeSystem-v2-0550` whose codes
    carry a non-breaking space.
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
