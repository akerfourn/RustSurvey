# Viewer et saisie locale

Ce document décrit l’implémentation actuelle.
L’[audit de couverture](form-coverage.md) compare les formulaires natifs de
6.5.14 à cette implémentation et planifie les manques ; son
[catalogue](form-catalog.md) distingue les options interprétées, tolérées et non
gérées. Zéro blocage détecté ne certifie pas l’équivalence d’un formulaire.

## Essayer le parcours

Démarrer le backend et Vite avec les commandes du [README](../README.md), puis
ouvrir **Essayer un LSS** (`#/import`). Le bouton **Essayer l’exemple**
télécharge un LSS synthétique et utilise le même endpoint d’analyse que l’upload
d’un fichier local.

1. Renseigner un nom de projet, choisir Rust et indiquer la taille de l’équipe.
2. Passer au groupe suivant, choisir une expérience et un objectif.
3. Cocher « Autre objectif » pour faire apparaître la précision obligatoire.
4. Essayer les trois tableaux. Choisir « À l’aise » dans la colonne « Après »
   affiche une autre précision obligatoire.
5. Terminer par une note et valider la saisie locale.
6. Revenir au premier groupe et changer de technologie pour masquer le groupe
   Rust et effacer ses réponses.

Le changement de langue conserve la saisie. Le bouton de réinitialisation,
l’import d’une autre étude et le rechargement repartent de zéro. Le JSON
téléchargeable conserve les cellules locales ; ce format n’est pas un export de
réponses LimeSurvey et sa réimportation n’est pas encore implémentée.

Les aperçus décrits ici gardent une navigation par groupe. Les campagnes natives
peuvent choisir groupe, question ou page unique ; leur curseur est enregistré
avec les réponses. Leur [parcours durable](collection.md) reste distinct des
aperçus et de l'exécution d'une étude LimeSurvey importée.

Un affichage d'accueil hérité ou absent reste inconnu dans les nouveaux imports.
L'aperçu peut montrer les questions, mais signale ce réglage non résolu et ne
valide pas un parcours complet. Les valeurs explicites Y/N restent appliquées.
Voir [la résolution des options](source-settings.md).

L’[éditeur natif](authoring.md) utilise aussi ce viewer pour ses aperçus et ses
liens publics. La définition est alors chargée depuis SQL ; les réponses restent
locales dans les deux parcours. Le compte administrateur n’intervient que pour
créer et modifier les définitions. L’import depuis l’administration conserve la
source et une définition v4 en SQL. Ses conditions de groupes et questions sont
éditables ; l’aperçu reçoit aussi le rapport d’import pour garder les
diagnostics, notamment les sections non prises en charge. L’inspection des
valeurs et de la visibilité est réservée à l’aperçu d’administration.

## Types et règles couverts

Le viewer admet les **28 types natifs sans fichiers**, dans un profil local
partiel. Les variantes avancées ne sont pas toutes implémentées.

| Types LSS  | Saisie dans cet aperçu                                    |
| ---------- | --------------------------------------------------------- |
| S, T, U    | Texte court ou multiligne                                 |
| N          | Nombre                                                    |
| D          | Date calendaire ISO, sans heure                           |
| L, !       | Choix simple radio ou menu, avec autre                    |
| 5, Y, G    | Note 1–5, oui/non, genre historique                       |
| M, P       | Choix multiples, avec autre et commentaires pour P        |
| O          | Choix simple avec commentaire facultatif                  |
| Q, K       | Textes/nombres multiples ; curseurs K à bornes littérales |
| A, B, C, E | Tableaux à domaines fixes natifs                          |
| F, H       | Choix par ligne ou colonne, menus possibles par ligne     |
| 1          | Tableau à deux échelles indépendantes                     |
| :          | Matrice numérique par menu/champ, ou cases à cocher       |
| ;          | Matrice de textes, éventuellement numériques              |
| R          | Classement par positions, sans doublons ni trous          |
| I          | Changement de langue du questionnaire                     |
| *          | Équation simple, résultat calculé non modifiable          |
| X          | Information sans réponse                                  |

Le bouton **Essayer les 28 types** importe `examples/surveys/controls.lss` par
le même parcours HTTP que les autres fichiers. Il contient 28 questions, 3
groupes, 2 langues et 52 cellules. La fixture initiale reste disponible avec
**Essayer l’exemple**.

### Cellules, commentaires et validation

Les [lignes exclusives](exclusive-options.md) désactivent les autres lignes en
les gardant visibles. Leur effacement se propage aux commentaires et aux
questions filtrées. Les types M/P proposent la sélection automatique d'une
option exclusive unique. Les attributs invalides continuent de produire un
diagnostic et de bloquer la validation du parcours.

Les cellules portent un rôle et une échelle explicites. Le JSON local utilise
`CODE_ligne` pour un tableau simple, `CODE_ligne_0`/`CODE_ligne_1` pour les deux
échelles, `CODE_ligne_colonne` pour une matrice et `CODE_position` pour le
classement. Les commentaires utilisent `CODE_comment` pour O et
`CODE_lignecomment` pour P. Le champ autre est `CODE_other`, accompagné de
`CODE_othercomment` pour P. Ces noms locaux ne constituent pas encore un
`FieldBinding` complet vers les colonnes de réponses LimeSurvey.

