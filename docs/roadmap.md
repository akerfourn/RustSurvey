# Plan de travail et reprise du projet

Ce fichier est le point d’entrée pour continuer sur une autre machine ou avec un
nouvel agent. Les [besoins](requirements.md), le
[modèle interne](domain-model.md) et les décisions restent les références de
conception ; ce plan organise les prochains incréments.

La priorité acquise est de **stabiliser une version pilote utilisable** avant
d’ouvrir de nouvelles fonctions. Lire le [guide du pilote](pilot.md) et la
[décision 0016](decisions/0016-pilot-web-delivery.md). La parité complète reste
la cible ultérieure ; les lots F0–F7 de
l’[audit des formulaires](form-coverage.md) en donnent l’inventaire détaillé.

## État au point de reprise

État rapproché le **16 novembre 2026** : le pilote et les incréments de
publication et de protection de saisie partagent maintenant le même modèle de
campagnes. Aucun second moteur de collecte ni schéma parallèle de réponses ne
doit être réintroduit.

| Domaine       | Disponible                                                                              | Limite principale                                                                        |
| ------------- | --------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| Conception    | Admin unique, 28 types, traductions, calculs, paramètres, historique et format natif v4 | Imports modifiables surtout sur leurs pertinences                                        |
| Publication   | Brouillon privé, publication explicite, aperçu et images par révision                   | Une campagne ouverte garde sa révision, même pour ses nouveaux participants              |
| Collecte      | Validation Rust, réponses SQL, campagne figée, reprise et trois présentations           | Médias, fichiers et règles hors profil refusés                                           |
| Protection    | Sauvegarde après 2 s de pause, délai visé de 10 s, réessais et garde de navigation      | Dernières frappes perdues si arrêt brutal avant confirmation ; recette visuelle restante |
| Participation | Cookies facultatifs, invitations, dates, limites, révocation et correction autorisée    | Aucun transfert des anciens droits LimeSurvey                                            |
| Réponses      | Liste admin, JSON, coordonnées natives, export VV progressif et préparation LSR privée  | Import VV, conversion en participations et archives LSA à compléter                      |
| Structure LSS | Import, original privé, pertinences réexportables et export des créations natives       | Fidélité limitée au profil représentable et aux cas de référence                         |
| Formulaires   | Défauts v4, exclusivités, filtres, calculs et profil importé qualifié                   | Anciennes conditions, autres options et interactions à compléter                         |
| Médias        | Port objet SQL, fichiers, mémoire et S3 indexé ; images natives                         | Médias importés et fichiers participants absents de la collecte                          |
| Livraison     | Client servi par Rust, paquet pilote, recette HTTP et guides                            | Recette navigateur, déploiement cible et restauration à vérifier                         |

Les périmètres [éditeur](authoring.md), [viewer](viewer.md),
[collecte](collection.md), [participation](participation.md) et
[exécution importée](import-execution.md) décrivent les garanties exactes.
L’absence de diagnostic sur un import ne prouve pas la parité.

Des preuves réelles dans LimeSurvey 6.5.14 existent déjà pour le LSS natif, les
cellules, deux réponses VV, les défauts et plusieurs règles de formulaire. Voir
la [procédure de référence](limesurvey-reference.md). Elles ne certifient ni
tous les parcours ni le transfert d’une étude en cours.

Les vérifications automatisées couvrent TypeScript, Rust, SQL et HTTP après
redémarrage. Les trois bases ont leur lanceur jetable. Les dialogues natifs et
l’utilisation visuelle restent à vérifier ; ces résultats ne remplacent pas une
exécution sur la nouvelle machine.

## Contraintes à conserver

- Rust minimal côté serveur, TypeScript et Preact côté client.
- SQLite, PostgreSQL et MariaDB ; migrations SQL versionnées. La publication
  utilise la migration 0012, après les migrations du pilote.
