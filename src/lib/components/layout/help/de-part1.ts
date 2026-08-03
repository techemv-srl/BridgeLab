import type { ManualSection } from '../helpContent';
import { mockupAppShellDe as mockupAppShell, mockupContextMenuDe as mockupContextMenu } from './mockups';

export const getStarted: ManualSection = {
	id: 'getting-started',
	heading: 'Erste Schritte',
	body: `
<p>BridgeLab ist ein moderner Nachrichteneditor für HL7 v2.x und FHIR,
entwickelt für Fachleute der Gesundheitsintegration. Er basiert auf
einem Rust-Backend für schnelles Parsing (verarbeitet 10-MB-Nachrichten
in unter 2 Sekunden) und einem Svelte-5-Frontend mit dem Monaco-Editor.</p>

<p>Das Hauptfenster ist in vier Bereiche unterteilt:</p>
${mockupAppShell}

<ol>
	<li><strong>Menüleiste und Testversions-Banner</strong> oben - die
		Menüs Datei, Bearbeiten, Ansicht, Werkzeuge und Hilfe sowie ein
		gelbes/rotes Banner, das an den Status der Pro-Testversion
		erinnert.</li>
	<li><strong>Baum-Panel</strong> links - die analysierte
		Nachrichtenstruktur mit Pfeilen zum Auf- und Zuklappen sowie
		unten ein Feld-Inspektor mit den HL7-Schemainformationen zum
		ausgewählten Knoten.</li>
	<li><strong>Editor und Tabs</strong> in der Mitte - Monaco-Editor
		mit HL7-Syntaxhervorhebung; Multi-Tab-Leiste, um mehrere
		Nachrichten gleichzeitig geöffnet zu halten.</li>
	<li><strong>Statusleiste</strong> unten - Nachrichtentyp, Version,
		Segmentanzahl, Cursorposition.</li>
</ol>

<h3>Eine Nachricht öffnen</h3>
<ul>
	<li><strong>Datei → Datei öffnen</strong> (<kbd>Ctrl</kbd>+<kbd>O</kbd>) -
		nativer Dateidialog für <code>.hl7</code>, <code>.txt</code>,
		<code>.msg</code>, <code>.json</code>, <code>.xml</code>.</li>
	<li><strong>Drag &amp; Drop</strong> - ziehen Sie eine Datei auf den
		Editorbereich.</li>
	<li><strong>Einfügen</strong> - klicken Sie in den Editor und fügen
		Sie die Nachricht ein (<kbd>Ctrl</kbd>+<kbd>V</kbd>). Die
		automatische Analyse startet 500 ms nach dem letzten
		Tastenanschlag.</li>
	<li><strong>Datei → Neue Nachricht aus Vorlage</strong>
		(<kbd>Ctrl</kbd>+<kbd>N</kbd>) - vorbefüllte Vorlagen für ADT,
		ORM, ORU, SIU und mehr. Felder wie MSH-7 und MSH-10 werden mit
		dem aktuellen Zeitstempel und einer frischen GUID befüllt.</li>
</ul>

<div class="note">Beim ersten Start erhalten Sie eine <strong>7-tägige
Pro-Testversion</strong> mit allen freigeschalteten Funktionen. Nach
Ablauf arbeitet BridgeLab mit dem Community-Funktionsumfang weiter -
Ihre Nachrichten gehen nie verloren.</div>

<p>Die Karten <strong>BridgeLab entdecken</strong> auf dem Startbildschirm
öffnen direkt die Funktionen, die BridgeLab auszeichnen — Testnachrichten-
Generator, PHI-Anonymisierung, MLLP-Listener und XSD-Export — mit
PRO-Badges für die lizenzpflichtigen.</p>
`,
};

