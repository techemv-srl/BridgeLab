import type { ManualSection } from '../helpContent';
import { mockupAppShellIt as mockupAppShell, mockupContextMenuIt as mockupContextMenu } from './mockups';

export const itPart1: ManualSection[] = [
{
	id: 'getting-started',
	heading: 'Primi passi',
	body: `
<p>BridgeLab è un editor moderno di messaggi HL7 v2.x e FHIR, pensato
per chi lavora nell'integrazione sanitaria. Il backend è scritto in Rust
per un parsing veloce (un messaggio di 10 MB con un allegato base64 si apre in circa 2 secondi); il frontend usa
Svelte 5 e l'editor Monaco.</p>

<p>La finestra principale è divisa in quattro aree:</p>
${mockupAppShell}

<ol>
	<li><strong>Barra dei menu e banner Trial</strong> in alto -
		File, Modifica, Visualizza, Strumenti, Aiuto, più un banner
		giallo/rosso che ricorda lo stato del trial Pro.</li>
	<li><strong>Pannello tree</strong> a sinistra - la struttura del
		messaggio con frecce espandi/comprimi e l'Ispettore Campo in
		basso con le info dello schema HL7 del nodo selezionato.</li>
	<li><strong>Editor e tab</strong> al centro - Monaco con
		evidenziazione HL7 e multi-tab.</li>
	<li><strong>Status bar</strong> in basso - tipo messaggio,
		versione, numero segmenti, posizione del cursore.</li>
</ol>

<h3>Aprire un messaggio</h3>
<ul>
	<li><strong>File → Apri File</strong> (<kbd>Ctrl</kbd>+<kbd>O</kbd>) -
		selettore nativo per <code>.hl7</code>, <code>.txt</code>,
		<code>.msg</code>, <code>.json</code>, <code>.xml</code>.</li>
	<li><strong>Trascina e rilascia</strong> - trascina un file nell'area
		dell'editor.</li>
	<li><strong>Incolla</strong> - clicca nell'editor e incolla
		(<kbd>Ctrl</kbd>+<kbd>V</kbd>). L'analisi automatica parte 500 ms
		dopo l'ultima digitazione (ritardo, o disattivazione, in
		<strong>Impostazioni → Parser</strong>).</li>
	<li><strong>Cosa si apre:</strong> righe vuote, spazi, un BOM o la
		cornice MLLP prima di <code>MSH</code>, i file UTF-16 (il "Unicode"
		di Blocco note) e i file batch FHS/BHS vengono letti così come sono.
		Un file che BridgeLab non riesce ad analizzare si apre comunque,
		come testo da correggere, con una nota che spiega perché.</li>
	<li><strong>File → Nuovo Messaggio da Template...</strong>
		(<kbd>Ctrl</kbd>+<kbd>N</kbd>) - ADT, ORM, ORU, SIU e altri
		preconfigurati. MSH-7 e MSH-10 vengono compilati con il timestamp
		corrente e un identificativo univoco del messaggio.</li>
	<li><strong>File → Messaggi di esempio</strong> (anche dalla schermata
		iniziale) - messaggi completi e realistici invece di scheletri: ADT
		di ricovero, registrazione, aggiornamento, dimissione e unione, ORU
		con risultati (un emocromo con dodici valori, un pannello
		metabolico), ORM, SIU, MDM, DFT, VXU e un ACK, nelle versioni 2.3,
		2.5 e 2.5.1. Filtra per versione, guarda l'anteprima e aprine uno in
		una nuova scheda. Ogni esempio supera la validazione; pazienti e dati
		sono fittizi.</li>
</ul>

<div class="note">Al primo avvio ricevi un <strong>trial Pro di 14
giorni</strong> con tutte le funzionalità Pro sbloccate (SOAP e supporto
prioritario sono funzioni Enterprise). Alla scadenza,
BridgeLab continua a funzionare con il livello Community - non perdi
mai i tuoi messaggi.</div>

<p>Le card <strong>Scopri BridgeLab</strong> nella schermata di benvenuto
aprono direttamente le funzioni che distinguono BridgeLab — generatore di
messaggi di test, anonimizzazione PHI, listener MLLP ed export XSD — con i
badge PRO a indicare quelle su licenza.</p>
`,
},
{
	id: 'editor',
	heading: 'Editor',
	body: `
<p>L'area dell'editor è un'istanza <strong>Monaco</strong> con una
grammatica specifica per HL7. I codici di segmento sono colorati viola,
i separatori di campo grigi, e i payload ED/base64 e gli altri valori lunghi
sono mostrati compattati, così l'editor resta reattivo sui messaggi grandi
(vedi sotto).</p>

<h3>Auto-completamento e hover</h3>
<p>Digita <code>P</code> a inizio riga - Monaco suggerisce
<code>PID</code>, <code>PV1</code>, <code>PV2</code>, ecc. Dopo aver
inserito un segmento, l'autocomplete propone valori di campo
(codici gender, codici ACK, classe paziente...). Passando con il mouse
su un campo appaiono nome, tipo di dato e flag di obbligatorietà tratti
dallo standard HL7.</p>

<h3>Campi lunghi compattati</h3>
<p>I campi più lunghi della soglia di compattazione (100 caratteri per
impostazione predefinita, in <strong>Impostazioni → Parser</strong>) sono
mostrati <em>compattati</em>: un allegato base64 in OBX-5, una nota lunga o
una stringa JSON <code>data</code> appaiono come un chip compatto, per
esempio <code>⟨Base64 · 5.1 KB⟩</code>, mentre separatori e altri
componenti restano visibili. La compattazione è solo una vista: il
messaggio è sempre completo, e salvataggio, sessione, validazione, albero
e copia usano il testo intero.</p>
<ul>
	<li><strong>Espandere:</strong> clic sul chip, oppure cursore accanto
		e <kbd>Alt</kbd>+<kbd>Invio</kbd>. Passandoci sopra si vedono i
		primi caratteri.</li>
	<li><strong>Ricompattare:</strong> tasto destro → <em>Compatta questo
		campo</em> su qualsiasi valore lungo, oppure <em>Compatta tutti i
		campi lunghi</em>.</li>
	<li><strong>Tutto insieme:</strong> il badge <em>N compattati</em>
		nella barra di stato espande tutto; il menu contestuale ha
		entrambi i comandi.</li>
	<li>Il chip si muove come un'unità: le frecce lo saltano e
		Backspace/Canc prima lo selezionano, così non si cancella mai a
		metà. Copiare una selezione copia il contenuto completo.</li>
	<li><strong>Trova e Sostituisci</strong> (<kbd>Ctrl</kbd>+<kbd>F</kbd>,
		<kbd>Ctrl</kbd>+<kbd>H</kbd>) cercano nel testo completo: un campo
		compattato che contiene una corrispondenza si espande, così viene
		contata, mostrata e sostituita come le altre.</li>
</ul>

<h3>Impostazioni dell'editor</h3>
<p><strong>Modifica → Impostazioni → Editor</strong> (Ctrl+,) cambia aspetto e
comportamento dell'editor: carattere e dimensione, larghezza del tab, a capo
automatico, spazi visibili, minimappa, numeri di riga, scorrimento fluido,
colori delle parentesi, evidenziazione delle altre occorrenze della parola
sotto il cursore, link cliccabili, intestazione fissa durante lo scorrimento
(sticky scroll) e origine dei suggerimenti di parole (questo messaggio, tutti
i messaggi aperti o nessuno; i suggerimenti di campi e valori HL7 funzionano
in ogni modalità). Le modifiche si applicano subito. Ogni scheda conserva la
propria cronologia di annulla e ripeti (Ctrl+Z, Ctrl+Y) quando passi da una
scheda all'altra.</p>

<h3>Menu contestuale (tasto destro)</h3>
${mockupContextMenu}
<p>Il menu raggruppa le azioni in tre sezioni:</p>
<ul>
	<li><strong>Navigazione:</strong> Mostra Segmento nel Tree
		(<kbd>Alt</kbd>+<kbd>T</kbd>) - apre il tree e evidenzia il
		campo esatto sotto il cursore; Espandi / Compatta per valori
		lunghi.</li>
	<li><strong>Appunti:</strong> Copia Segmento
		(<kbd>Alt</kbd>+<kbd>C</kbd>), Copia Messaggio Completo (con
		campi espansi), Copia Messaggio Troncato (campi lunghi accorciati, per stare in una email; i dati del paziente sono copiati così come sono: anonimizza prima).</li>
</ul>

<div class="note">Le scorciatoie native di Monaco
(<kbd>Ctrl</kbd>+<kbd>F</kbd> trova, <kbd>Ctrl</kbd>+<kbd>H</kbd>
sostituisci, <kbd>Ctrl</kbd>+<kbd>Z</kbd> annulla,
<kbd>Ctrl</kbd>+<kbd>D</kbd> multi-cursore) funzionano tutte quando
l'editor ha il focus.</div>
`,
},
{
	id: 'tree-view',
	heading: 'Vista ad albero e Ispettore Campo',
	body: `
<p>Il tree a sinistra rispecchia la gerarchia del messaggio HL7:
<strong>segmenti</strong> → <strong>campi</strong> →
<strong>componenti</strong>; un campo ripetuto elenca ogni ripetizione
(<code>PID-3(1)</code>, <code>PID-3(2)</code>) con i suoi componenti. Mostralo/nascondilo con
<kbd>Ctrl</kbd>+<kbd>B</kbd> o <strong>Visualizza → Struttura
Messaggio</strong>.</p>

<h3>Navigare tra tree ed editor</h3>
<ul>
	<li><strong>Editor → Tree:</strong> tasto destro su un campo in
		Monaco, scegli <em>Mostra Segmento nel Tree</em>. Il tree espande il
		segmento, seleziona il campo esatto (fino al componente) e
		scrolla fino a renderlo visibile.</li>
	<li><strong>Tree → Editor:</strong> tasto destro su un nodo del
		tree, scegli <em>Mostra nell'Editor</em>. Monaco salta alla
		riga, posiziona il cursore nella colonna corretta e seleziona
		l'intervallo del campo.</li>
</ul>

<p>Le righe vuote tra i segmenti non spostano i salti. Nel tree,
<kbd>↑</kbd>/<kbd>↓</kbd> spostano la selezione, <kbd>→</kbd> espande un
nodo e <kbd>←</kbd> lo chiude o risale al padre;
<kbd>Home</kbd>/<kbd>Fine</kbd> e <kbd>PgSu</kbd>/<kbd>PgGiù</kbd>
saltano. Mentre modifichi, il tree mantiene ciò che hai espanso e
selezionato, e l'Ispettore Campo mostra il valore attuale.</p>

<h3>Pannello Ispettore Campo</h3>
<p>Clicca l'icona <strong>ⓘ</strong> nell'intestazione del pannello
tree (o <strong>Visualizza → Ispettore Campo</strong>) per mostrare i
metadati dello schema per il nodo selezionato:</p>
<ul>
	<li>Posizione HL7 (es. <code>PID-5</code>) e nome canonico (Patient
		Name)</li>
	<li>Tipo di dato (XPN, CX, ST, ...), lunghezza max, flag
		obbligatorio/ripetibile, descrizione</li>
	<li>Valore corrente e lunghezza; pulsante <em>Mostra valore
		completo</em> per i campi lunghi che l'editor mostra compattati</li>
</ul>
<p>I segmenti sconosciuti (Z-segment o codici custom non nello standard)
mostrano <em>Non nello standard HL7</em> ma restano perfettamente
modificabili.</p>

<h3>Ricerca nell'albero</h3>
<p>La casella di ricerca in cima all'albero cerca per <strong>tipo di
segmento</strong> (<code>PID</code>), <strong>nome campo dello
schema</strong> (<code>Patient Name</code>) e <strong>valore del
campo</strong> — inclusi i campi nei segmenti non ancora espansi. Premi
<kbd>Invio</kbd>/<kbd>Shift</kbd>+<kbd>Invio</kbd> per scorrere i
risultati, <kbd>Esc</kbd> per cancellare, <kbd>Ctrl</kbd>+<kbd>F</kbd>
con il focus sull'albero per saltare alla casella. Cliccando un
risultato il segmento si espande, il campo viene selezionato e portato
in vista.</p>
<p class="note">La ricerca nell'albero funziona sui messaggi HL7 v2. Per
le risorse FHIR usa il filtro del visualizzatore Bundle o il
<kbd>Ctrl</kbd>+<kbd>F</kbd> dell'editor.</p>

<h3>Griglia dei segmenti</h3>
<p>Un messaggio di risultati può contenere decine di OBX, e l'albero li
mostra uno per nodo. <strong>Visualizza → Griglia segmenti</strong>
(<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd>), oppure <em>Mostra tutti
gli OBX in tabella</em> dal menu del tasto destro di un segmento, apre un
pannello in basso con tutte le occorrenze di un tipo di segmento in forma
di tabella: una riga per occorrenza, una colonna per ogni campo che ha un
valore in almeno una di esse, con la posizione e il nome del campo nella
versione HL7 del messaggio. I valori codificati mostrano il loro
significato sotto il codice, come nell'albero; i valori lunghi sono
accorciati con i puntini. Scegli un altro segmento dall'elenco (ognuno
indica quante volte compare), scrivi in <em>Filtra righe</em> per tenere
solo le righe che contengono un testo e fai clic su una cella per
selezionare quel campo nell'editor. La griglia è in sola lettura e segue
il messaggio mentre lo modifichi e lo rianalizzi.</p>

<h3>Confrontare due messaggi</h3>
<p><strong>Strumenti → Confronta messaggi…</strong> apre un diff
affiancato di due tab aperti con evidenziazione HL7. Scegli
sinistra/destra dai menu, usa il pulsante ⇆ per scambiare i lati,
<kbd>Esc</kbd> per chiudere. Servono almeno due tab aperti.</p>

<h3>Campi codificati: cosa significa un codice</h3>
<p>BridgeLab include le tabelle valori HL7 — 394 tabelle, circa 5.000
codici — e sa da quale tabella attinge ogni campo e componente
codificato, per versione. Un valore codificato viene spiegato ovunque
lo incontri: l'albero mostra il significato accanto al valore
(<code>M — Male</code>, <code>ADT — ADT message</code>,
<code>F — Final results</code>), l'hover sul campo nell'editor lo
riporta, e l'auto-completamento in un campo codificato propone tutti i
valori della sua tabella. Anche i componenti sono coperti: MSH-9.2 è
spiegato dalla tabella degli eventi, PID-3.5 da quella dei tipi di
identificativo.</p>
<p>L'Ispettore Campo elenca l'intera tabella del campo o componente
selezionato ed evidenzia il valore corrente. Se un valore fuori tabella
sia un problema dipende dal tipo di dato, e l'ispettore dice in quale
caso sei: un campo <code>ID</code> attinge da una tabella definita da
HL7 (<em>Valori ammessi</em>) e un valore non elencato è non standard —
compare un avviso; un campo <code>IS</code> attinge da una tabella
definita dall'utente (<em>Valori suggeriti</em>), dove ogni sito
aggiunge i propri codici e l'assenza non significa nulla. Alcune tabelle
utente non hanno alcun valore standard (IN1-2 Insurance Plan ID): quei
campi non mostrano nessun elenco.</p>
<p class="note">Quale tabella usa un campo segue la versione HL7
dichiarata; il contenuto delle tabelle è un unico insieme per tutte le
versioni, così come lo distribuisce la fonte a monte. Un codice
aggiunto in una release successiva è quindi accettato anche per una
precedente.</p>

<h3>Tree consapevole dello schema</h3>
<p><strong>Visualizza → Mostra campi dello standard</strong> inserisce
righe placeholder per ogni campo definito dallo standard HL7 ma
<em>assente</em> nel messaggio. I placeholder appaiono opachi e in
corsivo - servono a capire quali campi <em>potresti</em> aggiungere, ma
non sono navigabili nell'editor (non hanno ancora una posizione
fisica).</p>

<h3>Ridimensionare i pannelli</h3>
<p>Trascina lo splitter verticale tra tree ed editor per cambiarne la
larghezza; trascina lo splitter orizzontale sopra l'Ispettore per
cambiarne l'altezza. Entrambe le dimensioni sono persistite tra un
avvio e l'altro.</p>

<h3>Struttura standard completa</h3>
<p>Con <strong>Visualizza → Mostra campi dello standard</strong> attivo, l'albero
mostra anche i segmenti che lo standard prevede per il tipo di messaggio ma
assenti dal messaggio — righe in grigio nella posizione standard, annotate
con gruppo, cardinalità e stato di scelta. Espandili per sfogliare l'elenco
campi completo fino ai componenti dei tipi composti (es. OBX-16 → componenti
XCN). <strong>Click destro su un segmento grigio → Inserisci segmento</strong>
per aggiungerne lo scheletro al messaggio nella posizione standard, con i
separatori fino all'ultimo campo obbligatorio. Lo scheletro usa i separatori del
messaggio, e <kbd>Ctrl</kbd>+<kbd>Z</kbd> lo toglie di nuovo.</p>
`,
},
];
