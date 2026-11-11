# Consignes du projet RustSurvey

## Phase actuelle

Le développement couvre l’adaptateur LimeSurvey, son outil CLI et un premier
viewer web avec saisie locale, stockage SQL et premier éditeur natif. Un compte
administrateur protège les écritures ; les questionnaires restent publics.
Suivre les grandes lignes et livrer des incréments fonctionnels testés ; ne pas
remplacer le code demandé par un nouveau tour de cadrage général.

Lire [README.md](README.md), les [besoins](docs/requirements.md), le
[modèle interne](docs/domain-model.md) et le document du domaine concerné avant
une modification. Distinguer les contraintes acquises, les décisions de
conception et les fonctionnalités réellement vérifiées. Pour le parcours web,
lire aussi le [périmètre du viewer](docs/viewer.md).

La cible couvre tous les formulaires natifs de la version de référence. Pour les
types, rendus et règles, suivre l’[audit de couverture](docs/form-coverage.md)
et son [catalogue](docs/form-catalog.md). Un LSS sans blocage détecté ne prouve
pas la parité ; distinguer options interprétées, tolérées et non projetées.

La persistance actuelle couvre les définitions versionnées et les sessions
administrateur. Lire le [périmètre de l’éditeur](docs/authoring.md) avant une
modification de ces fonctions. Collecte et reprise des réponses restent à faire.
Pour les futurs médias et fichiers, suivre la
[décision de stockage objet](docs/decisions/0002-object-storage.md) : port
commun, adaptateur compatible S3 et adaptateur local sur fichiers.

## Contraintes du produit

- Rust minimal côté serveur ; TypeScript et Preact côté navigateur.
- Reprendre l’approche sobre de LiteCloud. Conserver cette base sauf besoin
  particulier documenté ; ne pas rouvrir le choix de pile par défaut.
- Cible d’échange : fichiers LimeSurvey **6.5.14+240624**, schéma **623**. Le
  contrat porte sur les fichiers, pas sur RemoteControl ou l’API web.
- SQLite, MariaDB et PostgreSQL font partie du périmètre de portabilité.
- Toute évolution de schéma passe par une migration versionnée.
- La sobriété serveur guide les choix ; une amélioration de performance doit
  être mesurée sur une charge documentée.
- Les calculs déportés au client ne remplacent jamais les contrôles serveur.
- La migration d’études en cours doit préserver données et comportement, y
  compris les réponses partielles et les moyens de reprise nécessaires. Lire le
  [protocole de transfert](docs/study-migration.md).

## Architecture et dépendances

- Préférer la bibliothèque standard Rust et des crates ciblées. Ajouter une
  couche ou un framework seulement pour un besoin concret du projet.
- Avancer sur l’architecture à partir des sources de la version cible ; un
  export utilisateur n’est pas un préalable au travail de conception.
- Séparer conservation, réexport, exécution et continuité. Une donnée inconnue
  préservée ne suffit pas à autoriser l’ouverture d’une étude.
- Garder le métier indépendant de HTTP, de l’ORM et des formats LimeSurvey.
- Centraliser les particularités SQL dans l’accès aux données/migrations.
- Ne pas ajouter Redis, une file de tâches, un runtime Node en production ou un
  autre service sans besoin démontré et décision documentée.
- Éviter l’état de session non borné en mémoire et les sessions attachées à un
  processus. Le modèle de reprise est documenté ; sa mise en œuvre et ses tests
  restent à réaliser.
- Justifier les dépendances ajoutées et leurs options dans la décision technique
  concernée ; désactiver les fonctions inutilisées lorsque possible.
- Versionner les lockfiles. Ne pas les régénérer sans raison liée au changement.

## Lisibilité et conventions

- Préférer un code explicite, des fonctions courtes et des erreurs utiles.
- Code, identifiants et messages de commit en anglais ; documentation et
  échanges en français par défaut.
- Rust : édition du workspace, rustfmt, Clippy, pas de code `unsafe`.
- TypeScript : mode strict, types publics explicites, pas de `any` injustifié.
- Preact : composants fonctionnels, traitements hors du JSX et effets nettoyés
  au démontage.
- Markdown : lignes de prose de 80 caractères ; les URL de référence peuvent
  dépasser cette limite.
- Utiliser les formateurs du dépôt ; éviter les modifications sans rapport.

## Sécurité et données

- Ne jamais versionner ni journaliser des secrets, des jetons, des réponses
  personnelles ou des fichiers de production non anonymisés.
- Le navigateur est une entrée non fiable : vérifier les droits, les tailles,
  les types, les règles de réponse et les transitions côté serveur.
- Aucun `eval` pour les expressions importées, aucune exécution de code PHP,
  JavaScript ou de plugin embarqué dans un questionnaire.
- Ne pas exécuter de SQL construit à partir de valeurs utilisateur.
- Prévoir des limites explicites pour imports, archives et traitements lourds.
- Une fonction importée non prise en charge doit produire un diagnostic ; ne
  jamais la supprimer ou modifier sa signification silencieusement.
- Ne jamais assimiler une archive `.lsa` à un transfert complet des sessions,
  reprises et ressources externes. Vérifier les compléments réellement fournis.

## Vérifications

```sh
node scripts/doctor.mjs
npm run check
npm run check:rust
```

L’adaptateur `.lss` possède des tests de lecture, de conservation, d’édition et
de CLI. L’API d’inspection et le moteur TypeScript ont leurs propres tests. La
saisie reste locale : aucune API de collecte n’accepte des réponses pour
l’instant. Ajouter des tests de comportement pour les nouvelles fonctions et un
test de régression pour chaque bogue corrigé. Éviter les tests qui ne font que
recopier l’implémentation.

- Tests rapides, déterministes et isolés ; pas d’accès à des services distants.
- SQLite en mémoire ou fichier temporaire pour les tests courants.
- PostgreSQL/MariaDB dans des environnements locaux jetables pour les tests
  d’intégration, jamais sur une base existante ou de production.
- Une compilation de pilote ne prouve pas la portabilité du schéma.
- Ne pas annoncer la compatibilité d’un format sans fixture de référence et
  vérification d’import/export dans LimeSurvey 6.5.14.

## Git et restitution

Ne jamais créer de commit directement. Proposer un message au format :

```text
type(scope) #number: summary
```

Utiliser `#0000` si aucun ticket n’est connu. Expliquer ce qui a changé,
pourquoi, les vérifications exécutées et les limites restantes.
