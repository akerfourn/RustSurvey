# Corpus partagés des règles

`conditions.json` contient des expressions synthétiques partagées par les tests
Rust et TypeScript. `variables` contient les références attendues, ou `null`
pour une syntaxe refusée. Les tests couvrent aussi séparément les bornes de
profondeur, longueur et nombre de jetons.

`evaluation.json` compare les résultats et erreurs de 87 cas sur les 47
fonctions exécutables dans les deux langages. `collection.json` associe une
définition native synthétique à dix parcours de collecte : visibilité,
effacement des réponses, calculs, codes lexicaux, obligatoire et erreurs.

Ces corpus ne contiennent aucune donnée de production. Ils vérifient l’accord
des moteurs Rust et TypeScript, sans certifier toute la sémantique
d’ExpressionScript ni remplacer des fixtures exécutées dans LimeSurvey.
