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
	<li>Guaranteed to compile under strict schema processors — the few
		HL7 structures whose definition violates XSD's Unique Particle
		Attribution rule are emitted as an annotated relaxed choice.</li>
</ul>

<h3>Actions</h3>
<ul>
	<li><strong>Copy</strong> — copies the XSD to the clipboard,
		handy when you want to paste it into an editor or chat.</li>
	<li><strong>Save as…</strong> — opens the OS file dialog with
		<code>{MESSAGE}.xsd</code> as the default name.</li>
</ul>

<h3>Coverage and tiers</h3>
<p>Ten HL7 versions ship complete: <strong>2.1, 2.2, 2.3, 2.3.1, 2.4,
2.5, 2.5.1, 2.6, 2.7 and 2.7.1</strong> — 2,320 selectable message
structures in the version dropdown.</p>
<p>HL7 v2.7.1 is a technical-correction release of v2.7 and carries the
same message definitions, so it is marked <strong>(= v2.7)</strong> in
the dropdown and exports from the v2.7 catalogue.</p>
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
<strong>Help → Buy a License</strong> (the pricing page) or
<strong>Help → Activate License</strong>.</p>

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
<p>The <strong>List / Graph</strong> toggle switches to a reference
graph: every entry is a node (colored by resource type), every
<code>reference</code> a directed arrow. Click a node to select it — the
detail pane follows. Available up to 150 entries; larger bundles use the
list.</p>

<h3>FHIRPath Evaluator (Pro)</h3>
<p><kbd>Ctrl</kbd>+<kbd>P</kbd> or <strong>Tools → FHIRPath Evaluator</strong>
opens an interactive console where you type FHIRPath expressions
against the current resource.</p>
<p>The evaluator implements the FHIRPath 2.0 language: the full operator
set with the specification's precedence and three-valued logic, and
around seventy functions.</p>
<ul>
	<li><strong>Navigation:</strong> <code>Patient.name.family</code>,
		<code>Bundle.entry.resource</code>, with choice elements reached by
		their base name — <code>Observation.value</code> finds
		<code>valueQuantity</code></li>
	<li><strong>Indexing:</strong> <code>Patient.name[0].given</code></li>
	<li><strong>Filters and projection:</strong>
		<code>where()</code>, <code>select()</code>, <code>repeat()</code>,
		<code>ofType()</code>, with <code>$this</code> and
		<code>$index</code> bound inside them</li>
	<li><strong>Collections:</strong> <code>count()</code>,
		<code>first()</code>, <code>last()</code>, <code>tail()</code>,
		<code>skip()</code>, <code>take()</code>, <code>distinct()</code>,
		<code>sort()</code>, <code>union()</code>, <code>combine()</code>,
		<code>intersect()</code>, <code>exclude()</code>,
		<code>aggregate()</code></li>
	<li><strong>Logic:</strong> <code>and</code>, <code>or</code>,
		<code>xor</code>, <code>implies</code>, <code>not()</code>,
		<code>exists()</code>, <code>all()</code>, <code>iif()</code> —
		all with the empty collection as the "unknown" value</li>
	<li><strong>Strings:</strong> <code>substring()</code>,
		<code>matches()</code>, <code>replace()</code>,
		<code>split()</code>, <code>join()</code>, <code>encode()</code>,
		<code>escape()</code> and friends</li>
	<li><strong>Dates and quantities:</strong> partial-precision literals
		(<code>@2015</code>, <code>@2015-02-04T14:34:28+10:00</code>),
		duration arithmetic (<code>Patient.birthDate + 18 years</code>) and
		unit conversion within a dimension
		(<code>4 'g' = 4000 'mg'</code>)</li>
	<li><strong>FHIR extras:</strong> <code>extension(url)</code>,
		<code>hasValue()</code>, and <code>resolve()</code>, which follows
		a Reference to a contained or bundled resource</li>
	<li><strong>Debugging:</strong> <code>trace('label')</code> passes its
		input through unchanged and shows it under the result, so you can
		see what a long path produced halfway along</li>
