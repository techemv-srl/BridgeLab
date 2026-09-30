import type { ManualSection } from '../helpContent';
import { mockupValidation, mockupCommunication } from './mockups';

export const validationSection: ManualSection = {
	id: 'validation',
	heading: 'Validation',
	body: `
<p>Press <kbd>F6</kbd> or choose <strong>Tools → Validate</strong> to run
all validation rules on the active message. Results appear in the
bottom-docked Validation panel, grouped by severity.</p>

${mockupValidation}

<h3>Built-in rules</h3>
<ul>
	<li><strong>Structural:</strong> First segment must be MSH; segment
		codes must be 3 alphanumeric characters; a second MSH (two messages in
		one text) is reported (STRUCT-004). A code longer than three characters (<code>PIDX</code>)
		is reported, never read as <code>PID</code>; a batch file may open
		with FHS/BHS before the MSH.</li>
	<li><strong>MSH header:</strong> MSH-9 (message type), MSH-10 (control
		ID), MSH-12 (version) are required. An MSH-12 version BridgeLab does
		not know (a future v2.8, a typo) is validated against the closest
		catalogue, v2.5 when there is none, and an info note (MSH-005) names
		it.</li>
	<li><strong>Required fields:</strong> per-segment required fields drawn
		from the HL7 standard (e.g. PID-3 Patient Identifier List).</li>
	<li><strong>Length limits:</strong> warns when a field exceeds the
		published <code>max_length</code>, counted in characters, for each
		repetition of a repeating field.</li>
	<li><strong>Data types</strong> (warnings): numbers (SI, NM), dates
		(DT, <code>YYYY[MM[DD]]</code>), timestamps (TS/DTM,
		<code>YYYY[MM[DD[HH[MM[SS[.S]]]]]][+/-ZZZZ]</code>) and times (TM)
		are checked for format and against the calendar and the clock, so
		<code>19801399</code> or <code>2024-01-01</code> is reported. OBX-5
		is checked as the type OBX-2 declares (NM, DT, TS…). Other types
		are not checked for format.</li>
</ul>

<h3>Filtering and navigation</h3>
<p>Click the Error / Warning / Info badges to filter. Click any issue row
to select the offending segment or field in the editor and in the tree.
Each row names the segment by its number, as the tree does
(<code>OBX (4)</code>), so identical issues on two segments can be told
apart. When you edit the message after a check, the report is run again
with the next background parse; with auto-parse off, the panel says the
report is out of date until you press <kbd>F6</kbd>.</p>

<h3>Custom rules from plugin packs</h3>
<p>Drop a JSON file under <code>&lt;config&gt;/BridgeLab/plugins/validation/</code>
to add your own checks without recompiling. See <em>Plugins</em> below.</p>

<h3>Batch validation (Pro)</h3>
<p><strong>Tools → Batch validation…</strong> validates a whole folder
(or a hand-picked set) of <code>.hl7</code>/<code>.txt</code>/<code>.dat</code>
files in one pass: one row per file with message type, version, segment
count and error/warning totals. Filter to failures only, click a row to
open that file in the editor, and export the whole table as CSV for the
change-review ticket. Files are processed in memory — nothing is added
to your tabs.</p>

<h3>Test-message generator</h3>
<p><strong>Tools → Generate test messages…</strong> creates syntactically
valid ADT/ORU/ORM messages with plausible <em>synthetic</em> patient
data — names, birth dates, MRNs, addresses, and lab panels with
reference ranges (a realistic share of results is deliberately abnormal
and flagged). No real PHI is ever used. Provide a <strong>seed</strong>
to make a set reproducible, then open the messages in tabs or save them
to a folder as numbered <code>.hl7</code> files — instant regression
fixtures for the batch validator above. A file of the same name already
in the folder is left as it is and listed as not saved.</p>

<h3>CLI validation</h3>
<p>The <code>bridgelab-cli</code> companion runs the same validators —
HL7 v2 and FHIR, with the built-in R4 core, installed packages and plugin
packs — headless, for CI pipelines and batch screening. It reads no
licence and needs none. Besides <code>validate</code>, <code>info</code>,
<code>anonymize</code>, <code>to-json</code> and <code>batch</code> it
runs the test case packs exported from the Test Case Library
(<code>test</code>, with a JUnit report for CI), evaluates FHIRPath
(<code>fhirpath</code>), sends a message over MLLP and exits non-zero
unless the ACK is AA or CA (<code>send</code>; a file holding several
messages is sent one frame per message), and exports the XSD of the
Community set of messages (<code>xsd</code>; the full catalogue is in
Pro). Every command that reads one message accepts <code>-</code> for
standard input. A binary for each platform ships with every release.
Put patterns and arguments with <code>^</code> in double quotes
(<code>"*.hl7"</code>, <code>"ORU^R01"</code>): Windows <code>cmd.exe</code>
drops an unquoted <code>^</code> and keeps single quotes.</p>
<p>The CLI's <code>batch</code> walks subfolders, takes <code>.hl7</code>
files unless <code>--extension</code> says otherwise, and checks FHIR as
well as HL7 v2, so its counts can differ from <strong>Tools → Batch
validation…</strong> on the same folder (one folder, <code>.hl7</code>,
<code>.txt</code> and <code>.dat</code>, HL7 v2 only):
<code>--extension hl7,txt,dat</code> scans the same files.</p>
<pre><code>bridgelab-cli validate message.hl7 bundle.json
bridgelab-cli validate "*.hl7" --format junit &gt; report.xml
bridgelab-cli batch ./inbox --json
bridgelab-cli test regression.bltests.json --format junit &gt; tests.xml
bridgelab-cli fhirpath "Patient.name.family" patient.json
cat message.hl7 | bridgelab-cli send - --host 10.0.0.5 --port 2575</code></pre>
`,
};

