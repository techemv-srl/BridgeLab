import type { ManualSection } from '../helpContent';
import { mockupValidationDe as mockupValidation, mockupCommunicationDe as mockupCommunication } from './mockups';

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
		ein zweites MSH (zwei Nachrichten in einem Text) wird gemeldet
		(STRUCT-004). Ein Code mit mehr als drei Zeichen (<code>PIDX</code>)
		wird gemeldet, nie als <code>PID</code> gelesen; eine Batchdatei darf
		vor dem MSH mit FHS/BHS beginnen.</li>
	<li><strong>MSH-Header:</strong> MSH-9 (Nachrichtentyp), MSH-10
		(Control-ID), MSH-12 (Version) sind Pflichtfelder. Eine MSH-12-Version,
		die BridgeLab nicht kennt (eine künftige v2.8, ein Tippfehler), wird
		gegen den nächstliegenden Katalog geprüft, ersatzweise v2.5, und ein
		Info-Hinweis (MSH-005) nennt ihn.</li>
	<li><strong>Pflichtfelder:</strong> Pflichtfelder je Segment gemäß
		HL7-Standard (z. B. PID-3 Patient Identifier List).</li>
	<li><strong>Längenlimits:</strong> warnt, wenn ein Feld die
		veröffentlichte <code>max_length</code> überschreitet, gezählt in
		Zeichen, für jede Wiederholung eines wiederholten Feldes.</li>
	<li><strong>Datentypen</strong> (Warnungen): Zahlen (SI, NM), Daten
		(DT, <code>YYYY[MM[DD]]</code>), Zeitstempel (TS/DTM,
		<code>YYYY[MM[DD[HH[MM[SS[.S]]]]]][+/-ZZZZ]</code>) und Uhrzeiten
		(TM) werden auf Format sowie gegen Kalender und Uhr geprüft, also
		wird <code>19801399</code> oder <code>2024-01-01</code> gemeldet.
		OBX-5 wird nach dem Typ aus OBX-2 geprüft (NM, DT, TS…). Andere
		Typen haben keine Formatprüfung.</li>
</ul>

<h3>Filtern und Navigation</h3>
<p>Klicken Sie auf die Badges Fehler / Warnung / Info, um zu filtern.
Ein Klick auf eine Problemzeile markiert das betroffene Segment oder Feld
im Editor und im Baum. Jede Zeile nennt das Segment mit seiner Nummer wie
der Baum (<code>OBX (4)</code>), sodass sich zwei gleiche Probleme in zwei
Segmenten unterscheiden lassen. Wenn Sie die Nachricht nach einer Prüfung
bearbeiten, wird der Bericht mit der nächsten Hintergrundanalyse erneuert;
bei abgeschalteter automatischer Analyse zeigt das Panel an, dass der
Bericht veraltet ist, bis Sie <kbd>F6</kbd> drücken.</p>

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
Regressions-Fixtures für die Stapelvalidierung oben. Eine gleichnamige
Datei, die schon im Ordner liegt, bleibt unverändert und wird als nicht
gespeichert aufgeführt.</p>

