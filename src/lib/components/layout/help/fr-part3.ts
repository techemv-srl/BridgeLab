import type { ManualSection } from '../helpContent';

export const schemaExportSection: ManualSection = {
	id: 'schema-export',
	heading: 'Export de schéma (XSD)',
	body: `
<p>Besoin d'un XSD décrivant un message HL7 v2 pour un pipeline basé sur
XML, une intégration contract-first, ou simplement à charger dans un
outil tiers ? Ouvrez <strong>Outils → Exporter le schéma du message au
format XSD…</strong> — choisissez une version HL7 et un type de message,
prévisualisez le schéma généré et enregistrez-le en un clic.</p>

<h3>Ce que vous obtenez</h3>
<p>Un XSD autonome suivant la convention d'encodage standard HL7
v2.xml :</p>
<ul>
	<li>Un élément racine par message (p. ex. <code>ADT_A01</code>) avec
		un type complexe inline listant les segments et groupes de
		segments dans l'ordre.</li>
	<li>Chaque segment déclaré comme <code>xsd:complexType</code> de
		premier niveau (<code>MSH</code>, <code>PID</code>, <code>OBX</code>, …)
		avec chaque champ typé selon la référence de type de données HL7
		(<code>XPN</code>, <code>CX</code>, <code>HD</code>, …).</li>
	<li>Les types de données composites développés en leurs composants,
		les types primitifs (<code>ST</code>, <code>ID</code>, <code>NM</code>, …)
		émis comme restrictions <code>xsd:simpleType</code> de
		<code>xsd:string</code>.</li>
	<li>Cardinalité préservée : <code>minOccurs="0"</code> pour les
		champs optionnels, <code>maxOccurs="unbounded"</code> pour les
		champs répétables.</li>
	<li>Les groupes comme <code>ORM_O01.ORDER_DETAIL</code> rendus selon
		la convention de nommage <code>MESSAGE.GROUP</code> ; les blocs de
		choix définis par HL7 (<code>OBR | RQD | RQ1 | RXO | ODS | ODT</code>)
		émis comme <code>xsd:choice</code>.</li>
	<li>Compilation garantie sous les processeurs de schéma stricts — les
		rares structures HL7 dont la définition viole la règle Unique
		Particle Attribution de XSD sont émises comme un choix assoupli et
		annoté.</li>
</ul>

<h3>Actions</h3>
<ul>
	<li><strong>Copier</strong> — copie le XSD dans le presse-papiers,
		pratique pour le coller dans un éditeur ou un chat.</li>
	<li><strong>Enregistrer sous…</strong> — ouvre la boîte de dialogue
		de fichiers du système avec <code>{MESSAGE}.xsd</code> comme nom
		par défaut.</li>
</ul>

<h3>Couverture et éditions</h3>
<p>Dix versions HL7 sont livrées complètes : <strong>2.1, 2.2, 2.3,
2.3.1, 2.4, 2.5, 2.5.1, 2.6, 2.7 et 2.7.1</strong> — 2&nbsp;320
structures de message sélectionnables dans la liste déroulante des
versions.</p>
<p>HL7 v2.7.1 est une version de correction technique de la 2.7 et
reprend les mêmes définitions de message : elle est donc marquée
<strong>(= v2.7)</strong> dans la liste et exporte depuis le catalogue
2.7.</p>
<p>L'édition gratuite exporte quatre types de message très utilisés en
HL7 v2.5, couvrant entièrement le workflow typique de débogage MLLP :</p>
<ul>
	<li><strong>ADT^A01</strong> — Admit / Visit Notification</li>
	<li><strong>ADT^A40</strong> — Merge Patient (Patient Identifier
		List)</li>
	<li><strong>ORM^O01</strong> — Order Message</li>
	<li><strong>ORU^R01</strong> — Unsolicited Observation Result</li>
</ul>
<p>Tout autre type de message, ou toute autre version HL7, est marqué
<strong>(PRO)</strong> dans la liste déroulante et nécessite une licence
Professional (ou un essai actif). Si vous tentez d'exporter une entrée
réservée, BridgeLab affiche une invite de mise à niveau pointant vers
<strong>Aide → Acheter une licence</strong> (la page des tarifs) ou
<strong>Aide → Activer la licence</strong>.</p>

<h3>Note sur les licences</h3>
<p>BridgeLab ne redistribue aucun fichier XSD couvert par le copyright
HL7. Les métadonnées de schéma sont reconstruites à partir des
spécifications publiques HL7 v2 ; chaque fichier généré porte un en-tête
reconnaissant HL7® comme standard source et signalant le résultat comme
œuvre dérivée à des fins d'interopérabilité.</p>

<div class="info">Cible idéale : Astraia et les applications
d'intégration similaires qui acceptent des définitions XSD écrites à la
main pour les types de message que le moteur ne connaît pas nativement.
Exportez une fois, déposez dans le moteur, passez à la suite.</div>
`,
};

