/** Reusable SVG mockups of BridgeLab UI elements, used in the manual.
 *
 * Chrome labels (menus, panel titles, buttons, context menu) are localized
 * per manual language from the same strings the UI uses; backend-generated
 * content (HL7 payloads, validation rule messages) stays as the app renders
 * it. Labels are baked per locale below — regenerate them alongside any
 * i18n change to the source keys.
 */

interface MockupLabels {
	menuBar: string;
	trialBanner: string;
	untitled: string;
	treeHeader: string;
	inspectorTitle: string;
	inspName: string;
	inspType: string;
	inspRequired: string;
	inspMaxLen: string;
	statusBar: string;
	showInTree: string;
	expandField: string;
	expandAll: string;
	collapseAll: string;
	copyFull: string;
	copyTruncated: string;
	copySegment: string;
	host: string;
	port: string;
	history: string;
	send: string;
	listen: string;
	ack: string;
	validationTitle: string;
	errBadge: string;
	warnBadge: string;
}

function makeAppShell(l: MockupLabels): string {
	return `
<svg class="mockup" viewBox="0 0 720 400" xmlns="http://www.w3.org/2000/svg">
	<rect width="720" height="400" fill="#1e1e2e"/>
	<!-- Menu bar -->
	<rect x="0" y="0" width="720" height="24" fill="#313244"/>
	<text x="12" y="16" fill="#cdd6f4" font-family="sans-serif" font-size="11">${l.menuBar}</text>
	<!-- Trial banner -->
	<rect x="0" y="24" width="720" height="22" fill="#f9e2af"/>
	<text x="280" y="39" fill="#1e1e2e" font-family="sans-serif" font-size="11" font-weight="600">${l.trialBanner}</text>
	<!-- Tabs -->
	<rect x="0" y="46" width="720" height="28" fill="#181825"/>
	<rect x="8" y="50" width="140" height="24" rx="3" fill="#313244"/>
	<text x="18" y="66" fill="#cdd6f4" font-family="sans-serif" font-size="11">adt_a01.hl7</text>
	<rect x="156" y="50" width="110" height="24" rx="3" fill="#24253a"/>
	<text x="165" y="66" fill="#a6adc8" font-family="sans-serif" font-size="11">${l.untitled}</text>
	<!-- Tree panel -->
	<rect x="0" y="74" width="200" height="300" fill="#181825"/>
	<text x="12" y="92" fill="#89b4fa" font-family="sans-serif" font-size="11" font-weight="700">${l.treeHeader}</text>
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
	<text x="12" y="278" fill="#89b4fa" font-family="sans-serif" font-size="11" font-weight="700">${l.inspectorTitle}</text>
	<text x="12" y="298" fill="#f9e2af" font-family="monospace" font-size="12" font-weight="700">PID-5</text>
	<text x="12" y="316" fill="#a6adc8" font-family="sans-serif" font-size="10">${l.inspName}</text>
	<text x="12" y="330" fill="#a6adc8" font-family="sans-serif" font-size="10">${l.inspType}</text>
	<text x="12" y="344" fill="#a6adc8" font-family="sans-serif" font-size="10">${l.inspRequired}</text>
	<text x="12" y="358" fill="#a6adc8" font-family="sans-serif" font-size="10">${l.inspMaxLen}</text>
	<!-- Status bar -->
	<rect x="0" y="376" width="720" height="24" fill="#313244"/>
	<text x="12" y="392" fill="#a6adc8" font-family="sans-serif" font-size="10">${l.statusBar}</text>
</svg>`;
}

function makeContextMenu(l: MockupLabels): string {
	return `
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
	<text x="262" y="82" fill="#cdd6f4" font-family="sans-serif" font-size="12">${l.showInTree}</text>
	<text x="404" y="82" fill="#6c7086" font-family="sans-serif" font-size="11">Alt+T</text>
	<line x1="258" y1="92" x2="462" y2="92" stroke="#45475a"/>
	<text x="262" y="108" fill="#cdd6f4" font-family="sans-serif" font-size="12">${l.expandField}</text>
	<text x="262" y="128" fill="#cdd6f4" font-family="sans-serif" font-size="12">${l.expandAll}</text>
	<text x="262" y="148" fill="#cdd6f4" font-family="sans-serif" font-size="12">${l.collapseAll}</text>
	<line x1="258" y1="158" x2="462" y2="158" stroke="#45475a"/>
	<text x="262" y="174" fill="#cdd6f4" font-family="sans-serif" font-size="12">${l.copyFull}</text>
	<text x="262" y="194" fill="#cdd6f4" font-family="sans-serif" font-size="12">${l.copyTruncated}</text>
	<text x="262" y="214" fill="#cdd6f4" font-family="sans-serif" font-size="12">${l.copySegment}</text>
	<text x="404" y="214" fill="#6c7086" font-family="sans-serif" font-size="11">Alt+C</text>
</svg>`;
}

