import type { ManualSection } from '../helpContent';

export const schemaExportSection: ManualSection = {
	id: 'schema-export',
	heading: 'Schema Export (XSD)',
	body: `
<p>Need an XSD to describe an HL7 v2 message for an XML-based pipeline,
contract-first integration, or just to load into a third-party tool?
Open <strong>Tools → Export message schema as XSD…</strong> — pick an
HL7 version and a message type, preview the generated schema and save
it with one click.</p>

<h3>What you get</h3>
<p>A self-contained XSD using the standard HL7 v2.xml encoding
convention:</p>
<ul>
	<li>One root element per message (e.g. <code>ADT_A01</code>) with an
		inline complex type listing the segments and segment groups in
		order.</li>
	<li>Each segment declared as a top-level <code>xsd:complexType</code>
		(<code>MSH</code>, <code>PID</code>, <code>OBX</code>, …) with
		every field typed by the HL7 data-type reference
		(<code>XPN</code>, <code>CX</code>, <code>HD</code>, …).</li>
	<li>Composite data types expanded into their components, primitive
		data types (<code>ST</code>, <code>ID</code>, <code>NM</code>, …)
		as <code>xsd:simpleType</code> restrictions on
		<code>xsd:string</code>.</li>
	<li>Cardinality preserved: <code>minOccurs="0"</code> for optional
		fields, <code>maxOccurs="unbounded"</code> for repeating ones.</li>
	<li>Groups like <code>ORM_O01.ORDER_DETAIL</code> rendered with the
		<code>MESSAGE.GROUP</code> naming convention; HL7-defined choice
		blocks (<code>OBR | RQD | RQ1 | RXO | ODS | ODT</code>) emitted
		as <code>xsd:choice</code>.</li>
</ul>

<h3>Actions</h3>
<ul>
	<li><strong>Copy</strong> — copies the XSD to the clipboard,
		handy when you want to paste it into an editor or chat.</li>
	<li><strong>Save as…</strong> — opens the OS file dialog with
		<code>{MESSAGE}.xsd</code> as the default name.</li>
</ul>

<h3>Coverage and tiers</h3>
<p>The free tier exports four high-use message types in HL7 v2.5 so the
typical MLLP-debugging workflow is fully covered:</p>
<ul>
	<li><strong>ADT^A01</strong> — Admit / Visit Notification</li>
	<li><strong>ADT^A40</strong> — Merge Patient (Patient Identifier
		List)</li>
	<li><strong>ORM^O01</strong> — Order Message</li>
	<li><strong>ORU^R01</strong> — Unsolicited Observation Result</li>
</ul>
<p>Any other message type, or any other HL7 version, is tagged
<strong>(PRO)</strong> in the dropdown and requires a Professional
license (or an active trial). If you try to export a gated entry
BridgeLab shows an upgrade prompt pointing to
<strong>Help → Activation</strong>.</p>

<h3>Licensing note</h3>
<p>BridgeLab does not redistribute any HL7-copyrighted XSD file.
Schema metadata is rebuilt from public HL7 v2 specifications; every
generated file carries a header acknowledging HL7® as the source
standard and flagging the output as a derivative work for
interoperability purposes.</p>

<div class="info">Ideal target: Astraia and similar integration
applications that accept hand-authored XSD definitions for message
types the engine doesn't natively know. Export once, drop into the
engine, move on.</div>
`,
};

