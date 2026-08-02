import type { ManualSection } from '../helpContent';
import { mockupValidation, mockupCommunication } from './mockups';

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
		alphanumériques ; pas de MSH en double.</li>
	<li><strong>En-tête MSH :</strong> MSH-9 (type de message), MSH-10 (ID
		de contrôle), MSH-12 (version) sont obligatoires.</li>
	<li><strong>Champs obligatoires :</strong> champs obligatoires par
		segment, tirés du standard HL7 (p. ex. PID-3 Patient Identifier
		List).</li>
	<li><strong>Limites de longueur :</strong> avertit lorsqu'un champ
		dépasse la <code>max_length</code> publiée.</li>
	<li><strong>Types de données :</strong> les champs numériques (SI, NM)
		sont contrôlés pour détecter les caractères non numériques ; les
		formats d'horodatage (TS) sont vérifiés en longueur et en
		composition (chiffres uniquement).</li>
</ul>

<h3>Filtrage et navigation</h3>
<p>Cliquez sur les badges Erreur / Avertissement / Info pour filtrer.
Cliquez sur la ligne d'un problème pour atteindre le segment fautif dans
l'éditeur.</p>

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
instantanés pour le validateur par lots ci-dessus.</p>

<h3>Validation en CLI</h3>
<p>L'outil compagnon <code>bridgelab-cli</code> offre le même validateur
pour un usage headless (pipelines CI, criblage par lots) :</p>
<pre><code>bridgelab-cli validate message.hl7
bridgelab-cli validate '*.hl7' --format junit &gt; report.xml
bridgelab-cli batch ./inbox --json</code></pre>
`,
};

export const communicationSection: ManualSection = {
	id: 'communication',
	heading: 'Communication (MLLP / HTTP)',
	body: `
<p>Ouvrez le panneau de communication en bas avec <kbd>Ctrl</kbd>+<kbd>K</kbd>
ou <strong>Outils → Panneau de communication</strong>. Trois onglets :
MLLP, HTTP et Historique.</p>

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
		<code>MSA|AA|{control-id}</code> d'origine.</li>
</ol>

<h3>Générateur ACK</h3>
<p>La ligne <strong>Générateur ACK</strong> de l'onglet MLLP construit un
acquittement pour le message actuellement dans l'éditeur : choisissez le
code (AA accepter, AE erreur, AR rejeter) et cliquez sur
<strong>Générer un ACK</strong>. Le Message Control ID est lu depuis
MSH-10 (en respectant le séparateur de champ déclaré dans MSH-1) et
l'ACK obtenu s'ouvre dans un nouvel onglet — prêt à être renvoyé ou à
servir de message de référence. Si le message courant n'a pas de MSH-10,
le générateur refuse plutôt que de produire un ACK impossible à
corréler.</p>

<h3>Listener MLLP (Pro)</h3>
<p>Cliquez sur <strong>Démarrer l'écoute</strong> pour lancer un serveur
sur le port choisi. Les messages entrants s'ouvrent dans un nouvel
onglet (désactivable, voir plus bas) et un ACK automatique est renvoyé
avec le code configuré (AA/AE/AR). Utilisez-le pour valider rapidement
ce que votre système amont émet.</p>

<h3>Console du listener</h3>
<p>Pendant que le listener tourne, chaque message reçu apparaît comme une
ligne dans la console : heure locale, adresse du pair, taille du
payload, le code ACK réellement renvoyé (<code>AA</code> en vert,
<code>AE</code>/<code>AR</code> en rouge, — quand l'ACK automatique est
désactivé), l'encodage de caractères utilisé et la première ligne du
message. <strong>Cliquez sur une ligne pour rouvrir ce message dans un
onglet.</strong> Les erreurs du listener s'affichent au fil de l'eau
sous forme de lignes rouges.</p>
<p>L'option <em>« Ouvrir les messages reçus dans un nouvel onglet »</em>
(activée par défaut) peut être désactivée pendant les tests à fort
volume : les messages n'arrivent alors que dans la console et vous
n'ouvrez que ceux dont vous avez besoin.</p>
<p class="note">Les contenus complets conservés pour l'ouverture au clic
sont plafonnés par un budget glissant de 32&nbsp;Mo. Lors de longues
sessions sans surveillance, les lignes les plus anciennes perdent leur
contenu complet (elles apparaissent estompées) — la ligne de métadonnées
reste, et rien n'a été « perdu » : seule la copie servant à l'ouverture
au clic a été libérée pour borner la mémoire.</p>

<h3>Encodage de caractères</h3>
<p>L'émetteur comme le listener disposent d'un sélecteur
<strong>Encodage</strong> couvrant les jeux de caractères rencontrés
dans les déploiements réels : <code>UTF-8</code>,
<code>ISO-8859-1</code> (Latin-1), <code>ISO-8859-2</code>,
<code>ISO-8859-15</code>, <code>windows-1252</code>,
<code>windows-1250</code>, <code>windows-1251</code> et
<code>ASCII</code>. Le réglage par défaut est UTF-8 avec repli
automatique en Latin-1, ce qui accepte la plupart du trafic legacy même
quand MSH-18 est vide. Le listener ré-encode son ACK avec le même jeu de
caractères afin que le pair ne voie jamais de mojibake. Les encodages
d'envoi et de réception sont indépendants.</p>

