# Documentation de RustSurvey

Cette documentation accompagne le développement. Elle décrit le problème à
résoudre, les contraintes acquises et les choix à valider. Le premier code
fonctionnel couvre l’[adaptateur `.lss`](../crates/limesurvey/README.md) et le
[viewer avec saisie locale](viewer.md).

## Parcours de lecture

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

## Lire les statuts

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