Les codes sont des chaînes, avec conservation des zéros initiaux. Les domaines
fixes utilisent 1–5, 1–10, Y/U/N et I/S/D selon le type. Une double échelle
valide chaque cellule contre sa propre liste de choix. L’option autre d’un choix
simple utilise `-oth-` et demande une précision, sauf désactivation explicite de
`other_comment_mandatory`. Pour M/P, saisir le texte autre ajoute cette réponse
; ce rendu ne reproduit pas un état transitoire de case autre cochée sans texte.

Les commentaires O restent facultatifs. Pour P, les modes `checked`, `unchecked`
et `allways` contrôlent la saisie ; les commentaires devenus inaccessibles sont
effacés. Le mode `commented_checkbox_auto=0`, qui demande une autre sémantique
de conservation, est explicitement refusé. Les commentaires ne gonflent pas le
nombre de choix sélectionnés.

La validation couvre l’obligatoire N/S/Y, les domaines, les nombres finis,
l’entier seul, la date ISO, les limites par nombre et les contraintes de somme
`min_num_value`, `max_num_value`, `equals_num_value`. Elle couvre aussi
`min_answers`, `max_answers`, `maximum_chars` et `em_validation_q`. Le
classement peut rester partiel s’il est facultatif, avec des positions remplies
depuis la première, uniques et sans trou. Une limite littérale
`max_subquestions` réduit les positions. Avec une contrainte min/max de
réponses, l’obligatoire demande au moins un choix, puis la contrainte contrôle
le nombre. Le rendu utilise des menus accessibles au clavier ; le
glisser-déposer et les filtres de classement restent à faire.

Les matrices numériques à menus utilisent des bornes/pas littéraux, par défaut
1–10 avec un pas de 1. Les champs `input_boxes=1` restent sans borne implicite
et respectent les minimum/maximum explicitement fournis. Les cases conservent
absence, coché `1` et décoché `0`. Une matrice de cases obligatoire demande une
case par ligne active ; les autres tableaux obligatoires demandent les cellules
actives. Les curseurs K respectent leurs bornes et leur pas ; leur affichage
initial ne crée pas une réponse avant interaction.

### Pertinence, filtres et calculs

Le graphe inclut pertinence de groupe/question/sous-question, sources des
filtres et équations. Il détecte les cycles et variables inconnues. Les réponses
masquées sont effacées avant les règles dépendantes. La pertinence des deux axes
de matrice est prise en compte.

`array_filter` et `array_filter_exclude` acceptent les sources
M/P/Q/K/A/B/C/E/F/H/1/:/;/R, séparées par `;`, pour les lignes des tableaux et
saisies multiples. Les filtres des ancêtres sont conservés dans les cascades. Un
code absent d'une source n'ajoute pas de restriction de cette source. Le style 0
masque la ligne ; le style 1 la laisse désactivée. Ses réponses sont effacées et
elle ne reste pas obligatoire. Le code `other` et les commentaires possèdent
leurs rôles distincts. Les listes radio et classements cibles filtrent leurs
choix ; les positions de classement sont resserrées après retrait. Autre reste
disponible dans une liste radio, comme dans la référence. Voir les
[règles et preuves](array-filters.md), notamment la limite d'export du tableau H
filtré et le diagnostic bloquant de son aperçu importé.

Le type `*` utilise l’attribut `equation` ou une expression simple du texte de
base. Les équations sont recalculées après leurs dépendances ; une valeur
fournie dans la carte de réponses ne peut pas remplacer leur résultat.
`hidden=1` masque une question sans assimiler son état à une absence de
pertinence : une équation cachée conserve son résultat. Les affectations, les
équations composites et celles différentes par langue restent hors profil.

### Présentation et textes

Les libellés utilisent la langue choisie et la langue de base en repli. Le
changement de langue conserve la saisie et synchronise les questions I. Accueil,
fin, aides, descriptions de groupe, options et sous-questions prennent en charge
les substitutions du profil d’expression. Le HTML est converti en texte dans un
`template` inerte ; aucun HTML importé n’est inséré dans le DOM actif. Les
médias importés et le rendu HTML enrichi restent à réaliser. Les images
associées aux questions natives sont affichées via le port de stockage objet,
avec contrôle de propriété et de publication côté serveur.

`display_columns`, `display_rows`, `input_size`, préfixes, suffixes et
placeholders sont appliqués. Les en-têtes de double échelle peuvent être
localisés. Les tableaux peuvent défiler horizontalement. La navigation reste par
groupe ; les modes tout sur une page/question par question, le paramétrage
complet de collecte et les variantes de thème restent à faire.

Les noms des 29 rendus de base, tels que `listradio` ou `arrays/column`, sont
reconnus avec leur type. `question_theme_name` est projeté par Rust ; un thème
de question supplémentaire non implémenté bloque explicitement sa saisie.
Certaines options cosmétiques conservées mais non reproduites apparaissent dans
un rapport de différences de présentation.

