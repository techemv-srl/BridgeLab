import type { ManualSection } from '../helpContent';

export const schemaExportSection: ManualSection = {
	id: 'schema-export',
	heading: 'Schema-Export (XSD)',
	body: `
<p>Sie brauchen ein XSD, das eine HL7-v2-Nachricht für eine
XML-basierte Pipeline oder eine Contract-First-Integration beschreibt,
oder das Sie einfach in ein Drittanbieter-Werkzeug laden möchten?
Öffnen Sie <strong>Werkzeuge → Nachrichtenschema als XSD
exportieren…</strong> — wählen Sie eine HL7-Version und einen
Nachrichtentyp, prüfen Sie das erzeugte Schema in der Vorschau und
speichern Sie es mit einem Klick.</p>

<h3>Was Sie erhalten</h3>
<p>Ein eigenständiges XSD nach der Standard-Kodierungskonvention
HL7 v2.xml:</p>
<ul>
	<li>Ein Wurzelelement pro Nachricht (z. B. <code>ADT_A01</code>) mit
		einem Inline-Complex-Type, der die Segmente und Segmentgruppen
		in ihrer Reihenfolge auflistet.</li>
	<li>Jedes Segment als Top-Level-<code>xsd:complexType</code>
		deklariert (<code>MSH</code>, <code>PID</code>,
		<code>OBX</code>, …), jedes Feld typisiert über die
		HL7-Datentypreferenz (<code>XPN</code>, <code>CX</code>,
		<code>HD</code>, …).</li>
	<li>Zusammengesetzte Datentypen in ihre Komponenten aufgelöst,
		primitive Datentypen (<code>ST</code>, <code>ID</code>,
		<code>NM</code>, …) als <code>xsd:simpleType</code>-Restriktionen
		auf <code>xsd:string</code>.</li>
	<li>Kardinalität bleibt erhalten: <code>minOccurs="0"</code> für
		optionale Felder, <code>maxOccurs="unbounded"</code> für
		wiederholbare.</li>
	<li>Gruppen wie <code>ORM_O01.ORDER_DETAIL</code> folgen der
		Namenskonvention <code>MESSAGE.GROUP</code>; von HL7 definierte
		Choice-Blöcke (<code>OBR | RQD | RQ1 | RXO | ODS | ODT</code>)
		werden als <code>xsd:choice</code> ausgegeben.</li>
	<li>Garantiert kompilierbar unter strikten Schema-Prozessoren — die
		wenigen HL7-Strukturen, deren Definition die XSD-Regel der
		Unique Particle Attribution verletzt, werden als annotierte,
		gelockerte Choice ausgegeben.</li>
</ul>

<h3>Aktionen</h3>
<ul>
	<li><strong>Kopieren</strong> — kopiert das XSD in die
		Zwischenablage, praktisch zum Einfügen in einen Editor oder
		Chat.</li>
	<li><strong>Speichern unter…</strong> — öffnet den Dateidialog des
		Betriebssystems mit <code>{MESSAGE}.xsd</code> als
		vorgeschlagenem Namen.</li>
</ul>

<h3>Abdeckung und Stufen</h3>
<p>Zehn HL7-Versionen sind vollständig enthalten: <strong>2.1, 2.2,
2.3, 2.3.1, 2.4, 2.5, 2.5.1, 2.6, 2.7 und 2.7.1</strong> — 2.320 im
Versions-Dropdown wählbare Nachrichtenstrukturen.</p>
<p>HL7 v2.7.1 ist eine technische Korrekturversion von v2.7 und
enthält dieselben Nachrichtendefinitionen; sie ist im Dropdown daher
mit <strong>(= v2.7)</strong> gekennzeichnet und exportiert aus dem
v2.7-Katalog.</p>
<p>Die kostenlose Stufe exportiert vier häufig genutzte
Nachrichtentypen in HL7 v2.5, womit der typische
MLLP-Debugging-Workflow vollständig abgedeckt ist:</p>
<ul>
	<li><strong>ADT^A01</strong> — Admit / Visit Notification</li>
	<li><strong>ADT^A40</strong> — Merge Patient (Patient Identifier
		List)</li>
	<li><strong>ORM^O01</strong> — Order Message</li>
	<li><strong>ORU^R01</strong> — Unsolicited Observation Result</li>
</ul>
<p>Jeder andere Nachrichtentyp und jede andere HL7-Version ist im
Dropdown mit <strong>(PRO)</strong> markiert und erfordert eine
Professional-Lizenz (oder eine aktive Testversion). Beim Versuch,
einen gesperrten Eintrag zu exportieren, zeigt BridgeLab eine
Upgrade-Aufforderung mit Verweis auf
<strong>Hilfe → Lizenz kaufen</strong> (die Preisseite) oder
<strong>Hilfe → Lizenz aktivieren</strong>.</p>

<h3>Lizenzhinweis</h3>
<p>BridgeLab verteilt keine urheberrechtlich geschützten XSD-Dateien
von HL7 weiter. Die Schema-Metadaten werden aus öffentlichen
HL7-v2-Spezifikationen neu aufgebaut; jede erzeugte Datei trägt einen
Header, der HL7® als Quellstandard nennt und die Ausgabe als
abgeleitetes Werk zu Interoperabilitätszwecken kennzeichnet.</p>

<div class="info">Ideales Einsatzziel: Astraia und ähnliche
Integrationsanwendungen, die handgeschriebene XSD-Definitionen für
Nachrichtentypen akzeptieren, die die Engine nicht nativ kennt. Einmal
exportieren, in die Engine einspielen, weiterarbeiten.</div>
`,
};

