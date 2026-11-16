# Consignes du projet RustSurvey

## Phase actuelle

Le porteur du projet a recentré le cycle sur une version pilote utilisable.
Stabiliser et vérifier le paquet pilote avant d'ouvrir de nouvelles fonctions.
La parité complète reste un chantier ultérieur, sans conditionner ce jalon. Lire
le [guide du pilote](docs/pilot.md) et la
[décision 0016](docs/decisions/0016-pilot-web-delivery.md). Le build public est
servi par Rust depuis un instantané borné ; aucun chemin HTTP ne devient un
chemin disque. Ne jamais utiliser un répertoire de données comme racine web.

Le développement couvre l’adaptateur LimeSurvey, son outil CLI et un premier
viewer web avec saisie locale, stockage SQL et premier éditeur natif. Un compte
administrateur protège l’édition ; les questionnaires restent publics.
L’administration importe les LSS en SQL et édite les conditions de groupes et
questions. Brouillons et publications sont séparés : enregistrer ne change pas
la version publique. Publier sert la nouvelle révision aux nouveaux arrivants,
sans interruption ; un parcours déjà commencé conserve sa révision et ses
médias. La collecte existante conserve sa révision par campagne ; publier ne
change pas une campagne déjà ouverte. Lire la
[décision de publication](docs/decisions/0006-drafts-and-publications.md). Les
autres réglages importés restent protégés dans ce lot. Suivre les grandes lignes
et livrer des incréments fonctionnels testés ; ne pas remplacer le code demandé
par un nouveau tour de cadrage général.

Le porteur du projet a explicitement accepté de différer la continuité exacte
des études en cours : sessions, droits de reprise et états internes incomplets
dans les exports. Garder ce chantier documenté, sans en faire un préalable aux
créations, nouvelles collectes et échanges de fichiers. Ne pas présenter une
nouvelle campagne comme un transfert des anciennes participations.

Lire [README.md](README.md), les [besoins](docs/requirements.md), le
[modèle interne](docs/domain-model.md) et le document du domaine concerné avant
une modification. Distinguer les contraintes acquises, les décisions de
conception et les fonctionnalités réellement vérifiées. Pour le parcours web,
lire aussi le [périmètre du viewer](docs/viewer.md).

La cible couvre tous les formulaires natifs de la version de référence. Pour les
types, rendus et règles, suivre l’[audit de couverture](docs/form-coverage.md)
et son [catalogue](docs/form-catalog.md). Un LSS sans blocage détecté ne prouve
pas la parité ; distinguer options interprétées, tolérées et non projetées.

