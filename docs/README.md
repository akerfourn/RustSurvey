# Documentation de RustSurvey

Cette documentation précède le développement fonctionnel. Elle décrit le
problème à résoudre, les contraintes acquises et les choix à valider.

## Parcours de lecture

1. [Besoins et périmètre](requirements.md).
2. [Architecture proposée](architecture.md).
3. [Contrat de compatibilité LimeSurvey](limesurvey-compatibility.md).
4. [Bases de données et migrations](database.md).
5. [Environnement de développement](development.md).
6. [Choix de la pile technique](decisions/0001-technology-stack.md).
7. [Feuille de route](roadmap.md).

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
