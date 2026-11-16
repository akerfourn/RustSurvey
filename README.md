# RustSurvey

Un moteur de questionnaires en Rust, conçu pour limiter les ressources
nécessaires côté serveur et échanger des fichiers avec **LimeSurvey 6.5.14**. Le
navigateur prend en charge l’affichage et les interactions ; le serveur reste
responsable des autorisations, de la validation et de la persistance.

## État du projet

Le cycle actuel vise une **version pilote** sur les fonctions déjà couvertes.
`npm run package:pilot` prépare un paquet natif avec client web, exemples et
guide, puis vérifie ses parcours HTTP sur des données temporaires. Son exécution
ne demande ni Rust ni Node. Voir le [démarrage du pilote](docs/pilot.md). La
parité complète et la migration des anciennes participations restent des
chantiers ultérieurs.

**Éditeur de questionnaires avec stockage SQL.** Un compte administrateur permet
de créer et modifier des questionnaires. L’enregistrement garde le brouillon
privé ; une publication explicite rend la révision accessible :

- groupes et questions réordonnables, textes, codes et choix de réponse ;
- 28 types sans fichiers : textes, nombres, choix, tableaux, classement et
  calculs ;
- axes, doubles échelles, filtres en cascade, validations et présentation ;
- lignes exclusives et sélection automatique pour les choix multiples ;
- valeurs par défaut localisées sur 21 types, appliquées à l'entrée des pages ;
- langues et traductions des textes, sous-questions et choix ;
- caractère facultatif, recommandé ou obligatoire ; aperçu avec le viewer ;
- import LSS depuis l’administration, conservé en SQL avec son original ;
- préparation privée des réponses LSR, avec correspondances et chargement
  reprenable après interruption ;
- bilan des prérequis de collecte d'une étude importée, par révision ;
- résolution de 30 options héritées LimeSurvey, avec leur provenance ;
- contextes source privés par révision, sélection explicite et extraction depuis
  LimeSurvey par une commande de lecture ;
- nouvelles campagnes sur des imports qualifiés, avec paramètres source figés ;
- conditions de groupes/questions : constructeur ET/OU et mode expression ;
- catalogue des variables et inspection locale des valeurs dans l’aperçu ;
- définitions natives validées côté Rust et versions immuables en SQL ;
- sauvegarde protégée contre l’écrasement entre deux onglets ;
- publication sans pause, révision conservée pour les parcours déjà ouverts ;
- collecte SQL avec sauvegarde automatique et protection de la saisie pendant
  les envois, erreurs et changements de page ;
- historique consultable et restaurable, import/export JSON des définitions ;
- export LSS des créations natives, avec refus des données non représentables ;
- export VV des réponses des campagnes natives fermées, avec lecture progressive
  ;
- médiathèque PNG/JPEG, objets SQL privés et images associées aux questions ;
- réexport des conditions modifiées dans le LSS, avec blocages explicites ;
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

Dans ces aperçus, les réponses restent locales. Une première
[collecte native durable](docs/collection.md) est disponible séparément :
campagne sur une révision figée, 28 types, validation Rust, sauvegarde SQL, lien
privé de reprise et soumission. La présentation est configurable par groupe, par
question ou sur une seule page, avec progression facultative et reprise sur la
page enregistrée. Les [règles de participation](docs/participation.md) ajoutent
cookie facultatif, invitations limitées ou réutilisables et corrections
autorisées d'une même réponse. Un [profil importé](docs/import-execution.md)
distinct ouvre des campagnes neuves après qualification des réglages source. La
reprise des anciennes participations et les archives `.lsa` restent à réaliser.
Les conditions des études importées sont éditables ; leurs autres réglages sont
conservés et protégés. Deux téléchargements distinguent le LSS original inchangé
du LSS contenant les conditions de la révision enregistrée. Les anciennes
sections de conditions peuvent bloquer ce second export. Le moteur d’aperçu
couvre un sous-ensemble explicite ; il ne certifie pas encore le comportement
complet d’une étude LimeSurvey. Voir le [périmètre du viewer](docs/viewer.md).

Les [réponses LSR importées](docs/response-imports.md) sont conservées dans une
préparation privée distincte des participations natives. Le
[bilan des prérequis](docs/imported-collection.md) distingue questions prises en
charge, correspondances des cellules et réglages source restant à interpréter.
Il reste un inventaire conservateur. L'ouverture d'une campagne neuve utilise
séparément le profil importé ; la reprise des anciens participants est différée.