export const fhirSection: ManualSection = {
	id: 'fhir',
	heading: 'FHIR Support',
	body: `
<p>BridgeLab auto-detects FHIR resources when you paste or open a file
whose first non-whitespace character is <code>{</code> and that contains
<code>"resourceType"</code>. The tree switches to a FHIR-specific view
showing the resource hierarchy as JSON paths.</p>

<h3>Supported formats</h3>
<ul>
	<li><strong>JSON</strong> - Patient, Observation, Bundle, DiagnosticReport,
		MedicationRequest and any other FHIR R4/R5 resource.</li>
	<li><strong>XML</strong> - the same resources in XML encoding
		(<code>&lt;Patient xmlns="http://hl7.org/fhir"&gt;</code>).</li>
</ul>

<h3>Bundle Visualizer (Pro)</h3>
<p><strong>Tools → FHIR Bundle Visualizer</strong> opens a three-pane
view when the active message is a Bundle:</p>
<ul>
	<li><strong>Left pane:</strong> list of entries with resource type,
		display name (e.g. Patient name, Observation code), and an
		inbound-reference count.</li>
	<li><strong>Center pane:</strong> outgoing references from the selected
		entry - every <code>reference</code> field becomes a clickable
		link that navigates to the target entry.</li>
	<li><strong>Right pane:</strong> the raw JSON of the selected
		resource, with syntax highlighting.</li>
</ul>
<p><strong>Dangling references</strong> (pointing to entries not present
in the Bundle) are flagged with a red badge.</p>

<h3>FHIRPath Evaluator (Pro)</h3>
<p><kbd>Ctrl</kbd>+<kbd>P</kbd> or <strong>Tools → FHIRPath Evaluator</strong>
opens an interactive console where you type FHIRPath expressions
against the current resource. Supported operators include:</p>
<ul>
	<li><strong>Navigation:</strong> <code>Patient.name.family</code>,
		<code>Bundle.entry.resource</code></li>
	<li><strong>Indexing:</strong> <code>Patient.name[0].given</code></li>
	<li><strong>Filters:</strong>
		<code>Bundle.entry.where(resource.resourceType = 'Patient')</code></li>
	<li><strong>Aggregates:</strong> <code>count()</code>,
		<code>first()</code>, <code>last()</code>,
		<code>distinct()</code></li>
	<li><strong>Projection:</strong>
		<code>Bundle.entry.select(resource.id)</code></li>
</ul>
<p>Recent expressions are kept in a history dropdown for quick replay.</p>

<h3>FHIR validation</h3>
<p>F6 also works for FHIR resources. Errors highlight missing required
fields (e.g. <code>Patient.identifier</code>), invalid data types
(gender not in the value set), and structural issues.</p>
`,
};

export const pluginsSection: ManualSection = {
	id: 'plugins',
	heading: 'Plugin Packs',
	body: `
<p>Plugin packs let you extend BridgeLab's validator and anonymizer
<strong>without writing code</strong> and without allowing any code
execution. Each pack is a JSON file dropped in a user folder.</p>

<h3>Where plugins live</h3>
<p>Click <strong>Settings → Plugins → Open plugins folder</strong> to
reveal the directory in your file manager. The layout is:</p>
<pre><code>&lt;config&gt;/BridgeLab/plugins/
├── validation/
│   ├── hospital-adt-rules.json
│   └── z-segment-checks.json
└── anonymization/
    └── eu-national-id.json</code></pre>

<p>On Windows the root is <code>%APPDATA%\\BridgeLab\\plugins</code>, on
macOS <code>~/Library/Application Support/BridgeLab/plugins</code>, on
Linux <code>~/.config/BridgeLab/plugins</code>.</p>

<h3>Validation rule pack</h3>
<pre><code>{
  "id": "acme-adt-01",
  "name": "ACME ADT specific rules",
  "description": "Hospital-specific required fields",
  "version": "1.0",
  "enabled": true,
  "validation_rules": [
    {
      "rule_id": "ACME-PID-001",
      "severity": "error",
      "segment": "PID",
      "field": 3,
      "check": { "type": "not_empty" },
      "message": "PID-3 (Patient ID) is required"
    }
  ]
}</code></pre>

<h3>Supported check types</h3>
<table>
	<tr><th>Check</th><th>Parameters</th><th>Example use</th></tr>
	<tr><td><code>not_empty</code></td><td>—</td>
		<td>Field must be populated.</td></tr>
	<tr><td><code>regex</code></td><td><code>pattern</code></td>
		<td>Family name must start with uppercase.</td></tr>
	<tr><td><code>one_of</code></td><td><code>values[]</code></td>
		<td>Patient class must be I, O, E.</td></tr>
	<tr><td><code>max_length</code></td><td><code>max</code></td>
		<td>MRN ≤ 16 characters.</td></tr>
	<tr><td><code>min_length</code></td><td><code>min</code></td>
		<td>SSN ≥ 9 digits.</td></tr>
	<tr><td><code>contains</code></td><td><code>value</code></td>
		<td>Visit number must contain a dash.</td></tr>
</table>
<p>Add <code>"component": 1</code> to narrow a rule to a specific
component (e.g. family name inside PID-5.1).</p>

<h3>Anonymization rule pack</h3>
<pre><code>{
  "id": "eu-extra-phi",
  "name": "EU extra PHI fields",
  "enabled": true,
  "phi_rules": [
    { "segment": "PID", "field": 25, "sensitivity": "high",
      "name": "EU National ID" }
  ]
}</code></pre>

<h3>Managing packs</h3>
<p><strong>Settings → Plugins</strong> lists every pack with its author,
version, rule count, and path. Toggle individual packs on/off (the
choice is persisted), click <em>Reload</em> after editing a file, or
<em>Open plugins folder</em> to edit in your favourite IDE.</p>

<div class="note">Files that fail to parse appear with a red error
banner but do not break the registry - the rest of your packs keep
working.</div>
`,
};

