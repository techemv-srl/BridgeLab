import type { ManualSection } from '../helpContent';
import { mockupValidation, mockupCommunication } from './mockups';

export const validationSection: ManualSection = {
	id: 'validation',
	heading: 'Validierung',
	body: `
<p>Drücken Sie <kbd>F6</kbd> oder wählen Sie
<strong>Werkzeuge → Validieren</strong>, um alle Validierungsregeln auf
der aktiven Nachricht auszuführen. Die Ergebnisse erscheinen im unten
angedockten Validierungspanel, gruppiert nach Schweregrad.</p>

${mockupValidation}

<h3>Integrierte Regeln</h3>
<ul>
	<li><strong>Struktur:</strong> Das erste Segment muss MSH sein;
		Segmentcodes müssen aus 3 alphanumerischen Zeichen bestehen;
		kein doppeltes MSH.</li>
	<li><strong>MSH-Header:</strong> MSH-9 (Nachrichtentyp), MSH-10
		(Control-ID), MSH-12 (Version) sind Pflichtfelder.</li>
	<li><strong>Pflichtfelder:</strong> Pflichtfelder je Segment gemäß
		HL7-Standard (z. B. PID-3 Patient Identifier List).</li>
	<li><strong>Längenlimits:</strong> warnt, wenn ein Feld die
		veröffentlichte <code>max_length</code> überschreitet.</li>
	<li><strong>Datentypen:</strong> numerische Felder (SI, NM) werden
		auf nicht-numerische Zeichen geprüft; Zeitstempelformate (TS)
		auf Länge und reine Ziffernfolge.</li>
</ul>

<h3>Filtern und Navigation</h3>
<p>Klicken Sie auf die Badges Fehler / Warnung / Info, um zu filtern.
Ein Klick auf eine Problemzeile springt zum betroffenen Segment im
Editor.</p>

<h3>Eigene Regeln aus Plugin-Packs</h3>
<p>Legen Sie eine JSON-Datei unter
<code>&lt;config&gt;/BridgeLab/plugins/validation/</code> ab, um eigene
Prüfungen ohne Neukompilieren hinzuzufügen. Siehe
<em>Plugin-Packs</em> weiter unten.</p>

<h3>Stapelvalidierung (Pro)</h3>
<p><strong>Werkzeuge → Stapelvalidierung…</strong> validiert einen
ganzen Ordner (oder eine handverlesene Auswahl) von
<code>.hl7</code>-/<code>.txt</code>-/<code>.dat</code>-Dateien in
einem Durchlauf: eine Zeile pro Datei mit Nachrichtentyp, Version,
Segmentanzahl und Fehler-/Warnungssummen. Filtern Sie auf Fehlschläge,
öffnen Sie eine Datei per Klick auf ihre Zeile im Editor und
exportieren Sie die gesamte Tabelle als CSV für das
Change-Review-Ticket. Die Dateien werden im Speicher verarbeitet —
Ihren Tabs wird nichts hinzugefügt.</p>

<h3>Testnachrichten-Generator</h3>
<p><strong>Werkzeuge → Testnachrichten erzeugen…</strong> erstellt
syntaktisch gültige ADT-/ORU-/ORM-Nachrichten mit plausiblen
<em>synthetischen</em> Patientendaten — Namen, Geburtsdaten, MRNs,
Adressen und Laborpanels mit Referenzbereichen (ein realistischer
Anteil der Ergebnisse ist bewusst pathologisch und entsprechend
markiert). Echtes PHI wird niemals verwendet. Geben Sie einen
<strong>Seed</strong> an, um ein Set reproduzierbar zu machen, und
öffnen Sie die Nachrichten anschließend in Tabs oder speichern Sie sie
als nummerierte <code>.hl7</code>-Dateien in einen Ordner — fertige
Regressions-Fixtures für die Stapelvalidierung oben.</p>

<h3>Validierung über die CLI</h3>
<p>Das Begleitwerkzeug <code>bridgelab-cli</code> bietet denselben
Validator für den Headless-Einsatz (CI-Pipelines, Batch-Screening):</p>
<pre><code>bridgelab-cli validate message.hl7
bridgelab-cli validate '*.hl7' --format junit &gt; report.xml
bridgelab-cli batch ./inbox --json</code></pre>
`,
};

