# Filtres de lignes et de choix entre questions

## Configuration native

Dans **Calculs, validation et présentation**, les réglages **Filtre : questions
source à inclure** et **à exclure** acceptent des codes de questions séparés par
`;`. Ils correspondent à `array_filter` et `array_filter_exclude`. Les codes des
lignes cibles doivent correspondre à ceux des lignes ou choix source.

Les sources disponibles sont M/P/Q/K/A/B/C/E/F/H/1/:/;/R : choix multiples,
textes et nombres multiples, tableaux et classement. Les autres types de source
produisent un diagnostic. Un code cible absent d'une source ne reçoit aucune
restriction de cette source.

Les filtres inclusifs conservent les lignes sélectionnées ; les exclusifs les
retirent. Plusieurs filtres se combinent par intersection. Le style 0 masque les
lignes exclues ; le style 1 les garde visibles et désactivées. Leurs réponses
sont effacées avant les questions dépendantes et ne restent pas obligatoires.

Les listes radio L et les classements R peuvent aussi être filtrés. Leurs choix
utilisent les mêmes codes que les lignes source. Le menu déroulant `!` reste
refusé comme cible : la version 6.5.14 ne lui applique pas ce filtre. Le tableau
par colonnes H peut être filtré nativement, mais la référence ignore ce filtre
cible : l'export LSS natif refuse ce cas avec un diagnostic. L'aperçu d'un H
importé le signale et bloque sa saisie, plutôt que lui appliquer l'extension
native. H reste une source prise en charge dans les deux moteurs.

Une sélection radio devenue interdite est effacée avant validation. Dans un
classement, les choix retirés sont supprimés ; les positions restantes sont
resserrées sans changer leur ordre. Le nombre de positions actives et
l'obligatoire suivent le nombre de choix encore disponibles. Un code inconnu, un
doublon ou un trou restant dans le classement produit une erreur. L'éditeur
expose les filtres et leur style pour ces deux nouvelles cibles.

Dans une liste L, Autre reste disponible indépendamment du filtre, comme dans la
référence : la construction des champs n'y ajoute pas de règle de filtrage pour
ce choix. Cette particularité est vérifiée séparément des choix multiples M/P,
dont la ligne `other` est filtrée. Le texte Autre suit toujours sa sélection ;
il n'est conservé que si la valeur principale est `-oth-`.

## Sens d'une sélection

- Pour M/P, une case cochée sélectionne sa ligne ; les commentaires ne sont pas
  des sélections. Un vrai code de ligne finissant par `comment` reste distinct
  d'une cellule de commentaire.
- Pour Q/K et les tableaux, une cellule renseignée sélectionne sa ligne. Le
  nombre `0`, la réponse `N` d'un tableau oui/non et les espaces d'un texte sont
  des valeurs présentes.
- Dans une matrice, une seule colonne renseignée suffit. Une seule échelle d'un
  tableau double suffit également.
- Une matrice de cases utilise `1` comme sélection ; `0` reste non sélectionné.
- Un classement sélectionne les codes effectivement classés, indépendamment de
  leurs positions. Les codes ne passent pas par une conversion numérique.

Le code `other` représente l'option Autre d'un choix multiple. Son texte non
vide sélectionne la ligne correspondante ; son commentaire seul ne le fait pas.
Autre peut aussi être filtré dans une cible M/P : texte et commentaire sont
effacés ensemble. Sa visibilité suit le style du filtre, tout en gardant les
lignes simplement désactivées par une exclusivité visibles.

## Cascade et modifications

La cible conserve aussi les filtres des questions ancêtres, avec leur sens
inclusif ou exclusif d'origine. Par exemple, ACTIVITES contient A1/A2 et filtre
DETAILS, qui ne contient que A1. Si DETAILS filtre ensuite RESULTAT, une ligne
A2 de RESULTAT dépend encore du choix A2 dans ACTIVITES. L'absence d'A2 dans
DETAILS n'annule pas la restriction de cet ancêtre.

Pertinence, filtres et exclusivité de la source sont appliqués avant la lecture
de ses sélections. Une réponse source devenue inactive ne maintient pas une
ligne cible. Les codes de source inconnus, types incompatibles et cycles sont
diagnostiqués. Renommer ou remplacer une source par un type incompatible bloque
la sauvegarde des filtres devenus invalides, même si leur texte est inchangé.
Les réglages inchangés d'un import restent conservables.

Ce lot de filtres ne modifie ni format de définition ni schéma SQL. Les profils
natifs étendus utilisent les nouveaux contrôles ; `native-simple-v1` garde son
périmètre. Les réponses déjà stockées ne sont pas modifiées à la lecture. Une
nouvelle sauvegarde réévalue les règles, notamment la cascade corrigée, et peut
retirer des cellules précédemment conservées à tort. Les anciens reçus
idempotents restent traités avant une nouvelle évaluation.

## Coût et preuves

La compilation prépare les sources et les ancêtres, avec déduplication. Les
métadonnées des sources sont partagées ; aucun catalogue de classement n'est
dupliqué pour chaque position. Pendant une évaluation, chaque source utilisée
est lue une seule fois, puis son ensemble de sélections est réutilisé. Ce cache
est local à l'évaluation, sans session serveur ni état global. Les traitements
Rust sont soumis au budget commun de calcul. Aucune dépendance ni migration
n'est ajoutée ; les mesures comparatives CPU/RAM restent à réaliser.

Le corpus `examples/rules/filters.json` possède 46 cas communs Rust/TypeScript.
Les tests HTTP couvrent collecte, effacement des valeurs inactives, reprise et
réessais. Le parcours de stockage relit les réponses filtrées après arrêt
complet du serveur ; les tests externes couvrent PostgreSQL et MariaDB.

Le 14 novembre 2026, les 46 cas sont aussi comparés au moteur PHP de LimeSurvey
6.5.14 dans l'instance jetable. Le test importe chaque LSS, lie les mêmes
**valeurs source déjà normalisées**, initialise la pertinence des questions et
compare les règles de lignes et de choix produites par le gestionnaire amont.
Les 14 cas de listes et classements vérifient aussi l'obligatoire sur les mêmes
réponses normalisées. Le test contrôle les correspondances de champs, lie les
valeurs au moteur et à sa session, et refuse une expression amont en erreur.
Cela vérifie cette frontière de filtrage, sans certifier les parcours complets
du navigateur, la normalisation amont de chaque source ni la migration des
reprises. La collecte des études importées reste bloquée.

La référence est le [gestionnaire d'expressions du commit cible][manager], en
particulier la sélection des cellules et la recherche des filtres ancêtres. Le
[manuel][manual] décrit également la cascade et l'exception du tableau H.

[manager]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/expressions/em_manager_helper.php#L1142-L1310
[manual]: https://www.limesurvey.org/manual/QS:Array_filter