</ul>
<p>Comparing values of different precision returns the empty collection
rather than a guess: <code>@2015-02-04 = @2015-02</code> is neither true
nor false, because the second value could be that day or another one in
the same month.</p>
<p>Recent expressions are kept in a history dropdown for quick replay.</p>

<h3>FHIR validation</h3>
<p>F6 also works for FHIR resources. Errors highlight missing required
fields (e.g. <code>Patient.identifier</code>), invalid data types
(gender not in the value set), and structural issues. Declared
<code>meta.profile</code> canonical URLs are listed as info findings
(profile conformance itself is not checked); malformed entries are
flagged as warnings.</p>

<h3>Profile validation (Pro)</h3>
<p>By default a FHIR resource is checked structurally: is there a
<code>resourceType</code>, do the resource-specific rules hold. Checking it
against a <strong>StructureDefinition</strong> — the real definition of what
a Patient may contain — needs those definitions, and they ship as FHIR NPM
packages.</p>
<p>The FHIR R4 core (<code>hl7.fhir.r4.core</code> 4.0.1) is <strong>built
in</strong>, so the base definitions are always there — offline, in every
tier, nothing to download. <strong>Tools → FHIR profile packages…</strong>
installs national or site-specific implementation guides on top from a
<code>.tgz</code>; a package you install that carries the same definitions
replaces the built-in ones, a newer version outranks them.</p>
<p>Every FHIR validation checks, against the built-in core and whatever
is installed on top:</p>
<ul>
	<li><strong>Cardinality</strong> — a required element that is missing, or
		a <code>0..1</code> element that repeats.</li>
	<li><strong>Element types</strong> — a boolean written as a string, a
		number where an object belongs.</li>
	<li><strong>Choice elements</strong> — <code>value[x]</code> must appear
		as exactly one of <code>valueQuantity</code>,
		<code>valueString</code> and so on. Writing a plain <code>value</code>,
		or two forms at once, is reported.</li>
	<li><strong>Fixed values and patterns</strong> — what a profile pins
		down.</li>
	<li><strong>Unknown elements</strong> — a name the profile does not
		define. This is the check that catches a typo like
		<code>genderr</code> or an element belonging to a different resource
		type.</li>
</ul>
<p>Profiles a resource declares in <code>meta.profile</code> are applied
automatically when the package defining them is installed. When it is not,
the validator says so rather than quietly reporting a clean result — and
when profiles did run, the report says that too, because "no findings" means
very different things in the two cases.</p>
<p><strong>Terminology is out of scope.</strong> A <code>required</code>
binding can only be checked by expanding the ValueSet, which means shipping
the terminology packages or calling a server. BridgeLab leaves bindings
unchecked rather than half-checking them.</p>
<p>Installing a package requires a Professional license. Packages already
installed keep validating in every tier, so a lapsed trial never turns
previously clean resources red.</p>

<h3>Custom FHIR rules (Pro)</h3>
<p><strong>Tools → FHIR validation rules…</strong> opens an editor for your
own checks. They run alongside the built-in ones every time you validate a
resource, and they are stored as an ordinary plugin pack
(<code>plugins/fhir/user-rules.json</code>) you can copy between
machines or commit to a repository.</p>
<p>A rule takes one of two shapes:</p>
<ul>
	<li><strong>Expression must be true</strong> — a FHIRPath invariant, the
		way FHIR writes its own constraints:
		<code>identifier.exists()</code>, or
		<code>value.exists() xor dataAbsentReason.exists()</code>.</li>
	<li><strong>Path + check</strong> — a FHIRPath selector plus something to
		assert about the values it picks up: must be present, how many,
		matches a pattern, one of a list, contains text, or a length
		bound.</li>
</ul>
<p>Set <strong>Resource type</strong> to scope a rule to Patient,
Observation and so on, or leave it empty to apply it to everything. A rule
scoped to a type also fires for the matching resources inside a Bundle,
with the finding reported against <code>entry[n].resource.…</code>.</p>
<p><strong>Test on open resource</strong> runs the rule you are editing
against the resource in the active tab before you save it, and shows which
values the selector actually picked up — the fastest way to tell a rule that
passes from one that never ran. If the open resource is of a different type,
the editor says so rather than reporting a pass.</p>
<p>Rules you already have keep working in every tier; the editor itself
requires a Professional license. Hand-written packs are documented in
<code>docs/PLUGINS.md</code>.</p>

