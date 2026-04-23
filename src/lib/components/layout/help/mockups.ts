/** Reusable SVG mockups of BridgeLab UI elements, used in the manual. */

export const mockupAppShell = `
<svg class="mockup" viewBox="0 0 720 400" xmlns="http://www.w3.org/2000/svg">
	<rect width="720" height="400" fill="#1e1e2e"/>
	<!-- Menu bar -->
	<rect x="0" y="0" width="720" height="24" fill="#313244"/>
	<text x="12" y="16" fill="#cdd6f4" font-family="sans-serif" font-size="11">File  Edit  View  Tools  Help</text>
	<!-- Trial banner -->
	<rect x="0" y="24" width="720" height="22" fill="#f9e2af"/>
	<text x="280" y="39" fill="#1e1e2e" font-family="sans-serif" font-size="11" font-weight="600">Pro trial: 7 days remaining</text>
	<!-- Tabs -->
	<rect x="0" y="46" width="720" height="28" fill="#181825"/>
	<rect x="8" y="50" width="140" height="24" rx="3" fill="#313244"/>
	<text x="18" y="66" fill="#cdd6f4" font-family="sans-serif" font-size="11">adt_a01.hl7</text>
	<rect x="156" y="50" width="110" height="24" rx="3" fill="#24253a"/>
	<text x="165" y="66" fill="#a6adc8" font-family="sans-serif" font-size="11">Untitled</text>
	<!-- Tree panel -->
	<rect x="0" y="74" width="200" height="300" fill="#181825"/>
	<text x="12" y="92" fill="#89b4fa" font-family="sans-serif" font-size="11" font-weight="700">MESSAGE STRUCTURE</text>
	<text x="14" y="115" fill="#cba6f7" font-family="monospace" font-size="11">▾ MSH (0)</text>
	<text x="14" y="135" fill="#cba6f7" font-family="monospace" font-size="11">▸ EVN (1)</text>
	<text x="14" y="155" fill="#cba6f7" font-family="monospace" font-size="11">▾ PID (2)</text>
	<text x="28" y="175" fill="#89b4fa" font-family="monospace" font-size="11">PID-3 Patient ID</text>
	<rect x="8" y="178" width="180" height="18" fill="#45475a" opacity="0.4"/>
	<text x="28" y="195" fill="#89b4fa" font-family="monospace" font-size="11">PID-5 Patient Name</text>
	<text x="14" y="215" fill="#cba6f7" font-family="monospace" font-size="11">▸ PV1 (3)</text>
	<!-- Editor -->
	<rect x="200" y="74" width="520" height="220" fill="#1e1e2e"/>
	<text x="220" y="102" fill="#cba6f7" font-family="monospace" font-size="11">MSH</text>
	<text x="248" y="102" fill="#6c7086" font-family="monospace" font-size="11">|^~\\&amp;|SENDER|FAC|RECV|FAC|20260415||</text>
	<text x="220" y="122" fill="#cba6f7" font-family="monospace" font-size="11">EVN</text>
	<text x="248" y="122" fill="#6c7086" font-family="monospace" font-size="11">|A01|20260415120000</text>
	<text x="220" y="142" fill="#cba6f7" font-family="monospace" font-size="11">PID</text>
	<text x="248" y="142" fill="#6c7086" font-family="monospace" font-size="11">|1||MRN12345||DOE^JOHN||19800101|M</text>
	<text x="220" y="162" fill="#cba6f7" font-family="monospace" font-size="11">OBX</text>
	<text x="248" y="162" fill="#6c7086" font-family="monospace" font-size="11">|1|ED|^^PDF^Base64|| </text>
	<text x="430" y="162" fill="#f38ba8" font-family="monospace" font-size="11" font-style="italic">{...256000 bytes}</text>
	<!-- Field Inspector -->
	<rect x="0" y="260" width="200" height="114" fill="#24253a"/>
	<text x="12" y="278" fill="#89b4fa" font-family="sans-serif" font-size="11" font-weight="700">FIELD INSPECTOR</text>
	<text x="12" y="298" fill="#f9e2af" font-family="monospace" font-size="12" font-weight="700">PID-5</text>
	<text x="12" y="316" fill="#a6adc8" font-family="sans-serif" font-size="10">Name: Patient Name</text>
	<text x="12" y="330" fill="#a6adc8" font-family="sans-serif" font-size="10">Type: XPN</text>
	<text x="12" y="344" fill="#a6adc8" font-family="sans-serif" font-size="10">Required: Yes</text>
	<text x="12" y="358" fill="#a6adc8" font-family="sans-serif" font-size="10">Max length: 250</text>
	<!-- Status bar -->
	<rect x="0" y="376" width="720" height="24" fill="#313244"/>
	<text x="12" y="392" fill="#a6adc8" font-family="sans-serif" font-size="10">ADT^A01 · v2.5 · 4 segments · Ln 3, Col 22</text>
</svg>`;

