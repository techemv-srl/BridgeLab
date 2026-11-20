import type { ManualSection } from '../helpContent';
import { mockupValidationFr as mockupValidation, mockupCommunicationFr as mockupCommunication } from './mockups';

export const validationSection: ManualSection = {
	id: 'validation',
	heading: 'Validation',
	body: `
<p>Appuyez sur <kbd>F6</kbd> ou choisissez <strong>Outils → Valider</strong>
pour exécuter toutes les règles de validation sur le message actif. Les
résultats apparaissent dans le panneau Validation ancré en bas, groupés
par sévérité.</p>

${mockupValidation}

<h3>Règles intégrées</h3>
<ul>
	<li><strong>Structurelles :</strong> le premier segment doit être MSH ;
		les codes de segment doivent comporter 3 caractères
		alphanumériques ; un second MSH (deux messages dans un même texte)
		est signalé (STRUCT-004). Un code de plus de trois caractères
		(<code>PIDX</code>) est signalé, jamais lu comme <code>PID</code> ;
		un fichier de lot peut commencer par FHS/BHS avant le MSH.</li>
	<li><strong>En-tête MSH :</strong> MSH-9 (type de message), MSH-10 (ID
		de contrôle), MSH-12 (version) sont obligatoires. Une version MSH-12
		inconnue de BridgeLab (une future v2.8, une coquille) est validée
		avec le catalogue le plus proche, v2.5 à défaut, et une note
		d'information (MSH-005) l'indique.</li>
	<li><strong>Champs obligatoires :</strong> champs obligatoires par
		segment, tirés du standard HL7 (p. ex. PID-3 Patient Identifier
		List).</li>
	<li><strong>Limites de longueur :</strong> avertit lorsqu'un champ
		dépasse la <code>max_length</code> publiée, comptée en
		caractères, pour chaque répétition d'un champ répété.</li>
	<li><strong>Types de données</strong> (avertissements) : nombres (SI,
		NM), dates (DT, <code>YYYY[MM[DD]]</code>), horodatages (TS/DTM,
		<code>YYYY[MM[DD[HH[MM[SS[.S]]]]]][+/-ZZZZ]</code>) et heures (TM)
		sont vérifiés dans leur format et par rapport au calendrier et à
		l'horloge : <code>19801399</code> ou <code>2024-01-01</code> est
		signalé. OBX-5 est vérifié selon le type déclaré en OBX-2 (NM, DT,
		TS…). Les autres types n'ont pas de contrôle de format.</li>
</ul>

<h3>Filtrage et navigation</h3>
<p>Cliquez sur les badges Erreur / Avertissement / Info pour filtrer.
Cliquez sur la ligne d'un problème pour sélectionner le segment ou le champ
fautif dans l'éditeur et dans l'arbre. Chaque ligne désigne le segment par
son numéro, comme l'arbre (<code>OBX (4)</code>), pour distinguer deux
problèmes identiques sur deux segments. Si vous modifiez le message après
une vérification, le rapport est refait à l'analyse suivante en arrière-plan ;
sans analyse automatique, le panneau indique que le rapport n'est plus à
jour jusqu'à ce que vous appuyiez sur <kbd>F6</kbd>.</p>
<p>Un long rapport est listé par tranches de 200 problèmes : les suivants
apparaissent quand vous faites défiler la liste jusqu'au bout, ou avec
<em>Afficher plus</em>. Les badges, le filtre et le tri portent toujours
sur le rapport entier.</p>

<h3>Règles personnalisées via les packs de plugins</h3>
<p>Déposez un fichier JSON sous <code>&lt;config&gt;/BridgeLab/plugins/validation/</code>
pour ajouter vos propres contrôles sans recompiler. Voir <em>Plugins</em>
plus bas.</p>

<h3>Validation par lots (Pro)</h3>
<p><strong>Outils → Validation par lots…</strong> valide un dossier
entier (ou une sélection manuelle) de fichiers
<code>.hl7</code>/<code>.txt</code>/<code>.dat</code> en une seule
passe : une ligne par fichier avec type de message, version, nombre de
segments et totaux d'erreurs/avertissements. Filtrez sur les seuls
échecs, cliquez sur une ligne pour ouvrir ce fichier dans l'éditeur, et
exportez tout le tableau en CSV pour le ticket de revue de
modifications. Les fichiers sont traités en mémoire — rien n'est ajouté
à vos onglets.</p>

<h3>Générateur de messages de test</h3>
<p><strong>Outils → Générer des messages de test…</strong> crée des
messages ADT/ORU/ORM syntaxiquement valides avec des données patient
<em>synthétiques</em> plausibles — noms, dates de naissance, MRN,
adresses, et panels de laboratoire avec plages de référence (une part
réaliste des résultats est volontairement anormale et signalée). Aucune
donnée PHI réelle n'est jamais utilisée. Fournissez une
<strong>graine</strong> pour rendre un jeu reproductible, puis ouvrez
les messages dans des onglets ou enregistrez-les dans un dossier sous
forme de fichiers <code>.hl7</code> numérotés — des jeux de régression
instantanés pour le validateur par lots ci-dessus. Un fichier du même
nom déjà présent dans le dossier reste tel quel et est indiqué comme non
enregistré.</p>

<h3>Validation en CLI</h3>
<p>L'outil compagnon <code>bridgelab-cli</code> exécute les mêmes
validateurs — HL7 v2 et FHIR, avec le noyau R4 intégré, les packages
installés et les plugins — en mode headless, pour les pipelines CI et le
criblage par lots. Il ne lit aucune licence et n'en demande pas. En plus
de <code>validate</code>, <code>info</code>, <code>anonymize</code>,
<code>to-json</code> et <code>batch</code>, il exécute les paquets de
cas de test exportés de la bibliothèque (<code>test</code>, avec un
rapport JUnit pour la CI), évalue du FHIRPath (<code>fhirpath</code>),
envoie un message en MLLP et sort en erreur si l'ACK n'est pas AA ou CA
(<code>send</code> ; un fichier contenant plusieurs messages est envoyé
en une trame par message), et exporte le XSD des messages de l'édition
Community (<code>xsd</code> ; le catalogue complet est dans Pro). Chaque
commande qui lit un message accepte <code>-</code> pour l'entrée
standard. Un binaire par plateforme accompagne chaque version. Mettez
entre guillemets doubles les motifs et les arguments contenant
<code>^</code> (<code>"*.hl7"</code>, <code>"ORU^R01"</code>) :
<code>cmd.exe</code> de Windows supprime un <code>^</code> non protégé et
garde les apostrophes.</p>
<p>Le <code>batch</code> de la CLI parcourt les sous-dossiers, lit les
fichiers <code>.hl7</code> sauf si <code>--extension</code> dit autre
chose et vérifie FHIR autant que HL7 v2 : ses totaux peuvent donc
différer de <strong>Outils → Validation par lots…</strong> sur le même
dossier (un dossier, <code>.hl7</code>, <code>.txt</code> et
<code>.dat</code>, HL7 v2 seulement) ; <code>--extension hl7,txt,dat</code>
lit les mêmes fichiers.</p>
<pre><code>bridgelab-cli validate message.hl7 bundle.json
bridgelab-cli validate "*.hl7" --format junit --output report.xml
bridgelab-cli batch ./inbox --json
bridgelab-cli test regression.bltests.json --format junit --output tests.xml
bridgelab-cli fhirpath "Patient.name.family" patient.json
cat message.hl7 | bridgelab-cli send - --host 10.0.0.5 --port 2575</code></pre>
<p>Écrivez les rapports avec <code>--output</code>, comme ci-dessus :
dans Windows PowerShell 5.1, <code>&gt;</code> écrit en UTF-16, que les
outils de CI ne savent pas lire.</p>
`,
};

