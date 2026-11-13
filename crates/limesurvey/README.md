# Adaptateur LimeSurvey

Première bibliothèque Rust pour les structures `.lss`, avec un outil CLI. Le
modèle exposé décrit le fichier source et ses identifiants ; il ne remplace pas
le modèle de persistance des études.

## Commandes

Depuis la racine du dépôt :

```sh
cargo run -p rustsurvey-limesurvey --locked -- inspect \
  crates/limesurvey/tests/fixtures/minimal-623.lss
```

```sh
cargo run -p rustsurvey-limesurvey --locked -- export-original \
  crates/limesurvey/tests/fixtures/minimal-623.lss original.lss
```

```sh
cargo run -p rustsurvey-limesurvey --locked -- set-title \
  crates/limesurvey/tests/fixtures/minimal-623.lss renamed.lss fr \
  'Nouveau titre'
```

Les fichiers de sortie doivent être nouveaux : aucun écrasement n’est autorisé.
`inspect` retourne 0 sans erreur structurelle, 2 avec erreurs dans le rapport,
et 1 en cas d’échec de lecture ou d’analyse XML.

`export-original` conserve les octets reçus, même si le rapport structurel
contient des erreurs. Cette commande archive la source ; elle ne certifie pas
son fonctionnement. `set-title` exige le schéma 623 et l’absence d’erreurs
structurelles. Il modifie seulement le contenu du titre dans la langue choisie.
Les autres octets, extensions et attributs sont conservés.

## Contrôles présents

- XML 1.0 UTF-8, sans DTD ni instructions de traitement.
- Limites configurables via `Limits` : 8 Mio, profondeur 64, 100 000 éléments,
  32 attributs par élément et 128 diagnostics par défaut.
- Métadonnées, langues, identifiants et déclarations de colonnes.
- Groupes, questions, sous-questions, options et références de leurs
  traductions.
- Caractère obligatoire strict/souple et familles des types de questions.
- Conservation des valeurs absentes/vides et des données non interprétées.

Les rapports comptabilisent toutes les erreurs même lorsque la liste affichée
est tronquée. Ils indiquent les sections effectivement contrôlées et ne
reprennent pas les valeurs utilisateur dans leurs messages.

Le modèle de lecture expose aussi les textes localisés, choix de réponse,
attributs bruts et expressions de pertinence nécessaires au
[viewer web](../../docs/viewer.md). En JSON, ses identifiants sont des chaînes
afin de conserver leur précision dans JavaScript. `metadata()` reste disponible
pour les autres schémas sans prétendre en valider la structure.

## Réexport des pertinences

`LssDocument::with_relevances` applique un lot de modifications aux groupes et
questions identifiés par leurs alias source. Les champs `grelevance`/`relevance`
sont modifiés ou ajoutés avec leur déclaration ; les autres octets restent
inchangés. L’API d’administration utilise ce mécanisme pour exporter une
révision enregistrée, séparément de l’original immuable.

Une transformation est refusée si une section d’anciennes `conditions` contient
des lignes. Celles-ci pourraient régénérer des pertinences contradictoires lors
de l’import. Les cibles ambiguës, caractères XML invalides et dépassements sont
refusés. Le résultat est relu avec les mêmes limites et contrôles structurels.
La certification d’import dans LimeSurvey 6.5.14 reste à réaliser.

## Limites explicites

Les autres révisions de schéma sont conservées sans projection ni contrôle
structurel complet. Elles ne sont pas modifiables par cet outil.

Conditions, quotas, thèmes et plugins sont préservés sans interprétation dans
cette bibliothèque. Les attributs et règles sont exposés comme données source ;
un moteur TypeScript distinct en interprète un sous-ensemble pour l’aperçu. Le
rapport de l’adaptateur continue d’annoncer l’exécution complète et la
continuité comme non implémentées. Les routes HTTP résident dans `crates/server`
; aucune table SQL métier n’est ajoutée.

La fixture versionnée est synthétique. Les tests unitaires et CLI vérifient les
invariants de cet adaptateur ; l’import dans une instance LimeSurvey 6.5.14
reste à vérifier avant d’annoncer une compatibilité fonctionnelle.

## Dépendances

`quick-xml` 0.42 est utilisé sans fonctionnalités optionnelles pour la lecture
événementielle et les positions nécessaires aux modifications ciblées. La
représentation conserve la source originale et un arbre borné, plutôt qu’une
désérialisation qui oublierait les champs inconnus. `serde` et `serde_json`
produisent le rapport CLI ; aucune nouvelle dépendance de runtime web ou de base
de données n’est introduite.

## Tests

```sh
cargo test -p rustsurvey-limesurvey --locked
```
