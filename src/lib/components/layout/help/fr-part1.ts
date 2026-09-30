import type { ManualSection } from '../helpContent';
import { mockupAppShellFr as mockupAppShell, mockupContextMenuFr as mockupContextMenu } from './mockups';

export const getStarted: ManualSection = {
	id: 'getting-started',
	heading: 'Premiers pas',
	body: `
<p>BridgeLab est un éditeur de messages moderne pour HL7 v2.x et FHIR,
conçu pour les ingénieurs d'intégration en santé. Il repose sur un
backend Rust pour une analyse rapide (un message de 10 Mo avec une pièce
jointe base64 s'ouvre en 2 secondes environ) et un frontend Svelte 5 avec l'éditeur Monaco.</p>

<p>La fenêtre principale est divisée en quatre zones :</p>
${mockupAppShell}

<ol>
	<li><strong>Barre de menus et bandeau d'essai</strong> en haut - les
		menus Fichier, Édition, Affichage, Outils et Aide, plus un bandeau
		jaune/rouge qui rappelle l'état de l'essai Pro.</li>
	<li><strong>Panneau de l'arbre</strong> à gauche - la structure du
		message analysé avec des flèches développer/réduire, et un
		Inspecteur de champ en bas affichant les informations du schéma
		HL7 pour le nœud sélectionné.</li>
	<li><strong>Éditeur et onglets</strong> au centre - éditeur Monaco
		avec coloration syntaxique HL7 ; barre multi-onglets pour garder
		plusieurs messages ouverts à la fois.</li>
	<li><strong>Barre d'état</strong> en bas - type de message, version,
		nombre de segments, position du curseur.</li>
</ol>

<h3>Ouvrir un message</h3>
<ul>
	<li><strong>Fichier → Ouvrir un fichier</strong> (<kbd>Ctrl</kbd>+<kbd>O</kbd>) -
		sélecteur de fichiers natif pour <code>.hl7</code>, <code>.txt</code>,
		<code>.msg</code>, <code>.json</code>, <code>.xml</code>.</li>
	<li><strong>Glisser-déposer</strong> - déposez un fichier sur la zone
		de l'éditeur.</li>
	<li><strong>Coller</strong> - cliquez dans l'éditeur et collez
		(<kbd>Ctrl</kbd>+<kbd>V</kbd>). L'analyse automatique se déclenche
		500 ms après la dernière frappe (délai, ou désactivation, dans
		<strong>Paramètres → Analyseur</strong>).</li>
	<li><strong>Ce qui s'ouvre :</strong> lignes vides, espaces, BOM ou
		encadrement MLLP avant <code>MSH</code>, fichiers UTF-16 (le
		« Unicode » du Bloc-notes) et fichiers de lot FHS/BHS sont lus tels
		quels. Un fichier que BridgeLab ne sait pas analyser s'ouvre quand
		même, en texte à corriger, avec une note qui explique pourquoi.</li>
	<li><strong>Fichier → Nouveau Message à partir d'un Modèle...</strong> (<kbd>Ctrl</kbd>+<kbd>N</kbd>) -
		modèles préremplis ADT, ORM, ORU, SIU et plus encore. Les champs
		comme MSH-7 et MSH-10 sont renseignés avec l'horodatage courant et
		un identifiant de message unique.</li>
	<li><strong>Fichier → Messages d'exemple</strong> (aussi depuis l'écran
		d'accueil) - des messages complets et réalistes plutôt que des
		squelettes : ADT d'admission, d'enregistrement, de mise à jour, de
		sortie et de fusion, ORU avec résultats (une numération de douze
		valeurs, un bilan métabolique), ORM, SIU, MDM, DFT, VXU et un ACK, en
		versions 2.3, 2.5 et 2.5.1. Filtrez par version, prévisualisez et
		ouvrez-en un dans un nouvel onglet. Chaque exemple passe la
		validation ; patients et données sont fictifs.</li>
</ul>

<div class="note">Au premier lancement vous bénéficiez d'un <strong>essai
Pro de 14 jours</strong> avec toutes les fonctionnalités Pro activées (SOAP
et le support prioritaire relèvent d'Enterprise). À
l'expiration, BridgeLab continue de fonctionner avec les fonctionnalités
Community - vous ne perdez jamais vos messages.</div>

<p>Les cartes <strong>Découvrir BridgeLab</strong> de l'écran d'accueil
ouvrent directement les fonctions qui distinguent BridgeLab — générateur de
messages de test, anonymisation PHI, écouteur MLLP et export XSD — avec des
badges PRO pour celles sous licence.</p>
`,
};