export const communicationSection: ManualSection = {
	id: 'communication',
	heading: 'Kommunikation (MLLP / HTTP)',
	body: `
<p>Öffnen Sie das untere Kommunikationspanel mit
<kbd>Ctrl</kbd>+<kbd>K</kbd> oder über
<strong>Werkzeuge → Kommunikationspanel</strong>. Drei Tabs: MLLP,
HTTP und Verlauf.</p>

${mockupCommunication}

<h3>MLLP-Client</h3>
<ol>
	<li>Geben Sie <em>Host</em> + <em>Port</em> ein (z. B.
		<code>localhost:2575</code>).</li>
	<li>Die Nachricht im aktiven Tab wird automatisch verwendet.</li>
	<li>Klicken Sie auf <strong>Senden</strong>. Framing
		(<code>0x0B</code> ... <code>0x1C 0x0D</code>), Transport und
		das Warten auf das ACK übernimmt das Rust-Backend.</li>
	<li>Das ACK erscheint im Ergebnisbereich samt Umlaufzeit.
		<em>Accept</em> (AA), <em>Error</em> (AE) und <em>Reject</em>
		(AR) werden alle mit dem originalen
		<code>MSA|AA|{control-id}</code> angezeigt.</li>
</ol>

<h3>ACK-Generator</h3>
<p>Die Zeile <strong>ACK-Generator</strong> im MLLP-Tab baut eine
Bestätigung für die aktuell im Editor stehende Nachricht: Wählen Sie
den Code (AA akzeptieren, AE Fehler, AR ablehnen) und klicken Sie auf
<strong>ACK aus aktueller Nachricht erzeugen</strong>. Die Message
Control ID wird aus MSH-10 gelesen (unter Beachtung des in MSH-1
deklarierten Feldtrenners), und das erzeugte ACK öffnet sich in einem
neuen Tab — bereit zum Zurücksenden oder als Fixture. Hat die aktuelle
Nachricht kein MSH-10, verweigert der Generator die Erzeugung, statt
ein nicht zuordenbares ACK zu produzieren.</p>

<h3>MLLP-Listener (Pro)</h3>
<p>Klicken Sie auf <strong>Lauschen starten</strong>, um einen Server
auf dem gewählten Port zu betreiben. Eingehende Nachrichten öffnen sich
in einem neuen Tab (abschaltbar, siehe unten), und ein Auto-ACK mit dem
konfigurierten Code (AA/AE/AR) wird zurückgesendet. So prüfen Sie
schnell, was Ihr vorgelagertes System tatsächlich sendet.</p>

<h3>Listener-Konsole</h3>
<p>Solange der Listener läuft, erscheint jede empfangene Nachricht als
Zeile in der Konsole: lokale Zeit, Peer-Adresse, Payload-Größe, der
tatsächlich zurückgeschriebene ACK-Code (grünes <code>AA</code>, rotes
<code>AE</code>/<code>AR</code>, — wenn Auto-ACK aus ist), die
verwendete Zeichenkodierung und die erste Zeile der Nachricht.
<strong>Klicken Sie auf eine Zeile, um diese Nachricht erneut in einem
Tab zu öffnen.</strong> Listener-Fehler erscheinen inline als rote
Zeilen.</p>
<p>Der Schalter <em>„Empfangene Nachrichten in neuem Tab öffnen“</em>
(standardmäßig aktiv) lässt sich bei Tests mit hohem Volumen
deaktivieren: Die Nachrichten landen dann nur in der Konsole, und Sie
öffnen gezielt die, die Sie brauchen.</p>
<p class="note">Die für das Öffnen per Klick vorgehaltenen
vollständigen Nachrichteninhalte sind auf ein rollierendes Budget von
32&nbsp;MB begrenzt. In langen unbeaufsichtigten Sitzungen verlieren
die ältesten Zeilen ihren vollständigen Inhalt (sie erscheinen
abgedunkelt) — die Metadatenzeile bleibt, und nichts ist „verloren
gegangen“: Nur die Kopie für das Öffnen per Klick wurde freigegeben,
um den Speicher zu begrenzen.</p>

<h3>Zeichenkodierung</h3>
<p>Sender und Listener besitzen jeweils einen
<strong>Kodierung</strong>-Selektor mit den in realen Umgebungen
üblichen Zeichensätzen: <code>UTF-8</code>, <code>ISO-8859-1</code>
(Latin-1), <code>ISO-8859-2</code>, <code>ISO-8859-15</code>,
<code>windows-1252</code>, <code>windows-1250</code>,
<code>windows-1251</code> und <code>ASCII</code>. Standard ist UTF-8
mit automatischem Latin-1-Fallback, der den meisten Legacy-Verkehr
auch bei leerem MSH-18 akzeptiert. Der Listener kodiert sein ACK mit
demselben Zeichensatz, sodass die Gegenstelle nie Zeichensalat sieht.
Sende- und Empfangskodierung sind unabhängig voneinander.</p>

<h3>HTTP</h3>
<p>GET-Anfragen sind in der Community-Stufe verfügbar. POST/PUT/DELETE,
eigene Authentifizierungs-Header (Basic, Bearer) und das Folgen von
Weiterleitungen erfordern Pro. Als Body wird standardmäßig die
Nachricht des aktuellen Tabs verwendet; er lässt sich aber
überschreiben.</p>

<h3>Verlauf</h3>
<p>Jeder Sende- und Empfangsvorgang wird protokolliert (Host, Port,
Größe, Antwortcode, Umlaufzeit). Die letzten 100 Einträge bleiben über
Neustarts hinweg erhalten; ein Klick auf eine Zeile zeigt Anfrage und
Antwort vollständig an.</p>

<h3>Verbindungsprofile</h3>
<p>Speichern Sie häufig genutzte Endpunkte als benannte Profile über
die Zeile <strong>Profil</strong>: Namen eingeben und auf
<em>Speichern</em> klicken. MLLP-Profile speichern Host, Port, Timeout
und Auto-ACK; HTTP-Profile speichern URL, Header und Timeout. Die
Auswahl eines Profils übernimmt es ins Formular; Speichern unter einem
vorhandenen Namen überschreibt es; <em>Löschen</em> entfernt das
ausgewählte Profil. Profile liegen in der lokalen Datenbank und
überstehen Neustarts.</p>
`,
};