export const editorSection: ManualSection = {
	id: 'editor',
	heading: 'Editor',
	body: `
<p>Der Editorbereich ist eine <strong>Monaco</strong>-Instanz mit einer
HL7-spezifischen Grammatik. Segmentcodes werden violett eingefärbt,
Feldtrenner grau, und ED-/Base64-Payloads werden automatisch gekürzt,
damit der Editor auch bei großen Nachrichten schnell bleibt.</p>

<h3>Autovervollständigung und Hover</h3>
<p>Tippen Sie <code>P</code> am Anfang einer neuen Zeile - Monaco
schlägt <code>PID</code>, <code>PV1</code>, <code>PV2</code> usw. vor.
Sobald ein Segment eingegeben ist, schlägt die
Pipe-Autovervollständigung Feldwerte vor (Geschlechtscodes, ACK-Codes,
Patientenklasse...). Beim Überfahren eines Feldes mit der Maus
erscheinen sein Name, sein Datentyp und das Pflichtkennzeichen aus dem
HL7-Standard.</p>

<h3>Kürzung großer Felder</h3>
<p>Felder oberhalb der Kürzungsschwelle (Standard 100 Bytes,
einstellbar unter <strong>Einstellungen → Analysator</strong>)
erscheinen als <code>{...N bytes}</code>. Der vollständige Inhalt geht
nie verloren - erweitern Sie ihn bei Bedarf über das Kontextmenü oder
den <em>Feld-Inspektor</em>.</p>

<h3>Kontextmenü (rechte Maustaste)</h3>
${mockupContextMenu}
<p>Das Menü gruppiert die Aktionen in drei Abschnitte:</p>
<ul>
	<li><strong>Navigation:</strong> Segment im Baum anzeigen
		(<kbd>Alt</kbd>+<kbd>T</kbd>) - öffnet den Baum und hebt exakt
		das Feld unter dem Cursor hervor; Erweitern / Kürzen für
		gekürzte Werte.</li>
	<li><strong>Zwischenablage:</strong> Segment kopieren
		(<kbd>Alt</kbd>+<kbd>C</kbd>), Vollständige Nachricht kopieren
		(mit erweiterten Feldern), Gekürzte Nachricht kopieren
		(unbedenklich für E-Mails).</li>
</ul>

<div class="note">Die nativen Monaco-Tastenkombinationen
(<kbd>Ctrl</kbd>+<kbd>F</kbd> Suchen, <kbd>Ctrl</kbd>+<kbd>H</kbd>
Ersetzen, <kbd>Ctrl</kbd>+<kbd>Z</kbd> Rückgängig,
<kbd>Ctrl</kbd>+<kbd>D</kbd> Multi-Cursor) funktionieren im Editor
alle wie gewohnt.</div>
`,
};