- Cible de fichiers : LimeSurvey 6.5.14+240624, schéma 623 ; pas son API web.
- Une participation conserve la révision de sa campagne, y compris en reprise.
  Modifier ou publier ne doit jamais réécrire ces références.
- Réponses et transitions contrôlées côté Rust ; aucun code importé exécuté.
- Pas de réponses ni secrets ajoutés au stockage persistant du navigateur.
- Médias derrière le port objet ; aucun accès disque dans le métier.
- VV et LSR/LSA restent requis. La continuité exacte des anciennes
  participations et l’outil externe de transfert restent différés.
- Avancer depuis les sources de référence sans attendre un export utilisateur.
  Aucun nouveau benchmark ne conditionne la recette fonctionnelle du pilote.
- Ne pas régénérer les lockfiles ou ajouter un service sans besoin documenté.

## Ordre de travail proposé

Les statuts ci-dessous décrivent du travail restant. Les extensions viennent
après la recette et la stabilisation du pilote ; leur ordre reste proposé, sans
calendrier.

| Lot | Livrable                                        | Statut                                      | Dépendance                                             |
| --- | ----------------------------------------------- | ------------------------------------------- | ------------------------------------------------------ |
| 0   | Recette du pilote et de la protection de saisie | Priorité actuelle                           | Navigateur et environnement jetable                    |
| 1   | Entrée de collecte suivant les publications     | Écart identifié, conception ciblée restante | Publications et campagnes figées                       |
| 2   | Export CSV pour analyse                         | À faire                                     | Coordonnées de cellules et export progressif existants |
| 3   | Compléter les échanges de réponses              | Partiel                                     | VV, LSR et correspondances existants                   |
| 4   | Gestion du cycle de collecte                    | Partiel                                     | Campagnes, invitations et reprise existantes           |
| 5   | Parité des formulaires                          | Partiel                                     | Audit F0–F7 et référence 6.5.14                        |
| 6   | Édition étendue des imports                     | Partiel                                     | Conservation source et profil d’export                 |
| 7   | Médias et fichiers participants                 | Partiel                                     | Port objet et adaptateur S3 existants                  |
| 8   | Déploiement et restauration                     | À vérifier sur la cible                     | Paquet pilote et sauvegarde cohérente                  |

### Lot 0 — Recette du pilote

- [ ] Construire et exécuter le paquet sur la plateforme choisie, en suivant le
      [guide](pilot.md), sur des données synthétiques.
- [ ] Créer un questionnaire avec obligatoire, condition et calcul ; enregistrer
      un brouillon, publier et vérifier l’aperçu.
- [ ] Ouvrir une campagne et vérifier les modes groupe, question et tout, avec
      valeurs par défaut, filtres et langue.
- [ ] Vérifier la sauvegarde après une pause et la poursuite de la frappe
      pendant l’envoi ; un accusé ancien ne doit pas remplacer une modification
      récente.
- [ ] Annuler puis accepter une navigation interne et un retour navigateur ;
      vérifier URL privée, formulaire, rechargement et reprise.
- [ ] Vérifier erreur réseau, réessai de la même opération, conflit entre deux
      onglets et consultation après soumission.
- [ ] Vérifier invitation avec reprise, correction autorisée et révocation.
- [ ] Publier r2 pendant une saisie r1 : r1 et ses médias restent disponibles,
      la reprise garde r1 ; l’aperçu d’entrée reçoit r2. Une campagne déjà
      ouverte reste sur r1, conformément à la limite du lot 1.
- [ ] Vérifier LSS/VV d’une campagne fermée et import LSS qualifié. Les anciens
      cas de référence restent des preuves bornées, pas une certification.

**Terminé lorsque :** les résultats et écarts sont consignés ; chaque bogue
corrigé possède une régression adaptée. Si le navigateur est indisponible,
consigner ce manque sans annoncer que la recette visuelle est faite.

### Lot 1 — Publication et nouvelles participations

