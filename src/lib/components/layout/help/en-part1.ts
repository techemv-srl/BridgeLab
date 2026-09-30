import type { ManualSection } from '../helpContent';
import { mockupAppShell, mockupContextMenu } from './mockups';

export const getStarted: ManualSection = {
	id: 'getting-started',
	heading: 'Getting Started',
	body: `
<p>BridgeLab is a modern message editor for HL7 v2.x and FHIR, designed
for healthcare integration engineers. It is built on a Rust backend for
fast parsing (a 10 MB message with a base64 attachment opens in about 2 seconds) and a Svelte 5
frontend with the Monaco editor.</p>

<p>The main window is divided into four regions:</p>
${mockupAppShell}

<ol>
	<li><strong>Menu bar and trial banner</strong> at the top - File, Edit,
		View, Tools, Help menus plus a yellow/red banner reminding you of
		the Pro trial status.</li>
	<li><strong>Tree panel</strong> on the left - the parsed message
		structure with expand/collapse arrows, and a Field Inspector at
		the bottom showing HL7 schema info for the selected node.</li>
	<li><strong>Editor and tabs</strong> in the center - Monaco editor with
		HL7 syntax highlighting; multi-tab toolbar to keep several
		messages open at once.</li>
	<li><strong>Status bar</strong> at the bottom - message type, version,
		segment count, cursor position.</li>
</ol>

<h3>Opening a message</h3>
<ul>
	<li><strong>File → Open File</strong> (<kbd>Ctrl</kbd>+<kbd>O</kbd>) - native
		file picker for <code>.hl7</code>, <code>.txt</code>, <code>.msg</code>,
		<code>.json</code>, <code>.xml</code>.</li>
	<li><strong>Drag &amp; drop</strong> - drop a file onto the editor area.</li>
	<li><strong>Paste</strong> - click in the editor and paste
		(<kbd>Ctrl</kbd>+<kbd>V</kbd>). Auto-parse runs 500 ms after the
		last keystroke (delay, or off, under <strong>Settings →
		Parser</strong>).</li>
	<li><strong>What opens:</strong> blank lines, spaces, a byte-order mark
		or MLLP framing before <code>MSH</code>, UTF-16 files (Notepad's
		"Unicode") and FHS/BHS batch files are read as they are. A file
		BridgeLab cannot parse still opens, as text you can fix, and a note
		says why.</li>
	<li><strong>File → New Message from Template...</strong> (<kbd>Ctrl</kbd>+<kbd>N</kbd>) -
		pre-filled ADT, ORM, ORU, SIU and more. Fields like MSH-7 and
		MSH-10 are filled with the current timestamp and a unique message
		id.</li>
	<li><strong>File → Sample Messages</strong> (also on the welcome
		screen) - complete, realistic messages rather than skeletons: ADT
		admission, registration, update, discharge and merge, ORU results
		(a twelve-result blood count, a metabolic panel), ORM orders, SIU,
		MDM, DFT, VXU and an ACK, in v2.3, v2.5 and v2.5.1. Filter by
		version, preview, and open one in a new tab. Every sample validates
		clean; patients and data are fictional.</li>
</ul>

<div class="note">On first launch you get a <strong>14-day Pro trial</strong>
with every Pro feature enabled (SOAP and priority support are
Enterprise features). After expiry, BridgeLab continues to work
with the Community feature set - you never lose your messages.</div>

<p>The <strong>Discover BridgeLab</strong> cards on the welcome screen open
the features that set BridgeLab apart — the test-message generator, PHI
anonymization, the MLLP listener and the XSD export — directly, with PRO
badges marking the licensed ones.</p>
`,
};