function makeCommunication(l: MockupLabels): string {
	return `
<svg class="mockup" viewBox="0 0 720 260" xmlns="http://www.w3.org/2000/svg">
	<rect width="720" height="260" fill="#181825"/>
	<!-- Tabs -->
	<rect x="0" y="0" width="720" height="30" fill="#24253a"/>
	<rect x="0" y="0" width="80" height="30" fill="#313244"/>
	<text x="22" y="20" fill="#89b4fa" font-family="sans-serif" font-size="12" font-weight="600">MLLP</text>
	<text x="100" y="20" fill="#a6adc8" font-family="sans-serif" font-size="12">HTTP</text>
	<text x="160" y="20" fill="#a6adc8" font-family="sans-serif" font-size="12">${l.history}</text>
	<!-- Form -->
	<text x="16" y="54" fill="#a6adc8" font-family="sans-serif" font-size="11">${l.host}</text>
	<rect x="16" y="60" width="220" height="26" fill="#1e1e2e" stroke="#45475a" rx="3"/>
	<text x="24" y="78" fill="#cdd6f4" font-family="monospace" font-size="12">localhost</text>
	<text x="250" y="54" fill="#a6adc8" font-family="sans-serif" font-size="11">${l.port}</text>
	<rect x="250" y="60" width="80" height="26" fill="#1e1e2e" stroke="#45475a" rx="3"/>
	<text x="258" y="78" fill="#cdd6f4" font-family="monospace" font-size="12">2575</text>
	<rect x="348" y="60" width="90" height="26" fill="#89b4fa" rx="3"/>
	<text x="360" y="78" fill="#1e1e2e" font-family="sans-serif" font-size="12" font-weight="700">▶ ${l.send}</text>
	<rect x="450" y="60" width="110" height="26" fill="#313244" stroke="#585b70" rx="3"/>
	<text x="458" y="78" fill="#cdd6f4" font-family="sans-serif" font-size="11">◉ ${l.listen}</text>
	<!-- Result -->
	<rect x="16" y="100" width="688" height="144" fill="#1e1e2e" stroke="#313244" rx="3"/>
	<text x="28" y="122" fill="#a6e3a1" font-family="sans-serif" font-size="12" font-weight="700">✓ ${l.ack}</text>
	<text x="28" y="146" fill="#cba6f7" font-family="monospace" font-size="11">MSH|^~\\&amp;|Recv||Send||20260415||ACK|ACK001|P|2.5</text>
	<text x="28" y="164" fill="#cba6f7" font-family="monospace" font-size="11">MSA|AA|MSG0001</text>
</svg>`;
}

function makeValidation(l: MockupLabels): string {
	return `
<svg class="mockup" viewBox="0 0 720 220" xmlns="http://www.w3.org/2000/svg">
	<rect width="720" height="220" fill="#181825"/>
	<!-- Header -->
	<rect x="0" y="0" width="720" height="30" fill="#313244"/>
	<text x="16" y="20" fill="#cdd6f4" font-family="sans-serif" font-size="12" font-weight="700">${l.validationTitle}</text>
	<rect x="100" y="6" width="54" height="18" fill="#f38ba8" rx="9"/>
	<text x="118" y="19" fill="#1e1e2e" font-family="sans-serif" font-size="11" font-weight="700">2 ✕</text>
	<rect x="162" y="6" width="54" height="18" fill="#f9e2af" rx="9"/>
	<text x="180" y="19" fill="#1e1e2e" font-family="sans-serif" font-size="11" font-weight="700">3 ⚠</text>
	<rect x="224" y="6" width="54" height="18" fill="#89b4fa" rx="9"/>
	<text x="246" y="19" fill="#1e1e2e" font-family="sans-serif" font-size="11" font-weight="700">1 ℹ</text>
	<!-- Rows (rule messages are backend output and render in English in-app) -->
	<text x="16" y="56" fill="#f38ba8" font-family="sans-serif" font-size="12" font-weight="700">✕ ${l.errBadge}</text>
	<text x="100" y="56" fill="#cdd6f4" font-family="monospace" font-size="11">MSH-002</text>
	<text x="180" y="56" fill="#cdd6f4" font-family="sans-serif" font-size="12">PID-3 (Patient ID) is required</text>
	<line x1="0" y1="70" x2="720" y2="70" stroke="#313244"/>
	<text x="16" y="90" fill="#f9e2af" font-family="sans-serif" font-size="12" font-weight="700">⚠ ${l.warnBadge}</text>
	<text x="100" y="90" fill="#cdd6f4" font-family="monospace" font-size="11">LEN-001</text>
	<text x="180" y="90" fill="#cdd6f4" font-family="sans-serif" font-size="12">PID-19 exceeds max_length (16)</text>
	<line x1="0" y1="104" x2="720" y2="104" stroke="#313244"/>
	<text x="16" y="124" fill="#f9e2af" font-family="sans-serif" font-size="12" font-weight="700">⚠ ${l.warnBadge}</text>
	<text x="100" y="124" fill="#cdd6f4" font-family="monospace" font-size="11">TYPE-SI-01</text>
	<text x="180" y="124" fill="#cdd6f4" font-family="sans-serif" font-size="12">Non-numeric value in SI field</text>
</svg>`;
}

