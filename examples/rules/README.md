# Corpus partagés des règles

`conditions.json` contient des expressions synthétiques partagées par les tests
Rust et TypeScript. `variables` contient les références attendues, ou `null`
pour une syntaxe refusée. Les tests couvrent aussi séparément les bornes de
profondeur, longueur et nombre de jetons.

`evaluation.json` compare les résultats et erreurs de 87 cas sur les 47
fonctions exécutables dans les deux langages. `collection.json` associe une
définition native synthétique à dix parcours de collecte : visibilité,
effacement des réponses, calculs, codes lexicaux, obligatoire et erreurs.

`collection-forms.json` ajoute 21 parcours sur les 28 types natifs, avec
filtres, axes conditionnels, commentaires, classement, matrices et curseurs. La
section `baseline` décrit les réponses et résultats explicites du parcours
complet. Chaque cas ajoute ses variations et les clés à effacer ; `base: false`
repart de zéro. Les résultats sont des attentes relues, pas des sorties
régénérées par un des moteurs. Les tests Rust et TypeScript consomment ce même
document.

Ces corpus ne contiennent aucune donnée de production. Ils vérifient l’accord
des moteurs Rust et TypeScript, sans certifier toute la sémantique
d’ExpressionScript ni remplacer des fixtures exécutées dans LimeSurvey.
