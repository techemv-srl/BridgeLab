import type { ManualSection } from '../helpContent';
import { mockupAppShellDe as mockupAppShell, mockupContextMenuDe as mockupContextMenu } from './mockups';

export const getStarted: ManualSection = {
	id: 'getting-started',
	heading: 'Erste Schritte',
	body: `
<p>BridgeLab ist ein moderner Nachrichteneditor für HL7 v2.x und FHIR,
entwickelt für Fachleute der Gesundheitsintegration. Er basiert auf
einem Rust-Backend für schnelles Parsing (eine 10-MB-Nachricht mit Base64-Anhang
öffnet sich in etwa 2 Sekunden) und einem Svelte-5-Frontend mit dem Monaco-Editor.</p>

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
		Tastenanschlag (Verzögerung, oder abschalten, unter
		<strong>Einstellungen → Analysator</strong>).</li>
	<li><strong>Was sich öffnen lässt:</strong> Leerzeilen, Leerzeichen,
		ein BOM oder MLLP-Rahmen vor <code>MSH</code>, UTF-16-Dateien (das
		„Unicode“ des Windows-Editors) und FHS/BHS-Batchdateien werden so
		gelesen, wie sie sind. Eine Datei, die BridgeLab nicht analysieren
		kann, öffnet sich trotzdem als Text zum Korrigieren, mit einem
		Hinweis auf den Grund.</li>
	<li><strong>Datei → Neue Nachricht aus Vorlage...</strong>
		(<kbd>Ctrl</kbd>+<kbd>N</kbd>) - vorbefüllte Vorlagen für ADT,
		ORM, ORU, SIU und mehr. Felder wie MSH-7 und MSH-10 werden mit
		dem aktuellen Zeitstempel und einer eindeutigen Nachrichten-ID
		befüllt.</li>
	<li><strong>Datei → Beispielnachrichten</strong> (auch auf dem
		Startbildschirm) - vollständige, realistische Nachrichten statt
		Gerüste: ADT für Aufnahme, Registrierung, Aktualisierung, Entlassung
		und Zusammenführung, ORU mit Befunden (ein Blutbild mit zwölf Werten,
		ein Stoffwechselprofil), ORM, SIU, MDM, DFT, VXU und ein ACK, in den
		Versionen 2.3, 2.5 und 2.5.1. Nach Version filtern, Vorschau ansehen
		und in einem neuen Tab öffnen. Jedes Beispiel besteht die
		Validierung; Patienten und Daten sind fiktiv.</li>
</ul>

<div class="note">Beim ersten Start erhalten Sie eine <strong>14-tägige
Pro-Testversion</strong> mit allen freigeschalteten Pro-Funktionen (SOAP
und Prioritäts-Support gehören zu Enterprise). Nach
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
Feldtrenner grau, und ED-/Base64-Payloads und andere lange Werte
werden gefaltet angezeigt, damit der Editor auch bei großen Nachrichten
schnell bleibt (siehe unten).</p>

<h3>Autovervollständigung und Hover</h3>
<p>Tippen Sie <code>P</code> am Anfang einer neuen Zeile - Monaco
schlägt <code>PID</code>, <code>PV1</code>, <code>PV2</code> usw. vor.
Sobald ein Segment eingegeben ist, schlägt die
Pipe-Autovervollständigung Feldwerte vor (Geschlechtscodes, ACK-Codes,
Patientenklasse...). Beim Überfahren eines Feldes mit der Maus
erscheinen sein Name, sein Datentyp und das Pflichtkennzeichen aus dem
HL7-Standard.</p>

<h3>Gefaltete lange Felder</h3>
<p>Felder, die länger als die Faltschwelle sind (Standard 100 Zeichen,
unter <strong>Einstellungen → Analysator</strong>), werden <em>gefaltet</em>
angezeigt: ein Base64-Anhang in OBX-5, eine lange Notiz oder ein
JSON-<code>data</code>-String erscheint als kompakter Chip wie
<code>⟨Base64 · 5.1 KB⟩</code>, während Trennzeichen und die übrigen
Komponenten sichtbar bleiben. Falten ist nur eine Ansicht: Die Nachricht
ist immer vollständig, und Speichern, Sitzung, Validierung, Baum und
Kopieren verwenden den ganzen Text.</p>
<ul>
	<li><strong>Aufklappen:</strong> auf den Chip klicken, oder den Cursor
		daneben setzen und <kbd>Alt</kbd>+<kbd>Eingabe</kbd> drücken. Beim
		Überfahren erscheinen die ersten Zeichen.</li>
	<li><strong>Wieder falten:</strong> Rechtsklick → <em>Dieses Feld
		falten</em> auf einem langen Wert, oder <em>Alle langen Felder
		falten</em>.</li>
	<li><strong>Alles auf einmal:</strong> das Abzeichen <em>N gefaltet</em>
		in der Statusleiste klappt alles auf; das Kontextmenü hat beide
		Befehle.</li>
	<li>Ein Chip bewegt sich als Einheit: Pfeiltasten überspringen ihn,
		Rücktaste/Entf markieren ihn zuerst, er wird also nie halb
		gelöscht. Beim Kopieren einer Auswahl wird der volle Inhalt
		kopiert.</li>
	<li><strong>Suchen und Ersetzen</strong> (<kbd>Ctrl</kbd>+<kbd>F</kbd>,
		<kbd>Ctrl</kbd>+<kbd>H</kbd>) arbeiten auf dem vollständigen Text:
		Ein gefaltetes Feld mit einem Treffer wird aufgeklappt, sodass der
		Treffer wie jeder andere gezählt, angezeigt und ersetzt wird.</li>
</ul>

<h3>Editor-Einstellungen</h3>
<p><strong>Bearbeiten → Einstellungen → Editor</strong> (Ctrl+,) ändert
Aussehen und Verhalten des Editors: Schrift und Größe, Tabulatorbreite,
Zeilenumbruch, sichtbare Leerzeichen, Minimap, Zeilennummern, weiches
Scrollen, Klammerfarben, Hervorhebung weiterer Vorkommen des Wortes unter dem
Cursor, anklickbare Links, fixierte Kopfzeile beim Scrollen (Sticky Scroll)
und die Quelle der Wortvorschläge (diese Nachricht, alle offenen Nachrichten
oder keine; Vorschläge für HL7-Felder und -Werte funktionieren in jedem
Modus). Änderungen gelten sofort. Jeder Tab behält beim Wechseln seinen
eigenen Rückgängig- und Wiederholen-Verlauf (Ctrl+Z, Ctrl+Y).</p>

<h3>Kontextmenü (rechte Maustaste)</h3>
${mockupContextMenu}
<p>Das Menü gruppiert die Aktionen in drei Abschnitte:</p>
<ul>
	<li><strong>Navigation:</strong> Segment im Baum anzeigen
		(<kbd>Alt</kbd>+<kbd>T</kbd>) - öffnet den Baum und hebt exakt
		das Feld unter dem Cursor hervor; Aufklappen / Falten für
		lange Werte.</li>
	<li><strong>Zwischenablage:</strong> Segment kopieren
		(<kbd>Alt</kbd>+<kbd>C</kbd>), Vollständige Nachricht kopieren
		(mit erweiterten Feldern), Gekürzte Nachricht kopieren
		(lange Felder gekürzt, damit sie in eine E-Mail passt; Patientendaten
		werden unverändert kopiert: vorher anonymisieren).</li>
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
<strong>Komponenten</strong>; ein wiederholtes Feld listet jede
Wiederholung (<code>PID-3(1)</code>, <code>PID-3(2)</code>) mit ihren
Komponenten. Ein- und ausblenden mit
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

<p>Leerzeilen zwischen Segmenten verfälschen die Sprünge nicht. Im Baum
bewegen <kbd>↑</kbd>/<kbd>↓</kbd> die Auswahl, <kbd>→</kbd> klappt einen
Knoten auf und <kbd>←</kbd> klappt ihn zu oder springt zum
übergeordneten Knoten; <kbd>Pos1</kbd>/<kbd>Ende</kbd> und
<kbd>Bild↑</kbd>/<kbd>Bild↓</kbd> springen. Beim Bearbeiten behält der
Baum, was Sie aufgeklappt und ausgewählt haben, und der Feld-Inspektor
zeigt den aktuellen Wert.</p>

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
	<li>Aktueller Wert und Länge; für lange Felder, die der Editor
		gefaltet zeigt, eine Schaltfläche <em>Vollständigen Wert
		anzeigen</em></li>
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

<h3>Segmentraster</h3>
<p>Eine Befundnachricht kann Dutzende OBX enthalten; der Baum zeigt sie
einzeln. <strong>Ansicht → Segmentraster</strong>
(<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd>) oder <em>Alle OBX als
Tabelle anzeigen</em> im Kontextmenü eines Segments öffnet unten ein
Panel, das alle Vorkommen eines Segmenttyps als Tabelle zeigt: eine
Zeile pro Vorkommen, eine Spalte für jedes Feld, das in mindestens einem
Vorkommen einen Wert hat, mit Position und Feldname in der HL7-Version
der Nachricht. Kodierte Werte zeigen ihre Bedeutung unter dem Code, wie
im Baum; lange Werte werden mit Auslassungspunkten gekürzt. Wählen Sie
ein anderes Segment aus der Liste (jedes mit seiner Anzahl), tippen Sie
in <em>Zeilen filtern</em>, um nur Zeilen mit einem Text zu behalten, und
klicken Sie auf eine Zelle, um das Feld im Editor zu markieren. Das
Raster ist schreibgeschützt und folgt der Nachricht beim Bearbeiten.</p>

<h3>Zwei Nachrichten vergleichen</h3>
<p><strong>Werkzeuge → Nachrichten vergleichen…</strong> öffnet einen
Seite-an-Seite-Diff zweier beliebiger geöffneter Tabs mit
HL7-Syntaxhervorhebung. Wählen Sie links/rechts über die Dropdowns,
tauschen Sie die Seiten mit der ⇆-Schaltfläche und schließen Sie mit
<kbd>Esc</kbd>. Es müssen mindestens zwei Tabs geöffnet sein.</p>

<h3>Codierte Felder: was ein Code bedeutet</h3>
<p>BridgeLab bringt die HL7-Wertetabellen mit — 394 Tabellen, rund
5.000 Codes — und weiß je Version, aus welcher Tabelle jedes codierte
Feld und jede codierte Komponente ihre Werte bezieht. Ein codierter
Wert wird überall erklärt, wo Sie ihm begegnen: der Baum zeigt die
Bedeutung neben dem Wert (<code>M — Male</code>,
<code>ADT — ADT message</code>, <code>F — Final results</code>), der
Hover über dem Feld im Editor nennt sie ebenfalls, und die
Autovervollständigung in einem codierten Feld bietet alle Werte seiner
Tabelle an. Komponenten sind eingeschlossen: MSH-9.2 wird aus der
Ereignistabelle erklärt, PID-3.5 aus der Tabelle der
Identifikatortypen.</p>
<p>Der Feld-Inspektor listet die ganze Tabelle des ausgewählten Feldes
oder der Komponente auf und hebt den aktuellen Wert hervor. Ob ein Wert
außerhalb der Tabelle ein Problem ist, hängt vom Datentyp ab, und der
Inspektor sagt, welcher Fall vorliegt: ein <code>ID</code>-Feld bezieht
seine Werte aus einer von HL7 definierten Tabelle (<em>Zulässige
Werte</em>), ein nicht gelisteter Wert ist nicht standardkonform — eine
Warnung erscheint; ein <code>IS</code>-Feld aus einer benutzerdefinierten
Tabelle (<em>Vorgeschlagene Werte</em>), in der jede Einrichtung eigene
Codes ergänzt und das Fehlen nichts bedeutet. Manche benutzerdefinierten
Tabellen haben gar keine Standardwerte (IN1-2 Insurance Plan ID); diese
Felder zeigen keine Liste.</p>
<p class="note">Welche Tabelle ein Feld verwendet, folgt der deklarierten
HL7-Version; der Tabelleninhalt ist ein einziger Satz für alle
Versionen, so wie die Quelle ihn liefert. Ein in einer späteren Version
ergänzter Code wird daher auch für eine frühere akzeptiert.</p>

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
Trennzeichen bis zum letzten Pflichtfeld. Das Grundgerüst verwendet die
Trennzeichen der Nachricht, und <kbd>Ctrl</kbd>+<kbd>Z</kbd> entfernt es
wieder.</p>
`,
};