export const communicationSection: ManualSection = {
	id: 'communication',
	heading: 'Communication (MLLP / HTTP / SOAP)',
	body: `
<p>Open the bottom Communication panel with <kbd>Ctrl</kbd>+<kbd>K</kbd>
or <strong>Tools → Communication Panel</strong>. Four tabs: MLLP, HTTP,
SOAP and History.</p>

${mockupCommunication}

<h3>MLLP client</h3>
<ol>
	<li>Enter <em>Host</em> + <em>Port</em> (e.g. <code>localhost:2575</code>).</li>
	<li>The current message in the active tab is used automatically.</li>
	<li>Click <strong>Send</strong>. Framing (<code>0x0B</code> ... <code>0x1C 0x0D</code>),
		transport and ACK wait are handled by the Rust backend.</li>
	<li>The ACK appears in the result area with round-trip time.
		<em>Accept</em> (AA), <em>Error</em> (AE) and <em>Reject</em>
		(AR) are all displayed with the original <code>MSA|AA|{control-id}</code>.</li>
</ol>

<h3>ACK generator</h3>
<p>The <strong>ACK generator</strong> row in the MLLP tab builds an
acknowledgment for the message currently in the editor: pick the code
(AA accept, AE error, AR reject) and click <strong>Generate ACK</strong>.
The ACK mirrors the message, as a receiver's should: the same field
separator and encoding characters, sender and receiver swapped (MSH-3/4
and MSH-5/6), <code>ACK^&lt;trigger&gt;^ACK</code> in MSH-9, the same
processing ID (MSH-11), version (MSH-12) and character set (MSH-18), and
the Message Control ID (MSH-10, read with the separator declared in
MSH-1) in MSA-2. Every ACK gets its own MSH-10. The listener's auto-ACK
is built the same way. The ACK opens in a new tab — ready to send back
or to keep as a fixture. If the current message has no MSH-10 the
generator refuses instead of producing an uncorrelatable ACK.</p>

<h3>MLLP listener (Pro)</h3>
<p>Click <strong>Start listening</strong> to run a server on the selected
port. Incoming messages open in a new tab (toggleable, see below) and an
auto-ACK is sent back with the configured code (AA/AE/AR). Use this to
quickly validate what your upstream system is emitting.</p>
<p>The listener binds to <code>127.0.0.1</code> by default, so only
programs on this computer can reach it. To receive a feed from another
machine, set <strong>Listen on</strong> to <code>0.0.0.0</code> (all
interfaces) or to the address of one network card, and allow the port
in the firewall only for the systems you expect.</p>
<p><strong>Stop</strong> also closes the connections that are still
open: after it, nothing more is received or acknowledged.</p>

<h3>Listener console</h3>
<p>While the listener runs, every received message appears as a row in
the console: local time, peer address, payload size, the ACK code that
was actually written back (green <code>AA</code>, red
<code>AE</code>/<code>AR</code>, — when auto-ACK is off), the character
encoding used, and the first line of the message. <strong>Click a row to
re-open that message in a tab.</strong> Listener errors appear inline as
red rows.</p>
<p>The chips in the console header filter the rows by outcome —
<em>AA</em>, <em>AE</em>, <em>AR</em>, <em>No ACK</em> (received with
auto-ACK off), <em>Errors</em> — and each carries a live count, so
"AE&nbsp;12" out of 300 stands out before you scroll. The same chips
sit on the History tab.</p>
<p>The <em>"Open received messages in a new tab"</em> toggle (on by
default) can be disabled during high-volume tests: messages then land
only in the console and you cherry-pick the ones you need.</p>
<p class="note">Full message contents kept for click-to-open are capped
at a 32&nbsp;MB rolling budget. In long unattended sessions the oldest
rows lose their full content (they appear dimmed) — the metadata row
stays, and nothing was "lost": only the click-to-open copy was released
to keep memory bounded.</p>

<h3>Character encoding</h3>
<p>Both the sender and the listener have an <strong>Encoding</strong>
selector covering the charsets seen in real deployments:
<code>UTF-8</code>, <code>ISO-8859-1</code> (Latin-1),
<code>ISO-8859-2</code>, <code>ISO-8859-15</code>,
<code>windows-1252</code>, <code>windows-1250</code>,
<code>windows-1251</code> and <code>ASCII</code>. The default is
<strong>Auto</strong>. On the listener it decodes each message in the
charset its MSH-18 declares, otherwise as UTF-8 with an automatic
Latin-1 fallback, which accepts most legacy traffic even when MSH-18 is
empty; the console shows the charset chosen, and the ACK is re-encoded
in it so the peer never sees mojibake. On the sender it uses the charset
MSH-18 declares, otherwise the one the tab's file was read in, otherwise
UTF-8, and the result names the charset sent. A message with characters
the chosen charset cannot represent is not sent: the error names them,
so a name never arrives with <code>?</code> in it. Send and Receive
encodings are independent.</p>
<p>Segments go on the wire ending with CR, whatever the editor's line
endings: LF and CR LF are converted, as the CLI does, and so is an HL7 v2
body sent over HTTP or SOAP.</p>
<p><strong>Files</strong> follow the same rules. A file that is not UTF-8
opens in the charset its MSH-18 declares (<code>8859/1</code>,
<code>8859/2</code>, <code>8859/15</code>…, and <code>BIG-5</code>,
<code>GB 18030-2000</code> and <code>KS X 1001</code>), or as
Windows-1252 when MSH-18 is empty, so accented names read correctly
instead of being refused. A file whose MSH-18 names a charset BridgeLab
cannot decode (<code>CNS 11643-1992</code>, the ISO 2022 Japanese sets)
opens with a warning, since its text and even its fields may be read
wrongly; batch validation and batch anonymisation refuse it. Save writes
the file back in the charset it was read in, and so do the CLI's
<code>anonymize</code> and batch anonymisation; <code>send</code> without
<code>--encoding</code> transmits in it too (a UTF-16 file goes out in
the charset MSH-18 declares, else UTF-8). When saving, a character the
charset cannot hold is written as <code>?</code>; the CLI's
<code>send</code>, like the app, refuses such a message instead.</p>

<h3>HTTP</h3>
<p>GET requests are available in the Community tier. POST/PUT/DELETE/PATCH
and authentication of any kind require Pro: an Authorization or
Proxy-Authorization header, a cookie, any header whose name suggests a
key, token, secret, signature or session (such as <code>X-API-Key</code>
or <code>Ocp-Apim-Subscription-Key</code>), a user:password in the URL,
or a key or token in the query string (<code>access_token</code>,
<code>api_key</code>, <code>key</code>, <code>token</code>,
<code>sig</code>, <code>client_secret</code>…). FHIR search parameters
such as <code>code</code> are not credentials. Redirects are
followed in every edition, but only on the same server (or from http to
https on it): a redirect to another server is shown as the 3xx it is and
never followed, so a message is not sent somewhere you did not choose.
When a redirect was followed, the result names the URL that answered.
Responses larger than 50 MB are cut and say so, and a response is read in
the charset its Content-Type declares. POST, PUT and PATCH send the
current tab's message when the Body box is empty; GET and DELETE send a
body only when you type one.</p>

<h3>SOAP client (Enterprise)</h3>
<p>The SOAP tab sends the current message (or a custom body) to SOAP
1.1/1.2 endpoints — IHE-style middlewares, regional gateways and legacy
hospital web services. Set the endpoint URL, the SOAP version and the
<em>SOAPAction</em>; BridgeLab builds the envelope, posts it with the
correct content type (<code>text/xml</code> plus the SOAPAction header
for 1.1, <code>application/soap+xml</code> with the action parameter
for 1.2) and shows the HTTP status, round-trip time, the inner Body XML
and any SOAP Fault, decoded for both versions.</p>
<p>A raw HL7 v2 message is XML-escaped (its CR segment ends as
<code>&amp;#13;</code>, so XML parsing does not turn them into LF) and
wrapped in a <code>&lt;payload&gt;</code> element automatically; content
that is already XML is inserted as-is. Advanced settings add
<strong>WS-Security UsernameToken</strong> credentials,
<strong>WS-Addressing</strong> headers (To / Action / MessageID) and a
<strong>custom envelope template</strong> in which the literal
<code>{payload}</code> placeholder is replaced with the message — use
it when the target service expects a specific wrapper. With a template,
the WS-Security and WS-Addressing headers go into the template's SOAP
Header (one is added before its Body when it has none); a template with
no SOAP Envelope and Body cannot carry them, and the send is refused
rather than made without them. WSDL import is planned as a follow-up.</p>

<h3>History</h3>
<p>Every send, and every message the listener receives, is logged:
target (host and port, or the URL with any password or key replaced by
<code>***</code>), size, response code and round-trip time. The last
100 entries are persisted between restarts and older ones are deleted;
click any row to see the full request and response — the message and
its ACK, or the HTTP body and the reply (for SOAP the payload, never the
envelope with its password). A request or response over 256 KB is kept
cut. An MLLP send also records the <strong>ACK code</strong> the
receiver answered with (MSA-1), shown as a green or red badge on the
row: a send that reached the peer and got an <code>AE</code> back is
"OK" at the transport level and a rejection at the application level,
and the badge tells the two apart. Filter chips over the list —
<em>AA</em>, <em>AE</em>, <em>AR</em>, <em>No ACK</em>, <em>Failed</em>
— carry counts and narrow the list to one outcome; the commit-mode codes
CA, CE and CR count under AA, AE and AR. <em>Failed</em> is a request
that got no reply: a server that answered with an error (HTTP 404,
500…) is listed with its status code.</p>

<h3>Connection profiles</h3>
<p>Save frequently-used endpoints as named profiles from the
<strong>Profile</strong> row: type a name and click <em>Save</em>. MLLP
profiles store host, port, timeout and auto-ACK; HTTP profiles store
URL, headers and timeout; SOAP profiles store endpoint, SOAPAction and
timeout. Selecting a profile applies it to the form;
saving with an existing name overwrites it; <em>Delete</em> removes the
selected one. Profiles are stored in the local database and survive
restarts.</p>
<p class="note">HTTP profiles save the Headers box as typed, an
<code>Authorization</code> header included, in the local database. To
keep a password or token out of it, enter it under
<em>Authentication</em> in the advanced HTTP settings, which is never
saved.</p>
`,
};