export const communicationSection: ManualSection = {
	id: 'communication',
	heading: 'Communication (MLLP / HTTP / SOAP)',
	body: `
<p>Ouvrez le panneau de communication en bas avec <kbd>Ctrl</kbd>+<kbd>K</kbd>
ou <strong>Outils → Panneau de communication</strong>. Quatre onglets :
MLLP, HTTP, SOAP et Historique. Le panneau s'ouvre assez haut pour
afficher tous les réglages de l'onglet affiché, jusqu'à environ 70&nbsp;%
de la fenêtre ; réduisez-le et une barre de défilement apparaît, et la
hauteur que vous choisissez est conservée jusqu'à ce que vous fermiez le
panneau. Avant d'envoyer un message de plus de 10 Mo, par MLLP, HTTP ou
SOAP, BridgeLab demande confirmation : l'envoi prend du temps, et beaucoup
de destinataires refusent un message aussi gros (l'écouteur de BridgeLab
lui-même accepte jusqu'à 10 Mo).</p>

${mockupCommunication}

<h3>Client MLLP</h3>
<ol>
	<li>Saisissez <em>Hôte</em> + <em>Port</em> (p. ex. <code>localhost:2575</code>).</li>
	<li>Le message de l'onglet actif est utilisé automatiquement.</li>
	<li>Cliquez sur <strong>Envoyer</strong>. Le framing (<code>0x0B</code> ... <code>0x1C 0x0D</code>),
		le transport et l'attente de l'ACK sont gérés par le backend
		Rust.</li>
	<li>L'ACK apparaît dans la zone de résultat avec le temps
		aller-retour. <em>Accept</em> (AA), <em>Error</em> (AE) et
		<em>Reject</em> (AR) sont tous affichés avec le
		<code>MSA|AA|{control-id}</code> d'origine. Une réponse qui n'est
		pas du HL7, un ACK dont le MSA-2 désigne un autre message, ou un AA
		ou CA au MSA-2 vide pour un message qui a un MSH-10 s'affiche en
		ambre avec la raison : il n'accepte pas le message envoyé.</li>
</ol>

<h3>Générateur ACK</h3>
<p>La ligne <strong>Générateur ACK</strong> de l'onglet MLLP
construit un acquittement pour le message actuellement dans l'éditeur :
choisissez le code (AA accepter, AE erreur, AR rejeter) et cliquez sur
<strong>Générer un ACK</strong>. L'ACK reflète le message, comme doit le
faire un destinataire : mêmes séparateur de champ et caractères
d'encodage, émetteur et destinataire échangés (MSH-3/4 et MSH-5/6),
<code>ACK^&lt;trigger&gt;^ACK</code> en MSH-9, mêmes processing ID
(MSH-11), version (MSH-12) et jeu de caractères (MSH-18), et le Message
Control ID (MSH-10, lu avec le séparateur déclaré en MSH-1) en MSA-2.
Chaque ACK a son propre MSH-10. L'auto-ACK du listener est construit de
la même façon. L'ACK s'ouvre dans un nouvel onglet — prêt à être
renvoyé ou conservé comme fixture. Si le message courant n'a pas de
MSH-10, le générateur refuse au lieu de produire un ACK impossible à
corréler.</p>

<h3>Listener MLLP (Pro)</h3>
<p>Cliquez sur <strong>Démarrer l'écoute</strong> pour lancer un serveur
sur le port choisi. Les messages entrants s'ouvrent dans un nouvel
onglet (désactivable, voir plus bas) et un ACK automatique est renvoyé
avec le code configuré (AA/AE/AR). Utilisez-le pour valider rapidement
ce que votre système amont émet.</p>
<p>Par défaut, le listener écoute sur <code>127.0.0.1</code> : seuls les
programmes de cet ordinateur peuvent le joindre. Pour recevoir un flux
d'une autre machine, réglez <strong>Écouter sur</strong> sur
<code>0.0.0.0</code> (toutes les interfaces) ou sur l'adresse d'une carte
réseau, et n'ouvrez le port dans le pare-feu que pour les systèmes
attendus. Un <strong>Écouter sur</strong> vide signifie lui aussi cet
ordinateur seulement. Une adresse IPv6 peut s'écrire avec ou sans
crochets (<code>::1</code> ou <code>[::1]</code>). Avec le port 0, le
système choisit un port libre ; l'état indique ensuite lequel, pour que
vous puissiez le donner à l'émetteur.</p>
<p><strong>Arrêter</strong> ferme aussi les connexions encore
ouvertes : ensuite, plus rien n'est reçu ni acquitté.</p>
<p>Le listener continue de fonctionner quand le panneau de communication
est fermé : les messages reçus s'ouvrent toujours dans de nouveaux
onglets et sont consignés dans la console, qui les affiche, avec ses
compteurs, quand vous rouvrez le panneau.</p>
<p>Un seul expéditeur ne peut ni arrêter le listener ni remplir la
mémoire de cet ordinateur. Le listener sert au plus 64 connexions à la
fois : une connexion de plus est acceptée puis fermée aussitôt, et une
ligne rouge dans la console en donne la raison. Un message peut atteindre
10&nbsp;Mo, et les messages encore en cours de réception peuvent occuper
64&nbsp;Mo au total, toutes connexions confondues : un message qui
dépasse l'une ou l'autre limite est refusé par un AR et sa connexion
fermée. Un message doit arriver en entier dans les 2 minutes qui suivent
son premier octet (ou dans le délai de lecture par connexion, s'il est
réglé plus long), sinon sa connexion est coupée : le délai de lecture
repart à chaque octet, si bien qu'un expéditeur envoyant un octet à la
fois garderait sa connexion indéfiniment. Le trafic ordinaire n'approche
jamais ces limites : des centaines de messages à la suite, quelques
expéditeurs à la fois, des messages de plusieurs Mo.</p>
<p>Quand l'acceptation d'une connexion échoue, par exemple parce que le
système n'a plus de fichiers ouverts disponibles, une ligne rouge dans la
console le signale et le listener réessaie un instant plus tard. S'il
s'arrête malgré tout, le panneau affiche de nouveau <strong>Démarrer
l'écoute</strong>, avec la raison en rouge, au lieu de « À l'écoute » sur
un port qui ne répond plus.</p>

<h3>Console du listener</h3>
<p>Pendant que le listener tourne, chaque message reçu apparaît comme une
ligne dans la console : heure locale, adresse du pair, taille du
payload, le code ACK réellement renvoyé (<code>AA</code> en vert,
<code>AE</code>/<code>AR</code> en rouge, — quand l'ACK automatique est
désactivé), l'encodage de caractères utilisé et la première ligne du
message. <strong>Cliquez sur une ligne pour rouvrir ce message dans un
onglet.</strong> Les erreurs du listener s'affichent au fil de l'eau
sous forme de lignes rouges. Une connexion qui n'envoie rien est fermée
après le délai de lecture par connexion, pour libérer sa place, avec une
ligne grise ordinaire, non rouge : un émetteur peut se connecter bien
avant d'avoir un message.</p>
<p>Les puces de l'en-tête de la console filtrent les lignes par
résultat — <em>AA</em>, <em>AE</em>, <em>AR</em>, <em>Sans ACK</em>
(reçus avec l'ACK automatique désactivé), <em>Erreurs</em> — et chacune
porte un compteur à jour : « AE&nbsp;12 » sur 300 saute aux yeux avant
même de faire défiler. Les compteurs portent sur toutes les lignes
depuis le dernier effacement de la console, alors que la console ne
conserve que les 200 dernières lignes : au-delà, une ligne au-dessus de
la liste le signale, et les puces ne filtrent que les lignes conservées. Les mêmes
puces figurent dans l'onglet Historique.</p>
<p>L'option <em>« Ouvrir les messages reçus dans un nouvel onglet »</em>
(activée par défaut) peut être désactivée pendant les tests à fort
volume : les messages n'arrivent alors que dans la console et vous
n'ouvrez que ceux dont vous avez besoin. Même activée, une rafale
n'ensevelit pas la fenêtre sous les onglets : après 20 nouveaux onglets,
ou 16 Mo de messages, sans touche pressée ni clic dans la fenêtre, les messages suivants ne
vont que dans la console et l'Historique, et un seul avis au-dessus de
l'éditeur les compte, avec un bouton qui affiche la console. Une touche
ou un clic fait de nouveau ouvrir les messages suivants dans un onglet ;
fermer l'avis aussi.</p>
<p class="note">Les contenus complets conservés pour l'ouverture au clic
sont plafonnés par un budget glissant de 32&nbsp;Mo. Lors de longues
sessions sans surveillance, les lignes les plus anciennes perdent leur
contenu complet (elles apparaissent estompées) — la ligne de métadonnées
reste, et rien n'a été « perdu » : seule la copie servant à l'ouverture
au clic a été libérée pour borner la mémoire.</p>

<h3>Encodage de caractères</h3>
<p>L'envoi et le listener disposent chacun d'un sélecteur
<strong>Encodage</strong> couvrant les jeux de caractères rencontrés en
production : <code>UTF-8</code>, <code>ISO-8859-1</code> (Latin-1),
<code>ISO-8859-2</code>, <code>ISO-8859-15</code>,
<code>windows-1252</code>, <code>windows-1250</code>,
<code>windows-1251</code> et <code>ASCII</code>. La valeur par défaut
est <strong>Auto</strong>. Côté listener, chaque message est décodé dans
le jeu de caractères déclaré en MSH-18, sinon en UTF-8 avec repli
automatique sur Latin-1, ce qui accepte la plupart du trafic ancien même
quand MSH-18 est vide ; la console affiche le jeu choisi, et l'ACK est
ré-encodé avec lui pour que le pair ne voie jamais de caractères
corrompus. Côté envoi, c'est le jeu déclaré en MSH-18, sinon celui avec
lequel le fichier de l'onglet a été lu, sinon UTF-8, et le résultat
indique le jeu envoyé. En Auto, un message dont le MSH-18 désigne un jeu
que BridgeLab ne sait pas écrire (<code>ISO IR87</code>,
<code>UNICODE UTF-16</code>…) est refusé au lieu de partir en UTF-8 sous
cet en-tête, de même qu'un onglet dont les messages déclarent des jeux
différents quand les envoyer ensemble, dans un seul, changerait leur
texte (l'ASCII simple se lit de la même façon dans tous et est envoyé).
En choisissant un encodage dans les paramètres avancés, ils partent
quand même ; <code>bridgelab-cli send</code> sans
<code>--encoding</code> refuse les mêmes fichiers. Un message contenant des caractères que le jeu
choisi ne peut pas représenter n'est pas envoyé : l'erreur les cite,
pour qu'un nom n'arrive jamais avec un <code>?</code>. Les encodages
d'envoi et de réception sont indépendants.</p>
<p>Les segments partent terminés par CR, le dernier aussi, quelles que
soient les fins de ligne de l'éditeur : LF et CR LF sont convertis et
les lignes vides ne sont pas envoyées, comme le fait la CLI, et il en va
de même d'un corps HL7 v2 envoyé en HTTP ou SOAP.</p>
<p>Les <strong>fichiers</strong> suivent les mêmes règles. Un fichier qui
n'est pas en UTF-8 s'ouvre dans l'encodage déclaré par MSH-18
(<code>8859/1</code>, <code>8859/2</code>, <code>8859/15</code>…, ainsi
que <code>BIG-5</code>, <code>GB 18030-2000</code> et
<code>KS X 1001</code>), ou en Windows-1252 si MSH-18 est vide : les noms
accentués s'affichent correctement au lieu d'être refusés. Un fichier dont
le MSH-18 nomme un encodage que BridgeLab ne sait pas décoder
(<code>CNS 11643-1992</code>, les jeux japonais ISO 2022) s'ouvre avec un
avertissement, car son texte et même ses champs peuvent être mal lus ; la
validation et l'anonymisation par lot le refusent. L'enregistrement
réécrit le fichier dans l'encodage où il a été lu, comme
<code>anonymize</code> de la CLI et l'anonymisation par lot ;
<code>send</code> sans <code>--encoding</code> transmet dans cet encodage
(un fichier UTF-16 part dans l'encodage déclaré par MSH-18, sinon en
UTF-8). À l'enregistrement, un caractère que l'encodage ne peut pas
représenter est écrit <code>?</code> ; le <code>send</code> de la CLI,
comme l'application, refuse alors le message.</p>

<h3>HTTP</h3>
<p>Les requêtes GET sont disponibles dans l'édition Community. POST/PUT/DELETE/PATCH
et toute forme d'authentification nécessitent Pro : un en-tête
Authorization ou Proxy-Authorization, un cookie, tout en-tête dont le
nom évoque une clé, un jeton, un secret, une signature ou une session
(comme <code>X-API-Key</code> ou <code>Ocp-Apim-Subscription-Key</code>),
un utilisateur:mot de passe dans l'URL, une clé ou un jeton dans la
query string (<code>access_token</code>, <code>api_key</code>,
<code>key</code>, <code>token</code>, <code>sig</code>,
<code>client_secret</code>…), ou un identifiant de session dans le
chemin de l'URL (<code>;jsessionid=…</code>) ou dans un paramètre
<code>PHPSESSID</code>. Les paramètres de recherche FHIR comme
<code>code</code> ne sont pas des identifiants. Les redirections sont
suivies dans toutes les éditions, mais uniquement sur le même serveur (ou de
http vers https sur celui-ci) : une redirection vers un autre serveur est
affichée comme le 3xx qu'elle est et n'est jamais suivie, afin qu'un message
ne parte pas là où vous ne l'avez pas choisi. Quand une redirection a été
suivie, le résultat indique l'URL qui a répondu. Les réponses de plus de
50 Mo sont tronquées et le signalent, et une réponse est lue dans le jeu
de caractères déclaré par son Content-Type. POST, PUT et PATCH envoient
le message de l'onglet courant quand le champ Corps est vide ; GET et
DELETE n'envoient un corps que si vous en saisissez un.</p>

<h3>Client SOAP (Enterprise)</h3>
<p>L'onglet SOAP envoie le message courant (ou un corps personnalisé)
vers des endpoints SOAP 1.1/1.2 — middlewares de type IHE, passerelles
régionales et services web hospitaliers legacy. Renseignez l'URL de
l'endpoint, la version SOAP et la <em>SOAPAction</em> ; BridgeLab
construit l'enveloppe, l'envoie avec le bon content type
(<code>text/xml</code> plus l'en-tête SOAPAction en 1.1,
<code>application/soap+xml</code> avec le paramètre action en 1.2) et
affiche le statut HTTP, le temps aller-retour, le XML interne du Body
et l'éventuel SOAP Fault, décodé pour les deux versions.</p>
<p>Un message HL7 v2 brut est automatiquement échappé en XML (ses
CR de fin de segment deviennent <code>&amp;#13;</code>, pour que
l'analyse XML ne les transforme pas en LF) et enveloppé dans un élément
<code>&lt;payload&gt;</code> ; un contenu déjà XML est inséré tel quel,
sans sa propre déclaration <code>&lt;?xml …?&gt;</code> ni son
indicateur d'ordre des octets (BOM) : l'enveloppe a déjà sa déclaration,
et une seconde dans le Body n'est pas du XML valide.
Les paramètres avancés ajoutent des identifiants <strong>WS-Security
UsernameToken</strong>, des en-têtes <strong>WS-Addressing</strong>
(To / Action / MessageID) et un <strong>modèle d'enveloppe
personnalisé</strong> dans lequel l'espace réservé littéral
<code>{payload}</code> est remplacé par le message — utile quand le
service cible attend un wrapper précis. Avec un modèle, les en-têtes
WS-Security et WS-Addressing sont placés dans le SOAP Header du modèle
(un Header est ajouté avant son Body s'il n'en a pas) ; un modèle sans
SOAP Envelope ni Body ne peut pas les porter, et l'envoi est refusé
plutôt que fait sans eux. L'import WSDL est prévu dans une étape
ultérieure.</p>

<h3>Proxy et certificats</h3>
<p>Les requêtes HTTP et SOAP, l'activation en ligne, le renouvellement
de la licence et les statistiques d'utilisation passent par le proxy
indiqué dans la variable d'environnement <code>HTTPS_PROXY</code> ou
<code>HTTP_PROXY</code>, avec <code>NO_PROXY</code> pour les hôtes à
joindre directement (<code>NO_PROXY=10.0.0.0/8,.hopital.local</code>).
Si aucune des deux n'est définie, ils utilisent le proxy manuel du
système : sous Windows <em>Paramètres → Réseau et Internet → Proxy →
Configuration manuelle du proxy</em>, sous macOS le <em>Proxy web
(HTTP)</em> et le <em>Proxy web sécurisé (HTTPS)</em> des réglages proxy
du réseau. La configuration automatique du proxy n'est pas lue : ni
script de configuration (PAC), ni détection automatique (WPAD). Sur un
tel réseau, demandez l'adresse du proxy à votre service informatique et
indiquez-la dans <code>HTTPS_PROXY</code> ou comme proxy manuel. MLLP ne
passe jamais par un proxy.</p>
<p><code>localhost</code>, <code>127.0.0.1</code> et <code>::1</code> sont
toujours joints directement. Parmi les exceptions des paramètres proxy
de Windows, seuls les noms d'hôte et de domaine s'appliquent
(<code>fhir.example.org</code>, <code>*.example.org</code>) : « Ne pas
utiliser le serveur proxy pour les adresses locales (intranet) » et les
motifs d'adresse comme <code>10.*</code> ne s'appliquent pas, et la liste
d'exceptions de macOS n'est pas lue ; indiquez ces hôtes dans
<code>NO_PROXY</code>. Un proxy différent pour chaque protocole (Options
Internet → Paramètres réseau → Avancé) n'est pas lu non plus.
L'activation et le renouvellement lisent les paramètres proxy une fois
par démarrage : après les avoir modifiés, redémarrez BridgeLab.</p>
<p>Les certificats HTTPS sont vérifiés avec le magasin de certificats du
système et les racines Mozilla intégrées à BridgeLab ; un certificat qui
échoue à la vérification ne peut pas être accepté malgré tout. Derrière
un proxy qui inspecte le trafic TLS, son certificat racine doit se
trouver dans le magasin du système, où le service informatique
l'installe d'habitude. Un certificat refusé est signalé comme tel, avec
la raison (non approuvé, expiré, émis pour un autre nom), et non comme
une panne réseau.</p>

<h3>Historique</h3>
<p>Chaque envoi, et chaque message reçu par le listener, est
journalisé : cible (hôte et port, ou l'URL avec mot de passe ou clé
remplacés par <code>***</code>, dans le fragment aussi et dans une URL
qui n'a pas pu être envoyée), taille, code de réponse et temps
aller-retour. Les 100 dernières entrées sont conservées entre les
redémarrages et les plus anciennes sont supprimées ; cliquez sur une
ligne pour voir la requête et la réponse complètes — le message et son
ACK, ou le corps HTTP et la réponse (pour SOAP la charge utile, jamais
l'enveloppe avec son mot de passe). Une requête ou une réponse de plus
de 256 Ko est conservée tronquée. Un envoi MLLP enregistre aussi le
<strong>code ACK</strong> renvoyé par le destinataire (MSA-1), affiché
en badge vert ou rouge sur la ligne : un envoi arrivé au pair qui reçoit
un <code>AE</code> est « OK » au niveau transport et un rejet au niveau
applicatif, et le badge distingue les deux. Un ACK dont le MSA-2
désigne un autre message, ou un AA ou CA au MSA-2 vide (il ne désigne
aucun message), n'accuse réception de rien de ce qui a été
envoyé : il n'a pas de badge et figure sous <em>Sans ACK</em>, comme une
réponse qui n'est pas du HL7. Les puces de filtre
au-dessus de la liste — <em>AA</em>, <em>AE</em>, <em>AR</em>,
<em>Sans ACK</em>, <em>Échoués</em> — portent des compteurs et
restreignent la liste à un seul résultat ; les codes commit-mode CA, CE
et CR comptent sous AA, AE et AR. <em>Échoués</em> désigne une requête
restée sans réponse : un serveur qui a répondu par une erreur (HTTP 404,
500…) apparaît avec son code.</p>

<h3>Profils de connexion</h3>
<p>Enregistrez les endpoints fréquemment utilisés comme profils nommés
depuis la ligne <strong>Profil</strong> : saisissez un nom et cliquez
sur <em>Enregistrer</em>. Les profils MLLP stockent hôte, port, délai
d'expiration et ACK automatique ; les profils HTTP stockent URL,
en-têtes et délai d'expiration ; les profils SOAP stockent endpoint,
SOAPAction et délai d'expiration. Sélectionner un profil l'applique au
formulaire ; enregistrer sous un nom existant l'écrase ;
<em>Supprimer</em> retire le profil sélectionné. Les profils sont
stockés dans la base de données locale et survivent aux redémarrages.</p>
<p class="note">Les profils HTTP enregistrent l'URL et le champ
En-têtes tels qu'ils sont saisis, user:password ou clé dans l'URL et
en-tête <code>Authorization</code> compris, dans la base de données
locale ; les profils SOAP enregistrent leur endpoint de la même façon.
Pour ne pas y enregistrer un mot de passe ou un jeton, saisissez-le sous
<em>Authentification</em> dans les paramètres HTTP avancés, qui ne sont
jamais enregistrés.</p>
`,
};