export const editorSection: ManualSection = {
	id: 'editor',
	heading: 'Editor',
	body: `
<p>The editor area is a <strong>Monaco</strong> instance with an HL7-specific
grammar. Segment codes are coloured purple, field separators grey, and
ED/base64 payloads and other long values are shown folded, so the editor
stays fast on large messages (see below).</p>

<h3>Auto-complete and hover</h3>
<p>Start typing <code>P</code> on a new line - Monaco suggests
<code>PID</code>, <code>PV1</code>, <code>PV2</code>, etc. Once you enter
a segment, pipe autocomplete proposes field values (gender codes, ACK
codes, patient class...). Hovering over any field displays its name,
data type, and required flag drawn from the HL7 standard.</p>

<h3>Folded long fields</h3>
<p>Fields longer than the fold threshold (default 100 characters, set in
<strong>Settings → Parser</strong>) are shown <em>folded</em>: a base64
attachment in OBX-5, a long note or a JSON <code>data</code> string
appears as a compact chip such as <code>⟨Base64 · 5.1 KB⟩</code>, while
separators and the other components stay visible. Folding is only a
view: the message itself is always complete, and Save, the session,
validation, the tree and Copy all use the full text.</p>
<ul>
	<li><strong>Expand:</strong> click the chip, or put the caret next
		to it and press <kbd>Alt</kbd>+<kbd>Enter</kbd>. Hovering shows
		the first characters.</li>
	<li><strong>Fold again:</strong> right-click → <em>Fold This
		Field</em> on any long value, or <em>Fold All Long Fields</em>.</li>
	<li><strong>Everything at once:</strong> the <em>N folded</em> badge
		in the status bar expands all; the context menu has both
		commands.</li>
	<li>A chip moves as one unit: the arrow keys jump over it and
		Backspace/Delete select it first, so it is never half-deleted.
		Copying a selection copies the full content.</li>
	<li><strong>Find and Replace</strong> (<kbd>Ctrl</kbd>+<kbd>F</kbd>,
		<kbd>Ctrl</kbd>+<kbd>H</kbd>) search the full text: a folded field
		that contains a match opens, so the match is counted, shown and
		replaced like any other.</li>
</ul>

<h3>Editor settings</h3>
<p><strong>Edit → Settings → Editor</strong> (Ctrl+,) changes how the editor
looks and behaves: font and size, tab width, word wrap, visible whitespace,
minimap, line numbers, smooth scrolling, bracket colours, highlighting of the
other occurrences of the word under the cursor, clickable links, sticky
scroll, and where word suggestions come from (this message, every open
message, or none; HL7 field and value suggestions work in every mode).
Changes apply at once. Each tab keeps its own undo and redo history
(Ctrl+Z, Ctrl+Y) while you switch between tabs.</p>

<h3>Right-click context menu</h3>
${mockupContextMenu}
<p>The menu groups actions into three sections:</p>
<ul>
	<li><strong>Navigation:</strong> Show Segment in Tree
		(<kbd>Alt</kbd>+<kbd>T</kbd>) - opens the tree and highlights the
		exact field under the cursor; Expand / Fold for long
		values.</li>
	<li><strong>Clipboard:</strong> Copy Segment
		(<kbd>Alt</kbd>+<kbd>C</kbd>), Copy Full Message (with expanded
		fields), Copy Truncated Message (long fields cut short, to fit in an email; patient data is copied as it is: anonymize first).</li>
</ul>

<div class="note">Monaco's native shortcuts (<kbd>Ctrl</kbd>+<kbd>F</kbd>
find, <kbd>Ctrl</kbd>+<kbd>H</kbd> replace, <kbd>Ctrl</kbd>+<kbd>Z</kbd>
undo, <kbd>Ctrl</kbd>+<kbd>D</kbd> multi-cursor) all work as expected
inside the editor.</div>
`,
};

