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

| Sujet          | Attente                                                   |
| -------------- | --------------------------------------------------------- |
| Serveur        | Rust minimal, ressources maîtrisées                       |
| Navigateur     | TypeScript et Preact                                      |
| Répartition    | Déporter les interactions et calculs sûrs côté client     |
| Sécurité       | Contrôles serveur même lorsque le client valide déjà      |
| Bases          | SQLite, MariaDB et PostgreSQL                             |
| Évolution      | Migrations de base automatisées et versionnées            |
| Échange        | Import/export de fichiers LimeSurvey                      |
| Référence      | LimeSurvey 6.5.14 en production                           |
| Étape actuelle | Import LSS, viewer et moteur de saisie locale sans compte |

L’équivalence avec LimeSurvey vise d’abord les questionnaires réellement
utilisés. Elle n’implique pas de reproduire immédiatement tous ses plugins,
thèmes, modes d’administration ou interfaces réseau.

La migration d’études en cours est une exigence acquise : conserver structure,
réponses complètes et partielles, participants, droits de reprise et
comportement attendu. Les ressources et états externes nécessaires font aussi
partie de l’inventaire de transfert. Une fonction non prise en charge bloque la
bascule concernée plutôt que provoquer une perte silencieuse.

## Périmètre initial proposé

Le premier adaptateur peut commencer par `.lss`, pour la structure. Les archives
`.lsa` et leurs données associées font partie de la cible de migration dès la
conception ; elles ne sont pas une extension facultative repoussée après une
architecture limitée aux nouveaux questionnaires.

Le premier parcours fonctionnel à implémenter couvre : importer un petit
questionnaire de référence, le faire remplir, enregistrer les réponses et
réexporter sa structure. Le [modèle interne](domain-model.md) couvre dès ce
premier tour les familles identifiées dans les sources. L’implémentation et la
certification des fonctions restent progressives.

Le [viewer actuel](viewer.md) réalise un premier parcours sans persistance : le
backend analyse la structure, le navigateur affiche les questions et évalue un
sous-ensemble des règles. Les comptes utilisateurs sont hors de cette étape.

Le [protocole de migration](study-migration.md) distingue conservation,
réexport, exécution et continuité. Le transfert complet inclut les compléments
nécessaires lorsqu’un fichier standard ne transporte pas tout l’état utile.

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

## Informations pour valider et dimensionner

| Priorité | À préciser                | Impact                    |
| -------- | ------------------------- | ------------------------- |
| Haute    | Exports de production     | Validation des usages     |
| Haute    | Questions et expressions  | Moteur de règles          |
| Haute    | Anonymat, jetons, reprise | Modèle des participations |
| Haute    | Machine et charge cible   | Budget de ressources      |
| Moyenne  | Comptes et rôles          | Accès administrateur      |
| Haute    | États de reprise source   | Continuité du transfert   |
| Moyenne  | Conservation, sauvegarde  | Exploitation              |

Ces points affineront la validation et l’exploitation. Ils ne bloquent pas
l’architecture, conçue à partir du code et du corpus amont de la version cible.
L’[audit des sources](limesurvey-source-audit.md) distingue les constats
vérifiés des comportements qui restent à tester dans une instance 6.5.14.