La persistance actuelle couvre les définitions versionnées, les sessions
administrateur, les originaux LSS privés et les images natives. L’éditeur
propose 28 types, paramètres, calculs et traductions. L’historique est
restaurable sans écrasement. L'export LSS natif possède une première preuve
structurelle sur 28 types et deux langues dans LimeSurvey 6.5.14 ; lire la
[procédure de référence](docs/limesurvey-reference.md). Le réexport d'un import
ne modifie que les pertinences enregistrées et bloque les anciennes conditions
contradictoires. Le format natif v4 est commun aux créations et imports ;
convertir explicitement les v1/v2/v3 sans réécrire les révisions historiques.
Lire le [périmètre de l’éditeur](docs/authoring.md) avant une modification de
ces fonctions. La [collecte native](docs/collection.md) couvre désormais 28
types, une campagne figée, la validation Rust, les réponses SQL et la reprise
privée. Le profil `native-pages-v1` ajoute trois présentations et un curseur de
page, sans convertir les anciennes campagnes ; lire la
[décision 0012](docs/decisions/0012-native-navigation.md). Les
[lignes exclusives](docs/exclusive-options.md) couvrent neuf types,
l'automatique M/P et la normalisation Rust des réponses ; leur échange est
vérifié dans la référence, sans certification de tous les parcours. Les
[filtres de lignes](docs/array-filters.md) couvrent 14 types de source, Autre et
la cascade, les listes radio et les classements cibles. Les règles de 46 cas
sont comparées au moteur PHP amont sur les mêmes valeurs normalisées. Le
filtrage natif du tableau H bloque l'export LSS fidèle ; son aperçu importé
signale l'écart avec la référence. Les
[valeurs par défaut](docs/default-values.md) disposent de 31 scénarios source et
d'un export natif vérifié sur 29 configurations. Le format v4 et l'éditeur
portent 21 types de cibles ; `native-defaults-v1` initialise les pages avec leur
langue, sans remplacer une clé présente, même vide. Les anciens profils ne sont
pas convertis. Les nouveaux imports persistants projettent les cibles de défauts
représentables, atomiquement par question et par identité source ; les anciennes
révisions restent inchangées. Les autres cibles sont diagnostiquées. Cette
projection seule n'autorise pas la collecte importée. Ne jamais copier la
session amont entière : ses écritures `updatedValues` peuvent être nulles pour
une réponse inactive. Lire le périmètre et ses restrictions NAOK.
L'[échange des réponses](docs/response-exchange.md) ajoute les coordonnées
natives, la lecture LSR et l'export VV progressif d'une campagne fermée. Les
cellules et deux réponses sont vérifiées dans la référence ; cela ne certifie
pas une migration de participations. Les langues initiales anciennes inconnues
ne doivent jamais être remplacées silencieusement par la langue courante. Les
[règles de participation](docs/participation.md) couvrent cookies, invitations,
limites d'usage et corrections autorisées. Le
[profil importé](docs/import-execution.md) ouvre des campagnes neuves sur un
périmètre qualifié, avec contexte explicite, faits d'exécution et instantané
privé SQL 0010. Lire aussi la
[décision 0014](docs/decisions/0014-import-execution-snapshots.md). Les autres
configurations, les fichiers et les transferts restent à compléter. Lire ce
périmètre avant de modifier la collecte. La
[préparation LSR](docs/response-imports.md) conserve les lignes source en SQL
avec reprise et correspondances ; elle n'autorise pas la collecte. Le
[bilan des prérequis](docs/imported-collection.md) relit chaque révision et
distingue moteur natif, projection source et continuité. Avant de modifier ces
fonctions, lire ces deux périmètres. Les
[options héritées](docs/source-settings.md) possèdent un résolveur borné de 30
réglages, vérifié dans la référence ; il ne remplace pas les options de
formulaire restant à implémenter. Un accueil hérité ou absent reste nul dans les
nouveaux imports, sans défaut natif implicite. Les contextes source sont des
instantanés SQL immuables liés à une révision ; un bilan les choisit par
identifiant explicite, jamais comme « dernier contexte ». La commande auxiliaire
`rustsurveysettings` lit seulement les 30 options et les groupes. L'auxiliaire
distinct `rustsurveyruntime` lit les faits bornés du profil importé, sans lignes
de participants ni de réponses. Lire la
[décision 0011](docs/decisions/0011-source-settings-contexts.md) avant de
modifier ce parcours. Pour les futurs médias et fichiers, suivre la
[décision de stockage objet](docs/decisions/0002-object-storage.md) : port
commun, adaptateurs SQL, fichiers locaux, mémoire et S3. Le
[guide S3](docs/s3-storage.md) et la
[décision 0015](docs/decisions/0015-s3-indexed-content.md) décrivent les chemins
par empreinte, l'index SQL 0011 et la publication atomique. Ne pas dépendre d'un
PUT conditionnel seul : la référence Garage l'ignore. Le
[stockage local](docs/local-storage.md) décrit `.data/`, les réglages et la
lecture des anciens objets SQL. Ne pas accéder au disque depuis le métier ni
servir directement les répertoires privés.

## Contraintes du produit

- Rust minimal côté serveur ; TypeScript et Preact côté navigateur.
- Reprendre l’approche sobre de LiteCloud. Conserver cette base sauf besoin
  particulier documenté ; ne pas rouvrir le choix de pile par défaut.
- Cible d’échange : fichiers LimeSurvey **6.5.14+240624**, schéma **623**. Le
  contrat porte sur les fichiers, pas sur RemoteControl ou l’API web.
- SQLite, MariaDB et PostgreSQL font partie du périmètre de portabilité.
- Toute évolution de schéma passe par une migration versionnée.
- La sobriété serveur guide les choix ; une amélioration de performance doit
  être mesurée sur une charge documentée.
- Le [banc HTTP local](docs/performance.md) mesure CPU/RAM en release.
  Distinguer succès et refus ; ne pas annoncer un gain contre LimeSurvey à
  partir de cette seule référence. Les écritures participant ont deux places
  actives et une attente bornée ; lire la
  [décision 0013](docs/decisions/0013-bounded-write-admission.md).
