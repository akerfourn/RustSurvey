# RustSurvey

Un moteur de questionnaires en Rust, conçu pour limiter les ressources
nécessaires côté serveur et échanger des fichiers avec **LimeSurvey 6.5.14**. Le
navigateur prend en charge l’affichage et les interactions ; le serveur reste
responsable des autorisations, de la validation et de la persistance.

## État du projet

**Premier éditeur avec stockage SQL.** Un compte administrateur permet de créer
et modifier des questionnaires. Ils sont publics dès leur enregistrement :

- groupes et questions réordonnables, textes, codes et choix de réponse ;
- huit types : texte court/long, nombre, date, choix unique/multiples, oui/non
  et information ;
- caractère facultatif, recommandé ou obligatoire ; aperçu avec le viewer ;
- définitions natives validées côté Rust et versions immuables en SQL ;
- sauvegarde protégée contre l’écrasement entre deux onglets ;
- sessions administrateur en SQL, mot de passe Argon2id et protection CSRF ;
- SQLite par défaut, migrations testées aussi sur PostgreSQL et MariaDB.

Le parcours d’import `.lss` permet toujours d’explorer un fichier et d’essayer
une saisie locale sans compte :

- lecture bornée, diagnostics et modèle de lecture du schéma LimeSurvey 623 ;
- réexport de l’original à l’octet près et modification ciblée d’un titre ;
- upload et analyse XML côté Rust, avec limite de 8 Mio ;
- titres, langues, groupes, questions, traductions et choix de réponse ;
- viewer Preact avec navigation par groupe et changement de langue ;
- 28 types sans fichiers : tableaux, commentaires, autre et classement ;
- matrices texte/numériques, curseurs et choix en colonnes ;
- sous-questions conditionnelles, filtres et équations simples ;
- textes d’accueil et de fin localisés ;
- réponses en mémoire dans l’onglet, téléchargeables en JSON ;
- premier moteur client de pertinence et de validation, avec diagnostics ;
- un exemple synthétique bilingue pour essayer le parcours immédiatement.

Les réponses ne sont jamais envoyées au serveur. La collecte, les sessions de
réponse durables et les archives `.lsa` restent à implémenter. L’éditeur natif
ne sait pas encore enregistrer ou modifier un LSS importé, ni produire un LSS.
Le moteur d’aperçu couvre un sous-ensemble explicite ; il ne certifie pas encore
le comportement complet d’une étude LimeSurvey. Voir le
[périmètre du viewer](docs/viewer.md).

La cible couvre l’ensemble des formulaires natifs de la version de référence.
L’[audit de couverture](docs/form-coverage.md) détaille les types, rendus et
règles manquants, avec un plan pour les fonctions demandant du stockage. Le
[catalogue](docs/form-catalog.md) recense les options et fonctions à suivre.
L’absence de blocage sur une étude n’équivaut pas à une compatibilité complète.

La persistance actuelle concerne les définitions et l’administration. Les futurs
médias et fichiers utiliseront une
[interface objet avec adaptateurs S3 et disque](docs/decisions/0002-object-storage.md).

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
| [Administration]       | Compte, éditeur et API SQL       |
| [Développement]        | Outillage et commandes           |
| [Choix techniques]     | Alternatives et compromis        |
| [Feuille de route]     | Prochaines étapes                |
| [AGENTS.md](AGENTS.md) | Consignes de contribution        |

## Vérifier l’environnement

Rust `1.97.1` avec rustfmt et Clippy, un compilateur C, Node.js et npm sont
nécessaires. Node.js 24 LTS est recommandé ; la version 22.15.0 présente sur
cette machine satisfait aussi les contraintes du squelette. Docker est optionnel
et permet les tests PostgreSQL/MariaDB sur des bases jetables.

```sh
node scripts/doctor.mjs
npm ci
npm run check:all
```

Le diagnostic vérifie la présence des outils. Les versions attendues et les
limites de cette vérification sont détaillées dans [Développement].

## Créer le premier compte administrateur