const EN_LABELS: MockupLabels = {
	menuBar: 'File  Edit  View  Tools  Help',
	trialBanner: 'Pro trial: 7 days remaining',
	untitled: 'Untitled',
	treeHeader: 'MESSAGE STRUCTURE',
	inspectorTitle: 'FIELD INSPECTOR',
	inspName: 'Name: Patient Name',
	inspType: 'Type: XPN',
	inspRequired: 'Required: Yes',
	inspMaxLen: 'Max length: 250',
	statusBar: 'ADT^A01 · v2.5 · 4 segments · Ln 3, Col 22',
	showInTree: 'Show Segment in Tree',
	expandField: 'Expand Truncated Field',
	expandAll: 'Expand All Truncated Fields',
	collapseAll: 'Collapse All Expanded Fields',
	copyFull: 'Copy Full Message',
	copyTruncated: 'Copy Truncated Message',
	copySegment: 'Copy Segment',
	host: 'Host',
	port: 'Port',
	history: 'History',
	send: 'Send',
	listen: 'Listen (Pro)',
	ack: 'ACK received (124 ms)',
	validationTitle: 'Validation',
	errBadge: 'ERROR',
	warnBadge: 'WARN',
};

const IT_LABELS: MockupLabels = {
	menuBar: 'File  Modifica  Visualizza  Strumenti  Aiuto',
	trialBanner: 'Trial Pro: 7 giorni rimanenti',
	untitled: 'Senza titolo',
	treeHeader: 'STRUTTURA MESSAGGIO',
	inspectorTitle: 'ISPETTORE CAMPO',
	inspName: 'Nome: Patient Name',
	inspType: 'Tipo: XPN',
	inspRequired: 'Obbligatorio: Sì',
	inspMaxLen: 'Lunghezza max: 250',
	statusBar: 'ADT^A01 · v2.5 · 4 segmenti · Ln 3, Col 22',
	showInTree: 'Mostra Segmento nel Tree',
	expandField: 'Espandi Campo Troncato',
	expandAll: 'Espandi Tutti i Campi Troncati',
	collapseAll: 'Comprimi Tutti i Campi Espansi',
	copyFull: 'Copia Messaggio Completo',
	copyTruncated: 'Copia Messaggio Troncato',
	copySegment: 'Copia Segmento',
	host: 'Host',
	port: 'Porta',
	history: 'Cronologia',
	send: 'Invia',
	listen: 'Ascolta (Pro)',
	ack: 'ACK ricevuto (124 ms)',
	validationTitle: 'Validazione',
	errBadge: 'ERRORE',
	warnBadge: 'AVVISO',
};

const FR_LABELS: MockupLabels = {
	menuBar: 'Fichier  Édition  Affichage  Outils  Aide',
	trialBanner: 'Essai Pro : 7 jours restants',
	untitled: 'Sans titre',
	treeHeader: 'STRUCTURE DU MESSAGE',
	inspectorTitle: 'INSPECTEUR DE CHAMP',
	inspName: 'Nom: Patient Name',
	inspType: 'Type: XPN',
	inspRequired: 'Obligatoire : Oui',
	inspMaxLen: 'Longueur max : 250',
	statusBar: 'ADT^A01 · v2.5 · 4 segments · Ln 3, Col 22',
	showInTree: 'Afficher le segment dans l\'arbre',
	expandField: 'Développer le champ tronqué',
	expandAll: 'Développer tous les champs tronqués',
	collapseAll: 'Réduire tous les champs développés',
	copyFull: 'Copier le message complet',
	copyTruncated: 'Copier le message tronqué',
	copySegment: 'Copier le segment',
	host: 'Hôte',
	port: 'Port',
	history: 'Historique',
	send: 'Envoyer',
	listen: 'Écouter (Pro)',
	ack: 'ACK reçu (124 ms)',
	validationTitle: 'Validation',
	errBadge: 'ERREUR',
	warnBadge: 'AVERT.',
};