export const mockupContextMenu = `
<svg class="mockup" viewBox="0 0 520 280" xmlns="http://www.w3.org/2000/svg">
	<rect width="520" height="280" fill="#1e1e2e"/>
	<!-- Editor lines -->
	<text x="20" y="36" fill="#cba6f7" font-family="monospace" font-size="13">MSH</text>
	<text x="52" y="36" fill="#6c7086" font-family="monospace" font-size="13">|^~\\&amp;|SENDER|FAC|RECV|FAC|...</text>
	<text x="20" y="60" fill="#cba6f7" font-family="monospace" font-size="13">PID</text>
	<text x="52" y="60" fill="#6c7086" font-family="monospace" font-size="13">|1||MRN12345||</text>
	<text x="176" y="60" fill="#cdd6f4" font-family="monospace" font-size="13" font-weight="700">DOE^JOHN</text>
	<rect x="174" y="46" width="74" height="18" stroke="#89b4fa" stroke-width="1.5" fill="none"/>
	<!-- Context menu anchored near PID-5 -->
	<rect x="250" y="64" width="220" height="168" fill="#24253a" stroke="#585b70" stroke-width="1" rx="3"/>
	<text x="262" y="82" fill="#cdd6f4" font-family="sans-serif" font-size="12">Show Segment in Tree</text>
	<text x="404" y="82" fill="#6c7086" font-family="sans-serif" font-size="11">Alt+T</text>
	<line x1="258" y1="92" x2="462" y2="92" stroke="#45475a"/>
	<text x="262" y="108" fill="#cdd6f4" font-family="sans-serif" font-size="12">Expand Truncated Field</text>
	<text x="262" y="128" fill="#cdd6f4" font-family="sans-serif" font-size="12">Expand All Truncated Fields</text>
	<text x="262" y="148" fill="#cdd6f4" font-family="sans-serif" font-size="12">Collapse All Expanded Fields</text>
	<line x1="258" y1="158" x2="462" y2="158" stroke="#45475a"/>
	<text x="262" y="174" fill="#cdd6f4" font-family="sans-serif" font-size="12">Copy Full Message</text>
	<text x="262" y="194" fill="#cdd6f4" font-family="sans-serif" font-size="12">Copy Truncated Message</text>
	<text x="262" y="214" fill="#cdd6f4" font-family="sans-serif" font-size="12">Copy Segment</text>
	<text x="404" y="214" fill="#6c7086" font-family="sans-serif" font-size="11">Alt+C</text>
</svg>`;

