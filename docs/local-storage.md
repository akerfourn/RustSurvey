# Données privées et stockage local

## Configuration de développement

Pour une nouvelle installation, `.env.example` propose :

```dotenv
DATABASE_URL=sqlite://.data/rustsurvey.db?mode=rwc
OBJECT_STORE_BACKEND=file
OBJECT_STORE_DIRECTORY=.data/media
```

Les chemins relatifs partent du répertoire courant du backend : la racine du
projet avec `npm run dev`. Les répertoires standards sont créés au démarrage.
Les métadonnées, droits, références et sessions restent en SQL.

```text
.data/
  rustsurvey.db
  media/
    <clé opaque de 64 caractères>/
      content
      metadata
```

`content` contient le fichier original brut. `metadata` porte le format interne
`rustsurvey-object-v1`, la taille et l’empreinte SHA-256. Le nom choisi par
l’utilisateur reste en SQL ; il ne devient pas un chemin de fichier.

`.data/` et `.private/` sont ignorés par Git et les outils de formatage/lint.
Vite refuse leur lecture directe, y compris via `/@fs/`. Les fichiers passent
uniquement par l’API qui contrôle leur accès. Sur Unix, les nouveaux répertoires
sont créés avec le mode 0700. Les fichiers du stockage objet sont créés avec le
mode 0600, soumis au masque de permissions du processus. Les permissions
existantes ne sont pas modifiées.

## Installation existante

Un `DATABASE_URL` explicite conserve son sens. Aucune base existante n’est
déplacée ou écrasée automatiquement. Sans cette variable, le serveur utilise
`.data/rustsurvey.db`, ou conserve `rustsurvey.db` à la racine si seule cette
ancienne base existe. Si les deux bases existent, il demande un choix explicite
avec `DATABASE_URL`.

Pour déplacer une ancienne base dans `.data`, arrêter toutes les instances qui
l’utilisent, conserver une sauvegarde, puis déplacer le fichier et ses éventuels
fichiers compagnons `-wal`/`-shm` sans écraser une destination existante.
Configurer ensuite le nouveau `DATABASE_URL` avant de redémarrer. Cette
opération de déploiement est distincte des migrations de schéma SQL.

Passer de `OBJECT_STORE_BACKEND=sql` à `file` conserve la lecture des objets
déjà enregistrés dans SQL. Les nouveaux envois vont dans le répertoire local,
sans duplication du binaire en SQL. Les anciens contenus ne sont ni copiés ni
supprimés automatiquement. Les références des questionnaires restent identiques.

Le mode fichiers lit d’abord le stockage local puis cherche une ancienne clé en
SQL seulement si elle est absente. Une corruption ou une indisponibilité locale
produit une erreur ; elle n’est pas masquée par une copie SQL.

Conserver la configuration du stockage avec la base. Revenir au mode SQL ne
recopie pas les nouveaux fichiers dans la base. Une sauvegarde complète en mode
fichiers conserve SQL **et** le répertoire de médias, y compris les objets de
l’historique et ceux dont l’accusé d’envoi est en attente.

## Écritures et reprise

Un objet est préparé dans un répertoire `.pending-*` privé. Contenu et
métadonnées sont écrits et synchronisés avant publication par renommage du
répertoire complet, sur le même système de fichiers. Une clé existante n’est pas
remplacée. Un échec normal nettoie le répertoire temporaire de cet envoi.

Un arrêt brutal peut conserver un répertoire `.pending-*` ; il n’est pas servi
comme objet. Le nouvel essai garde le même identifiant d’envoi. Si la
publication avait réussi avant la perte de l’accusé SQL, taille et empreinte
permettent de terminer l’envoi sans écrasement. Il s’agit d’un renvoi du fichier
complet, pas d’une reprise à l’octet.

Les lectures vérifient l’intégrité. `head` lit seulement les métadonnées et la
taille du fichier. `get` lit le contenu entier avant d’extraire une plage, dans
la limite commune de 8 Mio. L’I/O et le hachage local passent par
`spawn_blocking`, avec quatre opérations simultanées par adaptateur ; le permis
reste dans le travail même après annulation de la requête.

Les chemins racine sont configurés par l’opérateur et doivent rester privés. Les
liens symboliques aux emplacements d’objets contrôlés sont refusés. Aucun nom
fourni par le navigateur ne sert à construire un chemin.

## Périmètre

Les mêmes images natives, règles d’accès, historiques et reprises sont utilisés
avec les adaptateurs SQL, mémoire et fichiers. Aucun changement de schéma SQL ni
de format de questionnaire n’est nécessaire pour cet adaptateur.

Les tests couvrent contrat commun, redémarrage, concurrence, corruption,
permissions Unix, liens symboliques, nettoyage des échecs, compatibilité avec
les anciens objets SQL et refus HTTP des données privées par Vite.

Le contrôle HTTP complet se relance avec une base et des médias jetables :

```sh
npm run test:storage
```

Il vérifie la création de `.data`, les anciens objets SQL, les nouveaux
fichiers, les plages d’octets et les redémarrages avec les deux emplacements
SQLite. Il n’utilise pas la base, le compte ou les médias de l’installation
courante.

Le [mode S3](s3-storage.md) est disponible sans disque pour les binaires. La
copie des anciens objets fichiers vers S3 reste explicite et son outil reste à
réaliser. Streaming de gros fichiers, collecte des pièces jointes, import des
médias LimeSurvey et nettoyage des objets inutilisés restent à réaliser. La
[décision 0002](decisions/0002-object-storage.md) décrit le contrat commun.
