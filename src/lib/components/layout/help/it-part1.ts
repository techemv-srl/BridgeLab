import type { ManualSection } from '../helpContent';
import { mockupAppShell, mockupContextMenu } from './mockups';

export const itPart1: ManualSection[] = [
{
	id: 'getting-started',
	heading: 'Primi passi',
	body: `
<p>BridgeLab è un editor moderno di messaggi HL7 v2.x e FHIR, pensato
per chi lavora nell'integrazione sanitaria. Il backend è scritto in Rust
per un parsing veloce (10 MB in meno di 2 secondi); il frontend usa
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
	<li><strong>File → Apri</strong> (<kbd>Ctrl</kbd>+<kbd>O</kbd>) -
		selettore nativo per <code>.hl7</code>, <code>.txt</code>,
		<code>.msg</code>, <code>.json</code>, <code>.xml</code>.</li>
	<li><strong>Trascina e rilascia</strong> - trascina un file nell'area
		dell'editor.</li>
	<li><strong>Incolla</strong> - clicca nell'editor e incolla
		(<kbd>Ctrl</kbd>+<kbd>V</kbd>). L'analisi automatica parte 500 ms
		dopo l'ultima digitazione.</li>
	<li><strong>File → Nuovo da Modello</strong>
		(<kbd>Ctrl</kbd>+<kbd>N</kbd>) - ADT, ORM, ORU, SIU e altri
		preconfigurati. MSH-7 e MSH-10 vengono compilati con timestamp e
		GUID correnti.</li>
</ul>

<div class="note">Al primo avvio ricevi un <strong>trial Pro di 7
giorni</strong> con tutte le funzionalità sbloccate. Alla scadenza,
BridgeLab continua a funzionare con il livello Community - non perdi
mai i tuoi messaggi.</div>
`,
},
{
	id: 'editor',
	heading: 'Editor',
	body: `
<p>L'area dell'editor è un'istanza <strong>Monaco</strong> con una
grammatica specifica per HL7. I codici di segmento sono colorati viola,
i separatori di campo grigi, e i payload ED/base64 sono troncati
automaticamente per mantenere l'editor reattivo sui messaggi grandi.</p>

<h3>Auto-completamento e hover</h3>
<p>Digita <code>P</code> a inizio riga - Monaco suggerisce
<code>PID</code>, <code>PV1</code>, <code>PV2</code>, ecc. Dopo aver
inserito un segmento, l'autocomplete propone valori di campo
(codici gender, codici ACK, classe paziente...). Passando con il mouse
su un campo appaiono nome, tipo di dato e flag di obbligatorietà tratti
dallo standard HL7.</p>

<h3>Troncamento dei campi grandi</h3>
<p>I campi oltre la soglia di troncamento (default 100 byte,
modificabile in <strong>Impostazioni → Analizzatore</strong>) appaiono
come <code>{...N bytes}</code>. Il contenuto completo non viene mai
perso - puoi espanderlo su richiesta dal menu contestuale o
dall'Ispettore Campo.</p>

<h3>Menu contestuale (tasto destro)</h3>
${mockupContextMenu}
<p>Il menu raggruppa le azioni in tre sezioni:</p>
<ul>
	<li><strong>Navigazione:</strong> Mostra Segmento nel Tree
		(<kbd>Alt</kbd>+<kbd>T</kbd>) - apre il tree e evidenzia il
		campo esatto sotto il cursore; Espandi / Comprimi per valori
		troncati.</li>
	<li><strong>Appunti:</strong> Copia Segmento
		(<kbd>Alt</kbd>+<kbd>C</kbd>), Copia Messaggio Completo (con
		campi espansi), Copia Messaggio Troncato (sicuro per email).</li>
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
<strong>componenti</strong>. Mostralo/nascondilo con
<kbd>Ctrl</kbd>+<kbd>B</kbd> o <strong>Visualizza → Struttura
Messaggio</strong>.</p>

<h3>Navigare tra tree ed editor</h3>
<ul>
	<li><strong>Editor → Tree:</strong> tasto destro su un campo in
		Monaco, scegli <em>Mostra nel Tree</em>. Il tree espande il
		segmento, seleziona il campo esatto (fino al componente) e
		scrolla fino a renderlo visibile.</li>
	<li><strong>Tree → Editor:</strong> tasto destro su un nodo del
		tree, scegli <em>Mostra nell'Editor</em>. Monaco salta alla
		riga, posiziona il cursore nella colonna corretta e seleziona
		l'intervallo del campo.</li>
</ul>

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
		completo</em> per campi troncati</li>
</ul>
<p>I segmenti sconosciuti (Z-segment o codici custom non nello standard)
mostrano <em>Non nello standard HL7</em> ma restano perfettamente
modificabili.</p>

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
`,
},
];