const ES_LABELS: MockupLabels = {
	menuBar: 'Archivo  Editar  Ver  Herramientas  Ayuda',
	trialBanner: 'Prueba Pro: 7 días restantes',
	untitled: 'Sin título',
	treeHeader: 'ESTRUCTURA DEL MENSAJE',
	inspectorTitle: 'INSPECTOR DE CAMPO',
	inspName: 'Nombre: Patient Name',
	inspType: 'Tipo: XPN',
	inspRequired: 'Obligatorio: Sí',
	inspMaxLen: 'Longitud máx: 250',
	statusBar: 'ADT^A01 · v2.5 · 4 segmentos · Ln 3, Col 22',
	showInTree: 'Mostrar segmento en el árbol',
	expandField: 'Expandir campo truncado',
	expandAll: 'Expandir todos los campos truncados',
	collapseAll: 'Contraer todos los campos expandidos',
	copyFull: 'Copiar mensaje completo',
	copyTruncated: 'Copiar mensaje truncado',
	copySegment: 'Copiar segmento',
	host: 'Host',
	port: 'Puerto',
	history: 'Historial',
	send: 'Enviar',
	listen: 'Escuchar (Pro)',
	ack: 'ACK recibido (124 ms)',
	validationTitle: 'Validación',
	errBadge: 'ERROR',
	warnBadge: 'AVISO',
};

const DE_LABELS: MockupLabels = {
	menuBar: 'Datei  Bearbeiten  Ansicht  Werkzeuge  Hilfe',
	trialBanner: 'Pro-Testversion: 7 Tage verbleibend',
	untitled: 'Unbenannt',
	treeHeader: 'NACHRICHTENSTRUKTUR',
	inspectorTitle: 'FELD-INSPEKTOR',
	inspName: 'Name: Patient Name',
	inspType: 'Typ: XPN',
	inspRequired: 'Pflichtfeld: Ja',
	inspMaxLen: 'Max. Länge: 250',
	statusBar: 'ADT^A01 · v2.5 · 4 Segmente · Ln 3, Col 22',
	showInTree: 'Segment im Baum anzeigen',
	expandField: 'Gekürztes Feld erweitern',
	expandAll: 'Alle gekürzten Felder erweitern',
	collapseAll: 'Alle erweiterten Felder kürzen',
	copyFull: 'Vollständige Nachricht kopieren',
	copyTruncated: 'Gekürzte Nachricht kopieren',
	copySegment: 'Segment kopieren',
	host: 'Host',
	port: 'Port',
	history: 'Verlauf',
	send: 'Senden',
	listen: 'Lauschen (Pro)',
	ack: 'ACK empfangen (124 ms)',
	validationTitle: 'Validierung',
	errBadge: 'FEHLER',
	warnBadge: 'WARN.',
};

export const mockupAppShell = makeAppShell(EN_LABELS);
export const mockupContextMenu = makeContextMenu(EN_LABELS);
export const mockupCommunication = makeCommunication(EN_LABELS);
export const mockupValidation = makeValidation(EN_LABELS);

export const mockupAppShellIt = makeAppShell(IT_LABELS);
export const mockupContextMenuIt = makeContextMenu(IT_LABELS);
export const mockupCommunicationIt = makeCommunication(IT_LABELS);
export const mockupValidationIt = makeValidation(IT_LABELS);

export const mockupAppShellFr = makeAppShell(FR_LABELS);
export const mockupContextMenuFr = makeContextMenu(FR_LABELS);
export const mockupCommunicationFr = makeCommunication(FR_LABELS);
export const mockupValidationFr = makeValidation(FR_LABELS);

export const mockupAppShellEs = makeAppShell(ES_LABELS);
export const mockupContextMenuEs = makeContextMenu(ES_LABELS);
export const mockupCommunicationEs = makeCommunication(ES_LABELS);
export const mockupValidationEs = makeValidation(ES_LABELS);

export const mockupAppShellDe = makeAppShell(DE_LABELS);
export const mockupContextMenuDe = makeContextMenu(DE_LABELS);
export const mockupCommunicationDe = makeCommunication(DE_LABELS);
export const mockupValidationDe = makeValidation(DE_LABELS);