export const fhirSection: ManualSection = {
	id: 'fhir',
	heading: 'Prise en charge FHIR',
	body: `
<p>BridgeLab détecte automatiquement les ressources FHIR lorsque vous
collez ou ouvrez un fichier dont le premier caractère non blanc est
<code>{</code> et qui contient <code>"resourceType"</code>. L'arbre
bascule vers une vue spécifique FHIR montrant la hiérarchie de la
ressource sous forme de chemins JSON.</p>

<h3>Formats pris en charge</h3>
<ul>
	<li><strong>JSON</strong> - Patient, Observation, Bundle, DiagnosticReport,
		MedicationRequest et toute autre ressource FHIR R4/R5.</li>
	<li><strong>XML</strong> - les mêmes ressources en encodage XML
		(<code>&lt;Patient xmlns="http://hl7.org/fhir"&gt;</code>).</li>
</ul>

<h3>Visualiseur de Bundle (Pro)</h3>
<p><strong>Outils → Visualiseur de Bundle FHIR</strong> ouvre une vue à
trois panneaux lorsque le message actif est un Bundle :</p>
<ul>
	<li><strong>Panneau gauche :</strong> liste des entrées avec type de
		ressource, nom d'affichage (p. ex. nom du Patient, code de
		l'Observation) et compteur de références entrantes.</li>
	<li><strong>Panneau central :</strong> références sortantes de
		l'entrée sélectionnée - chaque champ <code>reference</code>
		devient un lien cliquable qui mène à l'entrée cible.</li>
	<li><strong>Panneau droit :</strong> le JSON brut de la ressource
		sélectionnée, avec coloration syntaxique.</li>
</ul>
<p>Les <strong>références pendantes</strong> (pointant vers des entrées
absentes du Bundle) sont signalées par un badge rouge.</p>
<p>Le bouton <strong>Liste / Graphe</strong> bascule vers un graphe de
références : chaque entrée est un nœud (coloré selon le type de
ressource), chaque <code>reference</code> une flèche orientée. Cliquez
sur un nœud pour le sélectionner — le panneau de détail suit. Disponible
jusqu'à 150 entrées ; les bundles plus grands utilisent la liste.</p>

<h3>Évaluateur FHIRPath (Pro)</h3>
<p><kbd>Ctrl</kbd>+<kbd>P</kbd> ou <strong>Outils → Évaluateur FHIRPath</strong>
ouvre une console interactive où vous saisissez des expressions FHIRPath
sur la ressource courante.</p>
<p>L'évaluateur implémente le langage FHIRPath 2.0 : l'ensemble complet
des opérateurs avec la précédence et la logique ternaire de la
spécification, et environ soixante-dix fonctions.</p>
<ul>
	<li><strong>Navigation :</strong> <code>Patient.name.family</code>,
		<code>Bundle.entry.resource</code> ; les éléments de choix
		s'atteignent par leur nom de base —
		<code>Observation.value</code> trouve
		<code>valueQuantity</code></li>
	<li><strong>Indexation :</strong> <code>Patient.name[0].given</code></li>
	<li><strong>Filtres et projection :</strong> <code>where()</code>,
		<code>select()</code>, <code>repeat()</code>,
		<code>ofType()</code>, avec <code>$this</code> et
		<code>$index</code> disponibles à l'intérieur</li>
	<li><strong>Collections :</strong> <code>count()</code>,
		<code>first()</code>, <code>last()</code>, <code>tail()</code>,
		<code>skip()</code>, <code>take()</code>, <code>distinct()</code>,
		<code>sort()</code>, <code>union()</code>, <code>combine()</code>,
		<code>intersect()</code>, <code>exclude()</code>,
		<code>aggregate()</code></li>
	<li><strong>Logique :</strong> <code>and</code>, <code>or</code>,
		<code>xor</code>, <code>implies</code>, <code>not()</code>,
		<code>exists()</code>, <code>all()</code>, <code>iif()</code> —
		la collection vide jouant le rôle de valeur « inconnue »</li>
	<li><strong>Chaînes :</strong> <code>substring()</code>,
		<code>matches()</code>, <code>replace()</code>,
		<code>split()</code>, <code>join()</code>, <code>encode()</code>,
		<code>escape()</code> et apparentées</li>
	<li><strong>Dates et quantités :</strong> littéraux à précision
		partielle (<code>@2015</code>,
		<code>@2015-02-04T14:34:28+10:00</code>), arithmétique de durées
		(<code>Patient.birthDate + 18 years</code>) et conversion d'unités
		au sein d'une même dimension
		(<code>4 'g' = 4000 'mg'</code>)</li>
	<li><strong>Extensions FHIR :</strong> <code>extension(url)</code>,
		<code>hasValue()</code> et <code>resolve()</code>, qui suit une
		Reference vers une ressource contained ou du Bundle</li>
	<li><strong>Débogage :</strong> <code>trace('libellé')</code> laisse
		passer son entrée et affiche les valeurs sous le résultat, pour
		voir ce qu'un chemin long produit à mi-parcours</li>
</ul>
<p>Comparer des valeurs de précision différente renvoie la collection
vide plutôt qu'une supposition :
<code>@2015-02-04 = @2015-02</code> n'est ni vrai ni faux, car la
seconde valeur pourrait être ce jour-là ou un autre du même mois.</p>
<p>Les expressions récentes sont conservées dans une liste d'historique
pour les rejouer rapidement.</p>

<h3>Validation FHIR</h3>
<p>F6 fonctionne aussi pour les ressources FHIR. Les erreurs signalent
les champs obligatoires manquants (p. ex. <code>Patient.identifier</code>),
les types de données invalides (genre hors du value set) et les
problèmes structurels. Les URL canoniques déclarées dans
<code>meta.profile</code> sont listées comme constats d'information (la
conformité au profil elle-même n'est pas vérifiée) ; les entrées
malformées sont signalées comme avertissements.</p>

<h3>Validation de profils (Pro)</h3>
<p>Par défaut, une ressource FHIR est contrôlée structurellement : le
<code>resourceType</code> est-il là, les règles propres à la ressource
tiennent-elles. La confronter à une <strong>StructureDefinition</strong> —
la définition réelle de ce qu'un Patient peut contenir — demande ces
définitions, distribuées sous forme de packages FHIR NPM.</p>
<p>Le noyau FHIR R4 (<code>hl7.fhir.r4.core</code> 4.0.1) est
<strong>intégré</strong> : les définitions de base sont toujours là — hors
ligne, dans toutes les éditions, rien à télécharger. <strong>Outils →
Packages de profils FHIR…</strong> installe par-dessus, depuis un
<code>.tgz</code>, les guides d'implémentation nationaux ou propres à votre
site ; un package installé portant les mêmes définitions remplace celles
intégrées, une version plus récente les supplante.</p>
<p>Chaque validation FHIR contrôle, contre le noyau intégré et ce qui est
installé par-dessus :</p>
<ul>
	<li><strong>Cardinalité</strong> — un élément requis absent, ou un
		élément <code>0..1</code> qui se répète.</li>
	<li><strong>Types d'éléments</strong> — un booléen écrit comme une
		chaîne, un nombre là où un objet est attendu.</li>
	<li><strong>Éléments de choix</strong> — <code>value[x]</code> doit
		apparaître sous exactement une des formes
		<code>valueQuantity</code>, <code>valueString</code>, etc. Un
		<code>value</code> nu, ou deux formes à la fois, sont signalés.</li>
	<li><strong>Valeurs fixes et motifs</strong> — ce que le profil
		impose.</li>
	<li><strong>Éléments inconnus</strong> — un nom que le profil ne définit
		pas. C'est le contrôle qui attrape une faute comme
		<code>genderr</code> ou un élément appartenant à un autre type de
		ressource.</li>
</ul>
<p>Les profils déclarés dans <code>meta.profile</code> sont appliqués
automatiquement si le package qui les définit est installé. Sinon, le
validateur le dit plutôt que de rapporter discrètement un résultat propre —
et lorsque des profils ont bien tourné, il le dit aussi, car « aucun
constat » n'a pas du tout le même sens dans les deux cas.</p>
<p><strong>La terminologie est hors périmètre.</strong> Une liaison
<code>required</code> ne se vérifie qu'en développant le ValueSet, ce qui
suppose les packages de terminologie ou un serveur. BridgeLab laisse les
liaisons non vérifiées plutôt que de les vérifier à moitié.</p>
<p>Installer un package nécessite une licence Professional. Les packages
déjà installés continuent de valider dans toutes les éditions : un essai
expiré ne fait jamais rougir des ressources auparavant propres.</p>

<h3>Règles FHIR personnalisées (Pro)</h3>
<p><strong>Outils → Règles de validation FHIR…</strong> ouvre un éditeur
pour vos propres contrôles. Ils s'exécutent avec les contrôles intégrés à
chaque validation et sont enregistrés comme un plugin pack ordinaire
(<code>plugins/fhir/user-rules.json</code>) que vous pouvez copier d'une
machine à l'autre ou versionner.</p>
<p>Une règle prend l'une de deux formes :</p>
<ul>
	<li><strong>L'expression doit être vraie</strong> — un invariant
		FHIRPath, comme FHIR écrit ses propres contraintes :
		<code>identifier.exists()</code>, ou
		<code>value.exists() xor dataAbsentReason.exists()</code>.</li>
	<li><strong>Chemin + contrôle</strong> — un sélecteur FHIRPath et une
		assertion sur les valeurs obtenues : doit être présent, combien,
		correspond à un motif, parmi une liste, contient un texte, ou une
		borne de longueur.</li>
</ul>
<p>Renseignez <strong>Type de ressource</strong> pour limiter la règle à
Patient, Observation, etc., ou laissez vide pour l'appliquer partout. Une
règle limitée à un type se déclenche aussi pour les ressources
correspondantes d'un Bundle, le constat étant rapporté sur
<code>entry[n].resource.…</code>.</p>
<p><strong>Tester sur la ressource ouverte</strong> exécute la règle en
cours d'édition sur la ressource de l'onglet actif avant enregistrement, et
montre les valeurs réellement sélectionnées — le moyen le plus rapide de
distinguer une règle qui passe d'une règle qui ne s'est jamais exécutée. Si
la ressource ouverte est d'un autre type, l'éditeur le signale au lieu
d'annoncer un succès.</p>
<p>Les règles déjà écrites fonctionnent dans toutes les éditions ;
l'éditeur nécessite une licence Professional. Les packs écrits à la main
sont documentés dans <code>docs/PLUGINS.md</code>.</p>

<h3>Modèles FHIR</h3>
<p><strong>Fichier → Nouveau depuis un modèle</strong> inclut une
catégorie FHIR : un Patient minimal, une Observation de pression
artérielle avec composants, et un Bundle transaction dont les entrées se
référencent mutuellement via <code>urn:uuid</code> — ouvrez-le et
essayez la vue graphe du visualiseur de Bundle.</p>
`,
};

