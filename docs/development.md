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

`rust-toolchain.toml` sélectionne le canal `stable` avec rustfmt et Clippy, sans
imposer une version exacte du compilateur. `rust-version = "1.97.1"` dans
`Cargo.toml` déclare la version minimale prise en charge ; les versions stables
plus récentes sont acceptées. Les plages comme `^1.97.1` ne sont pas admises
dans ces réglages.

Ce choix facilite le développement avec une chaîne stable déjà installée. Une
mise à jour peut ajouter des avertissements Clippy et faire échouer le contrôle
`-D warnings` ; relancer les vérifications après chaque mise à jour. Les builds
ne garantissent pas un compilateur identique entre machines. Les versions du
tableau ci-dessus décrivent uniquement l’inventaire historique.

`Cargo.lock` et `package-lock.json` fixent les résolutions de dépendances. Ne
pas les mettre à jour automatiquement dans un changement sans rapport.

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
contraintes Node/npm sont appliquées lors de l’installation et rustup
sélectionne la chaîne Rust stable. Le diagnostic du compilateur C est adapté à
un environnement POSIX ; une prise en charge Windows reste à vérifier.

Aucun nouvel outil global n’est requis sur la machine inventoriée. Les outils
TypeScript, Vite, ESLint et Prettier sont installés localement par `npm ci`.

## Commandes disponibles

| Commande               | Effet                           |
| ---------------------- | ------------------------------- |
| `npm run dev`          | Backend Rust et client Vite     |
| `npm run dev:web`      | Client Vite seul                |
| `npm run build`        | TypeScript et build statique    |
| `npm run lint`         | ESLint, aucun avertissement     |
| `npm run typecheck`    | TypeScript sans génération      |
| `npm test`             | Moteur client, sans navigateur  |
| `npm run format`       | Formatage Prettier              |
| `npm run format:check` | Contrôle Prettier               |
| `npm run check`        | Formatage, lint, tests et build |
| `npm run check:rust`   | Contrôles Rust, tous pilotes    |
| `npm run check:all`    | Vérifications client et Rust    |
| `npm run test:storage` | HTTP et stockage local jetables |

Les tests Rust couvrent l’adaptateur LSS, sa CLI, la projection du viewer et
l’API HTTP d’inspection, le domaine natif, les sessions et les sauvegardes SQL.
Les tests TypeScript couvrent les expressions, dépendances, validations et
substitutions du moteur local. Ils sont isolés et ne nécessitent aucun service
distant. Le test externe optionnel décrit dans [Bases de données](database.md)
utilise une base locale jetable fournie explicitement. Le
[contrôle du stockage local](local-storage.md) démarre des processus HTTP avec
SQLite et des médias jetables, sans toucher aux données de développement.

## Démarrage local

```sh
npm run dev
```

Cette commande lance le backend, attend son signal de disponibilité puis démarre
Vite avec la bonne cible de proxy. Ctrl+C arrête les deux services. Le lanceur
utilise les bibliothèques standard Node et l’API Vite existante ; aucune
dépendance supplémentaire n’est nécessaire.

- Client : adresse affichée par Vite, <http://127.0.0.1:5173> par défaut.
- API de disponibilité : adresse affichée sous `RustSurvey API`, puis
  `/api/health` ; <http://127.0.0.1:3000/api/health> par défaut.
- SQLite : `.data/rustsurvey.db` pour une nouvelle installation.
- Mode fichiers de l’exemple : `.data/media/`.
- Données privées ignorées par Git et refusées par le serveur Vite.

La page présente les questionnaires publics. **Administration** ouvre le premier
éditeur natif après connexion ; la configuration `.env` du compte est décrite
dans le [README](../README.md). **Essayer un LSS** ouvre le [viewer](viewer.md).
Son upload et le lien de contrôle utilisent le proxy Vite vers l’API ; les
réponses sont uniquement en mémoire dans le navigateur. Le serveur gère Ctrl+C
et, sur Unix, SIGTERM pour terminer proprement.

Si le port 5173 est occupé, Vite choisit automatiquement le prochain port
disponible et affiche l’adresse retenue dans le terminal.

Sans `BIND_ADDRESS`, le backend de développement essaie jusqu’à 100 ports à
partir de 3000, uniquement en cas d’adresse déjà utilisée. L’IP reste inchangée.
Le proxy reçoit le port réellement ouvert. Le signal d’adresse utilise
uniquement la sortie standard du processus enfant, sans fichier de découverte
persistant.

Un `BIND_ADDRESS` explicite, ainsi que les binaires compilés en mode release,
conservent un port strict : un conflit arrête le démarrage.

## Configuration actuelle

| Variable                   | Défaut                                                                   |
| -------------------------- | ------------------------------------------------------------------------ |
| `BIND_ADDRESS`             | `127.0.0.1:3000`                                                         |
| `DATABASE_URL`             | `sqlite://.data/rustsurvey.db?mode=rwc`, ou ancienne base racine seule   |
| `OBJECT_STORE_BACKEND`     | `sql` sans réglage ; `file` dans `.env.example`                          |
| `OBJECT_STORE_DIRECTORY`   | `.data/media`, utilisé en mode fichiers                                  |
| `API_PROXY_TARGET`         | Découvert par `npm run dev`                                              |
| `ADMIN_USERNAME`           | `admin`, pour le premier compte                                          |
| `ADMIN_PASSWORD`           | Aucun ; initialisation explicite du compte                               |
| `ADMIN_COOKIE_SECURE`      | `false` en debug, `true` en release                                      |
| `DEV_ALLOW_WEAK_PASSWORDS` | `false` ; autorise un mot de passe court pour le premier compte en debug |