export const anonymizationSection: ManualSection = {
	id: 'anonymization',
	heading: 'Anonymisation et export',
	body: `
<p><strong>Outils → Anonymiser</strong> détecte les champs PHI dans
les segments qui identifient le patient et les masque selon leur niveau
de sensibilité : 89 champs intégrés dans PID, PV1 (numéros de venue),
MRG, NK1, GT1, IN1 et IN2, ainsi que les commentaires NTE et les
résultats OBX en texte libre (TX/FT). Ils couvrent les noms, dates de
naissance et de décès, adresses, téléphones, SSN et autres identifiants
du patient, des proches, du garant et de l'assuré. Un identifiant garde
son autorité d'attribution et son type (<code>HOSP</code>,
<code>MR</code>). Les autres segments ne sont pas masqués : vérifiez le
texte libre ailleurs (OBR, ORC, segments Z) avant de partager, ou
ajoutez ces champs avec un plugin.</p>

<table>
	<tr><th>Niveau</th><th>Exemple</th><th>Stratégie</th></tr>
	<tr><td><strong>Élevé</strong></td><td>Nom du patient, date de
		naissance, adresse, téléphone du domicile, SSN, MRN, numéro de
		venue</td>
		<td>Le texte devient <code>REDACTED</code> ; le numérique devient
		des zéros de même longueur (la largeur du champ est préservée
		pour les parseurs en aval) ; une date devient 1900-01-01 avec la
		même précision.</td></tr>
	<tr><td><strong>Moyen</strong></td><td>Nom de jeune fille de la mère,
		alias, téléphone professionnel, contacts des proches, du garant
		et de l'assuré</td>
		<td>Premier caractère conservé, le reste remplacé par
		<code>***</code>.</td></tr>
	<tr><td><strong>Faible</strong></td><td>Aucun champ intégré ;
		disponible pour les règles des plugins</td>
		<td>3 premiers caractères conservés, suivis de <code>...</code> ;
		une valeur de 3 caractères ou moins est conservée
		entière.</td></tr>
</table>

<p>La boîte de dialogue liste chaque champ PHI détecté avant l'exécution
du masquage, pour que vous puissiez vérifier ce qui va changer. Le
résultat :</p>
<ul>
	<li><strong>S'ouvre dans un nouvel onglet</strong> - le message
		original reste intact dans son propre onglet.</li>
	<li><strong>Peut être copié dans le presse-papiers</strong>
		directement.</li>
	<li><strong>Préserve la structure</strong> - l'ordre des segments, le
		nombre de pipes et les séparateurs de composants sont inchangés,
		donc le résultat s'analyse toujours comme du HL7 valide.</li>
</ul>

<h3>Champs PHI personnalisés via plugins</h3>
<p>Les déploiements comportant des identifiants régionaux ou propres à un
fournisseur (identifiant national UE, champs internes de Z-segments)
peuvent étendre le catalogue en déposant un fichier JSON sous
<code>&lt;config&gt;/BridgeLab/plugins/anonymization/</code>.</p>

<h3>Anonymisation par lot (Pro)</h3>
<p><strong>Outils → Anonymisation par lot…</strong> masque un dossier
entier en une seule passe : choisissez des fichiers source ou un
dossier, choisissez un dossier de sortie, lancez. Chaque message passe
par le même pipeline que la boîte de dialogue interactive (catalogue PHI
intégré + règles des plugins actifs) et est écrit en copie dans le
dossier de sortie — <strong>les originaux ne sont jamais
modifiés</strong> : l'outil refuse d'écraser tout fichier source
sélectionné, et les fichiers homonymes issus de dossiers différents
reçoivent des suffixes numériques au lieu de s'écraser mutuellement. Un
fichier déjà présent dans le dossier de sortie n'est jamais remplacé (et
un lien n'y est jamais suivi) : sa ligne le signale et le fichier reste
tel quel, choisissez donc un dossier vide. Une
ligne par fichier indique le nombre de champs PHI masqués ou l'erreur ;
les mêmes plafonds de 5000 fichiers / 10&nbsp;Mo que la validation par
lots s'appliquent.</p>

<h3>Export</h3>
<p>Les utilisateurs Pro peuvent exporter le message structuré en JSON ou
CSV via <strong>Outils → Exporter JSON / CSV</strong> ; une boîte
d'enregistrement demande où écrire le fichier. Utile pour charger des
données HL7 dans des outils d'analyse (Power BI, Excel, pandas).</p>

<div class="warn">L'anonymisation écrit son résultat dans un nouvel
onglet ; l'onglet d'origine reste tel quel. Conservez toujours votre fichier source original comme
référence canonique - la copie anonymisée sert au partage, pas à
l'archivage de long terme.</div>
`,
};