Pour un essai local, copier la configuration fournie depuis la racine :

```sh
cp .env.example .env
npm run dev
```

Ouvrir l’adresse Vite affichée, puis **Administration**. L’exemple utilise
`admin` / `rustsurvey-dev-only`, des identifiants publics réservés aux tests
locaux. Choisir un mot de passe privé hors de ces tests. Après connexion,
cliquer sur **Créer un questionnaire**, ajouter les questions et enregistrer. Le
questionnaire apparaît dans le catalogue public ; son lien permet une saisie
locale sans compte. Les modifications sont publiques dès leur sauvegarde.

Le backend Rust charge automatiquement le `.env` du répertoire courant. Les
variables déjà définies dans l’environnement sont prioritaires. Cette
configuration n’est pas injectée dans le frontend ; le mot de passe du compte
est conservé haché en SQL.

Pour utiliser un mot de passe court pendant le développement, ajouter
`DEV_ALLOW_WEAK_PASSWORDS=true` dans `.env`. Ce flag concerne uniquement la
création du premier compte en build debug (`npm run dev` ou `cargo run`). Le mot
de passe doit rester non vide ; le flag est refusé en release. La règle par
défaut reste un minimum de 12 caractères.

Le compte est créé uniquement si aucun administrateur n’existe. Ensuite,
`ADMIN_PASSWORD` peut être retiré du `.env` et de l’environnement ; le compte et
les sessions survivent aux redémarrages. Modifier cette variable ne réinitialise
pas un compte existant. Sans compte configuré, le catalogue et l’import restent
accessibles. Voir [Administration] pour les limites et la configuration HTTPS.

## Essayer le viewer LSS

Depuis la racine, une commande lance Rust puis Vite :

```sh
npm run dev
```

Le backend essaie le port 3000 puis les suivants en développement. Le lanceur
transmet directement l’adresse retenue au proxy Vite. Ctrl+C arrête les deux
services. Un `BIND_ADDRESS` explicite conserve le port demandé.

Ouvrir l’adresse affichée par Vite, <http://127.0.0.1:5173> par défaut. Si ce
port est occupé, Vite choisit automatiquement le prochain port disponible.
Ouvrir **Essayer un LSS**, puis cliquer sur **Essayer l’exemple** ou
sélectionner un `.lss` et cliquer sur **Analyser le fichier**. L’exemple
contient douze questions, quatre groupes et deux langues. Choisir Rust fait
apparaître un groupe ; cocher « Autre objectif » fait apparaître une question de
précision.

Le bouton **Essayer les 28 types** ouvre une seconde démonstration couvrant les
28 types sans fichiers, leurs variantes locales et un calcul. Le
[périmètre détaillé](docs/viewer.md) précise les restrictions restantes.

Le groupe « Les tableaux » permet d’essayer F, H et une matrice de cases à
cocher. Choisir « À l’aise » dans la colonne « Après » fait apparaître une
précision obligatoire. Les autres cellules de ces tableaux sont facultatives.

La navigation contrôle les réponses obligatoires et les contraintes prises en
charge. Une réponse devenue non pertinente est effacée. Recharger la page ou
importer une nouvelle étude réinitialise la saisie ; télécharger le JSON permet
de conserver un brouillon local, sans constituer un export de réponses
LimeSurvey.

Le proxy Vite relaie `/api` au serveur Rust. Le lien d’état appelle
`/api/health` pour vérifier HTTP et la base.

Par défaut, le serveur crée `rustsurvey.db` dans le répertoire courant et
applique automatiquement les migrations des questionnaires et de
l’administration. `DATABASE_URL=sqlite::memory:` utilise une base volatile,
perdue à l’arrêt. `.env.example` décrit la configuration ; le `.env` local est
ignoré par Git.

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
[Administration]: docs/authoring.md
[Développement]: docs/development.md
[Choix techniques]: docs/decisions/0001-technology-stack.md
[Feuille de route]: docs/roadmap.md
[ls-version]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/config/version.php
[Compatibilité]: docs/limesurvey-compatibility.md