<h3>FHIR templates</h3>
<p><strong>File → New from template</strong> includes a FHIR category: a
minimal Patient, a blood-pressure Observation with components, and a
transaction Bundle whose entries reference each other via
<code>urn:uuid</code> — open it and try the Bundle visualizer's graph
view.</p>
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
├── fhir/
│   └── user-rules.json
└── anonymization/
    └── eu-national-id.json</code></pre>

<p>On Windows the root is <code>%APPDATA%\\BridgeLab\\plugins</code>, on
macOS <code>~/Library/Application Support/BridgeLab/plugins</code>, on
Linux <code>~/.config/BridgeLab/plugins</code>.</p>

<p>Three kinds of pack live there: HL7 v2 validation rules
(<code>validation/</code>, below), FHIR validation rules
(<code>fhir/</code> — a FHIRPath invariant, or a selector plus a check;
see <em>FHIR Support → Custom FHIR rules</em>) and extra PHI fields
for the anonymizer (<code>anonymization/</code>, below).</p>

<p><strong>Every tier has the whole mechanism</strong> — all three kinds,
every check type, reload and per-pack toggles. The only difference is how
many packs can be active at the same time: up to <strong>3</strong> in
Community, unlimited in Pro and Enterprise. The in-app editor that writes
FHIR packs is Pro; a FHIR pack written by hand runs in Community like any
other.</p>

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

<p class="note">On the Community tier up to <strong>3 packs</strong> are
active at once: extra enabled packs show an "inactive" badge and don't
contribute rules until a slot frees up (disable another pack, or
upgrade).</p>
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
	<tr><td>MLLP send, HTTP GET without an auth header</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>FHIR R4 core conformance (built in)</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>XSD export</td>
		<td>4 v2.5 messages</td><td>Full catalogue</td><td>Full catalogue</td></tr>
	<tr><td>PHI detection (view only)</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Plugin packs (all kinds, every check type)</td>
		<td>3 active at once</td><td>Unlimited</td><td>Unlimited</td></tr>
	<tr><td>MLLP listener</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>HTTP POST/PUT/DELETE/PATCH, and authentication on any method</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Anonymization masking</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Export JSON/CSV</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>FHIR profile packages &amp; rules builder</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>FHIRPath Evaluator + Bundle Visualizer</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Saved test cases</td>
		<td>10</td><td>Unlimited</td><td>Unlimited</td></tr>
	<tr><td>SOAP + priority support</td>
		<td>—</td><td>—</td><td>✓</td></tr>
</table>

<p class="note">Community keeps up to <strong>3 active plugin packs</strong>
and <strong>10 saved test cases</strong>. Nothing is ever locked or
deleted: items saved beyond the cap (e.g. during a trial) stay visible,
editable and runnable — only new saves and activations beyond the limit
ask for an upgrade, and freeing a slot re-enables them immediately.</p>

<h3>Trial</h3>
<p>First launch starts a <strong>14-day Pro trial</strong> with every Pro
feature enabled. The trial banner (yellow) is dismissible; when 3 days
remain it turns red and stays visible as a reminder.</p>

<p>When the trial expires BridgeLab <strong>does not stop working</strong>
- it falls back to the Community tier and the banner prompts you to
upgrade. Your messages, settings, plugins and test cases remain intact.</p>

