# 0001 — Pile technique initiale

## Statut

**Base retenue : Rust minimal et Preact**, dans la continuité de LiteCloud.
Conserver cette approche sauf besoin particulier documenté. TypeScript, les
trois bases et les migrations restent des contraintes acquises.

Les crates précises du squelette répondent aux besoins d’infrastructure
identifiés. Les alternatives ci-dessous explicitent les compromis ; elles ne
constituent pas un chantier de changement de pile.

## Contexte

L’objectif principal est la sobriété du backend, avec une maintenance accessible
à une petite équipe. Le navigateur dispose de ressources suffisantes pour les
interactions du questionnaire. Éviter les services annexes et limiter les
abstractions à des besoins identifiés.

Privilégier la bibliothèque standard et des crates ciblées, puis ajouter une
couche seulement lorsqu’un besoin concret le justifie. Cette décision reprend
l’approche de LiteCloud ; elle ne suppose pas connaître sa liste exacte de
dépendances, qui n’est pas disponible dans ce dépôt.

## HTTP : Axum et Tokio

| Option     | Avantage                     | Coût                          |
| ---------- | ---------------------------- | ----------------------------- |
| Axum/Tokio | Infrastructure async commune | Tâches à borner               |
| Actix Web  | Solution intégrée            | Autres conventions de runtime |

Axum 0.8.9 et Tokio 1.54.1 sont utilisés dans le squelette. Le runtime est
configuré sur un thread pour commencer ; ce choix ne dispense pas d’isoler les
traitements bloquants. Aucun résultat de performance comparatif n’est
revendiqué.

## Client : Preact et Vite

| Option | Avantage              | Coût                          |
| ------ | --------------------- | ----------------------------- |
| Preact | Runtime client réduit | Intégrations React à vérifier |
| React  | Écosystème plus large | Runtime client plus important |

Preact est le choix client de référence. Le squelette utilise Preact 11.2.0 avec
Vite 8.3.7 et TypeScript 6.0.3. Le [guide Preact][preact] présente l’intégration
Vite et les points d’intégration avec l’écosystème React.

Le gain de taille du client est secondaire au coût serveur. Un passage à React
devrait répondre à un besoin de l’éditeur que Preact ne satisfait pas. Aucun kit
UI, routeur, gestionnaire d’état global ou framework SSR n’est ajouté
maintenant.

## Données : SeaORM, SeaQuery et SQLx

| Option          | Avantage                  | Coût                       |
| --------------- | ------------------------- | -------------------------- |
| SeaORM/SeaQuery | Schéma commun aux moteurs | Abstraction supplémentaire |
| SQLx direct     | Contrôle explicite du SQL | Variantes SQL à maintenir  |

SeaORM et `sea-orm-migration` 2.0.4 sont retenus provisoirement pour vérifier la
connexion et préparer le point d’entrée des migrations. Les pilotes et runtimes
sont décrits dans la [documentation officielle][seaorm].

Les options par défaut sont désactivées dans les dépendances directes, puis les
pilotes sont activés via les fonctionnalités Cargo du projet. SQLite est le
pilote par défaut ; PostgreSQL et MariaDB sont optionnels. Rustls est activé
pour les connexions aux moteurs externes.

L’ORM est justifié par la contrainte des trois moteurs et des migrations. Son
coût de compilation, sa requêtologie et sa consommation à l’exécution devront
être observés avant de généraliser son usage. Une optimisation ciblée pourra
utiliser SQLx derrière la même frontière d’accès aux données.

## Lecture des fichiers XML

L’adaptateur `.lss` utilise `quick-xml` 0.42 sans options additionnelles. La
lecture événementielle fournit les positions nécessaires pour préserver les
données inconnues et limiter les éditions aux champs demandés. Les limites, le
modèle de lecture et les tests sont décrits dans le [module de
compatibilité][limesurvey]. La dépendance ne remplace pas les contrôles
explicites du profil LimeSurvey.

## Outils de dépôt

Cargo gère Rust ; npm gère un unique workspace `web`. Rustfmt et Clippy assurent
les contrôles Rust ; TypeScript strict, ESLint et Prettier assurent ceux du
client. Les lockfiles sont versionnés.

Ne pas introduire un second gestionnaire de paquets ou un outil de monorepo tant
que ces commandes suffisent. Aucun CLI ORM global n’est nécessaire pour le
squelette.

## Validation et réexamen

Valider les choix d’infrastructure avec le prototype d’import/export, la
première matrice de migrations et les mesures de charge représentatives.
Réexaminer une dépendance seulement si ces essais ou un besoin fonctionnel
révèlent une limite concrète. Les critères restent la lisibilité, la fiabilité
et le coût réel du serveur.

[preact]: https://preactjs.com/guide/v11/getting-started/
[limesurvey]: ../../crates/limesurvey/README.md
[seaorm]:
  https://www.sea-ql.org/SeaORM/docs/install-and-config/database-and-async-runtime/