export const anonymizationSection: ManualSection = {
	id: 'anonymization',
	heading: 'Anonymisierung &amp; Export',
	body: `
<p><strong>Werkzeuge → Anonymisieren</strong> erkennt PHI-Felder in den
gängigen patientenidentifizierenden Segmenten (PID, NK1, IN1, GT1) und
maskiert sie nach Sensibilitätsstufe.</p>

<table>
	<tr><th>Stufe</th><th>Beispiel</th><th>Strategie</th></tr>
	<tr><td><strong>Hoch</strong></td><td>Patientenname, SSN, MRN</td>
		<td>Text wird zu <code>REDACTED</code>; Zahlen werden zu Nullen
		gleicher Länge (erhält die Feldbreite für nachgelagerte
		Parser).</td></tr>
	<tr><td><strong>Mittel</strong></td><td>Mädchenname der Mutter, Telefon</td>
		<td>Erstes Zeichen bleibt erhalten, der Rest wird durch
		<code>***</code> ersetzt.</td></tr>
	<tr><td><strong>Niedrig</strong></td><td>Alias, Identifikatoren mit geringem Risiko</td>
		<td>Die ersten 3 Zeichen bleiben erhalten, der Rest wird durch
		<code>...</code> ersetzt.</td></tr>
</table>

<p>Der Dialog listet jedes erkannte PHI-Feld auf, bevor Sie die
Maskierung ausführen — Sie sehen also vorab, was sich ändern wird. Die
Ausgabe:</p>
<ul>
	<li><strong>Öffnet sich in einem neuen Tab</strong> - die
		Originalnachricht bleibt unangetastet in ihrem eigenen Tab.</li>
	<li><strong>Lässt sich direkt in die Zwischenablage
		kopieren</strong>.</li>
	<li><strong>Bewahrt die Struktur</strong> - Segmentreihenfolge,
		Pipe-Anzahl und Komponententrenner bleiben unverändert, das
		Ergebnis parst also weiterhin als gültiges HL7.</li>
</ul>

<h3>Eigene PHI-Felder über Plugins</h3>
<p>Installationen mit regionalen oder herstellerspezifischen
Identifikatoren (nationale EU-IDs, interne Z-Segment-Felder) können
den Katalog erweitern, indem sie eine JSON-Datei unter
<code>&lt;config&gt;/BridgeLab/plugins/anonymization/</code> ablegen.</p>

<h3>Batch-Anonymisierung (Pro)</h3>
<p><strong>Werkzeuge → Batch-Anonymisierung…</strong> maskiert einen
ganzen Ordner in einem Durchlauf: Quelldateien oder einen Ordner
wählen, Ausgabeordner wählen, ausführen. Jede Nachricht durchläuft
dieselbe Pipeline wie der interaktive Dialog (integrierter PHI-Katalog
+ aktive Plugin-Regeln) und wird als Kopie in den Ausgabeordner
geschrieben — <strong>Originale werden nie angetastet</strong>: Das
Werkzeug weigert sich, eine ausgewählte Quelldatei zu überschreiben,
und gleichnamige Eingaben aus verschiedenen Ordnern erhalten
numerische Suffixe, statt einander zu überschreiben. Eine Zeile pro
Datei meldet die Anzahl maskierter PHI-Felder oder den Fehler; es
gelten dieselben Limits wie bei der Stapelvalidierung (5000 Dateien /
10&nbsp;MB).</p>

<h3>Export</h3>
<p>Pro-Anwender können die strukturierte Nachricht über
<strong>Werkzeuge → Als JSON / CSV exportieren</strong> als JSON oder
CSV exportieren. Nützlich, um HL7-Daten in Analysewerkzeuge zu laden
(Power BI, Excel, pandas).</p>

<div class="warn">Die Anonymisierung ersetzt Werte <em>im Editor</em>.
Bewahren Sie Ihre Originaldatei stets als maßgebliche Quelle auf - die
anonymisierte Kopie ist zum Weitergeben gedacht, nicht zur
Langzeitablage.</div>
`,
};