export const anonymizationSection: ManualSection = {
	id: 'anonymization',
	heading: 'Anonymization &amp; Export',
	body: `
<p><strong>Tools → Anonymize</strong> detects PHI fields across the
patient-identifying segments and masks them by sensitivity level: 89
built-in fields in PID, PV1 (visit numbers), MRG, NK1, GT1, IN1 and IN2,
plus NTE comments and free-text (TX/FT) OBX results. They cover the
names, dates of birth and death, addresses, phone numbers, SSNs and
other identifiers of the patient, next of kin, guarantor and insured.
An identifier keeps its assigning authority and type (<code>HOSP</code>,
<code>MR</code>). Other segments are not masked: check free text
elsewhere (OBR, ORC, Z-segments) before sharing, or add those fields with
a plugin.</p>

<table>
	<tr><th>Level</th><th>Example</th><th>Strategy</th></tr>
	<tr><td><strong>High</strong></td><td>Patient name, date of birth,
		address, home phone, SSN, MRN, visit number</td>
		<td>Text becomes <code>REDACTED</code>; numeric becomes zeros of
		the same length (preserving field width for downstream
		parsers); a date becomes 1900-01-01 at the same
		precision.</td></tr>
	<tr><td><strong>Medium</strong></td><td>Mother's maiden name, alias,
		business phone, next of kin, guarantor and insured contacts</td>
		<td>First character kept, rest replaced with
		<code>***</code>.</td></tr>
	<tr><td><strong>Low</strong></td><td>No built-in field; available to
		plugin rules</td>
		<td>First 3 characters kept, followed by <code>...</code>;
		a value of 3 characters or fewer is kept whole.</td></tr>
</table>

<p>The dialog lists every detected PHI field before you run the masker,
so you can review what will change. The output:</p>
<ul>
	<li><strong>Opens in a new tab</strong> - the original message stays
		untouched in its own tab.</li>
	<li><strong>Can be copied to the clipboard</strong> directly.</li>
	<li><strong>Preserves structure</strong> - segment order, pipe count
		and component separators are unchanged, so the result still
		parses as valid HL7.</li>
</ul>

<h3>Custom PHI fields via plugins</h3>
<p>Deployments with regional or vendor-specific identifiers (EU national
ID, internal Z-segment fields) can extend the catalogue by dropping a
JSON file under
<code>&lt;config&gt;/BridgeLab/plugins/anonymization/</code>.</p>

<h3>Batch anonymization (Pro)</h3>
<p><strong>Tools → Batch anonymize…</strong> masks a whole folder in one
pass: pick source files or a folder, pick an output folder, run. Every
message goes through the same pipeline as the interactive dialog
(built-in PHI catalogue + active plugin rules) and is written as a copy
into the output folder — <strong>originals are never touched</strong>:
the tool refuses to overwrite any selected source file, and same-named
inputs from different folders get numeric suffixes instead of clobbering
each other. A file already in the output folder is never replaced (and a
link there is never followed): its row says so and the file stays as it
is, so pick an empty folder. One row per file reports the masked-PHI count or the error;
the same 5000-file / 10&nbsp;MB caps as batch validation apply.</p>

<h3>Export</h3>
<p>Pro users can export the structured message as JSON or CSV via
<strong>Tools → Export JSON / CSV</strong>; a save dialog asks where
to write the file. Useful for loading HL7 data into analytics tools
(Power BI, Excel, pandas).</p>

<div class="warn">Anonymization writes its result to a new tab; the
original tab is left as it is. Always keep your original source file as the canonical record - the
anonymized copy is for sharing, not for long-term storage.</div>
`,
};

