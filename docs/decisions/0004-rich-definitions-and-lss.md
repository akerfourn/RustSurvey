# 0004 — Définition enrichie, import LSS et conditions

## Décision initiale

La [décision 0005](0005-authoring-media-and-history.md) étend cet incrément au
format v3, aux calculs éditables, à l’historique, aux images et au réexport LSS
des conditions. Le texte ci-dessous décrit les choix du lot v2.

Le format natif v2 est commun aux créations et aux imports LSS du schéma 623. Il
représente les types du modèle de lecture, les traductions, sous-questions,
échelles, options, conditions et références source. Le métier reste indépendant
du XML : la conversion appartient à `workspace/lss_projection.rs`.

Les objets importés reçoivent des UUID natifs. Les identifiants et types source
restent des alias ; les codes de variables et de choix conservent leur forme
lexicale. Le moteur TypeScript reçoit la même projection de rendu pour les
créations et les imports. Il continue à signaler les fonctions non exécutables.

La v1 se convertit explicitement en v2 à la lecture et à la sauvegarde, sans
réécriture des révisions existantes. Les choix multiples deviennent des
sous-questions ; leurs identifiants et codes sont conservés. Les données
ambiguës et les versions futures sont refusées.

## Conservation et édition

Une migration SQL ajoute `import_sources` : XML original UTF-8, empreinte
SHA-256 et rapport d’import, liés à l’étude. Étude, première révision et source
sont créées dans la même transaction. `LONGTEXT` est utilisé sur MariaDB pour le
XML et le rapport ; les autres moteurs utilisent `TEXT`.

L’original reste privé, immuable et téléchargeable après authentification. Le
public reçoit la définition courante et les diagnostics, jamais le XML. Un
réexport original ne prétend pas inclure les éditions natives.

Pour cet incrément, une étude importée permet seulement l’édition des conditions
de groupes et questions. Le serveur compare les documents en neutralisant ces
seuls champs ; toute autre modification est refusée. Cela préserve les
traductions, options et références inconnues en attendant une édition plus large
et un véritable réexport modifié. Le client seul ne décide pas de cette
protection.

## Règles

Le moteur TypeScript conserve l’évaluation et expose son arbre syntaxique au
constructeur de conditions. Celui-ci sait représenter les comparaisons simples
et les ensembles ET/OU. Une expression complexe reste dans le mode avancé. Les
littéraux numériques et chaînes gardent leur nature lors d’une édition visuelle,
notamment les codes avec zéros initiaux.

Rust vérifie la syntaxe bornée, les fonctions et arités, les références aux
cellules et les cycles de dépendances. Les conditions de groupe sont prises en
compte pour chaque question. Les sous-questions, filtres et équations connus
participent à la recherche des cycles. Les règles importées inchangées sont
préservées même si leur syntaxe dépasse le profil ; leurs diagnostics restent
visibles. Une modification doit respecter le profil actuellement admis.

Le corpus `examples/rules/conditions.json` est exécuté en Rust et TypeScript. Il
porte sur la syntaxe et les références, pas sur une preuve complète de la
sémantique LimeSurvey. Rust n’évalue pas encore les réponses : il n’y a aucune
collecte persistante dans ce lot. Un moteur serveur d’évaluation reste requis
avant son introduction. Aucun `eval`, WASM ou service supplémentaire n’est
ajouté.

## Ressources et limites

Le document v2 et l’import XML sont limités à 8 Mio. Cette limite commune
accepte les projections enrichies et l’augmentation de taille lors d’une
conversion v1. Les bornes structurelles restent explicites : 500 questions, 200
éléments par axe, 20000 cellules et des textes limités.

Lecture/analyse/conversion XML sont bornées à deux imports en concurrence. Le
permis du calcul est déplacé dans `spawn_blocking` pour survivre à une
annulation HTTP. L’API conserve son délai de 15 secondes. Le client effectue les
interactions d’affichage sans requête SQL pour chaque changement de réponse.

Aucune dépendance n’est ajoutée et les lockfiles ne sont pas modifiés. Collecte,
médias, campagne figée, édition des calculs et réexport LSS modifié restent des
étapes distinctes. Les médias suivront le port objet de la décision 0002 ;
stocker le XML n’exige pas cet adaptateur.