La cible couvre l’ensemble des formulaires natifs de la version de référence. Un
[aller-retour structurel réel](docs/limesurvey-reference.md) vérifie les 28
types sans fichiers en deux langues dans LimeSurvey 6.5.14. L'import de réponses
VV est aussi vérifié sur deux études aux identifiants différents.
L'[échange des réponses](docs/response-exchange.md) décrit ce parcours et ses
limites. La migration des reprises et la parité de tous les réglages restent à
réaliser. L’[audit de couverture](docs/form-coverage.md) détaille les types,
rendus et règles manquants, avec un plan pour les fonctions demandant du
stockage. Le [catalogue](docs/form-catalog.md) recense les options et fonctions
à suivre. L’absence de blocage sur une étude n’équivaut pas à une compatibilité
complète.

La persistance couvre les définitions, l’administration et les images natives.
Les contenus binaires passent par un
[port objet](docs/decisions/0002-object-storage.md), avec adaptateurs SQL,
fichiers locaux, mémoire et [S3 privé](docs/s3-storage.md). Le mode local fourni
utilise `.data/`, ignoré par Git et protégé contre la lecture directe par Vite.
Le mode S3 conserve son index en SQL et ne demande aucun disque pour les
binaires ; les anciens objets SQL restent lisibles. Les fichiers envoyés par les
participants ne sont pas encore pris en charge.

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
- Import/export standard des réponses LimeSurvey, sans reprise des anciennes
  participations obligatoire. L’extraction de métadonnées source pour une
  migration complète relève d’un outil externe ultérieur.
- Validation serveur obligatoire, même lorsqu’une règle est évaluée côté client.

## Documentation

Pour collecter des réponses natives, suivre le
[parcours de collecte](docs/collection.md).

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

Rust stable, au minimum `1.97.1`, avec rustfmt et Clippy, un compilateur C,
Node.js et npm sont nécessaires. Node.js 24 LTS est recommandé ; la version
22.15.0 présente sur cette machine satisfait aussi les contraintes du squelette.
Docker est optionnel et permet les tests PostgreSQL/MariaDB sur des bases
jetables.

```sh
node scripts/doctor.mjs
npm ci
npm run check:all
# Optionnel, avec les images Docker déjà chargées :
npm run test:databases
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
questionnaire reste un brouillon privé. **Publier cette révision** le rend
visible dans le catalogue public ; son lien permet une saisie locale sans
compte. Les modifications suivantes restent privées jusqu’à publication. Les
parcours déjà ouverts conservent leur révision sans interrompre l’étude.

Pour reprendre un LSS, utiliser **Importer une étude LimeSurvey** dans
l’administration. Après enregistrement, **Modifier** ouvre ses groupes et
questions. Le panneau **Conditions d’affichage** propose les règles courantes ou
une expression avancée. **Aperçu** permet d’essayer le parcours et d’inspecter
les variables. L’original et le LSS contenant les conditions enregistrées se
téléchargent depuis cette page, après connexion. Le parcours **Essayer un LSS**
reste une inspection temporaire.

Pour les créations natives, **Calculs, validation et présentation** expose les
réglages interprétés. **Traductions** permet d’éditer les langues
supplémentaires. Après une première sauvegarde, la **Médiathèque** permet
d’enregistrer une image, de l’associer à une question et d’en décrire le
contenu. Enregistrer puis publier pour rendre cette association publique.
**Historique des révisions** permet de consulter, exporter ou restaurer un état
antérieur.

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

Pour une nouvelle installation, la configuration d’exemple place SQLite dans
`.data/rustsurvey.db` et les nouveaux médias dans `.data/media/`. Le serveur
crée les répertoires et applique automatiquement les migrations. Un ancien
`DATABASE_URL` reste respecté ; les médias déjà présents en SQL restent lisibles
en mode fichiers. Voir le [stockage local](docs/local-storage.md).
`DATABASE_URL=sqlite::memory:` utilise une base volatile, perdue à l’arrêt.
`.env.example` décrit la configuration ; le `.env` local est ignoré par Git.

## Contribution et licence

Le [banc CPU/RAM](docs/performance.md) se lance avec `npm run bench`. Il utilise
un serveur optimisé et des bases synthétiques jetables, puis vérifie les
réponses après redémarrage. Les chiffres locaux restent distincts d'une
comparaison avec LimeSurvey et d'un dimensionnement de production.

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