export const pluginsSection: ManualSection = {
	id: 'plugins',
	heading: 'Packs de plugins',
	body: `
<p>Les packs de plugins vous permettent d'étendre le validateur et
l'anonymiseur de BridgeLab <strong>sans écrire de code</strong> et sans
autoriser la moindre exécution de code. Chaque pack est un fichier JSON
déposé dans un dossier utilisateur.</p>

<h3>Où vivent les plugins</h3>
<p>Cliquez sur <strong>Paramètres → Plugins → Ouvrir le dossier plugins</strong>
pour révéler le répertoire dans votre gestionnaire de fichiers.
L'organisation est la suivante :</p>
<pre><code>&lt;config&gt;/BridgeLab/plugins/
├── validation/
│   ├── hospital-adt-rules.json
│   └── z-segment-checks.json
├── fhir/
│   └── user-rules.json
└── anonymization/
    └── eu-national-id.json</code></pre>

<p>Sous Windows la racine est <code>%APPDATA%\\BridgeLab\\plugins</code>,
sous macOS <code>~/Library/Application Support/BridgeLab/plugins</code>,
sous Linux <code>~/.config/BridgeLab/plugins</code>.</p>

<p>Trois sortes de packs y vivent : les règles de validation HL7 v2
(<code>validation/</code>, ci-dessous), les règles de validation FHIR
(<code>fhir/</code> — un invariant FHIRPath, ou un sélecteur plus une
vérification ; voir <em>Support FHIR → Règles FHIR
personnalisées</em>) et les champs PHI supplémentaires de l'anonymiseur
(<code>anonymization/</code>, ci-dessous).</p>

<p><strong>Chaque édition dispose de tout le mécanisme</strong> — les trois
sortes, chaque type de vérification, le rechargement et l'activation pack
par pack. La seule différence est le nombre de packs actifs en même temps :
jusqu'à <strong>3</strong> en Community, illimité en Pro et Enterprise.
L'éditeur intégré qui écrit les packs FHIR est Pro ; un pack FHIR écrit à
la main s'exécute en Community comme n'importe quel autre.</p>

<h3>Pack de règles de validation</h3>
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

<h3>Types de contrôle pris en charge</h3>
<table>
	<tr><th>Contrôle</th><th>Paramètres</th><th>Exemple d'usage</th></tr>
	<tr><td><code>not_empty</code></td><td>—</td>
		<td>Le champ doit être renseigné.</td></tr>
	<tr><td><code>regex</code></td><td><code>pattern</code></td>
		<td>Le nom de famille doit commencer par une majuscule.</td></tr>
	<tr><td><code>one_of</code></td><td><code>values[]</code></td>
		<td>La classe patient doit être I, O, E.</td></tr>
	<tr><td><code>max_length</code></td><td><code>max</code></td>
		<td>MRN ≤ 16 caractères.</td></tr>
	<tr><td><code>min_length</code></td><td><code>min</code></td>
		<td>SSN ≥ 9 chiffres.</td></tr>
	<tr><td><code>contains</code></td><td><code>value</code></td>
		<td>Le numéro de visite doit contenir un tiret.</td></tr>
</table>
<p>Ajoutez <code>"component": 1</code> pour restreindre une règle à un
composant précis (p. ex. le nom de famille dans PID-5.1).</p>

<h3>Pack de règles d'anonymisation</h3>
<pre><code>{
  "id": "eu-extra-phi",
  "name": "EU extra PHI fields",
  "enabled": true,
  "phi_rules": [
    { "segment": "PID", "field": 25, "sensitivity": "high",
      "name": "EU National ID" }
  ]
}</code></pre>

<h3>Gérer les packs</h3>
<p><strong>Paramètres → Plugins</strong> liste chaque pack avec son
auteur, sa version, son nombre de règles et son chemin. Activez ou
désactivez les packs individuellement (le choix est persisté), cliquez
sur <em>Recharger</em> après avoir modifié un fichier, ou sur <em>Ouvrir
le dossier plugins</em> pour éditer dans votre IDE favori.</p>

<div class="note">Les fichiers dont l'analyse échoue apparaissent avec un
bandeau d'erreur rouge mais ne cassent pas le registre - le reste de vos
packs continue de fonctionner.</div>

<p class="note">Dans l'édition Community, jusqu'à <strong>3 packs</strong>
sont actifs à la fois : les packs activés au-delà affichent un badge
« inactif » et n'apportent aucune règle tant qu'un emplacement ne se
libère pas (désactivez un autre pack, ou passez à l'édition
supérieure).</p>
`,
};

