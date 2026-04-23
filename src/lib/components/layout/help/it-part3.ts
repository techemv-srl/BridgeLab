import type { ManualSection } from '../helpContent';

export const itPart3: ManualSection[] = [
{
	id: 'fhir',
	heading: 'Supporto FHIR',
	body: `
<p>BridgeLab rileva automaticamente le risorse FHIR quando incolli o
apri un file il cui primo carattere non bianco è <code>{</code> e che
contiene <code>"resourceType"</code>. Il tree passa a una vista
specifica FHIR che mostra la gerarchia della risorsa come path JSON.</p>

<h3>Formati supportati</h3>
<ul>
	<li><strong>JSON</strong> - Patient, Observation, Bundle,
		DiagnosticReport, MedicationRequest e ogni altra risorsa FHIR
		R4/R5.</li>
	<li><strong>XML</strong> - le stesse risorse in codifica XML
		(<code>&lt;Patient xmlns="http://hl7.org/fhir"&gt;</code>).</li>
</ul>

<h3>Visualizzatore Bundle (Pro)</h3>
<p><strong>Strumenti → Visualizzatore Bundle FHIR</strong> apre una
vista a tre pannelli quando il messaggio attivo è un Bundle:</p>
<ul>
	<li><strong>Pannello sinistro:</strong> elenco delle entry con tipo
		risorsa, nome (es. nome Patient, codice Observation) e numero
		di referenze in entrata.</li>
	<li><strong>Pannello centrale:</strong> referenze uscenti
		dall'entry selezionata - ogni campo
		<code>reference</code> diventa un link cliccabile che porta
		all'entry destinazione.</li>
	<li><strong>Pannello destro:</strong> JSON grezzo della risorsa
		selezionata, con evidenziazione della sintassi.</li>
</ul>
<p>Le <strong>referenze pendenti</strong> (che puntano a entry assenti
nel Bundle) sono segnalate con un badge rosso.</p>

<h3>Valutatore FHIRPath (Pro)</h3>
<p><kbd>Ctrl</kbd>+<kbd>P</kbd> o <strong>Strumenti → Valutatore
FHIRPath</strong> apre una console interattiva dove digiti espressioni
FHIRPath sulla risorsa corrente. Operatori supportati:</p>
<ul>
	<li><strong>Navigazione:</strong> <code>Patient.name.family</code>,
		<code>Bundle.entry.resource</code></li>
	<li><strong>Indicizzazione:</strong>
		<code>Patient.name[0].given</code></li>
	<li><strong>Filtri:</strong>
		<code>Bundle.entry.where(resource.resourceType = 'Patient')</code></li>
	<li><strong>Aggregati:</strong> <code>count()</code>,
		<code>first()</code>, <code>last()</code>,
		<code>distinct()</code></li>
	<li><strong>Proiezione:</strong>
		<code>Bundle.entry.select(resource.id)</code></li>
</ul>
<p>Le espressioni recenti sono nello storico per il riuso veloce.</p>

<h3>Validazione FHIR</h3>
<p>F6 funziona anche per le risorse FHIR. Gli errori evidenziano campi
obbligatori mancanti (es. <code>Patient.identifier</code>), tipi di
dato non validi (gender fuori dal value set) e problemi
strutturali.</p>
`,
},
{
	id: 'plugins',
	heading: 'Plugin Pack',
	body: `
<p>I plugin pack ti permettono di estendere validator e anonymizer di
BridgeLab <strong>senza scrivere codice</strong> e senza permettere
alcuna esecuzione di codice. Ogni pack è un file JSON in una cartella
utente.</p>

<h3>Dove vivono i plugin</h3>
<p>Clicca <strong>Impostazioni → Plugin → Apri cartella plugin</strong>
per aprire la directory nel file manager. Il layout è:</p>
<pre><code>&lt;config&gt;/BridgeLab/plugins/
├── validation/
│   ├── ospedale-adt-rules.json
│   └── z-segment-checks.json
└── anonymization/
    └── codice-fiscale-it.json</code></pre>

<p>Su Windows la radice è
<code>%APPDATA%\\BridgeLab\\plugins</code>, su macOS
<code>~/Library/Application Support/BridgeLab/plugins</code>, su Linux
<code>~/.config/BridgeLab/plugins</code>.</p>

<h3>Pack di regole di validazione</h3>
<pre><code>{
  "id": "acme-adt-01",
  "name": "Regole ADT specifiche ACME",
  "description": "Campi obbligatori interni",
  "version": "1.0",
  "enabled": true,
  "validation_rules": [
    {
      "rule_id": "ACME-PID-001",
      "severity": "error",
      "segment": "PID",
      "field": 3,
      "check": { "type": "not_empty" },
      "message": "PID-3 (Patient ID) è obbligatorio"
    }
  ]
}</code></pre>

<h3>Tipi di check supportati</h3>
<table>
	<tr><th>Check</th><th>Parametri</th><th>Esempio</th></tr>
	<tr><td><code>not_empty</code></td><td>—</td>
		<td>Campo deve essere valorizzato.</td></tr>
	<tr><td><code>regex</code></td><td><code>pattern</code></td>
		<td>Cognome inizia con maiuscola.</td></tr>
	<tr><td><code>one_of</code></td><td><code>values[]</code></td>
		<td>Patient class deve essere I, O, E.</td></tr>
	<tr><td><code>max_length</code></td><td><code>max</code></td>
		<td>MRN ≤ 16 caratteri.</td></tr>
	<tr><td><code>min_length</code></td><td><code>min</code></td>
		<td>SSN ≥ 9 cifre.</td></tr>
	<tr><td><code>contains</code></td><td><code>value</code></td>
		<td>Visit number deve contenere un trattino.</td></tr>
</table>
<p>Aggiungi <code>"component": 1</code> per restringere la regola a un
componente specifico (es. cognome dentro PID-5.1).</p>

<h3>Pack di regole di anonimizzazione</h3>
<pre><code>{
  "id": "eu-extra-phi",
  "name": "Campi PHI EU aggiuntivi",
  "enabled": true,
  "phi_rules": [
    { "segment": "PID", "field": 25, "sensitivity": "high",
      "name": "Codice fiscale" }
  ]
}</code></pre>

<h3>Gestione dei pack</h3>
<p><strong>Impostazioni → Plugin</strong> elenca ogni pack con autore,
versione, numero regole e percorso. Attiva/disattiva singoli pack
(la scelta è persistita), clicca <em>Ricarica</em> dopo aver
modificato un file, oppure <em>Apri cartella plugin</em> per editare
nel tuo IDE preferito.</p>

<div class="note">I file che falliscono il parsing appaiono con un
banner di errore rosso ma non rompono il registry - gli altri pack
continuano a funzionare.</div>
`,
},
{
	id: 'licensing',
	heading: 'Licenza',
	body: `
<p>BridgeLab include tre livelli. La divisione delle funzionalità è
pensata perché gli utenti Community possano fare lavoro HL7 quotidiano
reale per sempre, mentre Pro ed Enterprise sbloccano funzioni utili a
team di integrazione e ospedali.</p>

<table>
	<tr><th>Funzionalità</th><th>Community</th><th>Pro</th><th>Enterprise</th></tr>
	<tr><td>Editor HL7 v2.x, parser, validazione</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Parsing FHIR + tree</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Invio MLLP, HTTP GET</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Rilevamento PHI (solo visualizzazione)</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Plugin pack (base)</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Listener MLLP</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>HTTP POST/PUT/DELETE + auth</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Mascheramento anonimizzazione</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Export JSON/CSV</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Valutatore FHIRPath + Visualizzatore Bundle</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Plugin e test case illimitati</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>SOAP + supporto prioritario</td>
		<td>—</td><td>—</td><td>✓</td></tr>
</table>

<h3>Trial</h3>
<p>Al primo avvio parte un <strong>trial Pro di 7 giorni</strong> con
ogni funzionalità Pro abilitata. Il banner trial (giallo) è
chiudibile; quando restano 3 giorni diventa rosso e non si può più
chiudere come promemoria.</p>

<p>Quando il trial scade BridgeLab <strong>non smette di
funzionare</strong> - torna al livello Community e il banner ti invita
all'upgrade. Messaggi, impostazioni, plugin e test case restano
intatti.</p>

<h3>Attivazione</h3>
<p>Apri il dialog di attivazione da:</p>
<ul>
	<li><strong>Impostazioni → Licenza → Attiva</strong></li>
	<li><strong>Aiuto → Attiva una licenza</strong></li>
	<li>Il pulsante <em>Aggiorna</em> sul banner trial</li>
</ul>

<p>Per ottenere una chiave di licenza, scrivi a
<a href="mailto:info@techemv.it">info@techemv.it</a> con il tuo
<strong>Hardware ID</strong> (mostrato nel dialog di attivazione e in
Impostazioni → Licenza). TECHEMV SRL genera una licenza firmata
vincolata alla tua macchina e te la rispedisce. Incollala nel campo
chiave; il dialog mostra il nome del licensee e i diritti prima
dell'attivazione.</p>

<h3>Verifica offline</h3>
<p>Dopo la prima attivazione la verifica della licenza è puramente
locale - non serve nessuna chiamata di rete. La chiave porta una firma
Ed25519 che l'app verifica contro una public key embedded.</p>
`,
},
{
	id: 'shortcuts',
	heading: 'Scorciatoie da tastiera',
	body: `
<p>Le scorciatoie di BridgeLab sono configurabili da
<strong>Impostazioni → Scorciatoie</strong>. Clicca un binding, premi
una nuova combinazione di tasti, conferma con OK.</p>

<h3>Default</h3>
<table>
	<tr><td><kbd>Ctrl</kbd>+<kbd>O</kbd></td><td>Apri file</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>N</kbd></td><td>Nuovo da modello</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>L</kbd></td><td>Test Case Library</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>S</kbd></td><td>Salva</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>S</kbd></td><td>Salva con nome</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>W</kbd></td><td>Chiudi tab</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>B</kbd></td><td>Mostra/nascondi tree</td></tr>
	<tr><td><kbd>F5</kbd></td><td>Ri-analizza messaggio</td></tr>
	<tr><td><kbd>F6</kbd></td><td>Valida</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>K</kbd></td><td>Pannello comunicazione</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>P</kbd></td><td>Pannello FHIRPath</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>,</kbd></td><td>Impostazioni</td></tr>
	<tr><td><kbd>F1</kbd></td><td>Questo manuale</td></tr>
	<tr><td><kbd>Alt</kbd>+<kbd>T</kbd></td><td>Mostra nel Tree (menu contestuale editor)</td></tr>
	<tr><td><kbd>Alt</kbd>+<kbd>C</kbd></td><td>Copia Segmento (menu contestuale editor)</td></tr>
</table>

<h3>Rilevamento conflitti</h3>
<p>Se scegli una combinazione già assegnata a un'altra azione, l'editor
ti avverte - conferma per trasferire il binding o scegli tasti diversi.
Le scorciatoie native di Monaco
(<kbd>Ctrl</kbd>+<kbd>F</kbd>, <kbd>Ctrl</kbd>+<kbd>D</kbd>, ...) hanno
la precedenza quando l'editor ha il focus.</p>

<h3>Reset</h3>
<p>Clicca <em>Ripristina Tutto</em> per ripristinare ogni scorciatoia
al default, o il piccolo pulsante ↺ accanto a una voce per
ripristinare solo quella.</p>
`,
},
];