<h3>HTTP</h3>
<p>Les requêtes GET sont disponibles dans l'édition Community.
POST/PUT/DELETE, les en-têtes d'authentification personnalisés (Basic,
Bearer) et le suivi des redirections nécessitent Pro. Le corps reprend
par défaut le message de l'onglet actif mais peut être remplacé.</p>

<h3>Historique</h3>
<p>Chaque envoi et chaque réception sont journalisés (hôte, port, taille,
code de réponse, temps aller-retour). Les 100 dernières entrées sont
conservées entre les redémarrages ; cliquez sur une ligne pour voir la
requête et la réponse complètes.</p>

<h3>Profils de connexion</h3>
<p>Enregistrez les endpoints fréquemment utilisés comme profils nommés
depuis la ligne <strong>Profil</strong> : saisissez un nom et cliquez
sur <em>Enregistrer</em>. Les profils MLLP stockent hôte, port, délai
d'expiration et ACK automatique ; les profils HTTP stockent URL,
en-têtes et délai d'expiration. Sélectionner un profil l'applique au
formulaire ; enregistrer sous un nom existant l'écrase ;
<em>Supprimer</em> retire le profil sélectionné. Les profils sont
stockés dans la base de données locale et survivent aux redémarrages.</p>
`,
};

export const anonymizationSection: ManualSection = {
	id: 'anonymization',
	heading: 'Anonymisation et export',
	body: `
<p><strong>Outils → Anonymiser</strong> détecte les champs PHI dans les
segments d'identification du patient les plus courants (PID, NK1, IN1,
GT1) et les masque selon leur niveau de sensibilité.</p>

<table>
	<tr><th>Niveau</th><th>Exemple</th><th>Stratégie</th></tr>
	<tr><td><strong>Élevé</strong></td><td>Nom du patient, SSN, MRN</td>
		<td>Le texte devient <code>REDACTED</code> ; le numérique devient
		des zéros de même longueur (la largeur du champ est préservée
		pour les parseurs en aval).</td></tr>
	<tr><td><strong>Moyen</strong></td><td>Nom de jeune fille de la mère, téléphone</td>
		<td>Premier caractère conservé, le reste remplacé par
		<code>***</code>.</td></tr>
	<tr><td><strong>Faible</strong></td><td>Alias, identifiants à faible risque</td>
		<td>3 premiers caractères conservés, le reste remplacé par
		<code>...</code>.</td></tr>
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
reçoivent des suffixes numériques au lieu de s'écraser mutuellement. Une
ligne par fichier indique le nombre de champs PHI masqués ou l'erreur ;
les mêmes plafonds de 5000 fichiers / 10&nbsp;Mo que la validation par
lots s'appliquent.</p>

<h3>Export</h3>
<p>Les utilisateurs Pro peuvent exporter le message structuré en JSON ou
CSV via <strong>Outils → Exporter JSON / CSV</strong>. Utile pour
charger des données HL7 dans des outils d'analyse (Power BI, Excel,
pandas).</p>

<div class="warn">L'anonymisation remplace les valeurs <em>dans
l'éditeur</em>. Conservez toujours votre fichier source original comme
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
(<code>ADT</code> couvre tout événement ADT, <code>ADT^A01</code> est
exact) et un <strong>résultat de validation attendu</strong> (valide /
invalide). C'est ce qui transforme un simple extrait en test.</p>

<h3>Exécuter les vérifications</h3>
<p><strong>Vérifier</strong> analyse et valide réellement un cas — HL7 v2
ou FHIR, détecté automatiquement — et compare le résultat à ses
attentes. <strong>Tout exécuter</strong> fait de même pour chaque cas
correspondant à la recherche courante, avec un badge réussite/échec par
ligne et un récapitulatif réussis/total dans la barre d'outils. Après
une évolution d'interface, un seul clic vous dit lesquels de vos
messages de référence ont cassé. Modifier un cas efface son résultat
mémorisé jusqu'à la prochaine exécution.</p>

<h3>Restauration de session</h3>
<p>BridgeLab sauvegarde vos onglets ouverts (y compris les modifications
non enregistrées) et les rouvre au lancement suivant, à la manière de
Notepad++. Contrôlez ce comportement dans
<strong>Paramètres → Session</strong> : activez/désactivez <em>Restaurer
les onglets ouverts au démarrage</em>, ou utilisez <em>Effacer la
session enregistrée</em> pour purger le jeu d'onglets stocké (cela
désactive aussi la restauration, si bien que le prochain lancement
démarre sur l'écran d'accueil).</p>
`,
};