export const licensingSection: ManualSection = {
	id: 'licensing',
	heading: 'Licences',
	body: `
<p>BridgeLab est proposé en trois éditions. La répartition des
fonctionnalités est conçue pour que les utilisateurs Community puissent
faire du vrai travail HL7 au quotidien, indéfiniment, tandis que Pro et
Enterprise débloquent les fonctions dont ont besoin les équipes
d'intégration et les hôpitaux.</p>

<table>
	<tr><th>Fonctionnalité</th><th>Community</th><th>Pro</th><th>Enterprise</th></tr>
	<tr><td>Éditeur HL7 v2.x, analyseur, validation</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Analyse FHIR + arborescence</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Envoi MLLP, HTTP GET sans en-tête d'authentification</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Conformité au noyau FHIR R4 (intégré)</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Export XSD</td>
		<td>4 messages v2.5</td><td>Catalogue complet</td><td>Catalogue complet</td></tr>
	<tr><td>Détection PHI (visualisation seule)</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Packs de plugins (toutes sortes, toutes vérifications)</td>
		<td>3 actifs à la fois</td><td>Illimités</td><td>Illimités</td></tr>
	<tr><td>Listener MLLP</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>HTTP POST/PUT/DELETE/PATCH, et authentification sur toute méthode</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Masquage d'anonymisation</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Export JSON/CSV</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Packages de profils FHIR et éditeur de règles</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Évaluateur FHIRPath + Visualiseur de Bundle</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Cas de test enregistrés</td>
		<td>10</td><td>Illimités</td><td>Illimités</td></tr>
	<tr><td>SOAP + support prioritaire</td>
		<td>—</td><td>—</td><td>✓</td></tr>
</table>

<p class="note">Community conserve jusqu'à <strong>3 packs de plugins
actifs</strong> et <strong>10 cas de test enregistrés</strong>. Rien
n'est jamais verrouillé ni supprimé : les éléments enregistrés au-delà
du plafond (p. ex. pendant un essai) restent visibles, modifiables et
exécutables — seuls les nouveaux enregistrements et les nouvelles
activations au-delà de la limite demandent une mise à niveau, et libérer
un emplacement les réactive immédiatement.</p>

<h3>Essai</h3>
<p>Le premier lancement démarre un <strong>essai Pro de 14 jours</strong>
avec toutes les fonctionnalités Pro activées. Le bandeau d'essai (jaune)
peut être masqué ; quand il ne reste que 3 jours, il devient rouge et
reste affiché en guise de rappel.</p>

<p>À l'expiration de l'essai, BridgeLab <strong>ne cesse pas de
fonctionner</strong> - il revient à l'édition Community et le bandeau
vous invite à passer à l'édition supérieure. Vos messages, paramètres,
plugins et cas de test restent intacts.</p>

<h3>Mises à jour</h3>
<p>Rien n'est demandé tant que vous n'avez pas choisi. Au premier
démarrage, BridgeLab demande, dans un bandeau en haut de la fenêtre, s'il
peut rechercher les nouvelles versions (l'installateur Windows pose la
question pendant l'installation, et l'application ne la repose pas).
<em>Oui, vérifier</em> et <em>Non</em> sont enregistrés dans Paramètres →
Confidentialité ; fermer le bandeau sans répondre repose la question au
démarrage suivant.</p>
<p>Une fois par jour, quelques secondes après le démarrage, BridgeLab
demande à GitHub (<code>api.github.com</code>) la dernière version. Si une
version plus récente existe, un bandeau l'indique avec
<em>Télécharger</em> (ouvre la page de la version), <em>Ignorer cette
version</em> et un bouton de fermeture ; rien n'est installé
automatiquement. La requête ne contient rien sur vous, l'ordinateur ou vos
fichiers, et sans accès à Internet elle est ignorée silencieusement.
Désactivez-la dans <strong>Paramètres → Confidentialité → Rechercher les
nouvelles versions au démarrage</strong> ; <strong>Aide → Vérifier les
mises à jour</strong> fonctionne toujours à la demande.</p>
<p>L'installateur Windows pose la même question à la première
installation (<em>Oui</em> par défaut ; une installation silencieuse ne
demande rien). Sur les postes gérés, l'administrateur peut désactiver la
vérification pour tous les utilisateurs, et la case des Paramètres apparaît
verrouillée : variable d'environnement
<code>BRIDGELAB_DISABLE_UPDATE_CHECK=1</code>, ou fichier
<code>policy.json</code> contenant <code>{"disable_update_check": true}</code>
dans <code>%ProgramData%\\BridgeLab\\</code> (Windows),
<code>/Library/Application Support/BridgeLab/</code> (macOS) ou
<code>/etc/bridgelab/</code> (Linux).</p>

<h3>Acheter une licence</h3>
<p><strong>Aide → Acheter une licence…</strong> ouvre dans votre
navigateur la section tarifs du site BridgeLab, où Professional et
Enterprise s'achètent en ligne par carte ; le code d'activation arrive
par e-mail. La même page est à un clic des boutons <em>Tarifs et
achat</em> de la boîte d'activation, du bouton <em>Comparer les
offres</em> du bandeau d'essai et du bouton <em>Voir les tarifs</em> de
chaque message « nécessite une licence Professional ». Besoin d'une
facture, d'un bon de commande ou d'un devis ? Écrivez à
<a href="mailto:info@techemv.it">info@techemv.it</a>.</p>

<h3>Activation</h3>
<p>Ouvrez la boîte de dialogue d'activation depuis :</p>
<ul>
	<li><strong>Paramètres → Licence → Activer</strong></li>
	<li><strong>Aide → Activer la licence</strong></li>
	<li>Le bouton <em>Mettre à niveau</em> du bandeau d'essai</li>
</ul>

<p><strong>Activation en ligne (par défaut) :</strong> après l'achat,
vous recevez par e-mail un code d'activation du type
<code>BL-PRO-XXXX-XXXX-XXXX</code>. Collez-le dans le champ de clé :
l'application l'échange en un seul appel HTTPS contre une licence
signée liée à cette machine. Utilisez <em>Désactiver</em> pour libérer
le poste avant de passer à un autre ordinateur.</p>

<p>Abonnement renouvel&eacute; ? Cliquez sur <em>Mettre &agrave; jour la
licence</em> dans la bo&icirc;te de dialogue pour r&eacute;cup&eacute;rer
imm&eacute;diatement la nouvelle &eacute;ch&eacute;ance — ou ne faites
rien : dans les 14 jours pr&eacute;c&eacute;dant l'expiration,
l'application la r&eacute;cup&egrave;re seule au d&eacute;marrage, en
silence (jamais d'erreurs sur les machines hors ligne).</p>

<p><strong>Clé hors ligne (sites isolés / air-gapped) :</strong>
écrivez à <a href="mailto:info@techemv.it">info@techemv.it</a> en
indiquant votre <strong>ID Matériel</strong> (affiché sous « Besoin
d'une clé hors ligne ? » dans la boîte de dialogue d'activation,
également visible sous Paramètres → Licence). TECHEMV SRL vous renvoie
une licence signée liée à votre machine — aucun accès internet n'est
jamais nécessaire. La boîte de dialogue prévisualise le nom du
titulaire et les droits associés avant l'activation.</p>

<h3>Vérification hors ligne</h3>
<p>Quel que soit le flux utilisé, la vérification ordinaire de la
licence est purement locale - l'application n'a jamais besoin de
contacter le serveur de licences pour continuer à fonctionner. Les
appels au serveur n'ont lieu que lorsque vous les déclenchez
explicitement : activation par code, libération du poste via
<em>Désactiver</em>, ou statistiques d'utilisation opt-in. La clé
porte une signature Ed25519 que l'application vérifie contre une clé
publique embarquée.</p>

<h3>Confidentialité et statistiques d'utilisation</h3>
<p>BridgeLab peut envoyer des <strong>statistiques
d'utilisation</strong> à TECHEMV — désactivées par défaut, activables
sous <strong>Paramètres → Confidentialité</strong>. Une fois activées,
l'envoi automatique a lieu au plus une fois par jour ; le bouton
<em>Envoyer maintenant</em> transmet immédiatement. Chaque rapport
contient des compteurs d'utilisation, la version de l'application, le
système d'exploitation, le niveau de licence, un <strong>ID
d'installation aléatoire</strong> et — uniquement pour les licences
activées en ligne — le <strong>code d'activation</strong> (utilisé
pour signaler une licence révoquée). Les données sont donc
<strong>pseudonymes</strong> et non totalement anonymes : aucun
contenu de message, nom de fichier, nom d'hôte, nom d'utilisateur ni
donnée patient n'est jamais envoyé, et le JSON exact peut être
inspecté via <em>Voir ce qui est envoyé</em>. Interrupteur désactivé
(le défaut), rien n'est transmis, et les problèmes réseau ne
produisent jamais d'erreurs : une installation totalement hors ligne
est un scénario normal et pris en charge.</p>
<p>Sur les postes gérés, l'administrateur peut forcer la désactivation
des statistiques d'utilisation pour tous les utilisateurs : définissez
<code>BRIDGELAB_DISABLE_TELEMETRY=1</code>, ou ajoutez
<code>"disable_telemetry": true</code> au même <code>policy.json</code>
que pour la vérification des versions. Rien n'est alors envoyé, quel que
soit le choix de l'utilisateur, et la case des Paramètres apparaît
verrouillée.</p>
`,
};

