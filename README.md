# RustSurvey

Un moteur de questionnaires en Rust, conçu pour limiter les ressources
nécessaires côté serveur et échanger des fichiers avec **LimeSurvey 6.5.14**. Le
navigateur prend en charge l’affichage et les interactions ; le serveur reste
responsable des autorisations, de la validation et de la persistance.

## État du projet

**Premier parcours web fonctionnel.** Le dépôt permet d’importer un `.lss`,
d’explorer ses informations et d’essayer une saisie locale sans compte :

- lecture bornée, diagnostics et modèle de lecture du schéma LimeSurvey 623 ;
- réexport de l’original à l’octet près et modification ciblée d’un titre ;
- upload et analyse XML côté Rust, avec limite de 8 Mio ;
- titres, langues, groupes, questions, traductions et choix de réponse ;
- viewer Preact avec navigation par groupe et changement de langue ;
- réponses en mémoire dans l’onglet, téléchargeables en JSON ;
- premier moteur client de pertinence et de validation, avec diagnostics ;
- une connexion SQLite par défaut et des pilotes PostgreSQL/MariaDB activables ;
- un point d’entrée pour les migrations, sans migration métier ;
- un exemple synthétique bilingue pour essayer le parcours immédiatement.

Les réponses ne sont jamais envoyées au serveur. L’éditeur, les sessions de
réponse durables, la collecte, la persistance métier et les archives `.lsa`
restent à implémenter. Le moteur d’aperçu couvre un sous-ensemble explicite ; il
ne certifie pas encore le comportement complet d’une étude LimeSurvey. Voir le
[périmètre du viewer](docs/viewer.md).

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

## Essayer le viewer

Depuis la racine, une commande lance Rust puis Vite :

```sh
npm run dev
```

Le backend essaie le port 3000 puis les suivants en développement. Le lanceur
transmet directement l’adresse retenue au proxy Vite. Ctrl+C arrête les deux
services. Un `BIND_ADDRESS` explicite conserve le port demandé.

Ouvrir l’adresse affichée par Vite, <http://127.0.0.1:5173> par défaut. Si ce
port est occupé, Vite choisit automatiquement le prochain port disponible.
Cliquer sur **Essayer l’exemple** ou sélectionner un `.lss` et cliquer sur
**Analyser le fichier**. L’exemple contient huit questions, trois groupes et
deux langues. Choisir Rust fait apparaître un groupe ; cocher « Autre objectif »
fait apparaître une question de précision.

La navigation contrôle les réponses obligatoires et les contraintes prises en
charge. Une réponse devenue non pertinente est effacée. Recharger la page ou
importer une nouvelle étude réinitialise la saisie ; télécharger le JSON permet
de conserver un brouillon local, sans constituer un export de réponses
LimeSurvey.

Le proxy Vite relaie `/api` au serveur Rust. Le lien d’état appelle
`/api/health` pour vérifier HTTP et la base.

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