export const fhirSection: ManualSection = {
	id: 'fhir',
	heading: 'FHIR-Unterstützung',
	body: `
<p>BridgeLab erkennt FHIR-Ressourcen automatisch, wenn Sie eine Datei
einfügen oder öffnen, deren erstes Nicht-Leerraumzeichen
<code>{</code> ist und die <code>"resourceType"</code> enthält. Der
Baum wechselt in eine FHIR-spezifische Ansicht, die die
Ressourcenhierarchie als JSON-Pfade zeigt.</p>

<h3>Unterstützte Formate</h3>
<ul>
	<li><strong>JSON</strong> - Patient, Observation, Bundle,
		DiagnosticReport, MedicationRequest und jede andere
		FHIR-R4/R5-Ressource.</li>
	<li><strong>XML</strong> - dieselben Ressourcen in XML-Kodierung
		(<code>&lt;Patient xmlns="http://hl7.org/fhir"&gt;</code>).</li>
</ul>

<h3>Bundle-Visualisierer (Pro)</h3>
<p><strong>Werkzeuge → FHIR Bundle-Visualisierer</strong> öffnet eine
dreigeteilte Ansicht, wenn die aktive Nachricht ein Bundle ist:</p>
<ul>
	<li><strong>Linkes Panel:</strong> Liste der Einträge mit
		Ressourcentyp, Anzeigename (z. B. Patientenname,
		Observation-Code) und der Anzahl eingehender Referenzen.</li>
	<li><strong>Mittleres Panel:</strong> ausgehende Referenzen des
		ausgewählten Eintrags - jedes <code>reference</code>-Feld wird
		zu einem klickbaren Link, der zum Zieleintrag navigiert.</li>
	<li><strong>Rechtes Panel:</strong> das rohe JSON der ausgewählten
		Ressource, mit Syntaxhervorhebung.</li>
</ul>
<p><strong>Hängende Referenzen</strong> (die auf nicht im Bundle
vorhandene Einträge zeigen) werden mit einem roten Badge markiert.</p>
<p>Der Umschalter <strong>Liste / Graph</strong> wechselt zu einem
Referenzgraphen: Jeder Eintrag ist ein Knoten (nach Ressourcentyp
eingefärbt), jede <code>reference</code> ein gerichteter Pfeil. Ein
Klick auf einen Knoten wählt ihn aus — das Detailpanel folgt.
Verfügbar bis 150 Einträge; größere Bundles verwenden die Liste.</p>

<h3>FHIRPath-Evaluator (Pro)</h3>
<p><kbd>Ctrl</kbd>+<kbd>P</kbd> oder
<strong>Werkzeuge → FHIRPath-Evaluator</strong> öffnet eine
interaktive Konsole, in der Sie FHIRPath-Ausdrücke gegen die aktuelle
Ressource eingeben.</p>
<p>Der Evaluator implementiert die Sprache FHIRPath 2.0: den vollen
Operatorsatz mit der Präzedenz und der dreiwertigen Logik der
Spezifikation sowie rund siebzig Funktionen.</p>
<ul>
	<li><strong>Navigation:</strong> <code>Patient.name.family</code>,
		<code>Bundle.entry.resource</code>; Choice-Elemente werden über
		ihren Basisnamen erreicht — <code>Observation.value</code> findet
		<code>valueQuantity</code></li>
	<li><strong>Indizierung:</strong> <code>Patient.name[0].given</code></li>
	<li><strong>Filter und Projektion:</strong> <code>where()</code>,
		<code>select()</code>, <code>repeat()</code>,
		<code>ofType()</code>, mit <code>$this</code> und
		<code>$index</code> darin verfügbar</li>
	<li><strong>Sammlungen:</strong> <code>count()</code>,
		<code>first()</code>, <code>last()</code>, <code>tail()</code>,
		<code>skip()</code>, <code>take()</code>, <code>distinct()</code>,
		<code>sort()</code>, <code>union()</code>, <code>combine()</code>,
		<code>intersect()</code>, <code>exclude()</code>,
		<code>aggregate()</code></li>
	<li><strong>Logik:</strong> <code>and</code>, <code>or</code>,
		<code>xor</code>, <code>implies</code>, <code>not()</code>,
		<code>exists()</code>, <code>all()</code>, <code>iif()</code> —
		die leere Sammlung steht dabei für „unbekannt“</li>
	<li><strong>Zeichenketten:</strong> <code>substring()</code>,
		<code>matches()</code>, <code>replace()</code>,
		<code>split()</code>, <code>join()</code>, <code>encode()</code>,
		<code>escape()</code> und Verwandte</li>
	<li><strong>Daten und Mengen:</strong> Literale mit Teilpräzision
		(<code>@2015</code>, <code>@2015-02-04T14:34:28+10:00</code>),
		Dauer-Arithmetik (<code>Patient.birthDate + 18 years</code>) und
		Einheitenumrechnung innerhalb einer Dimension
		(<code>4 'g' = 4000 'mg'</code>)</li>
	<li><strong>FHIR-Ergänzungen:</strong> <code>extension(url)</code>,
		<code>hasValue()</code> und <code>resolve()</code>, das einer
		Reference zu einer contained- oder Bundle-Ressource folgt</li>
	<li><strong>Fehlersuche:</strong> <code>trace('Label')</code> reicht
		die Eingabe unverändert weiter und zeigt die Werte unter dem
		Ergebnis — so sieht man, was ein langer Pfad auf halbem Weg
		liefert</li>
</ul>
<p>Der Vergleich unterschiedlich präziser Werte liefert die leere
Sammlung statt einer Vermutung: <code>@2015-02-04 = @2015-02</code> ist
weder wahr noch falsch, denn der zweite Wert könnte dieser Tag oder ein
anderer desselben Monats sein.</p>
<p>Zuletzt verwendete Ausdrücke stehen in einem Verlaufs-Dropdown zur
schnellen Wiederverwendung bereit.</p>

<h3>FHIR-Validierung</h3>
<p>F6 funktioniert auch für FHIR-Ressourcen. Fehler markieren fehlende
Pflichtfelder (z. B. <code>Patient.identifier</code>), ungültige
Datentypen (gender außerhalb des Value Sets) und strukturelle
Probleme. Deklarierte kanonische <code>meta.profile</code>-URLs werden
als Info-Befunde gelistet (die Profilkonformität selbst wird nicht
geprüft); fehlerhaft aufgebaute Einträge werden als Warnungen
markiert.</p>

<h3>Profilvalidierung (Pro)</h3>
<p>Standardmäßig wird eine FHIR-Ressource strukturell geprüft: Gibt es einen
<code>resourceType</code>, gelten die ressourcenspezifischen Regeln. Der
Abgleich mit einer <strong>StructureDefinition</strong> — der eigentlichen
Definition dessen, was ein Patient enthalten darf — braucht diese
Definitionen, und die werden als FHIR-NPM-Pakete verteilt.</p>
<p>Der FHIR-R4-Kern (<code>hl7.fhir.r4.core</code> 4.0.1) ist
<strong>integriert</strong>: die Basisdefinitionen sind immer da — offline,
in jeder Edition, nichts herunterzuladen. <strong>Werkzeuge →
FHIR-Profilpakete…</strong> installiert darüber aus einer
<code>.tgz</code>-Datei nationale oder hauseigene
Implementierungsleitfäden; ein installiertes Paket mit denselben
Definitionen ersetzt die integrierten, eine neuere Version überstimmt sie.</p>
<p>Jede FHIR-Validierung prüft, gegen den integrierten Kern und alles, was
darüber installiert ist:</p>
<ul>
	<li><strong>Kardinalität</strong> — ein fehlendes Pflichtelement oder ein
		<code>0..1</code>-Element, das sich wiederholt.</li>
	<li><strong>Elementtypen</strong> — ein Boolean als Zeichenkette, eine
		Zahl, wo ein Objekt hingehört.</li>
	<li><strong>Choice-Elemente</strong> — <code>value[x]</code> muss als
		genau eine Form erscheinen: <code>valueQuantity</code>,
		<code>valueString</code> und so weiter. Ein nacktes
		<code>value</code> oder zwei Formen gleichzeitig werden gemeldet.</li>
	<li><strong>Feste Werte und Muster</strong> — was das Profil
		festlegt.</li>
	<li><strong>Unbekannte Elemente</strong> — ein Name, den das Profil nicht
		definiert. Diese Prüfung fängt Tippfehler wie <code>genderr</code>
		oder Elemente eines anderen Ressourcentyps ab.</li>
</ul>
<p>In <code>meta.profile</code> deklarierte Profile werden automatisch
angewandt, sofern das definierende Paket installiert ist. Ist es das nicht,
sagt der Validator das, statt stillschweigend ein sauberes Ergebnis zu
melden — und wenn Profile gelaufen sind, sagt er auch das, denn „keine
Befunde“ bedeutet in beiden Fällen sehr Unterschiedliches.</p>
<p><strong>Terminologie bleibt außen vor.</strong> Eine
<code>required</code>-Bindung lässt sich nur durch Expansion des ValueSets
prüfen, was die Terminologiepakete oder einen Server voraussetzt. BridgeLab
lässt Bindungen ungeprüft, statt sie halb zu prüfen.</p>
<p>Das Installieren eines Pakets erfordert eine Professional-Lizenz. Bereits
installierte Pakete validieren in jeder Stufe weiter — eine abgelaufene
Testphase färbt zuvor saubere Ressourcen nie rot.</p>

<h3>Eigene FHIR-Regeln (Pro)</h3>
<p><strong>Werkzeuge → FHIR-Validierungsregeln…</strong> öffnet einen Editor
für eigene Prüfungen. Sie laufen bei jeder Validierung zusammen mit den
eingebauten und werden als gewöhnliches Plugin-Pack gespeichert
(<code>plugins/fhir/user-rules.json</code>), das Sie zwischen Rechnern
kopieren oder versionieren können.</p>
<p>Eine Regel hat eine von zwei Formen:</p>
<ul>
	<li><strong>Ausdruck muss wahr sein</strong> — eine FHIRPath-Invariante,
		so wie FHIR seine eigenen Constraints schreibt:
		<code>identifier.exists()</code> oder
		<code>value.exists() xor dataAbsentReason.exists()</code>.</li>
	<li><strong>Pfad + Prüfung</strong> — ein FHIRPath-Selektor und eine
		Aussage über die gelieferten Werte: muss vorhanden sein, wie viele,
		entspricht einem Muster, eines aus einer Liste, enthält einen Text
		oder eine Längengrenze.</li>
</ul>
<p>Setzen Sie <strong>Ressourcentyp</strong>, um die Regel auf Patient,
Observation usw. zu begrenzen, oder lassen Sie das Feld leer, damit sie für
alles gilt. Eine auf einen Typ begrenzte Regel greift auch für die
passenden Ressourcen in einem Bundle; der Befund wird dann unter
<code>entry[n].resource.…</code> gemeldet.</p>
<p><strong>An offener Ressource testen</strong> führt die bearbeitete Regel
vor dem Speichern auf der Ressource des aktiven Tabs aus und zeigt, welche
Werte der Selektor tatsächlich erfasst hat — der schnellste Weg, eine Regel
die zutrifft von einer zu unterscheiden, die nie lief. Hat die offene
Ressource einen anderen Typ, sagt der Editor das, statt einen Erfolg zu
melden.</p>
<p>Bereits geschriebene Regeln funktionieren in jeder Stufe; der Editor
erfordert eine Professional-Lizenz. Von Hand geschriebene Packs sind in
<code>docs/PLUGINS.md</code> dokumentiert.</p>

<h3>FHIR-Vorlagen</h3>
<p><strong>Datei → Neue Nachricht aus Vorlage</strong> enthält eine
FHIR-Kategorie: einen minimalen Patient, eine Blutdruck-Observation
mit Komponenten und ein Transaktions-Bundle, dessen Einträge einander
über <code>urn:uuid</code> referenzieren — öffnen Sie es und probieren
Sie die Graphansicht des Bundle-Visualisierers aus.</p>
`,
};

