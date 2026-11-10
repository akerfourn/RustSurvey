# Besoins et périmètre

## Problème à résoudre

L’installation LimeSurvey utilisée en production souffre de gels récurrents du
serveur pour des questionnaires jugés simples. RustSurvey doit permettre de les
exploiter avec un backend plus sobre et plus prévisible.

L’origine précise des gels reste à établir. La gestion des sessions est une
hypothèse à confirmer ; le projet ne part pas d’un diagnostic technique déjà
démontré. L’expérience réussie de LiteCloud, alternative légère à Nextcloud,
motive la démarche. Son approche Rust minimal et Preact devient le point de
départ technique de RustSurvey, à conserver sauf besoin particulier.

## Contraintes acquises

| Sujet          | Attente                                                    |
| -------------- | ---------------------------------------------------------- |
| Serveur        | Rust minimal, ressources maîtrisées                        |
| Navigateur     | TypeScript et Preact                                       |
| Répartition    | Déporter les interactions et calculs sûrs côté client      |
| Sécurité       | Contrôles serveur même lorsque le client valide déjà       |
| Bases          | SQLite, MariaDB et PostgreSQL                              |
| Évolution      | Migrations de base automatisées et versionnées             |
| Échange        | Import/export de fichiers LimeSurvey                       |
| Référence      | LimeSurvey 6.5.14 en production                            |
| Étape actuelle | Documentation d’abord, squelette limité à l’infrastructure |

L’équivalence avec LimeSurvey vise d’abord les questionnaires réellement
utilisés. Elle n’implique pas de reproduire immédiatement tous ses plugins,
thèmes, modes d’administration ou interfaces réseau.

## Périmètre initial proposé

La première cible de fichiers est `.lss`, pour la structure des questionnaires.
Les archives `.lsa`, qui peuvent inclure des réponses et des participants,
demandent un cadrage distinct.

Le premier parcours fonctionnel à concevoir couvre : importer un petit
questionnaire de référence, le faire remplir, enregistrer les réponses et
réexporter sa structure. Les types de questions et les règles acceptées doivent
être définis à partir d’un corpus représentatif.

L’éditeur visuel avancé, les statistiques complexes, les invitations, les
fichiers joints et les extensions seront priorisés après ce premier parcours. Ce
découpage reste une proposition, pas une exclusion définitive.

## Attentes non fonctionnelles

- Borner les connexions, les requêtes, les imports et les tâches coûteuses.
- Éviter une empreinte mémoire proportionnelle aux sessions simplement ouvertes.
- Rendre la progression durable et la reprise explicite.
- Éviter les accès SQL répétés pour chaque interaction d’affichage.
- Préserver les règles d’anonymat et de confidentialité des questionnaires.
- Donner des erreurs exploitables pour les fonctions non prises en charge.
- Permettre une installation simple, sans services annexes par défaut.

Le matériel cible, le volume de réponses simultanées et les budgets chiffrés de
mémoire/latence sont **à définir**. Aucun gain n’est annoncé sans mesure.

## Informations à réunir avant le métier

| Priorité | À préciser                | Impact                    |
| -------- | ------------------------- | ------------------------- |
| Haute    | `.lss` de référence       | Fonctions couvertes       |
| Haute    | Questions et expressions  | Moteur de règles          |
| Haute    | Anonymat, jetons, reprise | Modèle des participations |
| Haute    | Machine et charge cible   | Budget de ressources      |
| Moyenne  | Comptes et rôles          | Accès administrateur      |
| Moyenne  | Reprise des réponses      | Archives et données       |
| Moyenne  | Conservation, sauvegarde  | Exploitation              |

Ces points sont consignés pour la prochaine étape. Ils ne bloquent pas la
préparation documentaire ni la vérification du squelette.