Le catalogue public suit la publication ; les campagnes sont figées sur une
révision. Publier r2 ne suffit donc pas encore à envoyer les nouveaux
participants d’une campagne existante vers r2. Voir la
[décision 0006](decisions/0006-drafts-and-publications.md).

- [ ] Définir une entrée stable résolvant la publication au démarrage, avec une
      association explicite aux campagnes et aux politiques de participation.
- [ ] Conserver les campagnes et participations déjà créées sur leur révision ;
      ne jamais remplacer leur définition, leurs ressources ou leur checkpoint.
- [ ] Définir le devenir des invitations, limites et cookies lors d’une nouvelle
      publication, sans réinitialisation silencieuse des droits.
- [ ] Tester publication concurrente au démarrage, reprise r1 après publication
      r2 et nouvelle participation sur r2, sans pause de collecte.

**Terminé lorsque :** une publication change la version des nouveaux démarrages
à la même entrée, sans interrompre ni convertir les participations existantes.

### Lot 2 — Export CSV pour analyse

L’export VV existe déjà, avec lecture progressive des campagnes fermées. Le CSV
natif serait un complément d’analyse, avec un contrat distinct.

- [ ] Définir colonnes, états et sélection par campagne/révision en réutilisant
      les correspondances existantes.
- [ ] Définir la cohérence d’un éventuel export de campagne ouverte pendant les
      écritures ; l’export VV actuel exige une campagne fermée.
- [ ] Préserver codes lexicaux, zéros initiaux, Unicode, valeurs
      absentes/nulles/ vides et retours à la ligne ; refuser les correspondances
      ambiguës.
- [ ] Documenter le traitement des formules de tableur sans transformer les
      valeurs brutes silencieusement.
- [ ] Tester confidentialité, parcours par lots et interruption du
      téléchargement.

**Terminé lorsque :** l’administration exporte des réponses analysables avec un
périmètre documenté, sans aligner plusieurs révisions sur le seul code de champ.

### Lot 3 — Compléter les échanges de réponses

Les coordonnées natives, l’export VV, la lecture/écriture LSR et la
[préparation SQL des LSR](response-imports.md) sont présents. La conversion
réversible par ligne conserve les valeurs et métadonnées source ; elle ne crée
pas de participation. Lire l’[échange actuel](response-exchange.md).

- [ ] Ajouter l’import VV avec diagnostic préalable et politique de doublons.
- [ ] Définir la publication des données préparées en un jeu de réponses
      d’analyse, sans inventer de droits de reprise.
- [ ] Ajouter l’export LSR des données SQL en tenant compte des altérations de
      l’importeur LimeSurvey 6.5.14 déjà documentées.
- [ ] Ajouter lecture puis production d’archives LSA pour un profil explicite.
- [ ] Borner archives, décompression, colonnes et lignes ; préserver les valeurs
      historiques sans les faire repasser silencieusement dans le moteur
      courant.
- [ ] Vérifier chaque nouveau format avec une référence synthétique 6.5.14.

**Terminé par format lorsque :** conservation, export et réimport sont vérifiés
pour le profil annoncé. Une archive sans les ressources et états de reprise
nécessaires ne constitue pas un transfert complet.

### Lot 4 — Cycle de collecte

La fermeture de campagne, les invitations avec dates et révocation, les limites
d’usage et les corrections autorisées existent déjà.

- [ ] Distinguer fermeture aux nouveaux démarrages et maintien des reprises
      autorisées ; documenter les transitions.
- [ ] Définir révocation et expiration des secrets de réponse indépendants.
- [ ] Compléter recherche et filtres des réponses selon les besoins de recette.
- [ ] Définir archivage, suppression et conservation des réponses, droits,
      définitions référencées et objets.
- [ ] Tester concurrence et migrations sur les trois bases.