export const testCasesSection: ManualSection = {
	id: 'testcases',
	heading: 'Testfall-Bibliothek',
	body: `
<p>Die Testfall-Bibliothek (<kbd>Ctrl</kbd>+<kbd>L</kbd>) speichert
wiederverwendbare Nachrichten mit Name, Kategorie, Tags und
Beschreibung. Mit <strong>Aktuelle Nachricht speichern</strong>
erfassen Sie den aktiven Tab, oder Sie legen Fälle von Grund auf neu
an. Die Fälle liegen dauerhaft in der lokalen Datenbank und lassen
sich über jedes ihrer Felder durchsuchen.</p>

<p class="note">Die Community-Stufe erlaubt bis zu 10 gespeicherte
Testfälle — vorhandene Fälle bleiben immer sichtbar, bearbeitbar und
ausführbar; nur neue Speicherungen über dem Limit fragen nach einem
Upgrade.</p>

<h3>Erwartete Ergebnisse</h3>
<p>Jeder Fall kann einen <strong>erwarteten Nachrichtentyp</strong>
deklarieren (<code>ADT</code> passt auf jedes ADT-Ereignis,
<code>ADT^A01</code> ist exakt) sowie ein <strong>erwartetes
Validierungsergebnis</strong> (gültig / ungültig). So wird aus einem
Snippet ein Test.</p>

<h3>Prüfungen ausführen</h3>
<p><strong>Prüfen</strong> parst und validiert einen einzelnen Fall
tatsächlich — HL7 v2 oder FHIR, automatisch erkannt — und vergleicht
das Ergebnis mit seinen Erwartungen. <strong>Alle ausführen</strong>
tut dasselbe für jeden Fall, der zur aktuellen Suche passt, mit einem
Bestanden/Fehlgeschlagen-Badge pro Zeile und einer
Bestanden/Gesamt-Zusammenfassung in der Werkzeugleiste. Nach einer
Schnittstellenänderung sagt Ihnen ein Klick, welche Ihrer
Referenznachrichten nicht mehr bestehen. Das Bearbeiten eines Falls
setzt sein gespeichertes Ergebnis bis zum nächsten Lauf zurück.</p>

<h3>Sitzungswiederherstellung</h3>
<p>BridgeLab speichert Ihre offenen Tabs (einschließlich
ungespeicherter Änderungen) und öffnet sie beim nächsten Start erneut
— wie in Notepad++. Steuern Sie dies unter
<strong>Einstellungen → Sitzung</strong>: Schalten Sie <em>Offene Tabs
beim Start wiederherstellen</em> um, oder löschen Sie mit
<em>Gespeicherte Sitzung löschen</em> die gespeicherten Tabs (das
deaktiviert auch die Wiederherstellung, sodass der nächste Start auf
dem Willkommensbildschirm beginnt).</p>
`,
};