export const pluginsSection: ManualSection = {
	id: 'plugins',
	heading: 'Plugin-Packs',
	body: `
<p>Mit Plugin-Packs erweitern Sie Validator und Anonymisierer von
BridgeLab, <strong>ohne Code zu schreiben</strong> und ohne jegliche
Codeausführung zuzulassen. Jedes Pack ist eine JSON-Datei in einem
Benutzerordner.</p>

<h3>Wo Plugins liegen</h3>
<p>Klicken Sie auf <strong>Einstellungen → Plugins → Plugin-Ordner
öffnen</strong>, um das Verzeichnis im Dateimanager anzuzeigen. Der
Aufbau:</p>
<pre><code>&lt;config&gt;/BridgeLab/plugins/
├── validation/
│   ├── hospital-adt-rules.json
│   └── z-segment-checks.json
├── fhir/
│   └── user-rules.json
└── anonymization/
    └── eu-national-id.json</code></pre>

<p>Unter Windows liegt die Wurzel in
<code>%APPDATA%\\BridgeLab\\plugins</code>, unter macOS in
<code>~/Library/Application Support/BridgeLab/plugins</code>, unter
Linux in <code>~/.config/BridgeLab/plugins</code>.</p>

<p>Dort liegen drei Arten von Packs: HL7-v2-Validierungsregeln
(<code>validation/</code>, unten), FHIR-Validierungsregeln
(<code>fhir/</code> — eine FHIRPath-Invariante oder ein Selektor plus
Prüfung; siehe <em>FHIR-Unterstützung → Eigene FHIR-Regeln</em>)
und zusätzliche PHI-Felder für den Anonymisierer
(<code>anonymization/</code>, unten).</p>

<p><strong>Jede Edition hat den gesamten Mechanismus</strong> — alle drei
Arten, jeden Prüftyp, Neuladen und das Schalten einzelner Packs. Der
einzige Unterschied ist, wie viele Packs gleichzeitig aktiv sein können:
bis zu <strong>3</strong> in Community, unbegrenzt in Pro und Enterprise.
Der integrierte Editor, der FHIR-Packs schreibt, ist Pro; ein von Hand
geschriebenes FHIR-Pack läuft in Community wie jedes andere.</p>

<h3>Validierungsregel-Pack</h3>
<pre><code>{
  "id": "acme-adt-01",
  "name": "ACME ADT specific rules",
  "description": "Hospital-specific required fields",
  "version": "1.0",
  "enabled": true,
  "validation_rules": [
    {
      "rule_id": "ACME-PID-001",
      "severity": "error",
      "segment": "PID",
      "field": 3,
      "check": { "type": "not_empty" },
      "message": "PID-3 (Patient ID) is required"
    }
  ]
}</code></pre>

<h3>Unterstützte Prüftypen</h3>
<table>
	<tr><th>Check</th><th>Parameter</th><th>Anwendungsbeispiel</th></tr>
	<tr><td><code>not_empty</code></td><td>—</td>
		<td>Feld muss befüllt sein.</td></tr>
	<tr><td><code>regex</code></td><td><code>pattern</code></td>
		<td>Nachname muss mit einem Großbuchstaben beginnen.</td></tr>
	<tr><td><code>one_of</code></td><td><code>values[]</code></td>
		<td>Patientenklasse muss I, O oder E sein.</td></tr>
	<tr><td><code>max_length</code></td><td><code>max</code></td>
		<td>MRN ≤ 16 Zeichen.</td></tr>
	<tr><td><code>min_length</code></td><td><code>min</code></td>
		<td>SSN ≥ 9 Ziffern.</td></tr>
	<tr><td><code>contains</code></td><td><code>value</code></td>
		<td>Visit Number muss einen Bindestrich enthalten.</td></tr>
</table>
<p>Ergänzen Sie <code>"component": 1</code>, um eine Regel auf eine
bestimmte Komponente einzugrenzen (z. B. den Nachnamen in PID-5.1).</p>

<h3>Anonymisierungsregel-Pack</h3>
<pre><code>{
  "id": "eu-extra-phi",
  "name": "EU extra PHI fields",
  "enabled": true,
  "phi_rules": [
    { "segment": "PID", "field": 25, "sensitivity": "high",
      "name": "EU National ID" }
  ]
}</code></pre>

<h3>Packs verwalten</h3>
<p><strong>Einstellungen → Plugins</strong> listet jedes Pack mit
Autor, Version, Regelanzahl und Pfad auf. Schalten Sie einzelne Packs
ein oder aus (die Wahl wird gespeichert), klicken Sie nach dem
Bearbeiten einer Datei auf <em>Neu laden</em>, oder öffnen Sie über
<em>Plugin-Ordner öffnen</em> den Ordner zum Bearbeiten in Ihrer
bevorzugten IDE.</p>

<div class="note">Dateien, die sich nicht parsen lassen, erscheinen
mit einem roten Fehlerbanner, brechen aber die Registry nicht - Ihre
übrigen Packs funktionieren weiter.</div>

<p class="note">In der Community-Stufe sind bis zu <strong>3
Packs</strong> gleichzeitig aktiv: Darüber hinaus aktivierte Packs
tragen ein „Inaktiv“-Badge und steuern keine Regeln bei, bis ein Platz
frei wird (ein anderes Pack deaktivieren oder upgraden).</p>
`,
};