`BIND_ADDRESS` définit l’IP et le port du serveur. `DATABASE_URL` définit la
connexion de base ; le défaut SQLite permet de créer le fichier. Avec
`npm run dev`, un `API_PROXY_TARGET` explicite utilise un backend déjà démarré
et lance uniquement Vite. Avec `npm run dev:web`, cette variable configure le
proxy Vite ; son défaut est alors `http://127.0.0.1:3000`.

Le backend Rust charge automatiquement le `.env` de son répertoire courant. Avec
`npm run dev`, c’est la racine du projet. Copier `.env.example` vers `.env`
suffit pour les tests locaux : compte `admin`, mot de passe
`rustsurvey-dev-only`. Ces identifiants publics doivent être remplacés hors des
tests locaux ; `.env` est ignoré par Git.

Pour un mot de passe court, ajouter `DEV_ALLOW_WEAK_PASSWORDS=true` dans ce
fichier. Ce flag s’applique à la création initiale en build debug et est refusé
en release. Le mot de passe reste non vide, limité à 1024 octets et haché en SQL
; un compte existant conserve son mot de passe.

L’environnement du processus prend priorité sur le fichier, y compris pour les
valeurs vides. Le chargement n’altère pas l’environnement global et ne transmet
pas les valeurs au frontend. Un fichier absent reste facultatif ; un fichier
illisible ou invalide arrête le serveur avec une erreur sans secrets. Seul
`.env` est chargé, sans recherche dans les répertoires parents ni variantes
automatiques `.env.local` ou `.env.production`. Le lanceur et Vite conservent
leur configuration propre, notamment `API_PROXY_TARGET` dans l’environnement.

Le serveur crée automatiquement les tables de définitions et d’administration.
Le mode `DATABASE_URL=sqlite::memory:` les conserve uniquement jusqu’à l’arrêt,
avec un pool d’une connexion. Le mode fichier par défaut conserve
questionnaires, compte et sessions ; modifier les variables de bootstrap ne
remplace pas le compte existant. Voir le [périmètre de l’éditeur](authoring.md).

Exemple avec une autre adresse d’API :

```sh
BIND_ADDRESS=127.0.0.1:4000 cargo run -p rustsurvey-server --locked
```

Dans l’autre terminal, pour lancer uniquement le client :

```sh
API_PROXY_TARGET=http://127.0.0.1:4000 npm run dev:web
```

`web/dist` contient le build statique. Son hébergement et le routage `/api` en
production restent à préparer ; `vite preview` n’est pas un déploiement.

## Vérifications et limites

Bilan complété avec le stockage et l’administration le 11 novembre 2026 :

| Contrôle                        | Résultat                                  |
| ------------------------------- | ----------------------------------------- |
| Outillage                       | Outils requis présents                    |
| Formatage, ESLint, TypeScript   | Réussis                                   |
| Compilation Rust, trois pilotes | Réussie                                   |
| Rustfmt et Clippy               | Aucun avertissement                       |
| Tests Cargo                     | Adaptateur, CLI, projection et API        |
| Tests TypeScript                | Moteur et validations locales             |
| SQLite                          | Démarrage et redémarrage vérifiés         |
| HTTP, Vite et proxy             | Réponses locales vérifiées                |
| SIGTERM                         | Arrêt propre                              |
| PostgreSQL 17/MariaDB 11.4      | Migration et écritures sur bases jetables |
| LimeSurvey                      | Lecture LSS et réexport original          |
| Viewer                          | Parcours HTTP vérifié, visuel à vérifier  |

Les tests des trois moteurs vérifient la première migration, son deuxième
passage, les documents UTF-8 supérieurs à 64 Kio et les modifications
concurrentes avec une seule sauvegarde gagnante. SQLite couvre aussi
l’authentification, CSRF, historique, expiration et révocation. Les sessions et
définitions ont été relues après redémarrage du processus HTTP sur une base
fichier temporaire, à travers le proxy Vite.

Le lot d’import persistant vérifie aussi l’archive XML privée, ses octets
exacts, la conversion v1 vers v2 et les conditions éditées. Le corpus de syntaxe
des règles est partagé entre Rust et TypeScript. Une comparaison entre
inspection LSS, définition enregistrée et projection client couvre les 28 types,
deux langues, les options, règles et sous-questions ; elle a aussi été effectuée
en mémoire sur la fixture locale privée sans journaliser ses textes. Les
sessions, conditions et originaux ont été relus après redémarrage du processus
HTTP.

L’hôte d’automatisation du navigateur n’est pas disponible sur cette machine.
Les contrôles HTTP, les tests du moteur et le build TypeScript ne constituent
pas une validation visuelle du parcours d’édition.

L’[audit des sources](limesurvey-source-audit.md) identifie des fixtures amont
et leurs versions. La certification d’échange, la CI et les futurs tests de
montée de version avec données restent à réaliser. Le dépôt n’impose pour
l’instant aucun hébergeur Git ni service de CI particulier.

[vite]: https://vite.dev/guide/
[node-releases]: https://nodejs.org/en/about/previous-releases
[ts-eslint]: https://registry.npmjs.org/typescript-eslint/8.72.2
[stack]: decisions/0001-technology-stack.md

Le [stockage local](local-storage.md) détaille les chemins, la conservation des
bases existantes, les objets fichiers et la reprise des anciens médias SQL.
