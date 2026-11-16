# Documentation de RustSurvey

Cette documentation accompagne le développement. Elle décrit le problème à
résoudre, les contraintes acquises et les choix à valider. Le premier code
fonctionnel couvre l’[adaptateur `.lss`](../crates/limesurvey/README.md) et le
[viewer avec saisie locale](viewer.md), le stockage SQL et
l’[éditeur avec administration](authoring.md).

## Parcours de lecture

Pour essayer le livrable actuel, commencer par le [guide du pilote](pilot.md).

1. [Besoins et périmètre](requirements.md).
2. [Analyse des sources LimeSurvey 6.5.14](limesurvey-source-audit.md).
3. [Architecture proposée](architecture.md).
4. [Modèle interne et stockage](domain-model.md).
5. [Contrat de compatibilité LimeSurvey](limesurvey-compatibility.md).
6. [Migration d’études en cours](study-migration.md).
7. [Bases de données et migrations](database.md).
8. [Environnement de développement](development.md).
9. [Choix de la pile technique](decisions/0001-technology-stack.md).
10. [Feuille de route](roadmap.md).
11. [Viewer et moteur d’aperçu](viewer.md).
12. [Couverture des formulaires et plan de réalisation](form-coverage.md).
13. [Catalogue des attributs et fonctions ExpressionScript](form-catalog.md).
14. [Stockage objet des médias et fichiers](decisions/0002-object-storage.md).
15. [Administration, éditeur natif et API SQL](authoring.md).
16. [Choix de persistance et d’authentification](decisions/0003-authoring.md).
17. [Définition enrichie, import LSS et conditions](decisions/0004-rich-definitions-and-lss.md).

18. [Édition étendue, historique et médias natifs](decisions/0005-authoring-media-and-history.md).

19. [Données privées et stockage local](local-storage.md).

20. [Vérification dans LimeSurvey 6.5.14](limesurvey-reference.md).

21. [Échange des réponses LSR et VV](response-exchange.md).
22. [Choix des cellules et de l'export VV](decisions/0009-response-exchange.md).
23. [Préparation privée des réponses LSR](response-imports.md).
24. [Choix de préparation des réponses](decisions/0010-response-import-staging.md).
25. [Prérequis de collecte des études importées](imported-collection.md).
26. [Résolution des options héritées LimeSurvey](source-settings.md).
27. [Choix de persistance des contextes source](decisions/0011-source-settings-contexts.md).
28. [Présentation figée des campagnes natives](decisions/0012-native-navigation.md).
29. [Lignes et choix exclusifs](exclusive-options.md).
30. [Filtres de lignes, sources et cascade](array-filters.md).
31. [Édition, collecte et échange des valeurs par défaut](default-values.md).
32. [Mesures CPU et mémoire](performance.md).
33. [Nouvelles campagnes sur des imports qualifiés](import-execution.md).
34. [Instantanés d'exécution des campagnes importées](decisions/0014-import-execution-snapshots.md).
35. [Configuration et vérification du stockage S3](s3-storage.md).
36. [Contenus S3 et publication SQL immuable](decisions/0015-s3-indexed-content.md).
37. [Distribution du client web et paquet pilote](decisions/0016-pilot-web-delivery.md).
38. [Participants nombreux, pauses et pointes de charge](performance-participants.md).

39. [Brouillons et publication sans interruption](decisions/0006-drafts-and-publications.md).

## Lire les statuts

La [collecte native et reprise](collection.md) décrit le premier profil durable.
La [décision 0006](decisions/0006-native-collection.md) explique ses choix et
ses limites. Les liens publics du viewer restent des aperçus locaux distincts.

| Statut    | Signification                                            |
| --------- | -------------------------------------------------------- |
| Acquis    | Contrainte exprimée pour le projet                       |
| Proposé   | Choix de conception révisable, à valider avant le métier |
| Squelette | Infrastructure minimale présente dans le dépôt           |
| Vérifié   | Contrôle exécuté, avec un périmètre précisé              |
| À définir | Information ou critère d’acceptation encore manquant     |

Les commandes disponibles sont décrites dans le document de développement. Les
étapes de la feuille de route décrivent du travail futur et ne doivent pas être
présentées comme des fonctionnalités existantes.