export const licensingSection: ManualSection = {
	id: 'licensing',
	heading: 'Lizenzierung',
	body: `
<p>BridgeLab gibt es in drei Stufen. Die Aufteilung der Funktionen ist
so gestaltet, dass Community-Anwender dauerhaft echte tägliche
HL7-Arbeit erledigen können, während Pro und Enterprise Funktionen
freischalten, die Integrationsteams und Krankenhäuser benötigen.</p>

<table>
	<tr><th>Funktion</th><th>Community</th><th>Pro</th><th>Enterprise</th></tr>
	<tr><td>HL7-v2.x-Editor, Parser, Validierung</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>FHIR-Parsing + Baumansicht</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>MLLP-Versand, HTTP GET ohne Auth-Header</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Konformität mit dem FHIR-R4-Kern (integriert)</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>XSD-Export</td>
		<td>4 v2.5-Nachrichten</td><td>Vollständiger Katalog</td><td>Vollständiger Katalog</td></tr>
	<tr><td>PHI-Erkennung (nur Anzeige)</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Plugin-Packs (alle Arten, jeder Prüftyp)</td>
		<td>3 gleichzeitig aktiv</td><td>Unbegrenzt</td><td>Unbegrenzt</td></tr>
	<tr><td>MLLP-Listener</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>HTTP POST/PUT/DELETE/PATCH, und Authentifizierung bei jeder Methode</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Anonymisierungsmaskierung</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>JSON-/CSV-Export</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>FHIR-Profilpakete und Regeleditor</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>FHIRPath-Evaluator + Bundle-Visualisierer</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Gespeicherte Testfälle</td>
		<td>10</td><td>Unbegrenzt</td><td>Unbegrenzt</td></tr>
	<tr><td>SOAP + Prioritätsunterstützung</td>
		<td>—</td><td>—</td><td>✓</td></tr>
</table>

<p class="note">Community erlaubt bis zu <strong>3 aktive
Plugin-Packs</strong> und <strong>10 gespeicherte Testfälle</strong>.
Nichts wird jemals gesperrt oder gelöscht: Über dem Limit gespeicherte
Elemente (z. B. aus der Testphase) bleiben sichtbar, bearbeitbar und
ausführbar — nur neue Speicherungen und Aktivierungen über dem Limit
fragen nach einem Upgrade, und das Freiwerden eines Platzes
reaktiviert sie sofort.</p>

<h3>Testversion</h3>
<p>Beim ersten Start beginnt eine <strong>14-tägige
Pro-Testversion</strong> mit allen aktivierten Pro-Funktionen. Das
(gelbe) Testversions-Banner lässt sich schließen; bei 3 verbleibenden
Tagen wird es rot und bleibt als Erinnerung sichtbar.</p>

<p>Läuft die Testversion ab, <strong>stellt BridgeLab die Arbeit nicht
ein</strong> - es fällt auf die Community-Stufe zurück, und das Banner
fordert Sie zum Upgrade auf. Ihre Nachrichten, Einstellungen, Plugins
und Testfälle bleiben unangetastet.</p>

<h3>Eine Lizenz kaufen</h3>
<p><strong>Hilfe → Lizenz kaufen…</strong> öffnet im Browser den
Preisbereich der BridgeLab-Website, wo Professional und Enterprise
online per Karte gekauft werden; der Aktivierungscode kommt per E-Mail.
Dieselbe Seite ist einen Klick entfernt über die Schaltflächen
<em>Preise &amp; Kauf</em> im Aktivierungsdialog, <em>Tarife
vergleichen</em> im Testversions-Banner und <em>Preise ansehen</em> in
jedem Hinweis „erfordert eine Professional-Lizenz“. Rechnung,
Bestellung oder Angebot nötig? Schreiben Sie an
<a href="mailto:info@techemv.it">info@techemv.it</a>.</p>

<h3>Aktivierung</h3>
<p>Öffnen Sie den Aktivierungsdialog über:</p>
<ul>
	<li><strong>Einstellungen → Lizenzaktivierung → Lizenz
		aktivieren</strong></li>
	<li><strong>Hilfe → Lizenz aktivieren</strong></li>
	<li>Die Schaltfläche <em>Aktivieren</em> im Testversions-Banner</li>
</ul>

<p><strong>Online-Aktivierung (Standard):</strong> Nach dem Kauf
erhalten Sie per E-Mail einen Aktivierungscode der Form
<code>BL-PRO-XXXX-XXXX-XXXX</code>. Fügen Sie ihn in das Schlüsselfeld
ein: Die App tauscht ihn mit einem einzigen HTTPS-Aufruf gegen eine
signierte, an diesen Rechner gebundene Lizenz. Verwenden Sie
<em>Deaktivieren</em>, um den Arbeitsplatz freizugeben, bevor Sie auf
einen anderen Computer wechseln.</p>

<p>Abonnement verl&auml;ngert? Klicken Sie im Dialog auf <em>Lizenz
aktualisieren</em>, um das neue Ablaufdatum sofort zu &uuml;bernehmen —
oder tun Sie nichts: Innerhalb von 14 Tagen vor Ablauf holt die App es
beim Start selbst&auml;ndig und lautlos nach (niemals Fehlermeldungen
auf Offline-Rechnern).</p>

<p><strong>Offline-Schlüssel (isolierte / Air-Gapped-Umgebungen):</strong>
Senden Sie eine E-Mail an
<a href="mailto:info@techemv.it">info@techemv.it</a> mit Ihrer
<strong>Hardware-ID</strong> (angezeigt unter „Offline-Schlüssel
benötigt?" im Aktivierungsdialog, ebenso sichtbar unter
Einstellungen → Lizenzaktivierung). TECHEMV SRL sendet eine signierte,
an Ihren Rechner gebundene Lizenz zurück — ein Internetzugang ist zu
keinem Zeitpunkt erforderlich. Der Dialog zeigt vor der Aktivierung
den Namen des Lizenznehmers und die enthaltenen Berechtigungen an.</p>

<h3>Offline-Verifizierung</h3>
<p>Unabhängig vom gewählten Weg erfolgt die routinemäßige Lizenzprüfung
rein lokal - die App muss den Lizenzserver nie kontaktieren, um weiter
zu funktionieren. Serveraufrufe finden nur statt, wenn Sie sie
ausdrücklich auslösen: Aktivierung per Code, Freigabe des
Arbeitsplatzes über <em>Deaktivieren</em> oder Opt-in-
Nutzungsstatistiken. Der Schlüssel trägt eine Ed25519-Signatur, die
die App gegen einen eingebetteten öffentlichen Schlüssel
verifiziert.</p>

<h3>Datenschutz und Nutzungsstatistiken</h3>
<p>BridgeLab kann <strong>Nutzungsstatistiken</strong> an TECHEMV
senden — standardmäßig deaktiviert, aktivierbar unter
<strong>Einstellungen → Datenschutz</strong>. Wenn aktiviert, erfolgt
der automatische Versand höchstens einmal täglich; die Schaltfläche
<em>Jetzt senden</em> überträgt sofort. Jeder Bericht enthält
Nutzungszähler, App-Version, Betriebssystem, Lizenzstufe, eine
<strong>zufällige Installations-ID</strong> und — nur bei online
aktivierten Lizenzen — den <strong>Aktivierungscode</strong> (zur
Kennzeichnung widerrufener Lizenzen). Die Daten sind daher
<strong>pseudonym</strong>, nicht vollständig anonym:
Nachrichteninhalte, Dateinamen, Hostnamen, Benutzernamen oder
Patientendaten werden niemals gesendet, und das exakte JSON lässt sich
über <em>Zeigen, was gesendet wird</em> einsehen. Bei ausgeschaltetem
Schalter (Standard) wird nichts übertragen, und Netzwerkprobleme
erzeugen niemals Fehlermeldungen: Eine vollständig offline betriebene
Installation ist ein normales, unterstütztes Szenario.</p>
`,
};

