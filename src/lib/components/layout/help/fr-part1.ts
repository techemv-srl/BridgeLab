import type { ManualSection } from '../helpContent';
import { mockupAppShellFr as mockupAppShell, mockupContextMenuFr as mockupContextMenu } from './mockups';

export const getStarted: ManualSection = {
	id: 'getting-started',
	heading: 'Premiers pas',
	body: `
<p>BridgeLab est un éditeur de messages moderne pour HL7 v2.x et FHIR,
conçu pour les ingénieurs d'intégration en santé. Il repose sur un
backend Rust pour une analyse rapide (des messages de 10 Mo traités en
moins de 2 secondes) et un frontend Svelte 5 avec l'éditeur Monaco.</p>

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
		500 ms après la dernière frappe.</li>
	<li><strong>Fichier → Nouveau depuis un modèle</strong> (<kbd>Ctrl</kbd>+<kbd>N</kbd>) -
		modèles préremplis ADT, ORM, ORU, SIU et plus encore. Les champs
		comme MSH-7 et MSH-10 sont renseignés avec l'horodatage courant et
		un GUID neuf.</li>
</ul>

<div class="note">Au premier lancement vous bénéficiez d'un <strong>essai
Pro de 7 jours</strong> avec toutes les fonctionnalités activées. À
l'expiration, BridgeLab continue de fonctionner avec les fonctionnalités
Community - vous ne perdez jamais vos messages.</div>
`,
};

export const editorSection: ManualSection = {
	id: 'editor',
	heading: 'Éditeur',
	body: `
<p>La zone d'édition est une instance <strong>Monaco</strong> dotée d'une
grammaire spécifique à HL7. Les codes de segment sont colorés en violet,
les séparateurs de champ en gris, et les payloads ED/base64 sont tronqués
automatiquement pour garder l'éditeur réactif sur les gros messages.</p>

<h3>Auto-complétion et survol</h3>
<p>Commencez à taper <code>P</code> sur une nouvelle ligne - Monaco
suggère <code>PID</code>, <code>PV1</code>, <code>PV2</code>, etc. Une
fois le segment saisi, l'auto-complétion au pipe propose des valeurs de
champ (codes de sexe, codes ACK, classe patient...). Survoler un champ
affiche son nom, son type de données et son caractère obligatoire, tirés
du standard HL7.</p>

<h3>Troncature des champs volumineux</h3>
<p>Les champs qui dépassent le seuil de troncature (100 octets par
défaut, réglable dans <strong>Paramètres → Analyseur</strong>)
apparaissent sous la forme <code>{...N bytes}</code>. Le contenu complet
n'est jamais perdu - développez-le à la demande via le menu contextuel
(clic droit) ou l'<em>Inspecteur de champ</em>.</p>

<h3>Menu contextuel (clic droit)</h3>
${mockupContextMenu}
<p>Le menu regroupe les actions en trois sections :</p>
<ul>
	<li><strong>Navigation :</strong> Afficher le segment dans l'arbre
		(<kbd>Alt</kbd>+<kbd>T</kbd>) - ouvre l'arbre et met en évidence
		le champ exact sous le curseur ; Développer / Réduire pour les
		valeurs tronquées.</li>
	<li><strong>Presse-papiers :</strong> Copier le segment
		(<kbd>Alt</kbd>+<kbd>C</kbd>), Copier le message complet (avec les
		champs développés), Copier le message tronqué (sans risque pour
		l'email).</li>
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
<strong>composants</strong>. Affichez-le ou masquez-le avec
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
		complète</em> pour les champs tronqués</li>
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

<h3>Comparer deux messages</h3>
<p><strong>Outils → Comparer les messages…</strong> ouvre un diff côte à
côte de deux onglets ouverts, avec coloration syntaxique HL7. Choisissez
gauche/droite dans les listes déroulantes, utilisez le bouton ⇆ pour
inverser les côtés, appuyez sur <kbd>Esc</kbd> pour fermer. Au moins deux
onglets doivent être ouverts.</p>

<h3>Valeurs autorisées pour les champs codés</h3>
<p>Lorsque le champ sélectionné est adossé à une table de valeurs HL7
(PID-8 Administrative Sex, PV1-2 Patient Class, MSA-1 Acknowledgment
Code, ORC-1 Order Control, OBX-11 Result Status, …), l'inspecteur liste
les <strong>valeurs autorisées</strong> avec leur signification et met en
évidence celle présente dans le message. Si la valeur actuelle ne figure
pas dans la table, un avertissement s'affiche — un moyen rapide de
repérer les codes non standard avant que le système récepteur ne les
rejette.</p>

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
obligatoire.</p>
`,
};