<h3>Validierung über die CLI</h3>
<p>Das Begleitwerkzeug <code>bridgelab-cli</code> führt dieselben
Validatoren aus — HL7 v2 und FHIR, mit dem integrierten R4-Kern, den
installierten Paketen und den Plugins — headless, für CI-Pipelines und
Batch-Screening. Es liest keine Lizenz und braucht keine. Neben
<code>validate</code>, <code>info</code>, <code>anonymize</code>,
<code>to-json</code> und <code>batch</code> führt es die aus der
Bibliothek exportierten Testfallpakete aus (<code>test</code>, mit
JUnit-Bericht für die CI), wertet FHIRPath aus (<code>fhirpath</code>),
sendet eine Nachricht per MLLP und endet mit Fehler, wenn das ACK nicht
AA oder CA ist (<code>send</code>; eine Datei mit mehreren Nachrichten wird
als ein Frame pro Nachricht gesendet), und exportiert das XSD der Nachrichten der
Community-Edition (<code>xsd</code>; der vollständige Katalog ist in
Pro). Jeder Befehl, der eine Nachricht liest, akzeptiert <code>-</code>
für die Standardeingabe. Jedem Release liegt ein Binary pro Plattform
bei. Setzen Sie Muster und Argumente mit <code>^</code> in doppelte
Anführungszeichen (<code>"*.hl7"</code>, <code>"ORU^R01"</code>): Das
Windows-<code>cmd.exe</code> entfernt ein ungeschütztes <code>^</code> und
behält einfache Anführungszeichen bei.</p>
<p>Das <code>batch</code> der CLI durchsucht Unterordner, liest
<code>.hl7</code>-Dateien, sofern <code>--extension</code> nichts anderes
angibt, und prüft neben HL7 v2 auch FHIR; seine Zahlen können daher von
<strong>Werkzeuge → Stapelvalidierung…</strong> im selben Ordner abweichen
(ein Ordner, <code>.hl7</code>, <code>.txt</code> und <code>.dat</code>,
nur HL7 v2). Mit <code>--extension hl7,txt,dat</code> liest es dieselben
Dateien.</p>
<pre><code>bridgelab-cli validate message.hl7 bundle.json
bridgelab-cli validate "*.hl7" --format junit &gt; report.xml
bridgelab-cli batch ./inbox --json
bridgelab-cli test regression.bltests.json --format junit &gt; tests.xml
bridgelab-cli fhirpath "Patient.name.family" patient.json
cat message.hl7 | bridgelab-cli send - --host 10.0.0.5 --port 2575</code></pre>
`,
};

export const communicationSection: ManualSection = {
	id: 'communication',
	heading: 'Kommunikation (MLLP / HTTP / SOAP)',
	body: `
<p>Öffnen Sie das untere Kommunikationspanel mit
<kbd>Ctrl</kbd>+<kbd>K</kbd> oder über
<strong>Werkzeuge → Kommunikationspanel</strong>. Vier Tabs: MLLP,
HTTP, SOAP und Verlauf.</p>

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
<strong>ACK aus aktueller Nachricht erzeugen</strong>. Das ACK
spiegelt die Nachricht, wie es ein Empfänger tun soll: dieselben
Feldtrenner und Kodierungszeichen, Sender und Empfänger vertauscht
(MSH-3/4 und MSH-5/6), <code>ACK^&lt;Trigger&gt;^ACK</code> in MSH-9,
dieselbe Processing ID (MSH-11), Version (MSH-12) und derselbe
Zeichensatz (MSH-18) sowie die Message Control ID (MSH-10, gelesen mit
dem in MSH-1 deklarierten Trenner) in MSA-2. Jedes ACK erhält eine
eigene MSH-10. Das Auto-ACK des Listeners wird genauso gebaut. Das ACK
öffnet sich in einem neuen Tab — bereit zum Zurücksenden oder zum
Aufbewahren als Fixture. Hat die aktuelle Nachricht keine MSH-10,
verweigert der Generator, statt ein nicht zuordenbares ACK zu
erzeugen.</p>

<h3>MLLP-Listener (Pro)</h3>
<p>Klicken Sie auf <strong>Lauschen starten</strong>, um einen Server
auf dem gewählten Port zu betreiben. Eingehende Nachrichten öffnen sich
in einem neuen Tab (abschaltbar, siehe unten), und ein Auto-ACK mit dem
konfigurierten Code (AA/AE/AR) wird zurückgesendet. So prüfen Sie
schnell, was Ihr vorgelagertes System tatsächlich sendet.</p>
<p>Standardmäßig lauscht der Listener auf <code>127.0.0.1</code>, nur
Programme auf diesem Computer erreichen ihn. Um einen Datenstrom von
einem anderen Rechner zu empfangen, setzen Sie <strong>Lauschen auf</strong> auf
<code>0.0.0.0</code> (alle Schnittstellen) oder auf die Adresse einer
Netzwerkkarte und geben den Port in der Firewall nur für die erwarteten
Systeme frei.</p>
<p><strong>Stopp</strong> schließt auch die noch offenen
Verbindungen: danach wird nichts mehr empfangen oder bestätigt.</p>

