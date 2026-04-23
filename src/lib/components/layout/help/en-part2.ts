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
		codes must be 3 alphanumeric characters; no duplicate MSH.</li>
	<li><strong>MSH header:</strong> MSH-9 (message type), MSH-10 (control
		ID), MSH-12 (version) are required.</li>
	<li><strong>Required fields:</strong> per-segment required fields drawn
		from the HL7 standard (e.g. PID-3 Patient Identifier List).</li>
	<li><strong>Length limits:</strong> warns when a field exceeds the
		published <code>max_length</code>.</li>
	<li><strong>Data types:</strong> numeric fields (SI, NM) checked for
		non-numeric characters; timestamp formats (TS) checked for length
		and digit-only composition.</li>
</ul>

<h3>Filtering and navigation</h3>
<p>Click the Error / Warning / Info badges to filter. Click any issue row
to jump to the offending segment in the editor.</p>

<h3>Custom rules from plugin packs</h3>
<p>Drop a JSON file under <code>&lt;config&gt;/BridgeLab/plugins/validation/</code>
to add your own checks without recompiling. See <em>Plugins</em> below.</p>

<h3>CLI validation</h3>
<p>The <code>bridgelab-cli</code> companion offers the same validator for
headless usage (CI pipelines, batch screening):</p>
<pre><code>bridgelab-cli validate message.hl7
bridgelab-cli validate '*.hl7' --format junit &gt; report.xml
bridgelab-cli batch ./inbox --json</code></pre>
`,
};

export const communicationSection: ManualSection = {
	id: 'communication',
	heading: 'Communication (MLLP / HTTP)',
	body: `
<p>Open the bottom Communication panel with <kbd>Ctrl</kbd>+<kbd>K</kbd>
or <strong>Tools → Communication Panel</strong>. Three tabs: MLLP, HTTP
and History.</p>

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

<h3>MLLP listener (Pro)</h3>
<p>Click <strong>Listen</strong> to start a server on the selected port.
Incoming messages open in a new tab and an auto-ACK is sent back (you
can disable auto-ACK in the advanced options). Use this to quickly
validate what your upstream system is emitting.</p>

<h3>HTTP</h3>
<p>GET requests are available in the Community tier. POST/PUT/DELETE,
custom authentication headers (Basic, Bearer), and follow-redirects
require Pro. The body defaults to the current tab's message but can be
overridden.</p>

<h3>History</h3>
<p>Every send and receive is logged (host, port, size, response code,
round-trip time). The last 100 entries are persisted between restarts;
click any row to see the full request and response.</p>

<h3>Connection profiles</h3>
<p>Save frequently-used endpoints (Host + Port + Timeout + auto-ACK
preferences) as named profiles. They appear in the profile dropdown
next to the Send button.</p>
`,
};

export const anonymizationSection: ManualSection = {
	id: 'anonymization',
	heading: 'Anonymization &amp; Export',
	body: `
<p><strong>Tools → Anonymize</strong> detects PHI fields across the
common patient-identifying segments (PID, NK1, IN1, GT1) and masks them
by sensitivity level.</p>

<table>
	<tr><th>Level</th><th>Example</th><th>Strategy</th></tr>
	<tr><td><strong>High</strong></td><td>Patient name, SSN, MRN</td>
		<td>Text becomes <code>REDACTED</code>; numeric becomes zeros of
		the same length (preserving field width for downstream
		parsers).</td></tr>
	<tr><td><strong>Medium</strong></td><td>Mother's maiden name, phone</td>
		<td>First character kept, rest replaced with
		<code>***</code>.</td></tr>
	<tr><td><strong>Low</strong></td><td>Alias, low-risk identifiers</td>
		<td>First 3 characters kept, rest replaced with
		<code>...</code>.</td></tr>
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

<h3>Export</h3>
<p>Pro users can export the structured message as JSON or CSV via
<strong>Tools → Export JSON / CSV</strong>. Useful for loading HL7 data
into analytics tools (Power BI, Excel, pandas).</p>

<div class="warn">Anonymization replaces values <em>in the editor</em>.
Always keep your original source file as the canonical record - the
anonymized copy is for sharing, not for long-term storage.</div>
`,
};
