# Bases de données et migrations

## Périmètre

SQLite, MariaDB et PostgreSQL sont des cibles acquises. Le squelette utilise
SeaORM avec ses pilotes SQLx ; la [documentation officielle][drivers] indique
les pilotes SQLite, PostgreSQL et MySQL/MariaDB disponibles.

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

La liste des migrations est actuellement **vide**. Le squelette ne définit
aucune table métier, aucun modèle ORM de questionnaires ni commande de rollback.
Il permet seulement d’initialiser le mécanisme et de vérifier la connexion.

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
Les migrations isoleront les éventuels choix de type adaptés au moteur ; tester
aussi un document supérieur à 64 Kio.

Les changements de format des documents ont leur propre versionnement et leur
stratégie de conversion. Ils ne se confondent ni avec les migrations SQL, ni
avec le `DBVersion` LimeSurvey. Aucune de ces tables n’est encore créée.

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

## Vérifications à ajouter avec le premier schéma

La matrice d’intégration devra prouver au minimum : installation vierge,
deuxième passage sans changement, mise à niveau avec données, aller-retour UTF-8
et contraintes de relations. Les tests de réversibilité utiliseront uniquement
des bases jetables et des fixtures synthétiques.

Une migration qui compile pour trois pilotes n’est pas encore une migration
validée sur trois bases. Cette matrice fait partie de la prochaine étape de
développement, pas des fonctionnalités achevées du squelette.

[drivers]:
  https://www.sea-ql.org/SeaORM/docs/install-and-config/database-and-async-runtime/
[run]: https://www.sea-ql.org/SeaORM/docs/migration/running-migration/
[write]: https://www.sea-ql.org/SeaORM/docs/migration/writing-migration/