<h3>Listener-Konsole</h3>
<p>Solange der Listener läuft, erscheint jede empfangene Nachricht als
Zeile in der Konsole: lokale Zeit, Peer-Adresse, Payload-Größe, der
tatsächlich zurückgeschriebene ACK-Code (grünes <code>AA</code>, rotes
<code>AE</code>/<code>AR</code>, — wenn Auto-ACK aus ist), die
verwendete Zeichenkodierung und die erste Zeile der Nachricht.
<strong>Klicken Sie auf eine Zeile, um diese Nachricht erneut in einem
Tab zu öffnen.</strong> Listener-Fehler erscheinen inline als rote
Zeilen.</p>
<p>Die Chips in der Kopfzeile der Konsole filtern die Zeilen nach
Ergebnis — <em>AA</em>, <em>AE</em>, <em>AR</em>, <em>Ohne ACK</em>
(empfangen bei ausgeschaltetem Auto-ACK), <em>Fehler</em> — und jeder
trägt einen laufenden Zähler, sodass „AE&nbsp;12“ von 300 auffällt,
bevor jemand scrollt. Dieselben Chips gibt es im Tab Verlauf.</p>
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
<p>Sowohl der Sender als auch der Listener haben eine Auswahl
<strong>Kodierung</strong> mit den Zeichensätzen realer Installationen:
<code>UTF-8</code>, <code>ISO-8859-1</code> (Latin-1),
<code>ISO-8859-2</code>, <code>ISO-8859-15</code>,
<code>windows-1252</code>, <code>windows-1250</code>,
<code>windows-1251</code> und <code>ASCII</code>. Standard ist
<strong>Automatisch</strong>. Im Listener wird jede Nachricht im
Zeichensatz aus MSH-18 dekodiert, sonst als UTF-8 mit automatischem
Latin-1-Ersatz, was den meisten Altverkehr auch bei leerem MSH-18
annimmt; die Konsole zeigt den gewählten Zeichensatz, und das ACK wird
darin neu kodiert, damit die Gegenstelle nie verstümmelte Zeichen sieht.
Beim Senden gilt der Zeichensatz aus MSH-18, sonst der, mit dem die
Datei des Tabs gelesen wurde, sonst UTF-8, und das Ergebnis nennt den
gesendeten Zeichensatz. Eine Nachricht mit Zeichen, die der gewählte
Zeichensatz nicht darstellen kann, wird nicht gesendet: der Fehler nennt
sie, damit ein Name nie mit <code>?</code> ankommt. Sende- und
Empfangskodierung sind unabhängig.</p>
<p>Segmente gehen mit CR am Ende auf die Leitung, unabhängig von den
Zeilenenden im Editor: LF und CR LF werden umgewandelt, wie es die CLI
tut, ebenso bei einem HL7-v2-Body über HTTP oder SOAP.</p>
<p>Für <strong>Dateien</strong> gelten dieselben Regeln. Eine Datei, die
nicht UTF-8 ist, öffnet sich in der Kodierung aus MSH-18
(<code>8859/1</code>, <code>8859/2</code>, <code>8859/15</code>…, sowie
<code>BIG-5</code>, <code>GB 18030-2000</code> und <code>KS X 1001</code>)
oder als Windows-1252, wenn MSH-18 leer ist: Namen mit Umlauten werden
korrekt gelesen statt abgelehnt. Eine Datei, deren MSH-18 eine Kodierung
nennt, die BridgeLab nicht dekodieren kann (<code>CNS 11643-1992</code>,
die japanischen ISO-2022-Sätze), öffnet sich mit einer Warnung, da ihr
Text und sogar ihre Felder falsch gelesen werden können; die
Stapelvalidierung und die Batch-Anonymisierung lehnen sie ab. Speichern
schreibt die Datei in der Kodierung zurück, in der sie gelesen wurde,
ebenso <code>anonymize</code> der CLI und die Batch-Anonymisierung;
<code>send</code> ohne <code>--encoding</code> sendet in dieser Kodierung
(eine UTF-16-Datei geht in der Kodierung aus MSH-18 hinaus, sonst in
UTF-8). Beim Speichern wird ein Zeichen, das die Kodierung nicht
darstellen kann, als <code>?</code> geschrieben; <code>send</code> der
CLI lehnt eine solche Nachricht dagegen ab, wie die App.</p>

