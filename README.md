# RustSurvey

Un moteur de questionnaires en Rust, conçu pour limiter les ressources
nécessaires côté serveur et échanger des fichiers avec **LimeSurvey 6.5.14**. Le
navigateur prend en charge l’affichage et les interactions ; le serveur reste
responsable des autorisations, de la validation et de la persistance.

## État du projet

**Phase de cadrage et de documentation.** Le dépôt contient un squelette
technique minimal, pas encore une application de questionnaires :

- un serveur Rust avec un contrôle de disponibilité HTTP/base de données ;
- une connexion SQLite par défaut et des pilotes PostgreSQL/MariaDB activables ;
- un point d’entrée pour les migrations, sans migration métier ;
- une page statique Preact/TypeScript et les outils de formatage/vérification.

L’éditeur, les sessions de réponse, la collecte et les imports/exports ne sont
pas implémentés. Les propositions d’architecture ne constituent pas une promesse
de compatibilité ou de performance déjà vérifiée.

## Contraintes retenues

- Backend Rust minimal, avec peu de dépendances et des ressources bornées.
- Client TypeScript avec Preact, choix de référence comme pour LiteCloud.
- Ajouter une couche technique seulement pour un besoin concret et documenté.
- SQLite, MariaDB et PostgreSQL, avec des migrations automatisées.
- Compatibilité **des fichiers**, en priorité `.lss`, avec LimeSurvey
  `6.5.14+240624`, dont le schéma de référence est `DBVersion = 623`. Voir la
  [référence amont][ls-version] et le [contrat de compatibilité].
- Validation serveur obligatoire, même lorsqu’une règle est évaluée côté client.

## Documentation

Commencer par l’[index de documentation] et les [besoins du projet].

| Document               | Contenu                     |
| ---------------------- | --------------------------- |
| [Architecture]         | Client, serveur et sessions |
| [Compatibilité]        | Formats et vérifications    |
| [Bases de données]     | Portabilité et migrations   |
| [Développement]        | Outillage et commandes      |
| [Choix techniques]     | Alternatives et compromis   |
| [Feuille de route]     | Prochaines étapes           |
| [AGENTS.md](AGENTS.md) | Consignes de contribution   |

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

Les prochains travaux doivent d’abord s’appuyer sur le cadrage et sur des
exports de référence anonymisés. Garder les changements ciblés et proposer un
message de commit ; ne pas créer de commit automatiquement.

La licence de RustSurvey reste à choisir avant publication. L’étude du format
LimeSurvey n’implique pas de reprendre son code, ses thèmes ou ses extensions.

[index de documentation]: docs/README.md
[besoins du projet]: docs/requirements.md
[Architecture]: docs/architecture.md
[contrat de compatibilité]: docs/limesurvey-compatibility.md
[Bases de données]: docs/database.md
[Développement]: docs/development.md
[Choix techniques]: docs/decisions/0001-technology-stack.md
[Feuille de route]: docs/roadmap.md
[ls-version]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/config/version.php
[Compatibilité]: docs/limesurvey-compatibility.md