export const shortcutsSection: ManualSection = {
	id: 'shortcuts',
	heading: 'Tastenkombinationen',
	body: `
<p>Die Tastenkombinationen von BridgeLab sind unter
<strong>Einstellungen → Tastenkombinationen</strong> frei
konfigurierbar. Klicken Sie auf eine Belegung, drücken Sie eine neue
Tastenkombination und bestätigen Sie mit OK.</p>

<h3>Standardbelegung</h3>
<table>
	<tr><td><kbd>Ctrl</kbd>+<kbd>O</kbd></td><td>Datei öffnen</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>N</kbd></td><td>Neu aus Vorlage</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>L</kbd></td><td>Testfall-Bibliothek</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>S</kbd></td><td>Speichern</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>S</kbd></td><td>Speichern unter</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>W</kbd></td><td>Tab schließen</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>B</kbd></td><td>Baum ein-/ausblenden</td></tr>
	<tr><td><kbd>F5</kbd></td><td>Nachricht erneut analysieren</td></tr>
	<tr><td><kbd>F6</kbd></td><td>Validieren</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>K</kbd></td><td>Kommunikationspanel</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>P</kbd></td><td>FHIRPath-Panel</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>,</kbd></td><td>Einstellungen</td></tr>
	<tr><td><kbd>F1</kbd></td><td>Dieses Benutzerhandbuch</td></tr>
	<tr><td><kbd>Alt</kbd>+<kbd>T</kbd></td><td>Segment im Baum anzeigen (Kontextmenü des Editors)</td></tr>
	<tr><td><kbd>Alt</kbd>+<kbd>C</kbd></td><td>Segment kopieren (Kontextmenü des Editors)</td></tr>
</table>

<h3>Konflikterkennung</h3>
<p>Wählen Sie eine Tastenkombination, die bereits einer anderen Aktion
zugewiesen ist, warnt Sie der Editor - bestätigen Sie, um die Belegung
zu übertragen, oder wählen Sie eine andere Taste. Monacos eigene
Tastenkombinationen (<kbd>Ctrl</kbd>+<kbd>F</kbd>,
<kbd>Ctrl</kbd>+<kbd>D</kbd>, ...) haben Vorrang, wenn der Editor den
Fokus hat.</p>

<h3>Zurücksetzen</h3>
<p>Klicken Sie auf <em>Alle zurücksetzen</em>, um jede
Tastenkombination auf ihren Standard zurückzusetzen, oder auf die
kleine ↺-Schaltfläche neben einem Eintrag, um nur diesen
zurückzusetzen.</p>
`,
};