<h3>HTTP</h3>
<p>GET-Anfragen sind in der Community-Edition verfügbar. POST/PUT/DELETE/PATCH
und jede Form der Authentifizierung erfordern Pro: ein Authorization-
oder Proxy-Authorization-Header, ein Cookie, jeder Header, dessen Name
auf einen Schlüssel, ein Token, ein Geheimnis, eine Signatur oder eine
Sitzung hindeutet (etwa <code>X-API-Key</code> oder
<code>Ocp-Apim-Subscription-Key</code>), Benutzer:Passwort in der URL
oder ein Schlüssel oder Token im Query-String
(<code>access_token</code>, <code>api_key</code>, <code>key</code>,
<code>token</code>, <code>sig</code>, <code>client_secret</code>…).
FHIR-Suchparameter wie <code>code</code> sind keine Zugangsdaten.
Weiterleitungen werden in jeder Edition verfolgt, aber nur auf demselben
Server (oder von http zu https auf ihm): eine Weiterleitung zu einem anderen
Server wird als das 3xx angezeigt, das sie ist, und nie verfolgt, damit eine
Nachricht nicht an ein Ziel geht, das Sie nicht gewählt haben. Wurde eine
Weiterleitung verfolgt, nennt das Ergebnis die URL, die geantwortet hat.
Antworten über 50 MB werden gekürzt und melden das, und eine Antwort wird
im Zeichensatz gelesen, den ihr Content-Type angibt. POST, PUT und PATCH
senden die Nachricht des aktuellen Tabs, wenn das Feld Body leer ist; GET
und DELETE senden einen Body nur, wenn Sie einen eingeben.</p>

<h3>SOAP-Client (Enterprise)</h3>
<p>Der SOAP-Tab sendet die aktuelle Nachricht (oder einen eigenen Body)
an SOAP-1.1/1.2-Endpunkte — Middlewares im IHE-Stil, regionale Gateways
und ältere Krankenhaus-Webservices. Geben Sie Endpunkt-URL,
SOAP-Version und <em>SOAPAction</em> an; BridgeLab baut die Envelope,
sendet sie mit dem korrekten Content-Type (<code>text/xml</code> plus
SOAPAction-Header bei 1.1, <code>application/soap+xml</code> mit
action-Parameter bei 1.2) und zeigt HTTP-Status, Umlaufzeit, das innere
Body-XML sowie einen etwaigen SOAP Fault an — dekodiert für beide
Versionen.</p>
<p>Eine rohe HL7-v2-Nachricht wird automatisch XML-maskiert (ihre
CR am Segmentende werden zu <code>&amp;#13;</code>, damit das XML-Parsing
sie nicht in LF verwandelt) und in ein
<code>&lt;payload&gt;</code>-Element gepackt; Inhalt, der bereits XML
ist, wird unverändert eingefügt. Die erweiterten Einstellungen ergänzen
<strong>WS-Security-UsernameToken</strong>-Zugangsdaten,
<strong>WS-Addressing</strong>-Header (To / Action / MessageID) und eine
<strong>eigene Envelope-Vorlage</strong>, in der der wörtliche
Platzhalter <code>{payload}</code> durch die Nachricht ersetzt wird —
nützlich, wenn der Zieldienst einen bestimmten Wrapper erwartet. Mit
einer Vorlage kommen die WS-Security- und WS-Addressing-Header in den
SOAP-Header der Vorlage (hat sie keinen, wird einer vor ihrem Body
eingefügt); eine Vorlage ohne SOAP-Envelope und -Body kann sie nicht
tragen, und das Senden wird abgelehnt statt ohne sie zu erfolgen. Der
WSDL-Import ist als nächster Schritt geplant.</p>

<h3>Verlauf</h3>
<p>Jedes Senden und jede vom Listener empfangene Nachricht wird
protokolliert: Ziel (Host und Port, oder die URL mit Passwörtern oder
Schlüsseln durch <code>***</code> ersetzt), Größe, Antwortcode und
Round-Trip-Zeit. Die letzten 100 Einträge bleiben über Neustarts
erhalten, ältere werden gelöscht; ein Klick auf eine Zeile zeigt die
vollständige Anfrage und Antwort — die Nachricht und ihr ACK, oder den
HTTP-Body und die Antwort (bei SOAP die Nutzlast, nie den Envelope mit
seinem Passwort). Eine Anfrage oder Antwort über 256 KB wird gekürzt
aufbewahrt. Ein MLLP-Senden speichert außerdem den
<strong>ACK-Code</strong>, mit dem der Empfänger geantwortet hat
(MSA-1), als grünes oder rotes Badge in der Zeile: ein Senden, das die
Gegenstelle erreicht und ein <code>AE</code> zurückbekommt, ist auf
Transportebene "OK" und auf Anwendungsebene eine Ablehnung, und das
Badge unterscheidet beides. Filter-Chips über der Liste — <em>AA</em>,
<em>AE</em>, <em>AR</em>, <em>Ohne ACK</em>, <em>Fehlgeschlagen</em> —
tragen Zähler und schränken die Liste auf ein Ergebnis ein; die
Commit-Mode-Codes CA, CE und CR zählen unter AA, AE und AR.
<em>Fehlgeschlagen</em> ist eine Anfrage ohne Antwort: ein Server, der
mit einem Fehler geantwortet hat (HTTP 404, 500…), erscheint mit seinem
Statuscode.</p>