**Terminé par incrément lorsque :** les droits et historiques restent cohérents.
Aucun système de comptes participants n’est nécessaire pour ces lots.

### Lot 5 — Parité des formulaires

Suivre l’[audit](form-coverage.md) et le [catalogue](form-catalog.md). Les
valeurs par défaut représentables, exclusivités, filtres et trois présentations
sont déjà partiellement qualifiés ; étendre leurs profils plutôt que les
réécrire.

- [ ] Compléter anciennes conditions, cibles de défauts et interactions NAOK.
- [ ] Compléter règles par cellule, expressions, alias, dates et variantes.
- [ ] Compléter rendu riche sûr, index, retour configurable et délais.
- [ ] Ajouter ordres aléatoires persistants, timings, quotas et autres fonctions
      avec état, selon les dépendances du catalogue.

**Terminé par capacité lorsque :** import, domaine Rust, rendu, édition,
sauvegarde/reprise et diagnostics sont testés ensemble, puis comparés à la
référence dans le profil annoncé.

### Lot 6 — Édition étendue des imports

- [ ] Étendre l’édition au-delà des pertinences sans perdre les données source.
- [ ] Compléter les refus et diagnostics des exports non représentables.
- [ ] Vérifier codes, ordre, langues, identifiants et ressources dans la
      référence.

**Terminé par incrément lorsque :** l’export représente les modifications
réellement enregistrées. L’original privé et immuable reste distinct.

### Lot 7 — Médias et fichiers

Le stockage S3 indexé et ses tests de contrat sont disponibles. Ne pas ajouter
une seconde abstraction ; suivre les
[règles objet](decisions/0002-object-storage.md) et le
[guide S3](s3-storage.md).

- [ ] Importer et résoudre les médias LimeSurvey, avec diagnostic des absents.
- [ ] Qualifier les médias de collecte avec révision et droits explicites.
- [ ] Ajouter fichiers participants : propriété, types, tailles, nombre et
      accès.
- [ ] Prévoir copie entre adaptateurs, nettoyage et restauration SQL + objets.

**Terminé par incrément lorsque :** aucun accès disque n’entre dans le métier et
les adaptateurs partagent leur contrat. S3 ne requiert pas de disque persistant.

### Lot 8 — Déploiement et restauration

Le frontend statique servi par Rust et la commande de paquet pilote existent.
Les scripts de tests SQL, S3 et HTTP utilisent des environnements jetables.

- [ ] Vérifier le paquet et ses migrations sur la plateforme cible.
- [ ] Vérifier HTTPS, base gérée et fournisseur S3 réellement retenus.
- [ ] Tester sauvegarde/restauration et montée de version avec réponses
      synthétiques, anciennes révisions et ressources.
- [ ] Ajouter une CI lorsque l’hébergement Git sera fixé.

**Terminé lorsque :** installation, mise à jour et restauration sont
reproductibles sur la cible. Les outils de mesure existent ; aucune nouvelle
comparaison de performances ne bloque ces vérifications fonctionnelles.

## Reprendre sur une autre machine

### Récupérer le code et démarrer

Récupérer la branche contenant les changements **versionnés et transférés**.
Lire AGENTS.md, ce plan et le document du lot choisi. Utiliser
rust-toolchain.toml, .nvmrc, package.json et les lockfiles ; voir le
[guide de développement](development.md).

Depuis la racine, dans un terminal Bash :

```sh
set -euo pipefail
git status --short
git log -3 --oneline
node scripts/doctor.mjs
npm ci
if [ ! -e .env ]; then
  cp .env.example .env
fi
npm run check
npm run check:rust
npm run dev
```

L’exemple contient des identifiants publics de développement. Il initialise
seulement le premier compte et ne réinitialise aucun compte existant. Le lanceur
choisit des ports disponibles et applique les migrations. Ne pas dépendre d’un
cache Cargo, d’un dossier temporaire ou d’un port propre à l’ancienne machine.
`npm run dev:backend` lance seulement Rust.