export const editorSection: ManualSection = {
	id: 'editor',
	heading: 'Éditeur',
	body: `
<p>La zone d'édition est une instance <strong>Monaco</strong> dotée d'une
grammaire spécifique à HL7. Les codes de segment sont colorés en violet,
les séparateurs de champ en gris, et les payloads ED/base64 et les autres valeurs
longues sont affichés repliés, pour garder l'éditeur réactif sur les gros
messages (voir plus bas).</p>

<h3>Auto-complétion et survol</h3>
<p>Commencez à taper <code>P</code> sur une nouvelle ligne - Monaco
suggère <code>PID</code>, <code>PV1</code>, <code>PV2</code>, etc. Une
fois le segment saisi, l'auto-complétion au pipe propose des valeurs de
champ (codes de sexe, codes ACK, classe patient...). Survoler un champ
affiche son nom, son type de données et son caractère obligatoire, tirés
du standard HL7.</p>

<h3>Champs longs repliés</h3>
<p>Les champs plus longs que le seuil de repli (100 caractères par défaut,
dans <strong>Paramètres → Analyseur</strong>) sont affichés
<em>repliés</em> : une pièce jointe base64 en OBX-5, une longue note ou une
chaîne JSON <code>data</code> apparaît comme une puce compacte, par
exemple <code>⟨Base64 · 5.1 KB⟩</code>, tandis que les séparateurs et les
autres composants restent visibles. Le repli n'est qu'un affichage : le
message reste toujours complet, et l'enregistrement, la session, la
validation, l'arbre et la copie utilisent le texte entier.</p>
<ul>
	<li><strong>Développer :</strong> cliquer sur la puce, ou placer le
		curseur à côté et appuyer sur <kbd>Alt</kbd>+<kbd>Entrée</kbd>. Le
		survol montre les premiers caractères.</li>
	<li><strong>Replier à nouveau :</strong> clic droit → <em>Replier ce
		champ</em> sur une valeur longue, ou <em>Replier tous les champs
		longs</em>.</li>
	<li><strong>Tout à la fois :</strong> le badge <em>N repliés</em> de
		la barre d'état développe tout ; le menu contextuel propose les
		deux commandes.</li>
	<li>Une puce se déplace d'un bloc : les flèches la sautent et
		Retour arrière/Suppr la sélectionnent d'abord, elle n'est donc
		jamais supprimée à moitié. Copier une sélection copie le contenu
		complet.</li>
	<li><strong>Rechercher et Remplacer</strong> (<kbd>Ctrl</kbd>+<kbd>F</kbd>,
		<kbd>Ctrl</kbd>+<kbd>H</kbd>) portent sur le texte complet : un champ
		replié qui contient une occurrence se déplie, et l'occurrence est
		comptée, affichée et remplacée comme les autres.</li>
</ul>

<h3>Paramètres de l'éditeur</h3>
<p><strong>Édition → Paramètres → Éditeur</strong> (Ctrl+,) change l'apparence
et le comportement de l'éditeur : police et taille, largeur de tabulation,
retour à la ligne, espaces visibles, minimap, numéros de ligne, défilement
fluide, couleurs des crochets, surlignage des autres occurrences du mot sous
le curseur, liens cliquables, en-tête fixe au défilement (sticky scroll) et
source des suggestions de mots (ce message, tous les messages ouverts ou
aucune ; les suggestions de champs et de valeurs HL7 fonctionnent dans tous
les modes). Les changements s'appliquent immédiatement. Chaque onglet garde
son propre historique d'annulation et de rétablissement (Ctrl+Z, Ctrl+Y)
quand vous passez d'un onglet à l'autre.</p>

<h3>Menu contextuel (clic droit)</h3>
${mockupContextMenu}
<p>Le menu regroupe les actions en trois sections :</p>
<ul>
	<li><strong>Navigation :</strong> Afficher le segment dans l'arbre
		(<kbd>Alt</kbd>+<kbd>T</kbd>) - ouvre l'arbre et met en évidence
		le champ exact sous le curseur ; Développer / Replier pour les
		valeurs longues.</li>
	<li><strong>Presse-papiers :</strong> Copier le segment
		(<kbd>Alt</kbd>+<kbd>C</kbd>), Copier le message complet (avec les
		champs développés), Copier le message tronqué (champs longs
		raccourcis, pour tenir dans un email ; les données du patient sont
		copiées telles quelles : anonymisez d'abord).</li>
</ul>

<div class="note">Les raccourcis natifs de Monaco (<kbd>Ctrl</kbd>+<kbd>F</kbd>
rechercher, <kbd>Ctrl</kbd>+<kbd>H</kbd> remplacer, <kbd>Ctrl</kbd>+<kbd>Z</kbd>
annuler, <kbd>Ctrl</kbd>+<kbd>D</kbd> multi-curseur) fonctionnent tous
comme prévu dans l'éditeur.</div>
`,
};