<h3>Verbindungsprofile</h3>
<p>Speichern Sie häufig genutzte Endpunkte als benannte Profile über
die Zeile <strong>Profil</strong>: Namen eingeben und auf
<em>Speichern</em> klicken. MLLP-Profile speichern Host, Port, Timeout
und Auto-ACK; HTTP-Profile speichern URL, Header und Timeout;
SOAP-Profile speichern Endpunkt, SOAPAction und Timeout. Die
Auswahl eines Profils übernimmt es ins Formular; Speichern unter einem
vorhandenen Namen überschreibt es; <em>Löschen</em> entfernt das
ausgewählte Profil. Profile liegen in der lokalen Datenbank und
überstehen Neustarts.</p>
<p class="note">HTTP-Profile speichern das Feld Header so, wie es
eingegeben ist, einschließlich eines <code>Authorization</code>-Headers,
in der lokalen Datenbank. Um ein Passwort oder Token dort nicht zu
speichern, geben Sie es unter <em>Authentifizierung</em> in den
erweiterten HTTP-Einstellungen ein; diese werden nie gespeichert.</p>
`,
};

export const anonymizationSection: ManualSection = {
	id: 'anonymization',
	heading: 'Anonymisierung &amp; Export',
	body: `
<p><strong>Werkzeuge → Anonymisieren</strong> erkennt PHI-Felder in
den patientenidentifizierenden Segmenten und maskiert sie nach
Sensibilitätsstufe: 89 eingebaute Felder in PID, PV1 (Fallnummern), MRG,
NK1, GT1, IN1 und IN2, dazu NTE-Kommentare und OBX-Ergebnisse mit
Freitext (TX/FT). Sie umfassen Namen, Geburts- und Sterbedaten,
Adressen, Telefonnummern, SSN und weitere Kennungen von Patient,
Angehörigen, Garant und Versichertem. Eine Kennung behält ihre
vergebende Stelle und ihren Typ (<code>HOSP</code>, <code>MR</code>).
Andere Segmente werden nicht maskiert: Prüfen Sie Freitext an anderer
Stelle (OBR, ORC, Z-Segmente) vor dem Weitergeben, oder ergänzen Sie
diese Felder per Plugin.</p>

<table>
	<tr><th>Stufe</th><th>Beispiel</th><th>Strategie</th></tr>
	<tr><td><strong>Hoch</strong></td><td>Patientenname, Geburtsdatum,
		Adresse, private Telefonnummer, SSN, MRN, Fallnummer</td>
		<td>Text wird zu <code>REDACTED</code>; Zahlen werden zu Nullen
		gleicher Länge (erhält die Feldbreite für nachgelagerte
		Parser); ein Datum wird zu 1900-01-01 in gleicher
		Genauigkeit.</td></tr>
	<tr><td><strong>Mittel</strong></td><td>Mädchenname der Mutter, Alias,
		geschäftliche Telefonnummer, Kontakte von Angehörigen, Garant
		und Versichertem</td>
		<td>Erstes Zeichen bleibt erhalten, der Rest wird durch
		<code>***</code> ersetzt.</td></tr>
	<tr><td><strong>Niedrig</strong></td><td>Kein eingebautes Feld;
		verfügbar für Plugin-Regeln</td>
		<td>Die ersten 3 Zeichen bleiben erhalten, gefolgt von
		<code>...</code>; ein Wert mit höchstens 3 Zeichen bleibt
		unverändert.</td></tr>
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
numerische Suffixe, statt einander zu überschreiben. Eine Datei, die
schon im Ausgabeordner liegt, wird nie ersetzt (und ein Link dort nie
verfolgt): Ihre Zeile meldet das, und die Datei bleibt unverändert —
wählen Sie also einen leeren Ordner. Eine Zeile pro
Datei meldet die Anzahl maskierter PHI-Felder oder den Fehler; es
gelten dieselben Limits wie bei der Stapelvalidierung (5000 Dateien /
10&nbsp;MB).</p>

