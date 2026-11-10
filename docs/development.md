# Environnement de développement

## Outillage constaté

Inventaire effectué sur cette machine Linux le **10 novembre 2026**.

| Outil          | Version constatée | Usage                   |
| -------------- | ----------------- | ----------------------- |
| Rust / Cargo   | 1.97.1 / 1.97.1   | Backend                 |
| rustfmt        | 1.9.0             | Formatage Rust          |
| Clippy         | 0.1.97            | Analyse Rust            |
| Node.js        | 22.15.0           | Build client            |
| npm            | 11.11.0           | Outils client           |
| Compilateur C  | GCC 13.3.0        | SQLite embarqué         |
| Docker         | 29.9.3            | Tests locaux, optionnel |
| Docker Compose | 5.7.0             | Tests multi-bases       |

rust-analyzer est également installé. pnpm est disponible mais n’est pas
nécessaire : le dépôt utilise npm pour limiter les outils imposés.

Les CLI `sqlite3` et `mariadb`/`mysql` ne sont pas présentes. Elles ne sont pas
nécessaires au squelette : les pilotes Rust assurent les connexions et SQLite
est embarqué. Un client `psql` est disponible. Aucun serveur de base externe ne
doit être installé pour démarrer avec SQLite.

## Versions et dépendances

`rust-toolchain.toml` fixe Rust 1.97.1 avec rustfmt et Clippy. `Cargo.lock` et
`package-lock.json` fixent les résolutions de dépendances. Ne pas les mettre à
jour automatiquement dans un changement sans rapport.

Node.js 24 fait partie des [branches LTS][node-releases] et reste la cible
recommandée dans `.nvmrc`. Les versions acceptées sont déclarées dans
`package.json` ; Node.js 22.15.0 est utilisable sur cette machine. La contrainte
Vite est consultable dans son [guide officiel][vite].

TypeScript 6.0.3 est retenu pour rester dans la plage annoncée par
`typescript-eslint` 8.72.2 (`>=4.8.4 <6.1.0` dans ses [métadonnées
publiées][ts-eslint]). La version TypeScript ne doit pas être montée
indépendamment de la chaîne de lint. Les choix sont détaillés dans la [décision
technique][stack].

## Diagnostic et installation

Depuis la racine :

```sh
node scripts/doctor.mjs
npm ci
```

Le script de diagnostic affiche les versions et signale les outils absents. Il
ne compare pas les versions aux contraintes et ne lance pas de conteneur. Les
contraintes Node/npm sont appliquées lors de l’installation et la chaîne Rust
est fixée par rustup. Le diagnostic du compilateur C est adapté à un
environnement POSIX ; une prise en charge Windows reste à vérifier.

Aucun nouvel outil global n’est requis sur la machine inventoriée. Les outils
TypeScript, Vite, ESLint et Prettier sont installés localement par `npm ci`.

## Commandes disponibles

| Commande               | Effet                           |
| ---------------------- | ------------------------------- |
| `npm run dev`          | Développement Vite              |
| `npm run build`        | TypeScript et build statique    |
| `npm run lint`         | ESLint, aucun avertissement     |
| `npm run typecheck`    | TypeScript sans génération      |
| `npm test`             | Moteur client, sans navigateur  |
| `npm run format`       | Formatage Prettier              |
| `npm run format:check` | Contrôle Prettier               |
| `npm run check`        | Formatage, lint, tests et build |
| `npm run check:rust`   | Contrôles Rust, tous pilotes    |
| `npm run check:all`    | Vérifications client et Rust    |

Les tests Rust couvrent l’adaptateur LSS, sa CLI, la projection du viewer et
l’API HTTP d’inspection. Les tests TypeScript couvrent les expressions,
dépendances, validations et substitutions du moteur local. Ils sont isolés et ne
nécessitent aucun service distant.

## Démarrage local

```sh
cargo run -p rustsurvey-server --locked
```

Dans un autre terminal :

```sh
npm run dev
```

- Client : <http://127.0.0.1:5173>.
- API de disponibilité : <http://127.0.0.1:3000/api/health>.
- SQLite : `rustsurvey.db` créé dans le répertoire courant, ignoré par Git.

La page permet d’importer un `.lss` et d’essayer le [viewer](viewer.md). Son
upload et le lien de contrôle utilisent le proxy Vite vers l’API ; les réponses
sont uniquement en mémoire dans le navigateur. Le serveur gère Ctrl+C et, sur
Unix, SIGTERM pour terminer proprement.

## Configuration actuelle

| Variable           | Défaut                            |
| ------------------ | --------------------------------- |
| `BIND_ADDRESS`     | `127.0.0.1:3000`                  |
| `DATABASE_URL`     | `sqlite://rustsurvey.db?mode=rwc` |
| `API_PROXY_TARGET` | `http://127.0.0.1:3000`           |

`BIND_ADDRESS` définit l’IP et le port du serveur. `DATABASE_URL` définit la
connexion de base ; le défaut SQLite permet de créer le fichier.
`API_PROXY_TARGET` configure seulement le proxy de développement Vite.

Le serveur ne charge pas `.env` automatiquement. `.env.example` est une
référence à exporter dans l’environnement du processus, sans secrets réels. Les
valeurs ne sont pas injectées dans le bundle client.

Exemple avec une autre adresse d’API :

```sh
BIND_ADDRESS=127.0.0.1:4000 cargo run -p rustsurvey-server --locked
```

Dans l’autre terminal :

```sh
API_PROXY_TARGET=http://127.0.0.1:4000 npm run dev
```

`web/dist` contient le build statique. Son hébergement et le routage `/api` en
production restent à préparer ; `vite preview` n’est pas un déploiement.

## Vérifications et limites

Bilan du socle et du premier viewer, le 10 novembre 2026 :

| Contrôle                        | Résultat                                 |
| ------------------------------- | ---------------------------------------- |
| Outillage                       | Outils requis présents                   |
| Formatage, ESLint, TypeScript   | Réussis                                  |
| Compilation Rust, trois pilotes | Réussie                                  |
| Rustfmt et Clippy               | Aucun avertissement                      |
| Tests Cargo                     | Adaptateur, CLI, projection et API       |
| Tests TypeScript                | Moteur et validations locales            |
| SQLite                          | Démarrage et redémarrage vérifiés        |
| HTTP, Vite et proxy             | Réponses locales vérifiées               |
| SIGTERM                         | Arrêt propre                             |
| PostgreSQL/MariaDB              | Exécution non testée                     |
| LimeSurvey                      | Lecture LSS et réexport original         |
| Viewer                          | Parcours HTTP vérifié, visuel à vérifier |

Le contrôle SQLite a également confirmé que seule la table vide d’historique des
migrations est créée. Les pilotes PostgreSQL/MariaDB ont été compilés sans
serveur externe ; cela ne valide ni un schéma multi-bases ni les futures
migrations métier.

L’[audit des sources](limesurvey-source-audit.md) identifie des fixtures amont
et leurs versions. La certification d’échange, la CI et les tests de migrations
métier multi-bases restent à réaliser. Le dépôt n’impose pour l’instant aucun
hébergeur Git ni service de CI particulier.

[vite]: https://vite.dev/guide/
[node-releases]: https://nodejs.org/en/about/previous-releases
[ts-eslint]: https://registry.npmjs.org/typescript-eslint/8.72.2
[stack]: decisions/0001-technology-stack.md