<h3>Updates</h3>
<p>Nothing is requested until you decide. The first time BridgeLab
starts it asks, in a banner at the top of the window, whether it may
look for new versions (the Windows installer asks during setup instead,
and then the app does not ask again). <em>Yes, check</em> and
<em>No</em> are remembered in Settings → Privacy; closing the banner
without answering asks again at the next start.</p>
<p>Once a day, a few seconds after start-up, BridgeLab asks GitHub
(<code>api.github.com</code>) for the latest release. When a newer one
exists a banner says so, with <em>Download</em> (opens the release page),
<em>Skip this version</em> and a close button; nothing is installed
automatically. The request carries nothing about you, the computer or
your files, and without internet access it is skipped silently. Turn it
off under <strong>Settings → Privacy → Check for new versions at
startup</strong>; <strong>Help → Check for Updates</strong> always works
on demand.</p>
<p>The Windows installer asks the same question the first time
(<em>Yes</em> is the default; a silent install does not ask). For
managed machines an administrator can turn the check off for every user,
and the Settings checkbox then shows as locked: set the environment
variable <code>BRIDGELAB_DISABLE_UPDATE_CHECK=1</code>, or create
<code>policy.json</code> containing
<code>{"disable_update_check": true}</code> in
<code>%ProgramData%\\BridgeLab\\</code> (Windows),
<code>/Library/Application Support/BridgeLab/</code> (macOS) or
<code>/etc/bridgelab/</code> (Linux).</p>

<h3>Buying a license</h3>
<p><strong>Help → Buy a License…</strong> opens the pricing section of
the BridgeLab website in your browser, where Professional and
Enterprise are bought online by card; the activation code arrives by
e-mail. The same page is one click away from the <em>See prices &amp;
buy</em> buttons in the activation dialog, the <em>Compare plans</em>
button on the trial banner, and the <em>See prices</em> button of every
"requires a Professional license" prompt. Need an invoice, a purchase
order or a quote instead? Write to
<a href="mailto:info@techemv.it">info@techemv.it</a>.</p>

<h3>Activation</h3>
<p>Open the activation dialog from:</p>
<ul>
	<li><strong>Settings → License → Activate</strong></li>
	<li><strong>Help → Activate License</strong></li>
	<li>The <em>Upgrade</em> button on the trial banner</li>
</ul>

<p><strong>Online activation (default):</strong> after purchase you
receive an activation code like <code>BL-PRO-XXXX-XXXX-XXXX</code> by
e-mail. Paste it into the key field: the app exchanges it once over
HTTPS for a signed license bound to this machine. Use
<em>Deactivate</em> to free the seat before moving to another
computer.</p>

<p>Renewed your subscription? Press <em>Update license</em> in the
License dialog to pick the new expiry up instantly — or do nothing:
within 14 days of expiry the app fetches it silently at startup
(never producing errors on offline machines).</p>

<p><strong>Offline key (isolated / air-gapped sites):</strong> email
<a href="mailto:info@techemv.it">info@techemv.it</a> with your
<strong>Hardware ID</strong> (shown under "Need an offline key?" in
the activation dialog, also visible under Settings → License).
TECHEMV SRL emails back a signed license bound to your machine — no
internet access is ever required. The dialog previews the licensee
name and entitlements before activation.</p>

<h3>Offline verification</h3>
<p>Whichever flow you used, routine license verification is purely
local - the app never needs to contact the license server to keep
working. Server calls happen only when you explicitly trigger them:
activating with a code, freeing a seat via <em>Deactivate</em>, or
opt-in usage statistics. The key carries an Ed25519 signature that the
app verifies against an embedded public key.</p>

<h3>Privacy &amp; usage statistics</h3>
<p>BridgeLab can send <strong>usage statistics</strong> to TECHEMV —
disabled by default, opt-in under <strong>Settings → Privacy</strong>.
When enabled, automatic sending happens at most once a day; the
<em>Send now</em> button transmits immediately. Each report contains
usage counters, app version, OS, license tier, a <strong>random
installation ID</strong> and — only for licenses activated online —
the <strong>activation code</strong> (used to flag a revoked license).
The data is therefore <strong>pseudonymous</strong>, not fully
anonymous: no message content, file names, host names, user names or
patient data are ever sent, and the exact JSON payload can be
inspected with <em>Show what is sent</em>. With the toggle off (the
default) nothing is transmitted at all, and network problems never
produce errors — a fully offline installation is a normal, supported
setup.</p>
<p>On managed machines an administrator can force the usage statistics
off for every user: set <code>BRIDGELAB_DISABLE_TELEMETRY=1</code>, or add
<code>"disable_telemetry": true</code> to the same <code>policy.json</code>
used for the update check. Nothing is sent then, whatever the user
ticked, and the Settings checkbox shows as locked.</p>
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
