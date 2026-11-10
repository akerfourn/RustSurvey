# RustSurvey

Un moteur de questionnaires en Rust, conçu pour limiter les ressources
nécessaires côté serveur et échanger des fichiers avec **LimeSurvey 6.5.14**. Le
navigateur prend en charge l’affichage et les interactions ; le serveur reste
responsable des autorisations, de la validation et de la persistance.

## État du projet

**Début d’implémentation.** Le dépôt contient un premier adaptateur `.lss`
utilisable en bibliothèque et en ligne de commande, ainsi que le socle web :

- lecture bornée, diagnostics et modèle de lecture du schéma LimeSurvey 623 ;
- réexport de l’original à l’octet près et modification ciblée d’un titre ;
- un serveur Rust avec un contrôle de disponibilité HTTP/base de données ;
- une connexion SQLite par défaut et des pilotes PostgreSQL/MariaDB activables ;
- un point d’entrée pour les migrations, sans migration métier ;
- une page statique Preact/TypeScript et les outils de formatage/vérification.

L’éditeur, les sessions de réponse, la collecte, la persistance métier et les
archives `.lsa` restent à implémenter. Les tests de l’adaptateur ne constituent
pas encore une certification d’échange avec une instance LimeSurvey 6.5.14.

## Essayer l’adaptateur de fichiers

Inspecter la fixture synthétique fournie :

```sh
cargo run -p rustsurvey-limesurvey --locked -- inspect \
  crates/limesurvey/tests/fixtures/minimal-623.lss
```

La sortie JSON décrit les contrôles effectués et les limites de compatibilité,
sans afficher les textes du questionnaire. Voir les
[commandes d’édition et de réexport](crates/limesurvey/README.md).

## Contraintes retenues

- Backend Rust minimal, avec peu de dépendances et des ressources bornées.
- Client TypeScript avec Preact, choix de référence comme pour LiteCloud.
- Ajouter une couche technique seulement pour un besoin concret et documenté.
- SQLite, MariaDB et PostgreSQL, avec des migrations automatisées.
- Compatibilité **des fichiers** `.lss` et `.lsa` avec LimeSurvey
  `6.5.14+240624`, dont le schéma de référence est `DBVersion = 623`. Voir la
  [référence amont][ls-version] et le [contrat de compatibilité].
- Migration d’études en cours avec conservation des données, du comportement et
  des moyens de reprise nécessaires ; le protocole est à implémenter.
- Validation serveur obligatoire, même lorsqu’une règle est évaluée côté client.

## Documentation

Commencer par l’[index de documentation] et les [besoins du projet].

| Document               | Contenu                          |
| ---------------------- | -------------------------------- |
| [Architecture]         | Client, serveur et sessions      |
| [Modèle interne]       | Études, campagnes et réponses    |
| [Migration d’études]   | Transfert et continuité          |
| [Audit LimeSurvey]     | Constats dans les sources 6.5.14 |
| [Compatibilité]        | Formats et vérifications         |
| [Bases de données]     | Portabilité et migrations        |
| [Développement]        | Outillage et commandes           |
| [Choix techniques]     | Alternatives et compromis        |
| [Feuille de route]     | Prochaines étapes                |
| [AGENTS.md](AGENTS.md) | Consignes de contribution        |

## Vérifier l’environnement

Rust `1.97.1` avec rustfmt et Clippy, un compilateur C, Node.js et npm sont
nécessaires. Node.js 24 LTS est recommandé ; la version 22.15.0 présente sur
cette machine satisfait aussi les contraintes du squelette. Docker est optionnel
et servira aux futurs tests PostgreSQL/MariaDB.

```sh
node scripts/doctor.mjs
npm ci
npm run check:all
```

Le diagnostic vérifie la présence des outils. Les versions attendues et les
limites de cette vérification sont détaillées dans [Développement].

## Démarrer le squelette

Depuis la racine, dans deux terminaux :

```sh
cargo run -p rustsurvey-server --locked
```

```sh
npm run dev
```

Ouvrir <http://127.0.0.1:5173>. Le lien de la page appelle `/api/health` via le
proxy de développement. Un résultat `200 ok` indique que le serveur peut joindre
la base ; un résultat `503 unavailable` indique un échec.

Par défaut, le serveur crée `rustsurvey.db` dans le répertoire courant et
initialise l’historique des migrations. Aucune table de questionnaires n’est
créée. `.env.example` décrit les deux variables de configuration ; les fichiers
`.env` ne sont pas chargés automatiquement.

## Contribution et licence

Les prochains travaux s’appuient sur le modèle documenté et sur les sources de
la version cible. Le corpus amont permet de commencer sans export utilisateur ;
des fichiers de production anonymisés serviront ensuite à valider les usages
réels. Garder les changements ciblés et proposer un message de commit ; ne pas
créer de commit automatiquement.

La licence de RustSurvey reste à choisir avant publication. L’étude du format
LimeSurvey n’implique pas de reprendre son code, ses thèmes ou ses extensions.

[index de documentation]: docs/README.md
[besoins du projet]: docs/requirements.md
[Architecture]: docs/architecture.md
[Modèle interne]: docs/domain-model.md
[Migration d’études]: docs/study-migration.md
[Audit LimeSurvey]: docs/limesurvey-source-audit.md
[contrat de compatibilité]: docs/limesurvey-compatibility.md
[Bases de données]: docs/database.md
[Développement]: docs/development.md
[Choix techniques]: docs/decisions/0001-technology-stack.md
[Feuille de route]: docs/roadmap.md
[ls-version]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/config/version.php
[Compatibilité]: docs/limesurvey-compatibility.md
