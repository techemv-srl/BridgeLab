import type { ManualSection } from '../helpContent';

export const itPart3: ManualSection[] = [
{
	id: 'schema-export',
	heading: 'Esportazione schema (XSD)',
	body: `
<p>Serve un XSD per descrivere un messaggio HL7 v2 in una pipeline
XML-based, per un'integrazione contract-first, o per caricarlo in un
tool di terze parti? Apri <strong>Strumenti → Esporta schema messaggio
come XSD…</strong> — scegli una versione HL7 e un tipo di messaggio,
vedi l'anteprima e salva con un clic.</p>

<h3>Cosa ottieni</h3>
<p>Un XSD autoconsistente che segue la struttura degli elementi di HL7
v2.xml (segmenti, campi e componenti con i nomi di v2.xml), nella forma
senza namespace usata da Astraia e strumenti simili: lo schema non
dichiara un <code>targetNamespace</code>, quindi un'istanza non deve
portare il namespace di v2.xml (<code>urn:hl7-org:v2xml</code>) per
essere valida.</p>
<ul>
	<li>Un elemento root per ogni messaggio (es.
		<code>ADT_A01</code>) con un complex type inline che elenca
		segmenti e gruppi di segmenti in ordine.</li>
	<li>Ogni segmento dichiarato come <code>xsd:complexType</code>
		top-level (<code>MSH</code>, <code>PID</code>,
		<code>OBX</code>, …) con ogni campo tipizzato secondo il
		data-type HL7 (<code>XPN</code>, <code>CX</code>,
		<code>HD</code>, …).</li>
	<li>Data type compositi espansi nei loro componenti, data type
		primitivi (<code>ST</code>, <code>ID</code>, <code>NM</code>, …)
		emessi come <code>xsd:simpleType</code> con restrizione su
		<code>xsd:string</code>.</li>
	<li>Cardinalità preservata: <code>minOccurs="0"</code> per i
		campi opzionali, <code>maxOccurs="unbounded"</code> per quelli
		ripetibili.</li>
	<li>Gruppi come <code>ORM_O01.ORDER_DETAIL</code> nella
		convenzione <code>MESSAGGIO.GRUPPO</code>; blocchi di choice
		HL7 (<code>OBR | RQD | RQ1 | RXO | ODS | ODT</code>) emessi
		come <code>xsd:choice</code>.</li>
	<li>Compilazione garantita con i processori di schema strict — le
		poche strutture HL7 la cui definizione viola la regola Unique
		Particle Attribution di XSD sono emesse come choice rilassata
		annotata.</li>
</ul>

<h3>Azioni</h3>
<ul>
	<li><strong>Copia</strong> — copia l'XSD negli appunti.</li>
	<li><strong>Salva con nome…</strong> — apre la dialog di sistema
		con <code>{MESSAGGIO}.xsd</code> come nome suggerito.</li>
</ul>

<h3>Copertura e tier</h3>
<p>Dieci versioni HL7 sono incluse al completo: <strong>2.1, 2.2, 2.3,
2.3.1, 2.4, 2.5, 2.5.1, 2.6, 2.7 e 2.7.1</strong> — 2.320 strutture di
messaggio selezionabili dal dropdown delle versioni.</p>
<p>HL7 v2.7.1 è una release di correzione tecnica della 2.7 e usa le
stesse definizioni di messaggio: nel dropdown è quindi marcata
<strong>(= v2.7)</strong> ed esporta dal catalogo 2.7.</p>
<p>Il tier free esporta quattro message type ad alto uso in HL7 v2.5,
così il workflow tipico di debug MLLP è coperto:</p>
<ul>
	<li><strong>ADT^A01</strong> — Admit / Visit Notification</li>
	<li><strong>ADT^A40</strong> — Merge Patient (Patient Identifier
		List)</li>
	<li><strong>ORM^O01</strong> — Order Message</li>
	<li><strong>ORU^R01</strong> — Unsolicited Observation Result</li>
</ul>
<p>Ogni altro message type, o ogni altra versione HL7, è marcato
<strong>(PRO)</strong> nel dropdown e richiede una licenza Professional
(o un trial attivo). Se provi a esportare una voce gated, BridgeLab
mostra un prompt di upgrade con link a
<strong>Aiuto → Acquista una licenza</strong> (la pagina prezzi) o
<strong>Aiuto → Attiva Licenza</strong>.</p>

<h3>Nota sul licensing</h3>
<p>BridgeLab non ridistribuisce alcun file XSD coperto da copyright
HL7. I metadati di schema sono ricostruiti da specifiche HL7 v2
pubbliche. HL7® è lo standard sorgente e l'output è un derivative work
per scopi di interoperabilità. I file generati non hanno un header di
commento; aggiungi tu una nota se la tua policy la richiede.</p>

<div class="info">Target ideale: Astraia e applicazioni di
integrazione simili che accettano XSD hand-authored per message type
non nativamente riconosciuti dal motore. Esporti una volta, carichi
nell'engine, vai avanti.</div>
`,
},
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
		R4/R5. La validazione segue FHIR R4: una risorsa R5 si apre, si
		naviga e si interroga con FHIRPath, ma gli elementi cambiati in R5
		vengono segnalati secondo R4.</li>
	<li><strong>XML</strong> - le stesse risorse in codifica XML
		(<code>&lt;Patient xmlns="http://hl7.org/fhir"&gt;</code>).</li>
</ul>

<h3>Visualizzatore Bundle (Pro)</h3>
<p><strong>Strumenti → Visualizzatore Bundle FHIR</strong> apre una
vista a due pannelli quando il messaggio attivo è un Bundle:</p>
<ul>
	<li><strong>Pannello sinistro:</strong> elenco delle entry con tipo
		risorsa, nome (es. nome Patient, codice Observation), metodo della
		richiesta e numero di referenze che l'entry contiene.</li>
	<li><strong>Pannello destro:</strong> l'entry selezionata — i suoi
		dettagli, le referenze che contiene (ognuna un link che porta
		all'entry destinazione), le entry che la referenziano e il suo JSON
		grezzo.</li>
</ul>
<p>Le <strong>referenze pendenti</strong> (che puntano a entry assenti
nel Bundle) hanno un badge <em>orfano</em> e sono contate
nell'intestazione. Una referenza <code>#id</code> a una risorsa contenuta
nell'entry non lo è.</p>
<p>Il toggle <strong>Lista / Grafo</strong> passa a un grafo delle
referenze: ogni entry è un nodo (colorato per tipo di risorsa), ogni
<code>reference</code> una freccia orientata. Clicca un nodo per
selezionarlo — il pannello di dettaglio segue la selezione. Disponibile
fino a 150 entry; per i Bundle più grandi si usa la lista.</p>

<h3>Valutatore FHIRPath (Pro)</h3>
<p><kbd>Ctrl</kbd>+<kbd>P</kbd> o <strong>Strumenti → Valutatore
FHIRPath</strong> apre una console interattiva dove digiti espressioni
FHIRPath sulla risorsa corrente.</p>
<p>Il valutatore implementa il linguaggio FHIRPath 2.0: l'insieme
completo degli operatori con la precedenza e la logica a tre valori
della specifica, e circa settanta funzioni.</p>
<ul>
	<li><strong>Navigazione:</strong> <code>Patient.name.family</code>,
		<code>Bundle.entry.resource</code>; gli elementi choice si
		raggiungono col nome base — <code>Observation.value</code> trova
		<code>valueQuantity</code>. <code>ofType()</code>, <code>is</code> e
		<code>as</code> su un elemento a scelta seguono il tipo nel suo nome:
		<code>Observation.effective.ofType(dateTime)</code> trova
		<code>effectiveDateTime</code> anche quando contiene solo un giorno.
		Altrove il tipo è dedotto dal valore JSON</li>
	<li><strong>Indicizzazione:</strong>
		<code>Patient.name[0].given</code></li>
	<li><strong>Filtri e proiezione:</strong> <code>where()</code>,
		<code>select()</code>, <code>repeat()</code>,
		<code>ofType()</code>, con <code>$this</code> e
		<code>$index</code> disponibili all'interno</li>
	<li><strong>Collezioni:</strong> <code>count()</code>,
		<code>first()</code>, <code>last()</code>, <code>tail()</code>,
		<code>skip()</code>, <code>take()</code>, <code>distinct()</code>,
		<code>sort()</code>, <code>union()</code>, <code>combine()</code>,
		<code>intersect()</code>, <code>exclude()</code>,
		<code>aggregate()</code></li>
	<li><strong>Logica:</strong> <code>and</code>, <code>or</code>,
		<code>xor</code>, <code>implies</code>, <code>not()</code>,
		<code>exists()</code>, <code>all()</code>, <code>iif()</code> —
		con la collezione vuota come valore "sconosciuto"</li>
	<li><strong>Stringhe:</strong> <code>substring()</code>,
		<code>matches()</code>, <code>replace()</code>,
		<code>split()</code>, <code>join()</code>, <code>encode()</code>,
		<code>escape()</code> e simili</li>
	<li><strong>Date e quantità:</strong> letterali a precisione parziale
		(<code>@2015</code>, <code>@2015-02-04T14:34:28+10:00</code>),
		aritmetica con durate
		(<code>Patient.birthDate + 18 years</code>) e conversione di unità
		nella stessa dimensione (<code>4 'g' = 4000 'mg'</code>)</li>
	<li><strong>Estensioni FHIR:</strong> <code>extension(url)</code>,
		<code>hasValue()</code> e <code>resolve()</code>, che segue una
		Reference verso una risorsa contained o nel Bundle</li>
	<li><strong>Debug:</strong> <code>trace('etichetta')</code> lascia
		passare l'input e mostra i valori sotto il risultato, per vedere
		cosa produce un path lungo a metà strada</li>
</ul>
<p>Il confronto fra valori di precisione diversa restituisce la
collezione vuota invece di tirare a indovinare:
<code>@2015-02-04 = @2015-02</code> non è né vero né falso, perché il
secondo valore potrebbe essere quel giorno o un altro dello stesso
mese.</p>
<p>Le ultime espressioni valutate compaiono come pulsanti <em>Recenti</em>
accanto agli esempi (fino a quattro, finché l'app resta aperta). Quando il
testo nell'editor non si analizza più, il pannello e la barra di stato lo
segnalano: i risultati vengono allora dall'ultima versione valida della
risorsa.</p>

<h3>Validazione FHIR</h3>
<p>F6 funziona anche per le risorse FHIR. Gli errori evidenziano elementi
obbligatori mancanti (es. <code>Observation.status</code>), tipi di
dato non validi (gender diverso da male, female, other, unknown) e
problemi strutturali. I profili dichiarati in <code>meta.profile</code>
vengono applicati quando le loro definizioni sono disponibili (vedi
<em>Validazione dei profili</em> più avanti); le voci malformate sono
segnalate come avvisi.</p>

<h3>Validazione dei profili (Pro)</h3>
<p>Di default una risorsa FHIR viene controllata a livello strutturale:
c'è il <code>resourceType</code>, valgono le regole specifiche della
risorsa. Confrontarla con una <strong>StructureDefinition</strong> — la
definizione vera di cosa può contenere un Patient — richiede quelle
definizioni, che si distribuiscono come package FHIR NPM.</p>
<p>Il core FHIR R4 (<code>hl7.fhir.r4.core</code> 4.0.1) è
<strong>integrato</strong>: le definizioni base ci sono sempre — offline,
in ogni edizione, niente da scaricare. <strong>Strumenti → Package di
profili FHIR…</strong> installa sopra, da un <code>.tgz</code>, le guide di
implementazione nazionali o della tua struttura; un package installato che
porta le stesse definizioni sostituisce quelle integrate, una versione più
recente le scavalca.</p>
<p>Ogni validazione FHIR controlla, contro il core integrato e ciò che è
installato sopra:</p>
<ul>
	<li><strong>Cardinalità</strong> — un elemento obbligatorio assente, o un
		elemento <code>0..1</code> che si ripete.</li>
	<li><strong>Tipi degli elementi</strong> — un boolean scritto come
		stringa, un numero dove serve un oggetto.</li>
	<li><strong>Elementi choice</strong> — <code>value[x]</code> deve
		comparire come esattamente una fra <code>valueQuantity</code>,
		<code>valueString</code> e simili. Un <code>value</code> semplice, o
		due forme insieme, vengono segnalati.</li>
	<li><strong>Valori fissi e pattern</strong> — ciò che il profilo
		vincola.</li>
	<li><strong>Elementi sconosciuti</strong> — un nome che il profilo non
		definisce. È il controllo che intercetta un refuso come
		<code>genderr</code> o un elemento che appartiene a un altro tipo di
		risorsa.</li>
</ul>
<p>I profili dichiarati in <code>meta.profile</code> vengono applicati
automaticamente se il package che li definisce è installato. Se non lo è, il
validatore lo dice invece di riportare in silenzio un esito pulito — e
quando i profili sono stati applicati lo dice ugualmente, perché «nessun
finding» significa cose molto diverse nei due casi.</p>
<p><strong>La terminologia è fuori ambito.</strong> Un binding
<code>required</code> si può verificare solo espandendo il ValueSet, il che
richiede i package di terminologia o un server. BridgeLab lascia i binding
non verificati invece di verificarli a metà.</p>
<p>Installare un package richiede una licenza Professional. I package già
installati continuano a validare in ogni tier: un trial scaduto non fa mai
diventare rosse risorse che prima erano pulite.</p>

<h3>Regole FHIR personalizzate (Pro)</h3>
<p><strong>Strumenti → Regole di validazione FHIR…</strong> apre un editor
per i tuoi controlli. Girano insieme a quelli integrati a ogni validazione e
sono salvati come un normale plugin pack
(<code>plugins/fhir/user-rules.json</code>) che puoi copiare fra macchine o
mettere in un repository.</p>
<p>Una regola ha una di due forme:</p>
<ul>
	<li><strong>Espressione deve essere vera</strong> — un invariante
		FHIRPath, come scrive i propri vincoli lo standard FHIR:
		<code>identifier.exists()</code>, oppure
		<code>value.exists() xor dataAbsentReason.exists()</code>.</li>
	<li><strong>Percorso + controllo</strong> — un selettore FHIRPath più
		qualcosa da verificare sui valori selezionati: deve essere presente,
		quanti, corrisponde a un pattern, uno tra una lista, contiene un
		testo, o un limite di lunghezza.</li>
</ul>
<p>Imposta <strong>Tipo di risorsa</strong> per limitare la regola a
Patient, Observation e così via, o lascialo vuoto per applicarla a tutto.
Una regola limitata a un tipo scatta anche per le risorse corrispondenti
dentro un Bundle, con il finding riportato su
<code>entry[n].resource.…</code>.</p>
<p><strong>Prova sulla risorsa aperta</strong> esegue la regola in modifica
sulla risorsa della scheda attiva prima di salvarla, e mostra quali valori
il selettore ha davvero raccolto — il modo più rapido per distinguere una
regola che passa da una che non è mai stata eseguita. Se la risorsa aperta è
di un altro tipo, l'editor lo dice invece di segnalare un esito positivo.</p>
<p>Le regole già scritte funzionano in ogni tier; l'editor richiede una
licenza Professional. I pack scritti a mano sono documentati in
<code>docs/PLUGINS.md</code>.</p>
<p>Salva riscrive solo le regole: tutto il resto che hai scritto nel file
(uno <code>$schema</code>, un responsabile, un commento su una regola)
resta. Se il file è cambiato su disco mentre l'editor era aperto — una
regola di un collega, un <code>git pull</code> — Salva si rifiuta invece di
sovrascriverlo; chiudi e riapri l'editor per vedere le regole attuali.
Anche due regole con lo stesso id, o un pattern che non compila, vengono
rifiutati. Quando il pack è disattivato in <strong>Impostazioni →
Plugin</strong>, o escluso dal limite Community sui pack attivi, l'editor
lo dice: <em>Prova</em> mostra comunque cosa farebbe una regola, ma F6 non
la esegue.</p>

<h3>Template FHIR</h3>
<p><strong>File → Nuovo Messaggio da Template...</strong> include una categoria FHIR:
un Patient minimale, una Observation di pressione arteriosa con
componenti e un Bundle transaction le cui entry si referenziano a
vicenda tramite <code>urn:uuid</code> — aprilo e prova la vista a
grafo del Visualizzatore Bundle.</p>
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
├── fhir/
│   └── user-rules.json
└── anonymization/
    └── codice-fiscale-it.json</code></pre>

<p>Su Windows la radice è
<code>%APPDATA%\\BridgeLab\\plugins</code>, su macOS
<code>~/Library/Application Support/BridgeLab/plugins</code>, su Linux
<code>~/.config/BridgeLab/plugins</code>.</p>

<p>Nella cartella vivono tre tipi di pack: regole di validazione HL7 v2
(<code>validation/</code>, sotto), regole di validazione FHIR
(<code>fhir/</code> — un invariante FHIRPath, oppure un selettore più un
controllo; vedi <em>Supporto FHIR → Regole FHIR personalizzate</em>)
e campi PHI aggiuntivi per l'anonimizzatore (<code>anonymization/</code>,
sotto).</p>

<p><strong>Ogni tier ha l'intero meccanismo</strong> — tutti e tre i tipi,
ogni tipo di controllo, ricarica e attivazione per singolo pack. L'unica
differenza è quanti pack possono essere attivi contemporaneamente: fino a
<strong>3</strong> in Community, illimitati in Pro ed Enterprise. L'editor
in-app che scrive i pack FHIR è Pro; un pack FHIR scritto a mano gira in
Community come qualsiasi altro.</p>

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
    { "segment": "ZPI", "field": 2,  "sensitivity": "high",
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

<p class="note">Nel tier Community sono attivi al massimo
<strong>3 pack</strong> alla volta: i pack abilitati in eccesso
mostrano un badge "inattivo" e non forniscono regole finché non si
libera uno slot (disattiva un altro pack, oppure fai l'upgrade).</p>
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
	<tr><td>Invio MLLP, HTTP GET senza header di autenticazione</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Conformità al core FHIR R4 (integrato)</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Export XSD</td>
		<td>4 messaggi v2.5</td><td>Catalogo completo</td><td>Catalogo completo</td></tr>
	<tr><td>Rilevamento PHI (solo visualizzazione)</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Plugin pack (tutti i tipi, ogni controllo)</td>
		<td>3 attivi alla volta</td><td>Illimitati</td><td>Illimitati</td></tr>
	<tr><td>Listener MLLP</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>HTTP POST/PUT/DELETE/PATCH, e autenticazione su qualsiasi metodo</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Mascheramento anonimizzazione</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Export JSON/CSV</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Validazione batch</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Package di profili FHIR e rules builder</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Valutatore FHIRPath + Visualizzatore Bundle</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Test case salvati</td>
		<td>10</td><td>Illimitati</td><td>Illimitati</td></tr>
	<tr><td>SOAP + supporto prioritario</td>
		<td>—</td><td>—</td><td>✓</td></tr>
</table>

<p class="note">La tabella riguarda l'app desktop. La
<code>bridgelab-cli</code> gratuita non legge licenze: i suoi comandi
<code>anonymize</code>, <code>batch</code>, <code>to-json</code>,
<code>fhirpath</code> e i controlli con i pacchetti FHIR funzionano in
ogni edizione; solo l'export XSD è limitato al set Community, e applica
il limite di 3 plugin pack.</p>

<p class="note">Community mantiene fino a <strong>3 plugin pack
attivi</strong> e <strong>10 test case salvati</strong>. Nulla viene
mai bloccato o cancellato: i test case salvati oltre il limite (es.
durante un trial) restano visibili, modificabili ed eseguibili; i
plugin pack oltre il limite di 3 restano installati e visibili ma
appaiono inattivi e non applicano regole finché non si libera uno slot
o non si passa a una licenza superiore. Solo i nuovi salvataggi e le
nuove attivazioni oltre il limite chiedono l'upgrade.</p>

<h3>Trial</h3>
<p>Al primo avvio parte un <strong>trial Pro di 14 giorni</strong> con
ogni funzionalità Pro abilitata. Il banner trial (giallo) è
chiudibile; quando restano 3 giorni diventa rosso e non si può più
chiudere come promemoria.</p>

<p>Quando il trial scade BridgeLab <strong>non smette di
funzionare</strong> - torna al livello Community e il banner ti invita
all'upgrade. Messaggi, impostazioni, plugin e test case restano
intatti.</p>

<h3>Aggiornamenti</h3>
<p>Non viene richiesto nulla finché non decidi. Al primo avvio BridgeLab
chiede, con un banner in cima alla finestra, se può cercare nuove
versioni (il programma di installazione Windows, il <code>.exe</code>, lo chiede già durante l'installazione, e in
quel caso l'app non lo richiede). <em>Sì, controlla</em> e <em>No</em>
vengono salvati in Impostazioni → Privacy; chiudendo il banner senza
rispondere la domanda torna al prossimo avvio.</p>
<p>Una volta al giorno, qualche secondo dopo l'avvio, BridgeLab chiede a
GitHub (<code>api.github.com</code>) l'ultima release. Se ce n'è una più
recente compare un banner con <em>Scarica</em> (apre la pagina della
release), <em>Salta questa versione</em> e il pulsante di chiusura; non
viene installato nulla in automatico. La richiesta non contiene nulla su
di te, sul computer o sui tuoi file, e senza accesso a internet viene
saltata in silenzio. Si disattiva da <strong>Impostazioni → Privacy →
Controlla nuove versioni all'avvio</strong>; <strong>Aiuto → Controlla
Aggiornamenti</strong> funziona sempre su richiesta.</p>
<p>L'installer Windows pone la stessa domanda alla prima installazione
(<em>Sì</em> è la risposta predefinita; un'installazione silenziosa non
chiede nulla, e nemmeno il pacchetto <code>.msi</code>: allora lo chiede
l'app nel banner). Sulle macchine gestite l'amministratore può disattivare il
controllo per tutti gli utenti, e la casella nelle Impostazioni appare
bloccata: basta la variabile d'ambiente
<code>BRIDGELAB_DISABLE_UPDATE_CHECK=1</code>, oppure un file
<code>policy.json</code> con <code>{"disable_update_check": true}</code> in
<code>%ProgramData%\\BridgeLab\\</code> (Windows),
<code>/Library/Application Support/BridgeLab/</code> (macOS) o
<code>/etc/bridgelab/</code> (Linux).</p>

<h3>Acquistare una licenza</h3>
<p><strong>Aiuto → Acquista una licenza…</strong> apre nel browser la
sezione prezzi del sito di BridgeLab, dove Professional ed Enterprise si
acquistano online con carta; il codice di attivazione arriva via
e-mail. La stessa pagina è a un clic dai pulsanti <em>Prezzi e
acquisto</em> del dialog di attivazione, dal pulsante <em>Confronta i
piani</em> del banner trial. Serve fattura, ordine
d'acquisto o preventivo? Scrivi a
<a href="mailto:info@techemv.it">info@techemv.it</a>.</p>

<h3>Attivazione</h3>
<p>Apri il dialog di attivazione da:</p>
<ul>
	<li><strong>Impostazioni → Attivazione Licenza</strong></li>
	<li><strong>Aiuto → Attiva Licenza</strong></li>
	<li>Il pulsante <em>Attiva</em> sul banner trial</li>
</ul>

<p><strong>Attivazione online (predefinita):</strong> dopo l'acquisto
ricevi via email un codice di attivazione tipo
<code>BL-PRO-XXXX-XXXX-XXXX</code>. Incollalo nel campo chiave: l'app
lo scambia con una singola chiamata HTTPS per una licenza firmata
vincolata a questa macchina. Usa <em>Disattiva</em> per liberare la
postazione prima di passare a un altro computer.</p>

<p>Hai rinnovato l'abbonamento? Premi <em>Aggiorna licenza</em> nel
dialog per recuperare subito la nuova scadenza — oppure non fare
nulla: entro 14 giorni dalla scadenza l'app la recupera da sola
all'avvio, in silenzio (mai errori sulle macchine offline).</p>

<p><strong>Chiave offline (siti isolati / air-gapped):</strong> scrivi a
<a href="mailto:info@techemv.it">info@techemv.it</a> con il tuo
<strong>Hardware ID</strong> (mostrato sotto "Ti serve una chiave
offline?" nel dialog di attivazione e in Impostazioni → Attivazione Licenza).
TECHEMV SRL ti rispedisce una licenza firmata vincolata alla tua
macchina — non serve mai l'accesso a internet. Il dialog mostra il
nome del licensee e i diritti prima dell'attivazione.</p>

<h3>Verifica offline</h3>
<p>Qualunque flusso tu abbia usato, la verifica ordinaria della
licenza è puramente locale - l'app non ha mai bisogno di contattare il
server licenze per continuare a funzionare. Il server viene
contattato solo per: l'attivazione con codice; il controllo automatico
all'avvio per le licenze attivate online - una volta alla settimana, e
una volta al giorno nei 14 giorni prima della scadenza e dopo - che
recepisce un rinnovo e, se il codice è stato revocato (per esempio dopo
un rimborso), riporta l'app a Community; la liberazione della
postazione con <em>Disattiva</em>; e le statistiche d'uso opt-in. Se il
server non è raggiungibile, il controllo non cambia nulla. La chiave porta una firma Ed25519 che l'app
verifica contro una public key embedded.</p>

<h3>Privacy e statistiche d'uso</h3>
<p>BridgeLab può inviare <strong>statistiche d'uso</strong> a TECHEMV
— disattivate di default, attivabili in
<strong>Impostazioni → Privacy</strong>. Se attive, l'invio automatico
avviene al massimo una volta al giorno; il pulsante <em>Invia ora</em>
trasmette subito. Ogni invio contiene nome del prodotto, versione
dell'app, sistema operativo e architettura del processore, lingua
dell'interfaccia, tipo di licenza, giorni dall'installazione, numero di
plugin pack attivi, contatori d'uso, data e ora dell'invio, un
<strong>ID di installazione casuale</strong> e — solo per licenze
attivate online — il <strong>codice di attivazione</strong> (usato per segnalare una
licenza revocata). I dati sono quindi <strong>pseudonimi</strong>, non
del tutto anonimi: non vengono mai inviati contenuti dei messaggi,
nomi di file, nomi host, nomi utente o dati dei pazienti, e il JSON
esatto è ispezionabile con <em>Mostra cosa viene inviato</em>. Con
l'interruttore spento (il default) non viene trasmesso nulla, e i
problemi di rete non producono mai errori: un'installazione
completamente offline è uno scenario normale e supportato.</p>
<p>Sui computer gestiti l'amministratore può forzare le statistiche
d'uso spente per tutti gli utenti: imposta
<code>BRIDGELAB_DISABLE_TELEMETRY=1</code>, oppure aggiungi
<code>"disable_telemetry": true</code> allo stesso <code>policy.json</code>
usato per il controllo versioni. In quel caso non viene inviato nulla,
qualunque sia la scelta dell'utente, e la casella in Impostazioni appare
bloccata.</p>
`,
},
{
	id: 'shortcuts',
	heading: 'Scorciatoie da tastiera',
	body: `
<p>Le scorciatoie di BridgeLab sono configurabili da
<strong>Impostazioni → Scorciatoie da Tastiera</strong>. Clicca un binding, premi
una nuova combinazione di tasti, conferma con OK.</p>

<h3>Default</h3>
<table>
	<tr><td><kbd>Ctrl</kbd>+<kbd>O</kbd></td><td>Apri file</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>N</kbd></td><td>Nuovo da template</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>L</kbd></td><td>Test Case Library</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>S</kbd></td><td>Salva</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>S</kbd></td><td>Salva con nome</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>W</kbd></td><td>Chiudi tab</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>B</kbd></td><td>Mostra/nascondi tree</td></tr>
	<tr><td><kbd>F6</kbd></td><td>Valida</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>K</kbd></td><td>Pannello comunicazione</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>P</kbd></td><td>Pannello FHIRPath</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>J</kbd></td><td>Pannello validazione</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd></td><td>Griglia segmenti</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>,</kbd></td><td>Impostazioni</td></tr>
	<tr><td><kbd>F1</kbd></td><td>Questo manuale</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>P</kbd></td><td>Palette dei comandi dell'editor (tutti i comandi di Monaco: vai alla riga, piega, maiuscole/minuscole, ordina righe…; anche nel menu contestuale dell'editor)</td></tr>
	<tr><td><kbd>Alt</kbd>+<kbd>T</kbd></td><td>Mostra Segmento nel Tree (menu contestuale editor)</td></tr>
	<tr><td><kbd>Alt</kbd>+<kbd>C</kbd></td><td>Copia Segmento (menu contestuale editor)</td></tr>
</table>

<p>Su macOS, <kbd>Ctrl</kbd> in queste scorciatoie è <kbd>⌘</kbd> (Comando) e <kbd>Alt</kbd> è <kbd>⌥</kbd> (Opzione), e l'elenco nelle Impostazioni le mostra così. Lì Sostituisci dell'editor è <kbd>⌘⌥F</kbd>, Ripeti <kbd>⌘⇧Z</kbd> e Vai alla riga <kbd>⌃G</kbd> (Control).</p>

<h3>Rilevamento conflitti</h3>
<p>Se scegli una combinazione già assegnata a un'altra azione, l'editor
ti avverte - conferma per trasferire il binding o scegli tasti diversi.
Le scorciatoie di BridgeLab funzionano anche nell'editor: se ne assegni
una a un tasto che usa anche l'editor (per esempio
<kbd>Ctrl</kbd>+<kbd>G</kbd>), l'avviso te lo dice e dentro l'editor
vince l'azione di BridgeLab. I tasti propri dell'editor
(<kbd>Ctrl</kbd>+<kbd>F</kbd>, <kbd>Ctrl</kbd>+<kbd>D</kbd>, ...) sono
elencati come riferimento e non si possono riassegnare. Una scorciatoia
richiede <kbd>Ctrl</kbd> o <kbd>Alt</kbd>, oppure è uno tra
<kbd>F2</kbd>-<kbd>F12</kbd>: una lettera da sola scatterebbe mentre
scrivi. <kbd>F1</kbd> apre sempre questo manuale. <em>Ri-analizza
messaggio</em> non ha un tasto predefinito; assegnagliene uno se ti
serve.</p>

<h3>Reset</h3>
<p>Clicca <em>Ripristina Tutto</em> per ripristinare ogni scorciatoia
al default, o il piccolo pulsante ↺ accanto a una voce per
ripristinare solo quella.</p>
`,
},
];
