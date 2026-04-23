import type { ManualSection } from '../helpContent';
import { mockupAppShell, mockupContextMenu } from './mockups';

export const getStarted: ManualSection = {
	id: 'getting-started',
	heading: 'Getting Started',
	body: `
<p>BridgeLab is a modern message editor for HL7 v2.x and FHIR, designed
for healthcare integration engineers. It is built on a Rust backend for
fast parsing (handles 10 MB messages in under 2 seconds) and a Svelte 5
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
	<li><strong>File → Open</strong> (<kbd>Ctrl</kbd>+<kbd>O</kbd>) - native
		file picker for <code>.hl7</code>, <code>.txt</code>, <code>.msg</code>,
		<code>.json</code>, <code>.xml</code>.</li>
	<li><strong>Drag &amp; drop</strong> - drop a file onto the editor area.</li>
	<li><strong>Paste</strong> - click in the editor and paste
		(<kbd>Ctrl</kbd>+<kbd>V</kbd>). Auto-parse runs 500 ms after the
		last keystroke.</li>
	<li><strong>File → New from Template</strong> (<kbd>Ctrl</kbd>+<kbd>N</kbd>) -
		pre-filled ADT, ORM, ORU, SIU and more. Fields like MSH-7 and
		MSH-10 are filled with the current timestamp and a fresh GUID.</li>
</ul>

<div class="note">On first launch you get a <strong>7-day Pro trial</strong>
with every feature enabled. After expiry, BridgeLab continues to work
with the Community feature set - you never lose your messages.</div>
`,
};

export const editorSection: ManualSection = {
	id: 'editor',
	heading: 'Editor',
	body: `
<p>The editor area is a <strong>Monaco</strong> instance with an HL7-specific
grammar. Segment codes are coloured purple, field separators grey, and
ED/base64 payloads are automatically truncated to keep the editor fast
on large messages.</p>

<h3>Auto-complete and hover</h3>
<p>Start typing <code>P</code> on a new line - Monaco suggests
<code>PID</code>, <code>PV1</code>, <code>PV2</code>, etc. Once you enter
a segment, pipe autocomplete proposes field values (gender codes, ACK
codes, patient class...). Hovering over any field displays its name,
data type, and required flag drawn from the HL7 standard.</p>

<h3>Truncation of large fields</h3>
<p>Fields exceeding the truncation threshold (default 100 bytes, tunable
in <strong>Settings → Parser</strong>) appear as
<code>{...N bytes}</code>. The full content is never lost - expand it
on demand via the right-click menu or the <em>Field Inspector</em>.</p>

<h3>Right-click context menu</h3>
${mockupContextMenu}
<p>The menu groups actions into three sections:</p>
<ul>
	<li><strong>Navigation:</strong> Show Segment in Tree
		(<kbd>Alt</kbd>+<kbd>T</kbd>) - opens the tree and highlights the
		exact field under the cursor; Expand / Collapse for truncated
		values.</li>
	<li><strong>Clipboard:</strong> Copy Segment
		(<kbd>Alt</kbd>+<kbd>C</kbd>), Copy Full Message (with expanded
		fields), Copy Truncated Message (safe for email).</li>
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
<strong>components</strong>. Toggle visibility with
<kbd>Ctrl</kbd>+<kbd>B</kbd> or <strong>View → Message Tree</strong>.</p>

<h3>Navigating between tree and editor</h3>
<ul>
	<li><strong>Editor → Tree:</strong> Right-click a field in Monaco and
		choose <em>Show in Tree</em>. The tree expands the segment,
		selects the exact field (down to the component level) and scrolls
		it into view.</li>
	<li><strong>Tree → Editor:</strong> Right-click a tree node and choose
		<em>Show in Editor</em>. Monaco jumps to the line, places the
		cursor at the right column and selects the field range.</li>
</ul>

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
		truncated fields</li>
</ul>
<p>Unknown segments (Z-segments or custom codes not in the standard)
display <em>Not in HL7 standard</em> but remain fully editable.</p>

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
`,
};