export const treeSection: ManualSection = {
	id: 'tree-view',
	heading: 'Tree View &amp; Field Inspector',
	body: `
<p>The tree on the left mirrors the HL7 message hierarchy:
<strong>segments</strong> → <strong>fields</strong> →
<strong>components</strong>; a repeating field lists each repetition
(<code>PID-3(1)</code>, <code>PID-3(2)</code>) with its components. Toggle visibility with
<kbd>Ctrl</kbd>+<kbd>B</kbd> or <strong>View → Message Tree</strong>.</p>

<h3>Navigating between tree and editor</h3>
<ul>
	<li><strong>Editor → Tree:</strong> Right-click a field in Monaco and
		choose <em>Show Segment in Tree</em>. The tree expands the segment,
		selects the exact field (down to the component level) and scrolls
		it into view.</li>
	<li><strong>Tree → Editor:</strong> Right-click a tree node and choose
		<em>Show in Editor</em>. Monaco jumps to the line, places the
		cursor at the right column and selects the field range.</li>
</ul>

<p>Blank lines between segments do not throw the jumps off. In the
tree, <kbd>↑</kbd>/<kbd>↓</kbd> move the selection, <kbd>→</kbd> opens a
node and <kbd>←</kbd> closes it or goes to its parent;
<kbd>Home</kbd>/<kbd>End</kbd> and <kbd>PgUp</kbd>/<kbd>PgDn</kbd> jump.
While you edit, the tree keeps what you expanded and selected, and the
Field Inspector shows the current value.</p>

<h3>Field Inspector panel</h3>
<p>Click the <strong>ⓘ</strong> icon in the tree panel header (or
<strong>View → Field Inspector</strong>) to show schema-derived metadata
for the currently selected node:</p>
<ul>
	<li>HL7 position (e.g. <code>PID-5</code>) and canonical name
		(Patient Name)</li>
	<li>Data type (XPN, CX, ST, ...), max length, required/repeating
		flags, description</li>
	<li>Current value and length; a <em>View full value</em> button for
		long fields the editor shows folded</li>
</ul>
<p>Unknown segments (Z-segments or custom codes not in the standard)
display <em>Not in HL7 standard</em> but remain fully editable.</p>

<h3>Searching the tree</h3>
<p>The search box at the top of the tree matches on <strong>segment
type</strong> (<code>PID</code>), <strong>schema field name</strong>
(<code>Patient Name</code>) and <strong>field value</strong> — including
fields inside segments you have not expanded yet. Press
<kbd>Enter</kbd>/<kbd>Shift</kbd>+<kbd>Enter</kbd> to cycle through
matches, <kbd>Esc</kbd> to clear, and <kbd>Ctrl</kbd>+<kbd>F</kbd> while
the tree has focus to jump to the box. Clicking a result expands the
segment, selects the field and scrolls it into view.</p>
<p class="note">Tree search works on HL7 v2 messages. For FHIR
resources use the Bundle visualizer's own filter or the editor's
<kbd>Ctrl</kbd>+<kbd>F</kbd>.</p>

<h3>Segment grid</h3>
<p>A result message can carry dozens of OBX; the tree shows them one node
at a time. <strong>View → Segment Grid</strong>
(<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd>), or <em>Show all OBX in a
table</em> from a segment's right-click menu, opens a bottom panel that
lays every occurrence of one segment type out as a table: one row per
occurrence, one column per field that holds a value in at least one of
them, headed with the field's position and its name in the message's HL7
version. Coded values show their meaning under the code, like in the tree;
long values are cut with an ellipsis. Pick another segment from the list
(each shows how many times it occurs), type in <em>Filter rows</em> to
keep only the rows that contain some text, and click a cell to select
that field in the editor. The grid is read-only and follows the message
as you edit and re-parse.</p>

<h3>Comparing two messages</h3>
<p><strong>Tools → Compare messages…</strong> opens a side-by-side diff
of any two open tabs with HL7 syntax highlighting. Pick left/right from
the dropdowns, use the ⇆ button to swap sides, press <kbd>Esc</kbd> to
close. At least two tabs must be open.</p>

<h3>Coded fields: what a code means</h3>
<p>BridgeLab ships the HL7 value tables — 394 tables, about 5,000
codes — and knows which table every coded field and component draws
from, per version. A coded value is explained wherever you meet it:
the tree shows the meaning next to the value (<code>M — Male</code>,
<code>ADT — ADT message</code>, <code>F — Final results</code>),
hovering the field in the editor says it too, and auto-completion in a
coded field offers every value of its table. Components are covered as
well: MSH-9.2 is explained from the event-type table, PID-3.5 from the
identifier-type table.</p>
<p>The Field Inspector lists the whole table for the selected field or
component and highlights the current value. Whether a value outside the
table is a problem depends on the field's data type, and the inspector
says which case you are in: an <code>ID</code> field draws from an
HL7-defined table (<em>Allowed values</em>) and a value not listed is
non-standard — a warning appears; an <code>IS</code> field draws from a
user-defined table (<em>Suggested values</em>), where sites add their own
codes and absence means nothing. Some user-defined tables have no
standard values at all (IN1-2 Insurance Plan ID); those fields show no
list.</p>
<p class="note">Which table a field uses follows the declared HL7
version; the table contents are one set for all versions, as the
upstream source ships them. A code added in a later release is
therefore accepted for an earlier one.</p>

<h3>Schema-aware tree</h3>
<p><strong>View → Show Schema Fields</strong> injects placeholder rows
for every field defined by the HL7 standard that is <em>absent</em> from
the message. Placeholders appear dim and italic - they make it easy to
see which fields you <em>could</em> add, but they cannot be navigated to
in the editor (they have no physical position yet).</p>

<h3>Resizing panels</h3>
<p>Drag the vertical splitter between tree and editor to resize; drag
the horizontal splitter above the Field Inspector to change its height.
Both widths are persisted across restarts.</p>

<h3>Full standard structure</h3>
<p>With <strong>View → Show Schema Fields</strong> enabled, the tree also
shows the segments the standard defines for the message type but that are
absent from the message — grayed rows at their standard position, annotated
with group, cardinality and choice status. Expand them to browse their full
field list down to composite components (e.g. OBX-16 → XCN components).
<strong>Right-click a grayed segment → Insert segment</strong> to add its
skeleton to the message at the standard position, with separators up to the
last required field. The skeleton uses the message's own separators,
and <kbd>Ctrl</kbd>+<kbd>Z</kbd> takes it out again.</p>
`,
};