export const mockupCommunication = `
<svg class="mockup" viewBox="0 0 720 260" xmlns="http://www.w3.org/2000/svg">
	<rect width="720" height="260" fill="#181825"/>
	<!-- Tabs -->
	<rect x="0" y="0" width="720" height="30" fill="#24253a"/>
	<rect x="0" y="0" width="80" height="30" fill="#313244"/>
	<text x="22" y="20" fill="#89b4fa" font-family="sans-serif" font-size="12" font-weight="600">MLLP</text>
	<text x="100" y="20" fill="#a6adc8" font-family="sans-serif" font-size="12">HTTP</text>
	<text x="160" y="20" fill="#a6adc8" font-family="sans-serif" font-size="12">History</text>
	<!-- Form -->
	<text x="16" y="54" fill="#a6adc8" font-family="sans-serif" font-size="11">Host</text>
	<rect x="16" y="60" width="220" height="26" fill="#1e1e2e" stroke="#45475a" rx="3"/>
	<text x="24" y="78" fill="#cdd6f4" font-family="monospace" font-size="12">localhost</text>
	<text x="250" y="54" fill="#a6adc8" font-family="sans-serif" font-size="11">Port</text>
	<rect x="250" y="60" width="80" height="26" fill="#1e1e2e" stroke="#45475a" rx="3"/>
	<text x="258" y="78" fill="#cdd6f4" font-family="monospace" font-size="12">2575</text>
	<rect x="348" y="60" width="90" height="26" fill="#89b4fa" rx="3"/>
	<text x="360" y="78" fill="#1e1e2e" font-family="sans-serif" font-size="12" font-weight="700">▶ Send</text>
	<rect x="450" y="60" width="110" height="26" fill="#313244" stroke="#585b70" rx="3"/>
	<text x="458" y="78" fill="#cdd6f4" font-family="sans-serif" font-size="11">◉ Listen (Pro)</text>
	<!-- Result -->
	<rect x="16" y="100" width="688" height="144" fill="#1e1e2e" stroke="#313244" rx="3"/>
	<text x="28" y="122" fill="#a6e3a1" font-family="sans-serif" font-size="12" font-weight="700">✓ ACK received (124 ms)</text>
	<text x="28" y="146" fill="#cba6f7" font-family="monospace" font-size="11">MSH|^~\\&amp;|Recv||Send||20260415||ACK|ACK001|P|2.5</text>
	<text x="28" y="164" fill="#cba6f7" font-family="monospace" font-size="11">MSA|AA|MSG0001</text>
</svg>`;

export const mockupValidation = `
<svg class="mockup" viewBox="0 0 720 220" xmlns="http://www.w3.org/2000/svg">
	<rect width="720" height="220" fill="#181825"/>
	<!-- Header -->
	<rect x="0" y="0" width="720" height="30" fill="#313244"/>
	<text x="16" y="20" fill="#cdd6f4" font-family="sans-serif" font-size="12" font-weight="700">Validation</text>
	<rect x="100" y="6" width="54" height="18" fill="#f38ba8" rx="9"/>
	<text x="118" y="19" fill="#1e1e2e" font-family="sans-serif" font-size="11" font-weight="700">2 ✕</text>
	<rect x="162" y="6" width="54" height="18" fill="#f9e2af" rx="9"/>
	<text x="180" y="19" fill="#1e1e2e" font-family="sans-serif" font-size="11" font-weight="700">3 ⚠</text>
	<rect x="224" y="6" width="54" height="18" fill="#89b4fa" rx="9"/>
	<text x="246" y="19" fill="#1e1e2e" font-family="sans-serif" font-size="11" font-weight="700">1 ℹ</text>
	<!-- Rows -->
	<text x="16" y="56" fill="#f38ba8" font-family="sans-serif" font-size="12" font-weight="700">✕ ERROR</text>
	<text x="100" y="56" fill="#cdd6f4" font-family="monospace" font-size="11">MSH-002</text>
	<text x="180" y="56" fill="#cdd6f4" font-family="sans-serif" font-size="12">PID-3 (Patient ID) is required</text>
	<line x1="0" y1="70" x2="720" y2="70" stroke="#313244"/>
	<text x="16" y="90" fill="#f9e2af" font-family="sans-serif" font-size="12" font-weight="700">⚠ WARN</text>
	<text x="100" y="90" fill="#cdd6f4" font-family="monospace" font-size="11">LEN-001</text>
	<text x="180" y="90" fill="#cdd6f4" font-family="sans-serif" font-size="12">PID-19 exceeds max_length (16)</text>
	<line x1="0" y1="104" x2="720" y2="104" stroke="#313244"/>
	<text x="16" y="124" fill="#f9e2af" font-family="sans-serif" font-size="12" font-weight="700">⚠ WARN</text>
	<text x="100" y="124" fill="#cdd6f4" font-family="monospace" font-size="11">TYPE-SI-01</text>
	<text x="180" y="124" fill="#cdd6f4" font-family="sans-serif" font-size="12">Non-numeric value in SI field</text>
</svg>`;