<h3>Export</h3>
<p>Pro-Anwender können die strukturierte Nachricht über
<strong>Werkzeuge → Als JSON / CSV exportieren</strong> als JSON oder
CSV exportieren; ein Speichern-Dialog fragt, wohin die Datei
geschrieben wird. Nützlich, um HL7-Daten in Analysewerkzeuge zu laden
(Power BI, Excel, pandas).</p>

<div class="warn">Die Anonymisierung schreibt ihr Ergebnis in einen neuen
Tab; der ursprüngliche Tab bleibt unverändert. Bewahren Sie Ihre Originaldatei stets als maßgebliche Quelle auf - die
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
deklarieren (verglichen werden nur die angegebenen Komponenten: <code>ADT</code> passt
auf jedes ADT-Ereignis, <code>ADT^A01</code> auf <code>ADT^A01</code> und
<code>ADT^A01^ADT_A01</code>, aber nicht auf <code>ADT^A04</code>) sowie ein <strong>erwartetes
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

<h3>Testfälle teilen</h3>
<p><strong>Exportieren…</strong> schreibt die angezeigten Testfälle —
alle oder nur die zur Suche passenden — in ein Paket
<code>.bltests.json</code>, das Sie an Kollegen senden oder in einem
Git-Repository ablegen können. Vor dem Speichern prüft BridgeLab die
HL7-v2-Nachrichten auf personenbezogene Daten und listet die
betroffenen Testfälle und Felder auf; FHIR-Ressourcen werden als nicht
Feld für Feld geprüft angezeigt. Mit Pro können Sie <em>Personenbezogene
Daten maskieren</em> wählen, um die HL7-v2-Nachrichten nur in der
exportierten Datei zu anonymisieren – die Bibliothek bleibt
unverändert.</p>
<p><strong>Importieren…</strong> öffnet ein Paket und zeigt, bevor
etwas geschrieben wird, was jeder Testfall ist: <em>Neu</em>,
<em>Schon in der Bibliothek</em> (übersprungen) oder <em>Abweichend</em>
von einem vorhandenen – dann wählen Sie, ob Sie Ihren behalten, ihn
ersetzen oder beide behalten. Importierte Testfälle behalten ihre
Kennung; ein erneuter Import desselben Pakets bringt nur Geändertes. In
Community darf ein Import die Bibliothek nicht über 10 Testfälle
bringen; dann wird nichts geschrieben.</p>

<h3>Sitzungswiederherstellung</h3>
<p>BridgeLab speichert Ihre offenen Tabs (einschließlich
ungespeicherter Änderungen) und öffnet sie beim nächsten Start erneut
— wie in Notepad++. Steuern Sie dies unter
<strong>Einstellungen → Leistung</strong>, in der Gruppe <em>Sitzung</em>: Schalten Sie <em>Offene Tabs
beim Start wiederherstellen</em> um, oder löschen Sie mit
<em>Gespeicherte Sitzung löschen</em> die gespeicherten Tabs (das
deaktiviert auch die Wiederherstellung, sodass der nächste Start auf
dem Willkommensbildschirm beginnt). Das Abschalten der Wiederherstellung
löscht auch die gespeicherten Tabs, die Patientendaten enthalten können.</p>

<h3>Von anderen Programmen geänderte Dateien</h3>
<p>BridgeLab bemerkt, wenn ein anderes Programm eine geöffnete Datei
ändert oder löscht: Wenn Sie zum Fenster zurückkehren oder die Datei
erneut öffnen, bietet es an, die neue Version zu laden (und meldet,
wenn die Datei fehlt). <strong>Speichern</strong> fragt nach, bevor es
eine Datei überschreibt, die seit dem Öffnen oder letzten Speichern auf
dem Datenträger geändert wurde, oder eine gelöschte Datei neu anlegt.
Beim Start zeigt ein wiederhergestellter Tab ohne ungespeicherte
Änderungen seine Datei so, wie sie jetzt auf dem Datenträger ist. Ein wiederhergestellter Tab mit ungespeicherten Änderungen fragt beim ersten Speichern nach, da sich die Datei geändert haben kann, während BridgeLab geschlossen war.</p>
`,
};