export const testCasesSection: ManualSection = {
	id: 'testcases',
	heading: 'Bibliothèque de cas de test',
	body: `
<p>La Bibliothèque de cas de test (<kbd>Ctrl</kbd>+<kbd>L</kbd>) stocke
des messages réutilisables avec un nom, une catégorie, des étiquettes et
une description. Utilisez <strong>Enregistrer le message actuel</strong>
pour capturer l'onglet actif, ou créez des cas de toutes pièces. Les cas
persistent dans la base de données locale et peuvent être recherchés par
n'importe lequel de leurs champs.</p>

<p class="note">L'édition Community conserve jusqu'à 10 cas de test
enregistrés — les cas existants restent toujours visibles, modifiables
et exécutables ; seuls les nouveaux enregistrements au-delà du plafond
demandent une mise à niveau.</p>

<h3>Résultats attendus</h3>
<p>Chaque cas peut déclarer un <strong>type de message attendu</strong>
(seules les composantes indiquées sont comparées : <code>ADT</code> couvre
tout événement ADT, <code>ADT^A01</code> couvre <code>ADT^A01</code> et
<code>ADT^A01^ADT_A01</code> mais pas <code>ADT^A04</code>) et un <strong>résultat de validation attendu</strong> (valide /
invalide). C'est ce qui transforme un simple extrait en test.</p>

<h3>Exécuter les vérifications</h3>
<p><strong>Vérifier</strong> analyse et valide réellement un cas — HL7 v2
ou FHIR, détecté automatiquement — et compare le résultat à ses
attentes. <strong>Tout exécuter</strong> fait de même pour chaque cas
correspondant à la recherche courante, avec un badge par ligne
(✓ réussi, ✖ échoué, ~ non vérifié) et un récapitulatif réussis/total
dans la barre d'outils. Une ressource FHIR qui satisfait ses attentes
mais n'a pas été entièrement vérifiée — un profil qu'elle déclare n'est
pas installé, ou aucun paquet installé ne définit son type de
ressource — est <em>non vérifiée</em>, avec la raison, et comptée à part
(« 4/8 réussis, 1 non vérifiés »), comme <code>bridgelab-cli test</code>
la signale : jamais comme réussie. Après
une évolution d'interface, un seul clic vous dit lesquels de vos
messages de référence ont cassé. Modifier un cas efface son résultat
mémorisé jusqu'à la prochaine exécution.</p>

<h3>Partager des cas de test</h3>
<p><strong>Exporter…</strong> écrit les cas de test affichés — tous, ou
seulement ceux qui correspondent à la recherche — dans un paquet
<code>.bltests.json</code> à envoyer à un collègue ou à versionner dans
un dépôt Git. Avant l'enregistrement, BridgeLab recherche des données
personnelles dans les messages HL7 v2 et liste les cas et les champs
trouvés ; les ressources FHIR sont signalées comme non vérifiées champ
par champ. Avec Pro, cochez <em>Masquer les données personnelles</em>
pour anonymiser les messages HL7 v2 dans le fichier exporté uniquement :
la bibliothèque n'est pas modifiée.</p>
<p><strong>Importer…</strong> ouvre un paquet et montre, avant toute
écriture, ce qu'est chaque cas : <em>Nouveau</em>, <em>Déjà dans la
bibliothèque</em> (ignoré) ou <em>Différent</em> d'un cas existant,
auquel cas vous choisissez de garder le vôtre, de le remplacer ou de
garder les deux. Les cas importés gardent leur identifiant : réimporter
le même paquet n'apporte que ce qui a changé. En Community, un import
ne peut pas faire dépasser 10 cas de test à la bibliothèque ; rien
n'est écrit dans ce cas.</p>

<h3>Restauration de session</h3>
<p>BridgeLab sauvegarde vos onglets ouverts (y compris les modifications
non enregistrées) et les rouvre au lancement suivant, à la manière de
Notepad++. Contrôlez ce comportement dans
<strong>Paramètres → Performances</strong>, dans le groupe <em>Session</em> : activez/désactivez <em>Restaurer
les onglets ouverts au démarrage</em>, ou utilisez <em>Effacer la
session enregistrée</em> pour purger le jeu d'onglets stocké (cela
désactive aussi la restauration, si bien que le prochain lancement
démarre sur l'écran d'accueil). Désactiver la restauration supprime aussi
les onglets enregistrés, qui peuvent contenir des données de patients.</p>
<p>Restauration désactivée, fermer la fenêtre demande confirmation avant
de perdre les onglets non enregistrés. Quand c'est le système qui ferme
BridgeLab (déconnexion, arrêt ou redémarrage, <em>Quitter</em> depuis le
Dock sous macOS), personne ne peut être interrogé : les onglets non
enregistrés sont alors gardés une fois comme copie de récupération, dans
la même base de données, et le lancement suivant les propose avec
<em>Restaurer</em> et <em>Abandonner</em> ; dans les deux cas, la copie
est ensuite supprimée. Restauration activée, les onglets les plus
récents sont écrits comme session à ce moment-là et reviennent comme
d'habitude. Sous Windows, et sous macOS en quittant depuis le Dock ou en
se déconnectant, BridgeLab ne peut plus interroger la fenêtre à ce
moment-là et garde les onglets tels que la fenêtre les a transmis en
dernier (un tiers de seconde après chaque modification, et chaque fois
que vous quittez la fenêtre) : les modifications de la dernière fraction
de seconde peuvent être perdues. Rien ne peut être gardé quand BridgeLab est
tué brutalement, ou quand la session du bureau ferme l'affichage avant
de demander à l'application de quitter.</p>
<p>Un gros onglet restauré est lu depuis son fichier la première fois que
vous l'affichez ; tant que le texte n'est pas chargé, l'éditeur est en
lecture seule et <strong>Enregistrer</strong> attend, si bien que le
fichier n'est jamais écrit vide. Si la base de données de session
(<code>bridgelab.db</code>) ne peut pas être ouverte — endommagée,
verrouillée par un autre programme, sur un disque plein ou en lecture
seule —, BridgeLab démarre quand même, sur une base de données en
mémoire, et un avertissement indique le fichier et la raison. Tant que
le problème n'est pas corrigé, les onglets ouverts, les fichiers récents,
l'historique des communications, les profils de connexion, les cas de
test et les préférences ne sont pas conservés après la fermeture de
BridgeLab. Le fichier lui-même n'est jamais supprimé ni écrasé.</p>

<h3>Fichiers modifiés par d'autres programmes</h3>
<p>BridgeLab remarque quand un autre programme modifie ou supprime un
fichier ouvert : quand vous revenez dans la fenêtre, ou rouvrez le
fichier, il propose de charger la nouvelle version (et vous prévient si
le fichier a disparu). <strong>Enregistrer</strong> demande confirmation
avant d'écraser un fichier modifié sur le disque depuis son ouverture ou
le dernier enregistrement, ou avant de recréer un fichier supprimé. Au
lancement, un onglet restauré sans modifications non enregistrées
affiche son fichier tel qu'il est maintenant sur le disque. Un onglet restauré avec des modifications non enregistrées demande confirmation au premier enregistrement, car le fichier a pu changer pendant que BridgeLab était fermé.</p>
`,
};
