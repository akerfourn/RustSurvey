# Administration et édition de questionnaires

## Périmètre réalisé

Un unique administrateur crée et modifie des questionnaires natifs. Le catalogue
et les définitions sont publics, sans compte participant ni jeton d’accès. Toute
sauvegarde devient immédiatement visible. Il n’existe pas encore d’étape de
publication, de campagne ou de suppression de questionnaire.

L’éditeur gère titre, langue française/anglaise, accueil, fin, groupes,
questions, aide, caractère obligatoire et choix. Groupes et questions peuvent
être ajoutés, retirés et réordonnés. Les identifiants des éléments restent
stables pendant l’édition. Les huit types proposés sont texte court, texte long,
nombre, date, choix unique, choix multiples, oui/non et information.

L’aperçu et les liens publics réutilisent le moteur de saisie du viewer. **Les
réponses restent dans l’onglet** : recharger les efface. Aucune API ne collecte
de réponses. Le JSON téléchargeable reste un brouillon local.

L’administration dispose d’un import LSS persistant. Il alimente la même
définition v2 que les créations, en conservant langues, types, traductions,
sous-questions, échelles, paramètres et règles. La fixture des 28 types sans
fichiers est couverte. L’édition des imports est limitée aux conditions des
groupes et questions ; les autres propriétés sont protégées côté serveur.

Le XML original est conservé séparément en SQL et téléchargé uniquement après
authentification. Il reste identique après les modifications de conditions. Ce
téléchargement ne constitue pas un réexport de la définition modifiée. Le
parcours `#/import` reste une inspection temporaire, sans enregistrement.

## Importer et modifier des conditions

1. Dans **Administration**, sélectionner un LSS puis **Importer et enregistrer
   le LSS**. Le schéma 623 et une structure valide sont requis.
2. Retrouver l’étude dans le catalogue après un rechargement ou un redémarrage
   avec une base persistante, puis ouvrir **Modifier**.
3. Consulter **Plan du questionnaire**, **Variables du questionnaire** et les
   diagnostics d’import. Les identifiants natifs restent stables à chaque
   sauvegarde ; les alias source sont conservés séparément.
4. Ouvrir **Conditions d’affichage** sur un groupe ou une question. Le
   constructeur propose comparaisons, vide/renseigné et combinaisons ET/OU. Le
   mode expression permet des règles complexes et l’insertion d’un code de
   variable recherché. Il ne convertit pas de force les expressions complexes.
5. Essayer **Aperçu**, modifier les réponses locales et ouvrir l’inspection des
   conditions et valeurs. Enregistrer pour rendre la nouvelle révision publique.

Exemple : `AGE >= 18 && CONSENT == 'Y'`. Les codes de choix restent des chaînes,
notamment `'01'`. Les règles s’appuient sur le moteur borné existant, sans
`eval`. Le catalogue expose les cellules texte, numériques, date, choix et
calculées ; ce lot n’ajoute pas encore un éditeur de calculs ou de paramètres
externes, ni une inférence complète des types d’expression.

Rust contrôle syntaxe, fonctions admises, arité, références et cycles pour les
conditions éditées. Les conditions non interprétées d’un import restent
préservées lorsqu’elles sont inchangées, avec les diagnostics du viewer. Une
nouvelle condition invalide est refusée. Les anciennes sections `conditions`,
les fonctions et options non gérées ne deviennent pas exécutables par le seul
fait d’avoir été stockées. Le profil du [viewer](viewer.md) reste applicable.

## Installation et connexion

