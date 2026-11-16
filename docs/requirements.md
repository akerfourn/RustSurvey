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
| Étape actuelle | Import LSS, éditeur natif, collecte SQL et reprise privée |

La cible fonctionnelle couvre tous les formulaires natifs de la version 6.5.14 :
types de questions, variantes de rendu, règles et parcours. Les questionnaires
réellement utilisés orientent l’ordre de réalisation, sans limiter cette cible.
L’[inventaire des manques](form-coverage.md) et son [catalogue](form-catalog.md)
distinguent l’existant, les travaux restants et les fonctions nécessitant du
stockage ou une autorité serveur.

Les interfaces d’administration et réseau sont des chantiers distincts. Les
thèmes et plugins personnalisés nécessitent un inventaire et une adaptation
explicites ; leur code importé n’est pas exécuté automatiquement.

La persistance actuelle couvre les définitions et les sessions d’un unique
administrateur et les sources LSS, avec un éditeur natif et les premières
conditions de groupes/questions. Les questionnaires sont publics sans compte
participant. Les aperçus restent locaux. Une première
[collecte native](collection.md) prend en charge 28 types avec validation Rust,
révision figée, stockage SQL et reprise privée. Les
[règles de participation](participation.md) couvrent cookie facultatif,
invitations, limites d'usage et corrections autorisées. Un
[profil importé](import-execution.md) qualifie des campagnes neuves ; il ne
transfère pas les anciennes participations. Les autres fonctions de formulaire
nécessitant un état durable restent à compléter et leur présence dans un import
continue à être diagnostiquée. Les médias et pièces jointes utiliseront un port
de stockage objet, avec adaptateurs S3 et fichiers locaux, selon la
[décision 0002](decisions/0002-object-storage.md). Le déploiement S3 ne doit pas
dépendre d’un volume disque persistant attaché au serveur.

La migration d'études en cours reste un objectif ultérieur. Le porteur du projet
a explicitement accepté de différer la continuité exacte des états internes et
des droits de reprise. Elle ne conditionne pas la livraison des créations,
nouvelles collectes et échanges de fichiers. Lorsqu'une bascule sera proposée,
elle devra conserver réponses complètes et partielles, participants, ressources
et comportement ; une fonction manquante ne devra pas provoquer de perte.

L’enregistrement d’un questionnaire conserve un brouillon privé. La publication
change la définition proposée dans le catalogue sans modifier les campagnes
existantes. Chaque participation durable garde la révision de sa campagne et son
droit de reprise ; les aperçus publics restent locaux.

L’échange standard des réponses LimeSurvey reste requis : VV et réponses XML
LSR, notamment dans les archives LSA. Il conserve valeurs, correspondances et
états disponibles sans devoir recréer les anciennes participations reprenables.
L’extraction des métadonnées spécifiques à LimeSurvey pour une migration
complète relève d’un outil externe ultérieur. Aucun état absent n’est inventé.

## Périmètre initial proposé

La priorité actuelle est une [version pilote utilisable](pilot.md) sur les
fonctions déjà prises en charge. La parité complète reste une cible ultérieure.
La distribution et la recette du pilote ne doivent pas attendre fichiers, quotas
avancés ou transfert des anciennes participations.

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
sous-ensemble des règles. L’[éditeur](authoring.md) ajoute création et
sauvegarde SQL, import persistant et conditions éditables, sans changer le
périmètre local des réponses.

Le [protocole de migration](study-migration.md) distingue conservation,
réexport, exécution et continuité. Il documente la cible ultérieure de transfert
complet, distincte de l’échange standard des réponses.

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