export const licensingSection: ManualSection = {
	id: 'licensing',
	heading: 'Licensing',
	body: `
<p>BridgeLab ships with three tiers. The feature split is designed so
that Community users can do real day-to-day HL7 work forever, while Pro
and Enterprise unlock features needed by integration teams and
hospitals.</p>

<table>
	<tr><th>Feature</th><th>Community</th><th>Pro</th><th>Enterprise</th></tr>
	<tr><td>HL7 v2.x editor, parser, validation</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>FHIR parsing + tree view</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>MLLP send, HTTP GET</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>PHI detection (view only)</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Plugin packs (basic)</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>MLLP listener</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>HTTP POST/PUT/DELETE + auth</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Anonymization masking</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Export JSON/CSV</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>FHIRPath Evaluator + Bundle Visualizer</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Unlimited plugins &amp; test cases</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>SOAP + priority support</td>
		<td>—</td><td>—</td><td>✓</td></tr>
</table>

<h3>Trial</h3>
<p>First launch starts a <strong>7-day Pro trial</strong> with every Pro
feature enabled. The trial banner (yellow) is dismissible; when 3 days
remain it turns red and stays visible as a reminder.</p>

<p>When the trial expires BridgeLab <strong>does not stop working</strong>
- it falls back to the Community tier and the banner prompts you to
upgrade. Your messages, settings, plugins and test cases remain intact.</p>

<h3>Activation</h3>
<p>Open the activation dialog from:</p>
<ul>
	<li><strong>Settings → License → Activate</strong></li>
	<li><strong>Help → Activate License</strong></li>
	<li>The <em>Upgrade</em> button on the trial banner</li>
</ul>

<p>To obtain a license key, email <a href="mailto:info@techemv.it">info@techemv.it</a>
with your <strong>Hardware ID</strong> (shown in the activation dialog,
also visible under Settings → License). TECHEMV SRL generates a
signed license bound to your machine and emails it back. Paste it into
the key field; the dialog previews the licensee name and entitlements
before activation.</p>

<h3>Offline verification</h3>
<p>After the first activation, license verification is purely local -
no network call is required. The key carries an Ed25519 signature that
the app verifies against an embedded public key.</p>
`,
};

export const shortcutsSection: ManualSection = {
	id: 'shortcuts',
	heading: 'Keyboard Shortcuts',
	body: `
<p>BridgeLab shortcuts are user-configurable under
<strong>Settings → Shortcuts</strong>. Click any binding, press a new
key combination, confirm with OK.</p>

<h3>Defaults</h3>
<table>
	<tr><td><kbd>Ctrl</kbd>+<kbd>O</kbd></td><td>Open file</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>N</kbd></td><td>New from template</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>L</kbd></td><td>Test Case Library</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>S</kbd></td><td>Save</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>S</kbd></td><td>Save As</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>W</kbd></td><td>Close tab</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>B</kbd></td><td>Toggle tree panel</td></tr>
	<tr><td><kbd>F5</kbd></td><td>Re-parse message</td></tr>
	<tr><td><kbd>F6</kbd></td><td>Validate</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>K</kbd></td><td>Communication panel</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>P</kbd></td><td>FHIRPath panel</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>,</kbd></td><td>Settings</td></tr>
	<tr><td><kbd>F1</kbd></td><td>This user manual</td></tr>
	<tr><td><kbd>Alt</kbd>+<kbd>T</kbd></td><td>Show in Tree (editor context menu)</td></tr>
	<tr><td><kbd>Alt</kbd>+<kbd>C</kbd></td><td>Copy Segment (editor context menu)</td></tr>
</table>

<h3>Conflict detection</h3>
<p>If you pick a key combination already assigned to another action, the
editor warns you - confirm to transfer the binding, or choose a
different key. Monaco's own shortcuts
(<kbd>Ctrl</kbd>+<kbd>F</kbd>, <kbd>Ctrl</kbd>+<kbd>D</kbd>, ...) take
precedence when the editor has focus.</p>

<h3>Reset</h3>
<p>Click <em>Reset All</em> to restore every shortcut to its default, or
the small ↺ button next to each entry to reset just that one.</p>
`,
};