Le [README](../README.md#créer-le-premier-compte-administrateur) décrit la copie
de `.env.example` vers `.env` avant `npm run dev`. L’exemple fournit `admin` /
`rustsurvey-dev-only` pour les tests locaux. Le backend Rust charge le `.env` de
son répertoire courant, donc la racine avec le lanceur du projet. Les variables
déjà présentes dans l’environnement sont prioritaires. Les valeurs restent côté
serveur, hors du bundle client ; le fichier local est ignoré par Git. Une erreur
de syntaxe arrête le démarrage sans afficher les valeurs du fichier.

| Variable                   | Comportement                                                                            |
| -------------------------- | --------------------------------------------------------------------------------------- |
| `DATABASE_URL`             | SQLite fichier par défaut ; autres pilotes activables                                   |
| `ADMIN_USERNAME`           | `admin` par défaut, normalisé en minuscules                                             |
| `ADMIN_PASSWORD`           | Aucun défaut serveur ; valeur de test dans l’exemple ; crée seulement le premier compte |
| `ADMIN_COOKIE_SECURE`      | `false` en debug, `true` en release                                                     |
| `DEV_ALLOW_WEAK_PASSWORDS` | `false` par défaut ; mots de passe courts pour le bootstrap en debug                    |

L’identifiant accepte 1 à 64 caractères ASCII alphanumériques, tiret et
soulignement. Le mot de passe doit contenir au moins 12 caractères et au plus
1024 octets. Pour les essais avec un mot de passe plus court, définir
`DEV_ALLOW_WEAK_PASSWORDS=true` dans `.env`. Ce mode exige un mot de passe non
vide, conserve la limite de 1024 octets et le hachage Argon2id. Il est
disponible uniquement en build debug (`npm run dev`, `cargo run`) ; son
activation arrête le démarrage d’un build release. Identifiants invalides et
mots de passe refusés produisent des diagnostics distincts, sans afficher leur
valeur.

Une base contenant déjà le compte ignore les identifiants de bootstrap : ils ne
remplacent jamais le mot de passe existant.

Les questionnaires et le compte sont conservés dans `rustsurvey.db` par défaut.
`DATABASE_URL=sqlite::memory:` fournit le même schéma en mémoire, avec une seule
connexion, mais perd tout à l’arrêt. Les migrations sont automatiques.

La page `#/admin` vérifie la session puis affiche la connexion ou le catalogue
d’administration. Il n’y a ni inscription publique, ni rôles, ni écran de
réinitialisation du mot de passe dans cet incrément.

## Sessions et sécurité

- Argon2id protège le mot de passe avec un sel provenant du système.
- Un secret opaque de 256 bits identifie la session ; seule son empreinte
  SHA-256 est stockée en SQL. Aucun secret d’authentification n’utilise
  `localStorage`.
- Cookie `HttpOnly`, `SameSite=Strict`, durée de 12 heures, `Secure`
  configurable. En release, servir l’interface et `/api` sous la même origine
  HTTPS.
- Une écriture exige la session et un jeton CSRF associé. Connexion et
  déconnexion exigent aussi `X-Requested-With: RustSurvey`, sans CORS ouvert.
- Cinq échecs bloquent les connexions au compte pendant une minute. Les
  vérifications Argon2 sont exécutées hors du thread HTTP, une à la fois.
- La déconnexion révoque la session en SQL. Les sessions expirées sont refusées
  puis nettoyées à la prochaine connexion réussie.

Ces sessions survivent au redémarrage avec une base persistante. Elles sont
distinctes des futures participations et des moyens de reprise des réponses. Les
migrations simultanées sur plusieurs instances restent à coordonner.

## Définition et sauvegarde

`crates/domain` porte le document natif `schemaVersion: 2`, sans dépendance à
HTTP, SQL ou LimeSurvey. Les textes, groupes, questions et choix sont typés. Le
serveur refuse versions, champs et types inconnus au lieu de les perdre.

La lecture et la sauvegarde convertissent explicitement la v1 : les choix
multiples deviennent des sous-questions avec les mêmes identifiants et codes, et
les conditions absentes valent `1`. Les révisions historiques ne sont pas
réécrites. Une nouvelle sauvegarde produit une révision v2. Les informations
source non projetées restent dans l’archive XML privée.

Les limites actuelles sont 8 Mio par document et par XML, 50 groupes, 500
questions, 200 éléments par axe, 32 langues, 2000 cellules par question, 20000
cellules de réponse au total et 100 diagnostics de validation. Les textes,
identifiants, codes uniques et noms de cellules sont contrôlés côté Rust. Un
questionnaire vide peut être enregistré pour commencer son édition.

Chaque sauvegarde ajoute une révision immuable et met à jour la version courante
dans une transaction. Une mise à jour porte `expectedRevision`. Si un autre
onglet a enregistré entre-temps, la transaction échoue avec HTTP 409 sans
écraser sa version. Le brouillon reste dans l’éditeur ; le bouton de
rechargement propose de le remplacer explicitement. L’historique est stocké mais
n’a pas encore d’interface de restauration.

Le serveur borne à deux les écritures simultanées par processus. Les routes de
ce module ont un délai de 15 secondes, des corps limités et
`Cache-Control: no-store`. Le catalogue est paginé par 50 éléments. Il ne charge
pas les définitions complètes. Les imports ont également deux places de calcul
avec `spawn_blocking` ; leur permis reste détenu jusqu’à la fin du calcul, même
si la requête expire. Une expression admet au plus 4096 unités UTF-16, 512
jetons et une profondeur de 32.

Une création ou un import n’est pas encore idempotent en cas de perte de sa
réponse HTTP. L’interface désactive l’édition pendant la sauvegarde ; elle ne
conserve pas automatiquement un brouillon non enregistré après fermeture ou
changement de page. Les garanties de collecte durable décrites dans le modèle
général ne sont pas revendiquées par cet éditeur.

## API actuelle

Les documents utilisent les noms JSON en `camelCase`. Les dates sont des
secondes Unix UTC ; les identifiants d’études sont des UUID textuels.

| Méthode | Chemin                                 | Accès et résultat                               |
| ------- | -------------------------------------- | ----------------------------------------------- |
| GET     | `/api/studies?offset=0`                | Public ; `items` et `hasMore`                   |
| GET     | `/api/studies/{id}`                    | Public ; définition courante                    |
| GET     | `/api/admin/session`                   | État, identifiant et jeton CSRF si connecté     |
| POST    | `/api/admin/login`                     | `username`, `password` ; cookie                 |
| POST    | `/api/admin/logout`                    | Révocation et suppression cookie                |
| POST    | `/api/admin/studies`                   | Admin + CSRF ; définition, HTTP 201             |
| PUT     | `/api/admin/studies/{id}`              | Admin + CSRF ; `expectedRevision`, `definition` |
| POST    | `/api/admin/imports/lss`               | Admin + CSRF ; XML brut, HTTP 201               |
| GET     | `/api/admin/studies/{id}/original.lss` | Admin ; original inchangé, téléchargement       |

La représentation enregistrée contient `id`, `revision`, `createdAt`,
`updatedAt`, `definition` et `source` (`null` pour une création native).
`source` contient l’empreinte SHA-256 et le rapport d’import, jamais le XML. Les
erreurs métier exposent un code, un message et éventuellement des diagnostics
`path`/`message`, sans détail de connexion SQL.

La [stratégie SQL](database.md) détaille les tables et leurs tests. Les
[choix d’authentification](decisions/0003-authoring.md) justifient les nouvelles
dépendances et les limites de cette première étape. La
[décision 0004](decisions/0004-rich-definitions-and-lss.md) détaille le format
enrichi, les règles et la conservation des sources.
