import type { ManualSection } from '../helpContent';
import { mockupValidation, mockupCommunication } from './mockups';

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
		nessun MSH duplicato.</li>
	<li><strong>Header MSH:</strong> MSH-9 (tipo messaggio), MSH-10
		(control ID), MSH-12 (versione) sono obbligatori.</li>
	<li><strong>Campi obbligatori:</strong> campi richiesti per segmento
		presi dallo standard HL7 (es. PID-3 Patient Identifier
		List).</li>
	<li><strong>Lunghezze:</strong> avviso quando un campo supera il
		<code>max_length</code> pubblicato.</li>
	<li><strong>Tipi di dato:</strong> campi numerici (SI, NM)
		controllati per caratteri non numerici; timestamp (TS) per
		lunghezza e composizione solo con cifre.</li>
</ul>

<h3>Filtri e navigazione</h3>
<p>Clicca i badge Errore / Avviso / Info per filtrare. Clicca su una
riga del problema per saltare al segmento nell'editor.</p>

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
qui sopra.</p>

<h3>Validazione da CLI</h3>
<p>Il tool <code>bridgelab-cli</code> offre lo stesso validator per uso
headless (pipeline CI, screening batch):</p>
<pre><code>bridgelab-cli validate message.hl7
bridgelab-cli validate '*.hl7' --format junit &gt; report.xml
bridgelab-cli batch ./inbox --json</code></pre>
`,
},
{
	id: 'communication',
	heading: 'Comunicazione (MLLP / HTTP)',
	body: `
<p>Apri il pannello Comunicazione con <kbd>Ctrl</kbd>+<kbd>K</kbd> o
<strong>Strumenti → Pannello Comunicazione</strong>. Tre tab: MLLP,
HTTP e Cronologia.</p>

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

<h3>Listener MLLP (Pro)</h3>
<p>Clicca <strong>Avvia ascolto</strong> per avviare un server sulla
porta selezionata. I messaggi in arrivo si aprono in un nuovo tab
(disattivabile, vedi sotto) e viene inviato un auto-ACK con il codice
configurato (AA/AE/AR). Utile per validare rapidamente cosa emette il
sistema a monte.</p>

<h3>Console del listener</h3>
<p>Mentre il listener è attivo, ogni messaggio ricevuto compare come
riga nella console: ora locale, indirizzo del peer, dimensione del
payload, il codice ACK effettivamente inviato (<code>AA</code> verde,
<code>AE</code>/<code>AR</code> rosso, — se auto-ACK è spento), la
codifica caratteri usata e la prima riga del messaggio. <strong>Clicca
una riga per riaprire quel messaggio in un tab.</strong> Gli errori del
listener compaiono come righe rosse.</p>
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
<code>windows-1251</code> e <code>ASCII</code>. Il default è UTF-8 con
fallback automatico Latin-1, che accetta la maggior parte del traffico
legacy anche con MSH-18 vuoto. Il listener ri-codifica l'ACK con lo
stesso charset, così il peer non vede mai caratteri corrotti. Le
codifiche di invio e ricezione sono indipendenti.</p>

<h3>HTTP</h3>
<p>Le richieste GET sono disponibili in Community. POST/PUT/DELETE,
header di autenticazione personalizzati (Basic, Bearer) e
follow-redirect richiedono Pro. Il body usa di default il messaggio del
tab corrente ma può essere sovrascritto.</p>

<h3>Cronologia</h3>
<p>Ogni invio e ricezione viene loggata (host, porta, dimensione, codice
di risposta, tempo di andata/ritorno). Le ultime 100 voci persistono
tra un riavvio e l'altro; clicca una riga per vedere la richiesta e
risposta complete.</p>

<h3>Profili di connessione</h3>
<p>Salva gli endpoint usati di frequente come profili nominati dalla
riga <strong>Profilo</strong>: digita un nome e clicca <em>Salva</em>. I
profili MLLP memorizzano host, porta, timeout e auto-ACK; quelli HTTP
memorizzano URL, header e timeout. Selezionare un profilo lo applica al
form; salvare con un nome esistente lo sovrascrive; <em>Elimina</em>
rimuove quello selezionato. I profili sono salvati nel database locale e
sopravvivono ai riavvii.</p>
`,
},
{
	id: 'anonymization',
	heading: 'Anonimizzazione ed Export',
	body: `
<p><strong>Strumenti → Anonimizza</strong> rileva i campi PHI nei
segmenti di identificazione più comuni (PID, NK1, IN1, GT1) e li
maschera per livello di sensibilità.</p>

<table>
	<tr><th>Livello</th><th>Esempio</th><th>Strategia</th></tr>
	<tr><td><strong>Alta</strong></td>
		<td>Nome paziente, SSN, MRN</td>
		<td>Il testo diventa <code>REDACTED</code>; i numeri diventano
		zeri della stessa lunghezza (per non rompere i parser a
		valle).</td></tr>
	<tr><td><strong>Media</strong></td>
		<td>Cognome della madre, telefono</td>
		<td>Primo carattere mantenuto, resto sostituito con
		<code>***</code>.</td></tr>
	<tr><td><strong>Bassa</strong></td>
		<td>Alias, identificatori a basso rischio</td>
		<td>Primi 3 caratteri mantenuti, resto sostituito con
		<code>...</code>.</td></tr>
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

<h3>Export</h3>
<p>Gli utenti Pro possono esportare il messaggio strutturato come JSON
o CSV da <strong>Strumenti → Esporta JSON / CSV</strong>. Utile per
caricare dati HL7 in tool di analisi (Power BI, Excel, pandas).</p>

<div class="warn">L'anonimizzazione sostituisce i valori
<em>nell'editor</em>. Conserva sempre il file sorgente originale come
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

<h3>Esiti attesi</h3>
<p>Ogni caso può dichiarare un <strong>tipo di messaggio atteso</strong>
(<code>ADT</code> accetta qualsiasi evento ADT, <code>ADT^A01</code> è
esatto) e una <strong>validazione attesa</strong> (valido / non
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

<h3>Ripristino sessione</h3>
<p>BridgeLab salva i tab aperti (incluse le modifiche non salvate) e li
riapre al prossimo avvio, in stile Notepad++. Controlli tutto da
<strong>Impostazioni → Sessione</strong>: attiva/disattiva
<em>Ripristina i tab all'avvio</em>, oppure usa <em>Cancella sessione
salvata</em> per azzerare i tab memorizzati (disattiva anche il
ripristino, così il prossimo avvio parte dalla schermata di
benvenuto).</p>
`,
},
];
