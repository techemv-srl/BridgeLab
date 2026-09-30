import type { ManualSection } from '../helpContent';
import { mockupValidationIt as mockupValidation, mockupCommunicationIt as mockupCommunication } from './mockups';

export const itPart2: ManualSection[] = [
{
	id: 'validation',
	heading: 'Validazione',
	body: `
<p>Premi <kbd>F6</kbd> o scegli <strong>Strumenti → Valida</strong> per
eseguire tutte le regole di validazione sul messaggio attivo. I
risultati appaiono nel pannello Validazione in basso, raggruppati per
gravità.</p>

${mockupValidation}

<h3>Regole integrate</h3>
<ul>
	<li><strong>Struttura:</strong> il primo segmento deve essere MSH;
		i codici di segmento devono essere 3 caratteri alfanumerici;
		un secondo MSH (due messaggi in un solo testo) viene segnalato
		(STRUCT-004). Un codice di più di tre caratteri (<code>PIDX</code>)
		viene segnalato, mai letto come <code>PID</code>; un file batch può
		aprirsi con FHS/BHS prima dell'MSH.</li>
	<li><strong>Header MSH:</strong> MSH-9 (tipo messaggio), MSH-10
		(control ID), MSH-12 (versione) sono obbligatori. Una versione MSH-12
		che BridgeLab non conosce (una futura v2.8, un refuso) viene
		validata con il catalogo più vicino, v2.5 se non ce n'è, e una nota
		informativa (MSH-005) lo indica.</li>
	<li><strong>Campi obbligatori:</strong> campi richiesti per segmento
		presi dallo standard HL7 (es. PID-3 Patient Identifier
		List).</li>
	<li><strong>Lunghezze:</strong> avviso quando un campo supera il
		<code>max_length</code> pubblicato, contando i caratteri, per ogni
		ripetizione di un campo ripetuto.</li>
	<li><strong>Tipi di dato</strong> (avvisi): numeri (SI, NM), date
		(DT, <code>YYYY[MM[DD]]</code>), timestamp (TS/DTM,
		<code>YYYY[MM[DD[HH[MM[SS[.S]]]]]][+/-ZZZZ]</code>) e orari (TM)
		sono controllati nel formato e rispetto a calendario e orologio,
		quindi <code>19801399</code> o <code>2024-01-01</code> vengono
		segnalati. OBX-5 è controllato come il tipo dichiarato in OBX-2
		(NM, DT, TS…). Gli altri tipi non hanno controlli di formato.</li>
</ul>

<h3>Filtri e navigazione</h3>
<p>Clicca i badge Errore / Avviso / Info per filtrare. Clicca su una
riga del problema per selezionare il segmento o il campo nell'editor e
nel tree. Ogni riga indica il segmento con il suo numero, come il tree
(<code>OBX (4)</code>), così due problemi uguali su due segmenti si
distinguono. Se modifichi il messaggio dopo una verifica, il report viene
rifatto al successivo parsing in background; con l'analisi automatica
disattivata, il pannello segnala che il report non è aggiornato finché non
premi <kbd>F6</kbd>.</p>

<h3>Regole custom tramite plugin</h3>
<p>Inserisci un file JSON sotto
<code>&lt;config&gt;/BridgeLab/plugins/validation/</code> per aggiungere
controlli tuoi senza ricompilare. Vedi <em>Plugin</em> più sotto.</p>

<h3>Validazione batch (Pro)</h3>
<p><strong>Strumenti → Validazione batch…</strong> valida in un colpo
solo un'intera cartella (o una selezione) di file
<code>.hl7</code>/<code>.txt</code>/<code>.dat</code>: una riga per
file con tipo di messaggio, versione, numero di segmenti e totale
errori/avvisi. Filtra i soli falliti, clicca una riga per aprire il
file nell'editor ed esporta la tabella in CSV per il ticket di
revisione. I file sono elaborati in memoria — nulla viene aggiunto ai
tuoi tab.</p>

<h3>Generatore di messaggi di test</h3>
<p><strong>Strumenti → Genera messaggi di test…</strong> crea messaggi
ADT/ORU/ORM sintatticamente validi con dati paziente
<em>sintetici</em> plausibili — nomi, date di nascita, MRN, indirizzi e
pannelli di laboratorio con range di riferimento (una quota realistica
di risultati è volutamente fuori range e flaggata). Nessun PHI reale.
Con un <strong>seed</strong> il set è riproducibile; poi apri i
messaggi in tab o salvali in cartella come file <code>.hl7</code>
numerati — fixture di regressione istantanee per la validazione batch
qui sopra. Un file con lo stesso nome già presente nella cartella resta
com'è ed è elencato come non salvato.</p>

<h3>Validazione da CLI</h3>
<p>Il tool <code>bridgelab-cli</code> esegue gli stessi validatori — HL7
v2 e FHIR, con il core R4 integrato, i package installati e i plugin —
in modalità headless, per pipeline CI e screening batch. Non legge alcuna
licenza e non ne richiede. Oltre a <code>validate</code>,
<code>info</code>, <code>anonymize</code>, <code>to-json</code> e
<code>batch</code> esegue i pacchetti di test case esportati dalla
libreria (<code>test</code>, con report JUnit per la CI), valuta
espressioni FHIRPath (<code>fhirpath</code>), invia un messaggio via MLLP
ed esce con errore se l'ACK non è AA o CA (<code>send</code>; un file
con più messaggi viene inviato un frame per messaggio) ed esporta
l'XSD dei messaggi della Community (<code>xsd</code>; il catalogo
completo è in Pro). Ogni comando che legge un messaggio accetta
<code>-</code> per lo standard input. Un binario per piattaforma
accompagna ogni release. Metti tra virgolette doppie i pattern e gli
argomenti con <code>^</code> (<code>"*.hl7"</code>,
<code>"ORU^R01"</code>): <code>cmd.exe</code> di Windows elimina un
<code>^</code> senza virgolette e conserva gli apici singoli.</p>
<p>Il <code>batch</code> della CLI scende nelle sottocartelle, legge i
file <code>.hl7</code> se <code>--extension</code> non dice altro e
controlla anche FHIR oltre a HL7 v2, quindi i suoi conteggi possono
differire da <strong>Strumenti → Validazione batch…</strong> sulla
stessa cartella (una cartella, <code>.hl7</code>, <code>.txt</code> e
<code>.dat</code>, solo HL7 v2): <code>--extension hl7,txt,dat</code>
legge gli stessi file.</p>
<pre><code>bridgelab-cli validate message.hl7 bundle.json
bridgelab-cli validate "*.hl7" --format junit &gt; report.xml
bridgelab-cli batch ./inbox --json
bridgelab-cli test regression.bltests.json --format junit &gt; tests.xml
bridgelab-cli fhirpath "Patient.name.family" patient.json
cat message.hl7 | bridgelab-cli send - --host 10.0.0.5 --port 2575</code></pre>
`,
},
{
	id: 'communication',
	heading: 'Comunicazione (MLLP / HTTP / SOAP)',
	body: `
<p>Apri il pannello Comunicazione con <kbd>Ctrl</kbd>+<kbd>K</kbd> o
<strong>Strumenti → Pannello Comunicazione</strong>. Quattro tab: MLLP,
HTTP, SOAP e Cronologia.</p>

${mockupCommunication}

<h3>Client MLLP</h3>
<ol>
	<li>Inserisci <em>Host</em> + <em>Porta</em> (es.
		<code>localhost:2575</code>).</li>
	<li>Il messaggio nel tab attivo viene usato automaticamente.</li>
	<li>Clicca <strong>Invia</strong>. Framing
		(<code>0x0B</code> ... <code>0x1C 0x0D</code>), trasporto e
		attesa dell'ACK sono gestiti dal backend Rust.</li>
	<li>L'ACK appare nell'area risultato con il tempo di andata/ritorno.
		<em>Accept</em> (AA), <em>Error</em> (AE) e <em>Reject</em> (AR)
		vengono mostrati con il <code>MSA|AA|{control-id}</code>
		originale.</li>
</ol>

<h3>Generatore ACK</h3>
<p>La riga <strong>Generatore ACK</strong> nel tab MLLP costruisce un
acknowledgment per il messaggio attualmente nell'editor: scegli il
codice (AA accept, AE error, AR reject) e clicca <strong>Genera
ACK</strong>. L'ACK rispecchia il messaggio, come deve fare un
ricevente: stesso separatore di campo e stessi caratteri di codifica,
mittente e destinatario scambiati (MSH-3/4 e MSH-5/6),
<code>ACK^&lt;trigger&gt;^ACK</code> in MSH-9, stesso processing ID
(MSH-11), versione (MSH-12) e set di caratteri (MSH-18), e il Message
Control ID (MSH-10, letto con il separatore dichiarato in MSH-1) in
MSA-2. Ogni ACK ha il proprio MSH-10. L'auto-ACK del listener è
costruito allo stesso modo. L'ACK si apre in un nuovo tab — pronto da
rispedire o da conservare come fixture. Se il messaggio corrente non ha
MSH-10 il generatore si rifiuta, invece di produrre un ACK non
correlabile.</p>

<h3>Listener MLLP (Pro)</h3>
<p>Clicca <strong>Avvia ascolto</strong> per avviare un server sulla
porta selezionata. I messaggi in arrivo si aprono in un nuovo tab
(disattivabile, vedi sotto) e viene inviato un auto-ACK con il codice
configurato (AA/AE/AR). Utile per validare rapidamente cosa emette il
sistema a monte.</p>
<p>Il listener si mette in ascolto su <code>127.0.0.1</code> per
default, quindi lo raggiungono solo i programmi di questo computer. Per
ricevere un flusso da un'altra macchina imposta <strong>In ascolto su</strong> a
<code>0.0.0.0</code> (tutte le interfacce) o all'indirizzo di una scheda
di rete, e apri la porta nel firewall solo per i sistemi previsti.</p>
<p><strong>Ferma</strong> chiude anche le connessioni ancora aperte:
dopo, nulla viene più ricevuto né confermato.</p>

<h3>Console del listener</h3>
<p>Mentre il listener è attivo, ogni messaggio ricevuto compare come
riga nella console: ora locale, indirizzo del peer, dimensione del
payload, il codice ACK effettivamente inviato (<code>AA</code> verde,
<code>AE</code>/<code>AR</code> rosso, — se auto-ACK è spento), la
codifica caratteri usata e la prima riga del messaggio. <strong>Clicca
una riga per riaprire quel messaggio in un tab.</strong> Gli errori del
listener compaiono come righe rosse.</p>
<p>I chip nell'intestazione della console filtrano le righe per esito —
<em>AA</em>, <em>AE</em>, <em>AR</em>, <em>Senza ACK</em> (ricevuti con
auto-ACK spento), <em>Errori</em> — e ognuno porta un contatore
aggiornato, così "AE&nbsp;12" su 300 salta all'occhio prima di
scorrere. Gli stessi chip sono nella scheda Cronologia.</p>
<p>Il toggle <em>"Apri i messaggi ricevuti in un nuovo tab"</em> (attivo
di default) può essere spento nei test ad alto volume: i messaggi
finiscono solo nella console e scegli tu quali aprire.</p>
<p class="note">I contenuti completi conservati per il click-to-open
hanno un budget scorrevole di 32&nbsp;MB. Nelle sessioni lunghe senza
supervisione le righe più vecchie perdono il contenuto completo
(appaiono attenuate) — la riga coi metadati resta, e nulla è andato
"perso": è stata rilasciata solo la copia in memoria per il
click-to-open.</p>

<h3>Codifica caratteri</h3>
<p>Sia l'invio sia il listener hanno un selettore
<strong>Codifica</strong> con i charset dei deployment reali:
<code>UTF-8</code>, <code>ISO-8859-1</code> (Latin-1),
<code>ISO-8859-2</code>, <code>ISO-8859-15</code>,
<code>windows-1252</code>, <code>windows-1250</code>,
<code>windows-1251</code> e <code>ASCII</code>. Il default è
<strong>Automatica</strong>. Nel listener decodifica ogni messaggio con
il charset dichiarato in MSH-18, altrimenti come UTF-8 con fallback
automatico Latin-1, che accetta la maggior parte del traffico legacy
anche con MSH-18 vuoto; la console mostra il charset scelto e l'ACK
viene ri-codificato con quello, così il peer non vede mai caratteri
corrotti. Nell'invio usa il charset dichiarato in MSH-18, altrimenti
quello con cui è stato letto il file del tab, altrimenti UTF-8, e il
risultato indica il charset inviato. Un messaggio con caratteri che il
charset scelto non può rappresentare non viene inviato: l'errore li
elenca, così un nome non arriva mai con un <code>?</code>. Le codifiche
di invio e ricezione sono indipendenti.</p>
<p>I segmenti viaggiano terminati da CR qualunque siano i fine riga
dell'editor: LF e CR LF vengono convertiti, come fa la CLI, e lo stesso
vale per un body HL7 v2 inviato via HTTP o SOAP.</p>
<p>Per i <strong>file</strong> valgono le stesse regole. Un file che non è
UTF-8 si apre con la codifica dichiarata in MSH-18 (<code>8859/1</code>,
<code>8859/2</code>, <code>8859/15</code>…, e <code>BIG-5</code>,
<code>GB 18030-2000</code> e <code>KS X 1001</code>), oppure come
Windows-1252 se MSH-18 è vuoto: i nomi accentati si leggono correttamente
invece di far rifiutare il file. Un file il cui MSH-18 indica una codifica
che BridgeLab non sa decodificare (<code>CNS 11643-1992</code>, i set
giapponesi ISO 2022) si apre con un avviso, perché il testo e persino i
campi possono essere letti male; la validazione batch e
l'anonimizzazione batch lo rifiutano. Il salvataggio riscrive il file
nella codifica in cui è stato letto, e così fanno <code>anonymize</code>
della CLI e l'anonimizzazione batch; <code>send</code> senza
<code>--encoding</code> trasmette in quella codifica (un file UTF-16
parte nella codifica dichiarata in MSH-18, altrimenti in UTF-8). Al
salvataggio, un carattere che la codifica non può rappresentare viene
scritto come <code>?</code>; il <code>send</code> della CLI, come l'app,
rifiuta invece il messaggio.</p>

<h3>HTTP</h3>
<p>Le richieste GET sono disponibili nell'edizione Community. POST/PUT/DELETE/PATCH
e ogni forma di autenticazione richiedono Pro: un header Authorization o
Proxy-Authorization, un cookie, qualunque header il cui nome indichi una
chiave, un token, un segreto, una firma o una sessione (come
<code>X-API-Key</code> o <code>Ocp-Apim-Subscription-Key</code>),
utente:password nell'URL, oppure una chiave o un token nella query
string (<code>access_token</code>, <code>api_key</code>,
<code>key</code>, <code>token</code>, <code>sig</code>,
<code>client_secret</code>…). I parametri di ricerca FHIR come
<code>code</code> non sono credenziali. I redirect vengono seguiti
in tutte le edizioni, ma solo sullo stesso server (o da http a https sullo
stesso server): un redirect verso un altro server viene mostrato come il 3xx
che è e non viene seguito, così un messaggio non finisce dove non l'hai
scelto. Quando un redirect è stato seguito, il risultato indica l'URL che ha
risposto. Le risposte oltre 50 MB vengono troncate e lo segnalano, e una
risposta viene letta con il charset dichiarato nel suo Content-Type. POST,
PUT e PATCH inviano il messaggio della scheda corrente quando il campo Body
è vuoto; GET e DELETE inviano un body solo se lo scrivi.</p>

<h3>Client SOAP (Enterprise)</h3>
<p>Il tab SOAP invia il messaggio corrente (o un body personalizzato) a
endpoint SOAP 1.1/1.2 — middleware in stile IHE, gateway regionali e
web service ospedalieri legacy. Imposta l'URL dell'endpoint, la versione
SOAP e la <em>SOAPAction</em>; BridgeLab costruisce l'envelope, la invia
con il content type corretto (<code>text/xml</code> più header
SOAPAction per la 1.1, <code>application/soap+xml</code> con il
parametro action per la 1.2) e mostra lo status HTTP, il tempo di
andata/ritorno, l'XML interno del Body e l'eventuale SOAP Fault,
decodificato per entrambe le versioni.</p>
<p>Un messaggio HL7 v2 grezzo viene automaticamente sottoposto a
escape XML (i CR di fine segmento diventano <code>&amp;#13;</code>, così
il parsing XML non li trasforma in LF) e avvolto in un elemento
<code>&lt;payload&gt;</code>; il contenuto già XML viene inserito così
com'è. Le impostazioni avanzate aggiungono credenziali
<strong>WS-Security UsernameToken</strong>, header
<strong>WS-Addressing</strong> (To / Action / MessageID) e un
<strong>template envelope personalizzato</strong> in cui il segnaposto
letterale <code>{payload}</code> viene sostituito con il messaggio —
utile quando il servizio di destinazione richiede un wrapper specifico.
Con un template, gli header WS-Security e WS-Addressing vengono inseriti
nel SOAP Header del template (se non ne ha uno viene aggiunto prima del
Body); un template senza SOAP Envelope e Body non può contenerli, e
l'invio viene rifiutato invece di partire senza. L'import WSDL è
previsto come passo successivo.</p>

<h3>Cronologia</h3>
<p>Ogni invio, e ogni messaggio ricevuto dal listener, viene
registrato: destinazione (host e porta, oppure l'URL con password o
chiavi sostituite da <code>***</code>), dimensione, codice di risposta e
tempo di andata/ritorno. Le ultime 100 voci persistono tra un riavvio e
l'altro e le più vecchie vengono eliminate; clicca una riga per vedere
la richiesta e la risposta complete — il messaggio e il suo ACK, oppure
il body HTTP e la risposta (per SOAP il payload, mai l'envelope con la
password). Una richiesta o risposta oltre 256 KB viene conservata
troncata. Un invio MLLP registra anche il <strong>codice ACK</strong>
con cui il ricevente ha risposto (MSA-1), mostrato come badge verde o
rosso sulla riga: un invio arrivato al peer che riceve un
<code>AE</code> è "OK" a livello di trasporto e un rifiuto a livello
applicativo, e il badge distingue i due casi. I chip di filtro sopra
l'elenco — <em>AA</em>, <em>AE</em>, <em>AR</em>, <em>Senza ACK</em>,
<em>Falliti</em> — hanno i contatori e restringono l'elenco a un solo
esito; i codici commit-mode CA, CE e CR contano sotto AA, AE e AR.
<em>Falliti</em> è una richiesta senza risposta: un server che ha
risposto con un errore (HTTP 404, 500…) compare con il suo codice.</p>

<h3>Profili di connessione</h3>
<p>Salva gli endpoint usati di frequente come profili nominati dalla
riga <strong>Profilo</strong>: digita un nome e clicca <em>Salva</em>. I
profili MLLP memorizzano host, porta, timeout e auto-ACK; quelli HTTP
memorizzano URL, header e timeout; quelli SOAP memorizzano endpoint,
SOAPAction e timeout. Selezionare un profilo lo applica al
form; salvare con un nome esistente lo sovrascrive; <em>Elimina</em>
rimuove quello selezionato. I profili sono salvati nel database locale e
sopravvivono ai riavvii.</p>
<p class="note">I profili HTTP salvano il campo Header così come è
scritto, header <code>Authorization</code> compreso, nel database
locale. Per non salvarvi una password o un token, inseriscili in
<em>Autenticazione</em> nelle impostazioni HTTP avanzate, che non viene
mai salvata.</p>
`,
},
{
	id: 'anonymization',
	heading: 'Anonimizzazione ed Export',
	body: `
<p><strong>Strumenti → Anonimizza</strong> rileva i campi PHI nei
segmenti che identificano il paziente e li maschera per livello di
sensibilità: 89 campi integrati in PID, PV1 (numeri di visita), MRG,
NK1, GT1, IN1 e IN2, più i commenti NTE e i risultati OBX a testo libero
(TX/FT). Coprono nomi, date di nascita e di morte, indirizzi, telefoni,
SSN e altri identificativi di paziente, parenti, garante e assicurato.
Un identificativo conserva l'autorità assegnante e il tipo
(<code>HOSP</code>, <code>MR</code>). Gli altri segmenti non vengono
mascherati: controlla il testo libero altrove (OBR, ORC, segmenti Z)
prima di condividere, oppure aggiungi quei campi con un plugin.</p>

<table>
	<tr><th>Livello</th><th>Esempio</th><th>Strategia</th></tr>
	<tr><td><strong>Alta</strong></td>
		<td>Nome paziente, data di nascita, indirizzo, telefono di casa,
		SSN, MRN, numero di visita</td>
		<td>Il testo diventa <code>REDACTED</code>; i numeri diventano
		zeri della stessa lunghezza (per non rompere i parser a
		valle); una data diventa 1900-01-01 con la stessa
		precisione.</td></tr>
	<tr><td><strong>Media</strong></td>
		<td>Cognome della madre, alias, telefono di lavoro, contatti di
		parenti, garante e assicurato</td>
		<td>Primo carattere mantenuto, resto sostituito con
		<code>***</code>.</td></tr>
	<tr><td><strong>Bassa</strong></td>
		<td>Nessun campo integrato; disponibile per le regole dei
		plugin</td>
		<td>Primi 3 caratteri mantenuti, seguiti da <code>...</code>;
		un valore di 3 caratteri o meno resta intero.</td></tr>
</table>

<p>Il dialog elenca ogni campo PHI rilevato prima di eseguire il
masker, così puoi controllare cosa cambierà. L'output:</p>
<ul>
	<li><strong>Si apre in un nuovo tab</strong> - il messaggio
		originale resta intatto nel suo tab.</li>
	<li><strong>Può essere copiato negli appunti</strong>
		direttamente.</li>
	<li><strong>Preserva la struttura</strong> - ordine segmenti,
		numero pipe e separatori di componente invariati, così il
		risultato resta HL7 valido.</li>
</ul>

<h3>Campi PHI custom tramite plugin</h3>
<p>Installazioni con identificatori regionali o vendor-specific (codice
fiscale europeo, campi Z-segment interni) possono estendere il catalogo
inserendo un file JSON sotto
<code>&lt;config&gt;/BridgeLab/plugins/anonymization/</code>.</p>

<h3>Anonimizzazione batch (Pro)</h3>
<p><strong>Strumenti → Anonimizzazione batch…</strong> maschera
un'intera cartella in un colpo solo: scegli i file sorgente o una
cartella, scegli una cartella di output, esegui. Ogni messaggio passa
per la stessa pipeline del dialog interattivo (catalogo PHI integrato +
regole dei plugin attivi) e viene scritto come copia nella cartella di
output — <strong>gli originali non vengono mai toccati</strong>: il
tool si rifiuta di sovrascrivere qualsiasi file sorgente selezionato, e
input con lo stesso nome provenienti da cartelle diverse ricevono
suffissi numerici invece di sovrascriversi a vicenda. Un file già
presente nella cartella di output non viene mai sostituito (e un link
non viene mai seguito): la sua riga lo segnala e il file resta com'è,
quindi scegli una cartella vuota. Una riga per file
riporta il numero di PHI mascherati o l'errore; valgono gli stessi
limiti della validazione batch (5000 file / 10&nbsp;MB).</p>

<h3>Export</h3>
<p>Gli utenti Pro possono esportare il messaggio strutturato come JSON
o CSV da <strong>Strumenti → Esporta JSON / CSV</strong>; una finestra
di salvataggio chiede dove scrivere il file. Utile per caricare dati
HL7 in tool di analisi (Power BI, Excel, pandas).</p>

<div class="warn">L'anonimizzazione scrive il risultato in una nuova
scheda; la scheda originale resta com'è. Conserva sempre il file sorgente originale come
riferimento canonico - la copia anonimizzata è per la condivisione,
non per lo storage di lungo periodo.</div>
`,
},
{
	id: 'testcases',
	heading: 'Libreria Test Case',
	body: `
<p>La Libreria Test Case (<kbd>Ctrl</kbd>+<kbd>L</kbd>) conserva
messaggi riusabili con nome, categoria, tag e descrizione. Usa
<strong>Salva messaggio corrente</strong> per catturare il tab attivo,
oppure crea casi da zero. I casi persistono nel database locale e sono
ricercabili su tutti i campi.</p>

<p class="note">Il tier Community mantiene fino a 10 test case salvati
— i casi esistenti restano sempre visibili, modificabili ed
eseguibili; solo i nuovi salvataggi oltre il limite chiedono
l'upgrade.</p>

<h3>Esiti attesi</h3>
<p>Ogni caso può dichiarare un <strong>tipo di messaggio atteso</strong>
(si confrontano solo le componenti indicate: <code>ADT</code> accetta
qualsiasi evento ADT, <code>ADT^A01</code> accetta <code>ADT^A01</code> e
<code>ADT^A01^ADT_A01</code> ma non <code>ADT^A04</code>) e una <strong>validazione attesa</strong> (valido / non
valido). Così uno snippet diventa un test.</p>

<h3>Eseguire le verifiche</h3>
<p><strong>Verifica</strong> analizza e valida davvero un singolo caso
— HL7 v2 o FHIR, rilevato automaticamente — e confronta l'esito con le
attese. <strong>Esegui tutti</strong> fa lo stesso per ogni caso che
corrisponde alla ricerca corrente, con badge pass/fail per riga e
riepilogo superati/totale nella toolbar. Dopo una modifica
all'interfaccia, un click ti dice quali dei tuoi messaggi di
riferimento si sono rotti. Modificare un caso azzera il suo esito fino
alla verifica successiva.</p>

<h3>Condividere i test case</h3>
<p><strong>Esporta…</strong> scrive i test case visibili — tutti,
oppure solo quelli che corrispondono alla ricerca — in un pacchetto
<code>.bltests.json</code> da mandare a un collega o da salvare in un
repository Git. Prima di salvare, BridgeLab controlla i messaggi HL7 v2
alla ricerca di dati personali ed elenca i test case e i campi trovati;
le risorse FHIR sono indicate come non controllate campo per campo. Con
Pro puoi spuntare <em>Maschera i dati personali</em> per anonimizzare i
messaggi HL7 v2 solo nel file esportato: la libreria non cambia.</p>
<p><strong>Importa…</strong> apre un pacchetto e mostra, prima di
scrivere qualsiasi cosa, cos'è ogni test case: <em>Nuovo</em>, <em>Già
nella libreria</em> (saltato) o <em>Diverso</em> da uno che hai già,
dove scegli se tenere il tuo, sostituirlo o tenerli entrambi. I test
case importati mantengono il loro identificativo, quindi importare di
nuovo lo stesso pacchetto porta solo ciò che è cambiato. In Community
un import non può portare la libreria oltre 10 test case: in quel caso
non viene scritto nulla.</p>

<h3>Ripristino sessione</h3>
<p>BridgeLab salva i tab aperti (incluse le modifiche non salvate) e li
riapre al prossimo avvio, in stile Notepad++. Controlli tutto da
<strong>Impostazioni → Prestazioni</strong>, nel gruppo <em>Sessione</em>: attiva/disattiva
<em>Ripristina i tab all'avvio</em>, oppure usa <em>Cancella sessione
salvata</em> per azzerare i tab memorizzati (disattiva anche il
ripristino, così il prossimo avvio parte dalla schermata di
benvenuto). Disattivare il ripristino elimina anche i tab salvati, che
possono contenere dati dei pazienti.</p>

<h3>File modificati da altri programmi</h3>
<p>BridgeLab si accorge quando un altro programma modifica o elimina un
file aperto: quando torni alla finestra, o riapri il file, propone di
caricare la nuova versione (e ti avvisa se il file non c'è più).
<strong>Salva</strong> chiede conferma prima di sovrascrivere un file
modificato su disco dopo l'apertura o l'ultimo salvataggio, o prima di
ricreare un file eliminato. All'avvio, un tab ripristinato senza
modifiche non salvate mostra il file com'è ora su disco. Un tab ripristinato con modifiche non salvate chiede conferma al primo salvataggio, perché il file potrebbe essere cambiato mentre BridgeLab era chiuso.</p>
`,
},
];