export const treeSection: ManualSection = {
	id: 'tree-view',
	heading: 'Arborescence et Inspecteur de champ',
	body: `
<p>L'arbre à gauche reflète la hiérarchie du message HL7 :
<strong>segments</strong> → <strong>champs</strong> →
<strong>composants</strong> ; un champ répété liste chaque répétition
(<code>PID-3(1)</code>, <code>PID-3(2)</code>) avec ses composants. Affichez-le ou masquez-le avec
<kbd>Ctrl</kbd>+<kbd>B</kbd> ou <strong>Affichage → Structure du
message</strong>.</p>

<h3>Naviguer entre l'arbre et l'éditeur</h3>
<ul>
	<li><strong>Éditeur → Arbre :</strong> clic droit sur un champ dans
		Monaco, puis <em>Afficher le segment dans l'arbre</em>. L'arbre
		développe le segment, sélectionne le champ exact (jusqu'au niveau
		du composant) et le fait défiler jusqu'à le rendre visible.</li>
	<li><strong>Arbre → Éditeur :</strong> clic droit sur un nœud de
		l'arbre, puis <em>Afficher dans l'éditeur</em>. Monaco saute à la
		ligne, place le curseur dans la bonne colonne et sélectionne la
		plage du champ.</li>
</ul>

<p>Les lignes vides entre les segments ne faussent pas les sauts. Dans
l'arbre, <kbd>↑</kbd>/<kbd>↓</kbd> déplacent la sélection, <kbd>→</kbd>
développe un nœud et <kbd>←</kbd> le replie ou remonte au parent ;
<kbd>Début</kbd>/<kbd>Fin</kbd> et <kbd>Pg préc</kbd>/<kbd>Pg suiv</kbd>
sautent. Pendant l'édition, l'arbre garde ce que vous avez développé et
sélectionné, et l'Inspecteur de champ affiche la valeur actuelle.</p>

<h3>Panneau Inspecteur de champ</h3>
<p>Cliquez sur l'icône <strong>ⓘ</strong> dans l'en-tête du panneau de
l'arbre (ou <strong>Affichage → Inspecteur de champ</strong>) pour
afficher les métadonnées issues du schéma pour le nœud sélectionné :</p>
<ul>
	<li>Position HL7 (p. ex. <code>PID-5</code>) et nom canonique
		(Patient Name)</li>
	<li>Type de données (XPN, CX, ST, ...), longueur max., indicateurs
		obligatoire/répétable, description</li>
	<li>Valeur actuelle et longueur ; un bouton <em>Voir la valeur
		complète</em> pour les champs longs que l'éditeur affiche repliés</li>
</ul>
<p>Les segments inconnus (Z-segments ou codes personnalisés hors
standard) affichent <em>Non standard HL7</em> mais restent entièrement
modifiables.</p>

<h3>Rechercher dans l'arbre</h3>
<p>La zone de recherche en haut de l'arbre porte sur le <strong>type de
segment</strong> (<code>PID</code>), le <strong>nom de champ du
schéma</strong> (<code>Patient Name</code>) et la <strong>valeur du
champ</strong> — y compris les champs des segments que vous n'avez pas
encore développés. Appuyez sur
<kbd>Enter</kbd>/<kbd>Shift</kbd>+<kbd>Enter</kbd> pour parcourir les
résultats, sur <kbd>Esc</kbd> pour effacer, et sur
<kbd>Ctrl</kbd>+<kbd>F</kbd> quand l'arbre a le focus pour atteindre la
zone de recherche. Cliquer sur un résultat développe le segment,
sélectionne le champ et le fait défiler jusqu'à le rendre visible.</p>
<p class="note">La recherche dans l'arbre fonctionne sur les messages
HL7 v2. Pour les ressources FHIR, utilisez le filtre propre au
visualiseur de Bundle ou le <kbd>Ctrl</kbd>+<kbd>F</kbd> de
l'éditeur.</p>

<h3>Grille des segments</h3>
<p>Un message de résultats peut contenir des dizaines d'OBX ; l'arbre les
montre un nœud à la fois. <strong>Affichage → Grille des segments</strong>
(<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd>), ou <em>Afficher tous les
OBX en tableau</em> dans le menu contextuel d'un segment, ouvre un panneau
en bas qui présente toutes les occurrences d'un type de segment sous forme
de tableau : une ligne par occurrence, une colonne par champ renseigné
dans au moins l'une d'elles, avec la position et le nom du champ dans la
version HL7 du message. Les valeurs codées affichent leur signification
sous le code, comme dans l'arbre ; les valeurs longues sont coupées par
des points de suspension. Choisissez un autre segment dans la liste
(chacun indique son nombre d'occurrences), tapez dans <em>Filtrer les
lignes</em> pour ne garder que les lignes contenant un texte, et cliquez
sur une cellule pour sélectionner ce champ dans l'éditeur. La grille est
en lecture seule et suit le message au fil des modifications.</p>

<h3>Comparer deux messages</h3>
<p><strong>Outils → Comparer les messages…</strong> ouvre un diff côte à
côte de deux onglets ouverts, avec coloration syntaxique HL7. Choisissez
gauche/droite dans les listes déroulantes, utilisez le bouton ⇆ pour
inverser les côtés, appuyez sur <kbd>Esc</kbd> pour fermer. Au moins deux
onglets doivent être ouverts.</p>

<h3>Champs codés : ce que signifie un code</h3>
<p>BridgeLab embarque les tables de valeurs HL7 — 394 tables, environ
5 000 codes — et sait de quelle table chaque champ et composant codé
tire ses valeurs, par version. Une valeur codée est expliquée partout où
vous la rencontrez : l'arbre affiche la signification à côté de la
valeur (<code>M — Male</code>, <code>ADT — ADT message</code>,
<code>F — Final results</code>), le survol du champ dans l'éditeur la
rappelle, et l'auto-complétion dans un champ codé propose toutes les
valeurs de sa table. Les composants sont couverts aussi : MSH-9.2 est
expliqué par la table des événements, PID-3.5 par celle des types
d'identifiant.</p>
<p>L'Inspecteur de champ liste la table entière du champ ou du composant
sélectionné et met en évidence la valeur actuelle. Qu'une valeur hors
table soit un problème dépend du type de donnée, et l'inspecteur indique
dans quel cas vous êtes : un champ <code>ID</code> tire ses valeurs
d'une table définie par HL7 (<em>Valeurs autorisées</em>) et une valeur
absente est non standard — un avertissement s'affiche ; un champ
<code>IS</code> tire les siennes d'une table définie par l'utilisateur
(<em>Valeurs suggérées</em>), où chaque site ajoute ses propres codes et
où l'absence ne signifie rien. Certaines tables utilisateur n'ont aucune
valeur standard (IN1-2 Insurance Plan ID) : ces champs n'affichent pas
de liste.</p>
<p class="note">La table utilisée par un champ suit la version HL7
déclarée ; le contenu des tables est un jeu unique pour toutes les
versions, tel que la source amont le fournit. Un code ajouté dans une
version ultérieure est donc accepté pour une version antérieure.</p>

<h3>Arbre guidé par le schéma</h3>
<p><strong>Affichage → Afficher les champs du standard</strong> insère
des lignes fictives (placeholders) pour chaque champ défini par le
standard HL7 mais <em>absent</em> du message. Ces lignes apparaissent
estompées et en italique - elles permettent de voir facilement quels
champs vous <em>pourriez</em> ajouter, mais on ne peut pas y naviguer
dans l'éditeur (elles n'ont pas encore de position physique).</p>

<h3>Redimensionner les panneaux</h3>
<p>Faites glisser le séparateur vertical entre l'arbre et l'éditeur pour
les redimensionner ; faites glisser le séparateur horizontal au-dessus de
l'Inspecteur de champ pour modifier sa hauteur. Les deux dimensions sont
conservées d'un redémarrage à l'autre.</p>

<h3>Structure standard complète</h3>
<p>Avec <strong>Affichage → Afficher les champs du standard</strong> activé,
l'arbre montre aussi les segments que le standard définit pour le type de
message mais absents du message — lignes grisées à leur position standard,
annotées avec le groupe, la cardinalité et le statut de choix. Développez-les
pour parcourir la liste complète des champs jusqu'aux composants des types
composés (ex. OBX-16 → composants XCN). <strong>Clic droit sur un segment
grisé → Insérer le segment</strong> pour ajouter son squelette au message à
la position standard, avec les séparateurs jusqu'au dernier champ
obligatoire. Le squelette utilise les séparateurs du message, et
<kbd>Ctrl</kbd>+<kbd>Z</kbd> le retire.</p>
`,
};