Pour les frontières concernées par le lot, ajouter les contrôles adaptés :

```sh
npm run test:storage
npm run test:databases
npm run test:s3
```

Les tests des services externes nécessitent Docker et leurs images de référence
déjà chargées ; lire [SQL](database.md) et [S3](s3-storage.md). Ils utilisent
des données jetables, jamais une installation existante. Pour vérifier aussi le
client servi par Rust depuis un terminal Bash :

```sh
npm run build
RUSTSURVEY_TEST_WEB_DIRECTORY="$PWD/web/dist" npm run test:storage
```

Le [guide pilote](pilot.md) décrit la construction et la recette du paquet.

### Retrouver les mêmes données de développement

Git transporte le code et les fixtures publiques, pas .env, .data/, .private/,
.surveys-tests/ ou les bases externes. Les exemples synthétiques suffisent pour
reprendre le développement.

Pour retrouver les mêmes études et réponses, transférer séparément les données
privées. En mode SQLite/fichiers : arrêter le backend, faire une sauvegarde
cohérente et conserver base et objets ensemble, avec les éventuels fichiers
SQLite compagnons, sans écraser une destination existante. Respecter les anciens
chemins ou répertoires personnalisés ; voir le
[stockage local](local-storage.md). Pour une base externe ou S3, utiliser leurs
procédures de sauvegarde cohérente.

Les JSON individuels ne sont pas une sauvegarde de l’installation. Conserver
aussi les liens privés utiles aux essais : SQL garde l’empreinte des secrets,
sans permettre de les retrouver.

### Où intervenir

| Sujet                       | Points d’entrée                                                                                                                                |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Collecte et SQL             | [API](../crates/server/src/workspace/collection/), [domaine](../crates/domain/src/collection/)                                                 |
| Écrans de campagne          | [client collecte](../web/src/collection/)                                                                                                      |
| Saisie et protection        | [session viewer](../web/src/components/useStudySession.ts), [écritures](../web/src/collection/writer.ts), [navigation](../web/src/navigation/) |
| Publication                 | [API](../crates/server/src/workspace/publications.rs), [décision 0006](decisions/0006-drafts-and-publications.md)                              |
| Règles et rendu             | [moteur TypeScript](../web/src/survey/), [domaine Rust](../crates/domain/src/)                                                                 |
| Échanges et préparation LSR | [adaptateur et CLI](../crates/limesurvey/README.md), [API LSR](../crates/server/src/workspace/response_imports/)                               |
| Schéma SQL                  | [migrations](../crates/migration/src/)                                                                                                         |
| Objets                      | [port et adaptateurs](../crates/object-store/), [intégration serveur](../crates/server/src/object_store.rs)                                    |
| Recettes                    | [tests API](../crates/server/tests/workspace/), [scripts](../scripts/), [exemples](../examples/surveys/)                                       |

### Message prêt à donner à un nouvel agent

> Reprends RustSurvey en lisant AGENTS.md et docs/roadmap.md, puis les documents
> concernés. Le pilote, la collecte SQL par campagne, S3, les invitations,
> LSS/VV et la protection de saisie sont déjà présents, dans leurs profils
> explicites. Priorité au lot 0 : vérifier et stabiliser le pilote. Ne
> réintroduis pas l’ancien collecteur parallèle. Les campagnes restent figées
> sur leur révision ; l’entrée de collecte suivant les publications reste à
> harmoniser au lot 1. Préserve les droits, reprises et données existants. Ne
> recommence pas le cadrage général et ne crée pas de commit.

## Tenir ce plan à jour

Après chaque lot, indiquer ce qui est livré, les commandes réellement exécutées,
les limites et la prochaine tâche. Cocher seulement les éléments vérifiés.
Maintenir les documents du domaine ; distinguer contrôles automatisés, recette
navigateur et preuves de compatibilité.