export const treeSection: ManualSection = {
	id: 'tree-view',
	heading: 'Baumansicht &amp; Feld-Inspektor',
	body: `
<p>Der Baum links spiegelt die Hierarchie der HL7-Nachricht wider:
<strong>Segmente</strong> → <strong>Felder</strong> →
<strong>Komponenten</strong>. Ein- und ausblenden mit
<kbd>Ctrl</kbd>+<kbd>B</kbd> oder über
<strong>Ansicht → Nachrichtenstruktur</strong>.</p>

<h3>Zwischen Baum und Editor navigieren</h3>
<ul>
	<li><strong>Editor → Baum:</strong> Klicken Sie mit der rechten
		Maustaste auf ein Feld in Monaco und wählen Sie <em>Segment im
		Baum anzeigen</em>. Der Baum klappt das Segment auf, wählt exakt
		das Feld aus (bis auf Komponentenebene) und scrollt es in den
		sichtbaren Bereich.</li>
	<li><strong>Baum → Editor:</strong> Klicken Sie mit der rechten
		Maustaste auf einen Baumknoten und wählen Sie <em>Im Editor
		anzeigen</em>. Monaco springt zur Zeile, setzt den Cursor in die
		richtige Spalte und markiert den Feldbereich.</li>
</ul>

<h3>Feld-Inspektor-Panel</h3>
<p>Klicken Sie auf das <strong>ⓘ</strong>-Symbol in der Kopfzeile des
Baum-Panels (oder <strong>Ansicht → Feld-Inspektor</strong>), um die
aus dem Schema abgeleiteten Metadaten des aktuell ausgewählten Knotens
anzuzeigen:</p>
<ul>
	<li>HL7-Position (z. B. <code>PID-5</code>) und kanonischer Name
		(Patient Name)</li>
	<li>Datentyp (XPN, CX, ST, ...), maximale Länge,
		Pflicht-/Wiederholungskennzeichen, Beschreibung</li>
	<li>Aktueller Wert und Länge; für gekürzte Felder eine Schaltfläche
		<em>Vollständigen Wert anzeigen</em></li>
</ul>
<p>Unbekannte Segmente (Z-Segmente oder benutzerdefinierte Codes
außerhalb des Standards) zeigen <em>Nicht im HL7-Standard</em>, bleiben
aber uneingeschränkt bearbeitbar.</p>

<h3>Im Baum suchen</h3>
<p>Das Suchfeld oben im Baum findet Treffer nach
<strong>Segmenttyp</strong> (<code>PID</code>),
<strong>Schema-Feldname</strong> (<code>Patient Name</code>) und
<strong>Feldwert</strong> — einschließlich Feldern in Segmenten, die
Sie noch nicht aufgeklappt haben. Mit
<kbd>Enter</kbd>/<kbd>Shift</kbd>+<kbd>Enter</kbd> springen Sie durch
die Treffer, <kbd>Esc</kbd> leert die Suche, und
<kbd>Ctrl</kbd>+<kbd>F</kbd> springt zum Suchfeld, solange der Baum den
Fokus hat. Ein Klick auf einen Treffer klappt das Segment auf, wählt
das Feld aus und scrollt es in den sichtbaren Bereich.</p>
<p class="note">Die Baumsuche arbeitet auf HL7-v2-Nachrichten. Für
FHIR-Ressourcen verwenden Sie den eigenen Filter des
Bundle-Visualisierers oder das <kbd>Ctrl</kbd>+<kbd>F</kbd> des
Editors.</p>

<h3>Zwei Nachrichten vergleichen</h3>
<p><strong>Werkzeuge → Nachrichten vergleichen…</strong> öffnet einen
Seite-an-Seite-Diff zweier beliebiger geöffneter Tabs mit
HL7-Syntaxhervorhebung. Wählen Sie links/rechts über die Dropdowns,
tauschen Sie die Seiten mit der ⇆-Schaltfläche und schließen Sie mit
<kbd>Esc</kbd>. Es müssen mindestens zwei Tabs geöffnet sein.</p>

<h3>Zulässige Werte für codierte Felder</h3>
<p>Ist das ausgewählte Feld mit einer HL7-Wertetabelle hinterlegt
(PID-8 Administrative Sex, PV1-2 Patient Class, MSA-1 Acknowledgment
Code, ORC-1 Order Control, OBX-11 Result Status, …), listet der
Inspektor die <strong>zulässigen Werte</strong> mit ihrer Bedeutung auf
und hebt den aktuell in der Nachricht stehenden hervor. Steht der
aktuelle Wert nicht in der Tabelle, erscheint eine Warnung — ein
schneller Weg, nicht standardkonforme Codes zu entdecken, bevor das
empfangende System sie ablehnt.</p>

<h3>Schemabewusster Baum</h3>
<p><strong>Ansicht → Standardfelder anzeigen</strong> fügt
Platzhalterzeilen für jedes vom HL7-Standard definierte Feld ein, das
in der Nachricht <em>fehlt</em>. Platzhalter erscheinen abgedunkelt und
kursiv - so sehen Sie leicht, welche Felder Sie hinzufügen
<em>könnten</em>; im Editor lassen sie sich jedoch nicht ansteuern (sie
haben noch keine physische Position).</p>

<h3>Panels in der Größe anpassen</h3>
<p>Ziehen Sie den vertikalen Teiler zwischen Baum und Editor, um die
Breite zu ändern; ziehen Sie den horizontalen Teiler über dem
Feld-Inspektor, um dessen Höhe anzupassen. Beide Größen bleiben über
Neustarts hinweg erhalten.</p>

<h3>Vollständige Standardstruktur</h3>
<p>Mit aktiviertem <strong>Ansicht → Standardfelder anzeigen</strong> zeigt
der Baum auch die Segmente, die der Standard für den Nachrichtentyp
definiert, die aber in der Nachricht fehlen — ausgegraute Zeilen an ihrer
Standardposition, annotiert mit Gruppe, Kardinalität und Auswahlstatus. Beim
Aufklappen erscheint die vollständige Feldliste bis hinunter zu den
Komponenten zusammengesetzter Typen (z. B. OBX-16 → XCN-Komponenten).
<strong>Rechtsklick auf ein ausgegrautes Segment → Segment einfügen</strong>
fügt dessen Grundgerüst an der Standardposition in die Nachricht ein, mit
Trennzeichen bis zum letzten Pflichtfeld.</p>
`,
};
