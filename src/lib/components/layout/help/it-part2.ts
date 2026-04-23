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
<p>Clicca <strong>Ascolta</strong> per avviare un server sulla porta
selezionata. I messaggi in arrivo si aprono in un nuovo tab e viene
inviato un auto-ACK (disattivabile nelle opzioni avanzate). Utile per
validare rapidamente cosa emette il sistema a monte.</p>

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
<p>Salva endpoint usati di frequente (Host + Porta + Timeout + auto-ACK)
come profili nominati. Appaiono nel menu profili accanto al pulsante
Invia.</p>
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
];
