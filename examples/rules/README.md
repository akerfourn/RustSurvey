# Corpus de syntaxe des conditions

`conditions.json` contient des expressions synthétiques partagées par les tests
Rust et TypeScript. `variables` contient les références attendues, ou `null`
pour une syntaxe refusée. Les tests couvrent aussi séparément les bornes de
profondeur, longueur et nombre de jetons.

Ce corpus ne contient aucune donnée de production et ne certifie pas toute la
sémantique d’ExpressionScript. Les validations de réponses restent exécutées
uniquement dans le navigateur à cette étape.
