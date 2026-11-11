# Bases de données et migrations

## Périmètre

SQLite, MariaDB et PostgreSQL sont des cibles acquises. Le squelette utilise
SeaORM avec ses pilotes SQLx ; la [documentation officielle][drivers] indique
les pilotes SQLite, PostgreSQL et MySQL/MariaDB disponibles. L’accès aux
questionnaires utilise des constructeurs SeaQuery et des paramètres liés.

Cela fournit une abstraction, pas une preuve que tous les futurs schémas et
requêtes auront la même sémantique. La portabilité devra être testée sur les
trois moteurs.

| Cible      | Rôle proposé            | État actuel       |
| ---------- | ----------------------- | ----------------- |
| SQLite     | Tests et développement  | Pilote par défaut |
| MariaDB    | Production, intégration | Pilote optionnel  |
| PostgreSQL | Production, intégration | Pilote optionnel  |

Les versions minimales de serveurs ne sont pas encore fixées. MariaDB 11.4 et
PostgreSQL 17 sont des bases de matrice proposées pour les premiers tests, à
adapter aux versions réellement exploitées.

## Activer un pilote

Le build par défaut inclut SQLite. Pour choisir un seul autre pilote :

```sh
cargo build -p rustsurvey-server --locked --no-default-features \
  --features db-postgres
```

```sh
cargo build -p rustsurvey-server --locked --no-default-features \
  --features db-mariadb
```

`--all-features` inclut les trois pilotes. Le choix par fonctionnalités Cargo
permet de limiter les dépendances du binaire destiné à un seul moteur.

Les URL utilisent les schémas `sqlite:`, `postgres://` ou `mysql://`, ce dernier
étant aussi utilisé pour MariaDB. Fournir les identifiants via l’environnement,
sans les écrire dans le dépôt ni les journaux.

## Mécanisme de migrations présent

`crates/migration` expose le migrateur. Le serveur appelle `Migrator::up` avant
d’écouter en HTTP, selon le mécanisme de [migration au démarrage][run].
L’historique est géré par SeaORM dans `seaql_migrations`.

La migration `m20261111_000001_authoring` crée les premières tables :

| Table                    | Contenu                                      |
| ------------------------ | -------------------------------------------- |
| `administrator`          | Compte unique, mot de passe Argon2id, échecs |
| `administrator_sessions` | Empreintes de sessions, CSRF, expiration     |
| `studies`                | Identité, titre, version courante, dates     |
| `definition_revisions`   | Documents natifs versionnés et immuables     |

Les révisions ont une clé composée étude/version et une clé étrangère vers
l’étude. Les sessions référencent l’administrateur et possèdent un index
d’expiration. Les instants utilisent des secondes Unix UTC en entier 64 bits.
L’historique SQL est distinct de la version du document natif.

La migration `m20261111_000002_import_sources` ajoute `import_sources`, liée par
clé étrangère à `studies`. Elle conserve le XML LSS original, son empreinte
SHA-256 et le rapport d’import. Le XML est privé et immuable. Étude, première
révision et source sont créées dans une même transaction. XML et rapport
utilisent `LONGTEXT` sur MariaDB, `TEXT` sur SQLite/PostgreSQL.

La migration 0002 ne modifie aucune révision existante. Le document v1 est
converti en v2 à la lecture ; seule une nouvelle sauvegarde ajoute une révision
v2.

La sauvegarde compare la version attendue et insère le document dans une même
transaction. Un conflit ne crée pas de révision. Aucun opérateur JSON SQL n’est
utilisé. Le compte n’est initialisé que si aucun administrateur n’existe.

SQLite fichier est persistant par défaut. `sqlite::memory:` et les URL
`mode=memory` gardent une connexion ouverte, sans expiration du pool, pour ne
pas perdre le schéma après une période d’inactivité. Les autres URL utilisent au
plus quatre connexions.

## Schéma métier conçu

Le [modèle interne](domain-model.md) définit un schéma commun aux études :
identités, relations, états et versions en colonnes relationnelles ; définitions
immuables et blocs de réponses en documents JSON typés et versionnés. Ces
documents sont stockés comme texte et validés par Rust.

Ce choix évite les tables de réponses créées dynamiquement par étude ainsi que
la dépendance aux opérateurs JSON d’un moteur. La création ou activation d’une
étude n’exécutera pas de migration du schéma SQL de RustSurvey.