export const shortcutsSection: ManualSection = {
	id: 'shortcuts',
	heading: 'Raccourcis clavier',
	body: `
<p>Les raccourcis de BridgeLab sont configurables sous
<strong>Paramètres → Raccourcis Clavier</strong>. Cliquez sur une
association, appuyez sur une nouvelle combinaison de touches, confirmez
avec OK.</p>

<h3>Valeurs par défaut</h3>
<table>
	<tr><td><kbd>Ctrl</kbd>+<kbd>O</kbd></td><td>Ouvrir un fichier</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>N</kbd></td><td>Nouveau depuis un modèle</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>L</kbd></td><td>Bibliothèque de cas de test</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>S</kbd></td><td>Enregistrer</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>S</kbd></td><td>Enregistrer sous</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>W</kbd></td><td>Fermer l'onglet</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>B</kbd></td><td>Afficher/masquer l'arbre</td></tr>
	<tr><td><kbd>F5</kbd></td><td>Ré-analyser le message</td></tr>
	<tr><td><kbd>F6</kbd></td><td>Valider</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>K</kbd></td><td>Panneau de communication</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>P</kbd></td><td>Panneau FHIRPath</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>,</kbd></td><td>Paramètres</td></tr>
	<tr><td><kbd>F1</kbd></td><td>Ce manuel utilisateur</td></tr>
	<tr><td><kbd>Alt</kbd>+<kbd>T</kbd></td><td>Afficher le segment dans l'arbre (menu contextuel de l'éditeur)</td></tr>
	<tr><td><kbd>Alt</kbd>+<kbd>C</kbd></td><td>Copier le segment (menu contextuel de l'éditeur)</td></tr>
</table>

<h3>Détection des conflits</h3>
<p>Si vous choisissez une combinaison déjà assignée à une autre action,
l'éditeur vous avertit - confirmez pour transférer l'association, ou
choisissez une autre touche. Les raccourcis propres à Monaco
(<kbd>Ctrl</kbd>+<kbd>F</kbd>, <kbd>Ctrl</kbd>+<kbd>D</kbd>, ...) ont la
priorité lorsque l'éditeur a le focus.</p>

<h3>Réinitialisation</h3>
<p>Cliquez sur <em>Tout réinitialiser</em> pour restaurer chaque
raccourci à sa valeur par défaut, ou sur le petit bouton ↺ à côté d'une
entrée pour ne réinitialiser que celle-là.</p>
`,
};