export const testCasesSection: ManualSection = {
	id: 'testcases',
	heading: 'Test Case Library',
	body: `
<p>The Test Case Library (<kbd>Ctrl</kbd>+<kbd>L</kbd>) stores reusable
messages with a name, category, tags and description. Use
<strong>Save Current Message</strong> to capture the active tab, or
create cases from scratch. Cases persist in the local database and can
be searched by any of their fields.</p>

<p class="note">The Community tier keeps up to 10 saved test cases —
existing cases always stay visible, editable and runnable; only new
saves beyond the cap ask for an upgrade.</p>

<h3>Expected outcomes</h3>
<p>Each case can declare an <strong>expected message type</strong>
(only the components you give are compared: <code>ADT</code> matches any
ADT event, <code>ADT^A01</code> matches <code>ADT^A01</code> and
<code>ADT^A01^ADT_A01</code> but not <code>ADT^A04</code>)
and an <strong>expected validation result</strong> (valid / invalid).
That turns a snippet into a test.</p>

<h3>Running checks</h3>
<p><strong>Run check</strong> parses and validates a single case for
real — HL7 v2 or FHIR, detected automatically — and compares the
outcome against its expectations. <strong>Run all</strong> does the
same for every case matching the current search, with a pass/fail
badge per row and a passed/total summary in the toolbar. After an
interface change, one click tells you which of your reference messages
broke. Editing a case clears its stored result until the next run.</p>

<h3>Sharing test cases</h3>
<p><strong>Export…</strong> writes the test cases in view — all of
them, or only those matching the search — to a
<code>.bltests.json</code> pack you can send to a colleague or commit to
a Git repository. Before saving, BridgeLab checks the HL7 v2 messages
for personal data and lists the cases and fields it found; FHIR
resources are listed as not checked field by field. With Pro you can
tick <em>Mask personal data</em> to anonymize the HL7 v2 messages in
the exported file only — your library is not changed.</p>
<p><strong>Import…</strong> opens a pack and shows, before anything is
written, what each case is: <em>New</em>, <em>Already in the
library</em> (skipped) or <em>Differs</em> from one you have, where you
choose to keep yours, replace it or keep both. Imported cases keep
their id, so importing the same pack again only brings in what changed.
In Community an import may not take the library past 10 test cases;
nothing is written if it would.</p>

<h3>Session restore</h3>
<p>BridgeLab saves your open tabs (including unsaved edits) and reopens
them on the next launch, Notepad++-style. Control this under
<strong>Settings → Performance</strong>, in the <em>Session</em> group: toggle <em>Restore open tabs on
startup</em>, or use <em>Clear saved session</em> to wipe the stored
tab set (this also turns restore off, so the next launch starts on the
welcome screen). Turning restore off also deletes the saved tabs, which
can hold patient data.</p>

<h3>Files changed by other programs</h3>
<p>BridgeLab notices when another program changes or deletes a file you
have open: when you come back to the window, or open the file again, it
offers to load the new version (and tells you if the file is gone).
<strong>Save</strong> asks before overwriting a file that changed on disk
since you opened or last saved it, or before creating a deleted one
again. At launch, a restored tab with no unsaved edits shows its file as
it is on disk now. A restored tab with unsaved edits asks before its first save, since the file may have changed while BridgeLab was closed.</p>
`,
};