Le type de stockage doit accepter les grands documents sur les trois moteurs. En
particulier, ne pas présumer qu’une colonne `TEXT` a partout la même capacité.
La migration utilise `LONGTEXT` pour MariaDB et `TEXT` pour SQLite/PostgreSQL.
Ce choix propre au moteur reste isolé dans le migrateur. Les tests vérifient un
document Unicode supérieur à 64 Kio sur les trois moteurs.

Les changements de format des documents ont leur propre versionnement et leur
stratégie de conversion. Ils ne se confondent ni avec les migrations SQL, ni
avec le `DBVersion` LimeSurvey. Études, révisions, sources LSS privées et
administration sont implémentées. Campagnes, réponses, participants et
transferts restent à réaliser.

## Automatisation et déploiement

Les migrations automatiques au démarrage conviennent à ce socle local à un
processus. L’exécution unique avant un déploiement à plusieurs instances et la
coordination des démarrages restent à implémenter. Ne pas supposer que deux
processus peuvent appliquer simultanément une évolution sans risque.

## Règles pour les futures évolutions

1. Concevoir le modèle de domaine avant la première table métier.
2. Ajouter une migration datée, explicite et ordonnée dans le migrateur.
3. Préférer SeaQuery pour décrire le schéma ; isoler tout SQL propre à un
   moteur.
4. Garder immuables les migrations déjà partagées ; corriger avec une nouvelle
   migration.
5. Vérifier création, mise à niveau et conservation des données sur les trois
   moteurs, à partir d’états connus.
6. Documenter les étapes non réversibles et la reprise après échec.

La [documentation des migrations][write] présente les opérations `up`/`down` et
les constructeurs de schéma. La politique de sauvegarde, de restauration et de
déploiement doit accompagner les premières évolutions de données. Un `down` qui
supprime une table ne constitue pas une restauration.

## Conventions de portabilité proposées

- Privilégier tables, index et contraintes relationnelles simples.
- Éviter les enums SQL, tableaux natifs, procédures et index spécifiques dans le
  cœur portable.
- Définir une représentation commune des identifiants et des dates UTC.
- Ne pas faire dépendre le métier d’un opérateur JSON propre à une base.
- Définir explicitement la normalisation et la comparaison des codes ; les
  collations ne doivent pas décider seules de l’identité métier.
- Vérifier les comportements `NULL`, les booléens, les longueurs de texte, les
  clés étrangères et les contraintes d’unicité.
- Éviter de supposer que tous les changements de schéma sont transactionnels de
  la même façon sur les trois moteurs.

Pour SQLite en mémoire, les tests devront contrôler le partage des connexions et
du schéma ; une connexion unique est le point de départ le plus simple. Une base
fichier temporaire convient mieux aux tests de concurrence.

## Vérifications du premier schéma

Le 11 novembre 2026, les tests ont été exécutés sur SQLite embarqué, PostgreSQL
17 et MariaDB 11.4 en conteneurs locaux jetables : installation vierge, deuxième
passage sans changement, connexion administrateur, documents UTF-8 de plus de 64
Kio, lecture publique et sauvegarde versionnée. Le deuxième incrément vérifie
également les deux migrations, un original XML de plus de 64 Kio, l’édition des
conditions et la protection des propriétés importées sur les trois moteurs.
SQLite vérifie la montée de 0001 vers 0002 avec un document v1 déjà présent, sa
projection v2 et la conservation de la révision historique.

Les tests courants utilisent SQLite mémoire, un temps et une entropie injectés.
Ils couvrent aussi sessions, révocation, expiration, CSRF, validation, limites
de corps, historique et concurrence entre deux modifications.

Le test externe est ignoré par défaut. Après avoir créé une base locale vide
nommée `rustsurvey_test`, exporter son URL sous `RUSTSURVEY_TEST_DATABASE_URL`,
puis exécuter :

```sh
cargo test -p rustsurvey-server --all-features --locked --test workspace \
  test_external_database_should_apply_migrations_and_roundtrip_large_documents \
  -- --ignored
```

Le test refuse une base déjà initialisée et ne supprime aucune table.
L’exécution est explicite ; aucun service ni conteneur n’est démarré par les
tests courants. Les montées de version avec données et les migrations de
documents seront testées avec le prochain schéma. Le `down` existe pour cette
migration mais n’est pas une procédure de sauvegarde/restauration.

[drivers]:
  https://www.sea-ql.org/SeaORM/docs/install-and-config/database-and-async-runtime/
[run]: https://www.sea-ql.org/SeaORM/docs/migration/running-migration/
[write]: https://www.sea-ql.org/SeaORM/docs/migration/writing-migration/