Les substitutions de toutes les langues sont préanalysées avant d’ouvrir une
page. Les expressions des textes affichables sont compilées une fois par langue
et leurs erreurs d’évaluation bloquent la validation finale. Le résultat reste
du texte échappé par Preact. Aucun `eval`, script ou appel externe importé n’est
exécuté.

### Expressions et limites

Le registre local comporte **47 des 81 noms de fonction** natifs, chacun dans un
profil à vérifier contre LimeSurvey : calculs usuels, arrondis avec précision,
conditions à deux/trois arguments, chaînes, recherche, agrégats conditionnels et
unicité. Les comparateurs textuels `eq`, `ne`, `lt`, `le`, `gt`, `ge` sont
acceptés. Le [catalogue](form-catalog.md) donne les noms et les manques. `trim`
et ses variantes n’acceptent encore qu’un argument ; `strrev` est limité à ASCII
pour éviter de changer silencieusement la sémantique des octets source. Les
détails de conversion et de casse Unicode restent à confronter au corpus amont.

Les variables utilisent les cellules connues et le suffixe `.NAOK`, sans
reproduire encore toute la sémantique ExpressionScript de pertinence. Les alias
SGQA, attributs `.shown`/`.value`, variables réservées, évaluations natives et
extensions ne sont pas implémentés. Les défauts natifs sont appliqués à l'entrée
des pages ; [leur périmètre](default-values.md) distingue les limites NAOK et
leur projection dans les nouveaux imports persistants. L'inspecteur temporaire
et les anciennes révisions conservent leur diagnostic. Les anciennes conditions,
regex, ordres aléatoires, quotas et minuteries actives restent diagnostiqués.
Hors images natives, les fonctions avec stockage des participations demeurent
différées.

Un attribut inconnu de valeur `0` n’est plus supposé désactivé. Seuls les noms
et valeurs neutres explicitement reconnus sont tolérés. Les options secondaires
de minuterie restent sans effet sans durée. Les compteurs statistiques et les
préférences d’édition sont distincts du comportement du formulaire.

Une erreur de structure désactive la saisie. Un schéma différent de 623 permet
seulement l’inspection. Le programme reste borné à 2 000 questions, 2 000
cellules par question et 20 000 cellules par étude ; listes et produits de choix
rendus sont aussi bornés. Les matrices et classements sont contrôlés avant leur
expansion. Les expressions sont limitées en taille, profondeur et nombre de
jetons ; les textes calculés sont bornés à 65 536 unités UTF-16.

## Frontière HTTP

`POST /api/surveys/inspect` reçoit directement les octets XML dans le corps,
avec `Content-Type: application/xml`, `text/xml` ou `application/octet-stream`.
Pas de multipart : le fichier est le corps de la requête.

La réponse JSON contient `metadata`, `study` et `report`. Les identifiants
source sont des chaînes pour conserver les entiers 64 bits dans JavaScript.
`study` vaut `null` pour un schéma sans projection. Un XML lisible mais
comportant des erreurs de structure produit HTTP 200 et un rapport avec
`errors > 0` ; le client doit respecter ce blocage.

Les erreurs utilisent `{ "error": { "code": "…", "message": "…" } }` :

| HTTP | Cas                                        |
| ---- | ------------------------------------------ |
| 408  | Corps non reçu en 15 secondes              |
| 413  | Plus de 8 Mio                              |
| 415  | Type de contenu non pris en charge         |
| 422  | XML/LSS illisible ou limites XML atteintes |
| 503  | Deux analyses déjà en cours                |

Le serveur borne le corps avant l’analyse et conserve au plus deux traitements
simultanés, lecture du corps comprise. L’analyse XML et la sérialisation sont
déportées dans `spawn_blocking`. Aucun fichier, questionnaire ou état de saisie
n’est conservé après la requête. Les réponses d’analyse utilisent `no-store`.
Les limites XML existantes restent applicables.

`GET /api/surveys/example.lss` et `GET /api/surveys/controls.lss` servent les
fixtures synthétiques versionnées.

## Vérifications et suite

`npm test` exécute les tests purs du moteur avec le lanceur natif Node. Le mode
TypeScript utilise `--experimental-strip-types`, disponible sur Node 22.15 de
cette machine ; aucun framework de test supplémentaire n’est ajouté. Les tests
Rust exercent la projection, les messages localisés et la frontière HTTP sans
réseau ni base externe. Les fixtures synthétiques couvrent F/H, matrices,
dépendances, cellules facultatives/obligatoires et limites. Les exports locaux
de `.surveys-tests/` restent ignorés et hors des tests versionnés.

Le parcours HTTP local a été vérifié avec Vite, le backend, le décodeur JSON et
le moteur réels. Le navigateur pilotable étant indisponible dans la session de
développement, le contrôle visuel et les interactions DOM restent à vérifier.

La [collecte native](collection.md) réutilise ce viewer sur un profil de 17
types. Les campagnes ont leurs propres liens, la validation Rust et les API de
sauvegarde/reprise. L’inspection LSS, l’aperçu d’administration et les liens
`#/studies/...` restent locaux. Leur validation ne remplace aucun contrôle
serveur.