- Les calculs déportés au client ne remplacent jamais les contrôles serveur.
- L’échange standard des réponses LimeSurvey (VV et LSR/LSA) fait partie du
  périmètre, sans imposer la reprise des anciennes participations. L’extraction
  des métadonnées source nécessaires à une migration complète relève d’un outil
  externe ultérieur. Lire le [protocole de transfert](docs/study-migration.md).

## Architecture et dépendances

- Préférer la bibliothèque standard Rust et des crates ciblées. Ajouter une
  couche ou un framework seulement pour un besoin concret du projet.
- Avancer sur l’architecture à partir des sources de la version cible ; un
  export utilisateur n’est pas un préalable au travail de conception.
- Séparer conservation, réexport, exécution et continuité. Une donnée inconnue
  préservée ne suffit pas à autoriser l’ouverture d’une étude.
- Garder le métier indépendant de HTTP, de l’ORM et des formats LimeSurvey.
- Centraliser les particularités SQL dans l’accès aux données/migrations.
- Ne pas ajouter Redis, une file de tâches, un runtime Node en production ou un
  autre service sans besoin démontré et décision documentée.
- Éviter l’état de session non borné en mémoire et les sessions attachées à un
  processus. La reprise native est documentée et testée ; la continuité des
  reprises migrées reste à réaliser.
- Justifier les dépendances ajoutées et leurs options dans la décision technique
  concernée ; désactiver les fonctions inutilisées lorsque possible.
- Versionner les lockfiles. Ne pas les régénérer sans raison liée au changement.

## Lisibilité et conventions

- Préférer un code explicite, des fonctions courtes et des erreurs utiles.
- Code, identifiants et messages de commit en anglais ; documentation et
  échanges en français par défaut.
- Rust : édition du workspace, rustfmt, Clippy, pas de code `unsafe`.
- TypeScript : mode strict, types publics explicites, pas de `any` injustifié.
- Preact : composants fonctionnels, traitements hors du JSX et effets nettoyés
  au démontage.
- Markdown : lignes de prose de 80 caractères ; les URL de référence peuvent
  dépasser cette limite.
- Utiliser les formateurs du dépôt ; éviter les modifications sans rapport.

## Sécurité et données

- Ne jamais versionner ni journaliser des secrets, des jetons, des réponses
  personnelles ou des fichiers de production non anonymisés.
- Le navigateur est une entrée non fiable : vérifier les droits, les tailles,
  les types, les règles de réponse et les transitions côté serveur.
- Aucun `eval` pour les expressions importées, aucune exécution de code PHP,
  JavaScript ou de plugin embarqué dans un questionnaire.
- Ne pas exécuter de SQL construit à partir de valeurs utilisateur.
- Prévoir des limites explicites pour imports, archives et traitements lourds.
- Une fonction importée non prise en charge doit produire un diagnostic ; ne
  jamais la supprimer ou modifier sa signification silencieusement.
- L’original LSS archivé est immuable et accessible seulement à
  l’administrateur. Ne pas le présenter comme un export des modifications.
- Ne jamais assimiler une archive `.lsa` à un transfert complet des sessions,
  reprises et ressources externes. Vérifier les compléments réellement fournis.

## Vérifications

```sh
node scripts/doctor.mjs
npm run check
npm run check:rust
```

L’adaptateur `.lss` possède des tests de lecture, de conservation, d’édition et
de CLI. L’API d’inspection et le moteur TypeScript ont leurs propres tests. La
saisie d’aperçu reste locale. La collecte native possède des API distinctes et
deux corpus Rust/TypeScript. Ajouter des tests de comportement pour les
nouvelles fonctions et un test de régression pour chaque bogue corrigé. Éviter
les tests qui ne font que recopier l’implémentation.

- Tests rapides, déterministes et isolés ; pas d’accès à des services distants.
- SQLite en mémoire ou fichier temporaire pour les tests courants.
- PostgreSQL/MariaDB dans des environnements locaux jetables pour les tests
  d’intégration, jamais sur une base existante ou de production.
- Une compilation de pilote ne prouve pas la portabilité du schéma.
- Ne pas annoncer la compatibilité d’un format sans fixture de référence et
  vérification d’import/export dans LimeSurvey 6.5.14.

## Git et restitution

Ne jamais créer de commit directement. Proposer un message au format :

```text
type(scope) #number: summary
```

Utiliser `#0000` si aucun ticket n’est connu. Expliquer ce qui a changé,
pourquoi, les vérifications exécutées et les limites restantes.
