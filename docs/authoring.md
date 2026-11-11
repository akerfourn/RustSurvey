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

Le parcours LSS reste disponible sous `#/import`, avec ses 28 types sans
fichiers. Il conserve ses diagnostics et n’est pas converti vers les huit types
de l’éditeur. Enregistrement d’un import, édition et export LSS des définitions
natives restent à réaliser, avec conservation des éléments source.

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

`crates/domain` porte le document natif `schemaVersion: 1`, sans dépendance à
HTTP, SQL ou LimeSurvey. Les textes, groupes, questions et choix sont typés. Le
serveur refuse versions, champs et types inconnus au lieu de les perdre.

Les limites actuelles sont 1 Mio par document, 50 groupes, 500 questions, 2 à
200 choix pour les types concernés, 20000 cellules de réponse au total et 100
diagnostics de validation. Les textes, identifiants, codes uniques et noms de
cellules sont contrôlés côté Rust. Un questionnaire vide peut être enregistré
pour commencer son édition.

Chaque sauvegarde ajoute une révision immuable et met à jour la version courante
dans une transaction. Une mise à jour porte `expectedRevision`. Si un autre
onglet a enregistré entre-temps, la transaction échoue avec HTTP 409 sans
écraser sa version. Le brouillon reste dans l’éditeur ; le bouton de
rechargement propose de le remplacer explicitement. L’historique est stocké mais
n’a pas encore d’interface de restauration.

Le serveur borne à deux les écritures simultanées par processus. Les routes de
ce module ont un délai de 15 secondes, des corps limités et
`Cache-Control: no-store`. Le catalogue est paginé par 50 éléments. Il ne charge
pas les définitions complètes.

Une création n’est pas encore idempotente en cas de perte de sa réponse HTTP.
L’interface désactive l’édition pendant la sauvegarde ; elle ne conserve pas
automatiquement un brouillon non enregistré après fermeture ou changement de
page. Les garanties de collecte durable décrites dans le modèle général ne sont
pas revendiquées par cet éditeur.

## API actuelle

Les documents utilisent les noms JSON en `camelCase`. Les dates sont des
secondes Unix UTC ; les identifiants d’études sont des UUID textuels.

| Méthode | Chemin                    | Accès et résultat                               |
| ------- | ------------------------- | ----------------------------------------------- |
| GET     | `/api/studies?offset=0`   | Public ; `items` et `hasMore`                   |
| GET     | `/api/studies/{id}`       | Public ; définition courante                    |
| GET     | `/api/admin/session`      | État, identifiant et jeton CSRF si connecté     |
| POST    | `/api/admin/login`        | `username`, `password` ; cookie                 |
| POST    | `/api/admin/logout`       | Révocation et suppression cookie                |
| POST    | `/api/admin/studies`      | Admin + CSRF ; définition, HTTP 201             |
| PUT     | `/api/admin/studies/{id}` | Admin + CSRF ; `expectedRevision`, `definition` |

La représentation enregistrée contient `id`, `revision`, `createdAt`,
`updatedAt` et `definition`. Les erreurs métier exposent un code, un message et
éventuellement des diagnostics `path`/`message`, sans détail de connexion SQL.

La [stratégie SQL](database.md) détaille les tables et leurs tests. Les
[choix d’authentification](decisions/0003-authoring.md) justifient les nouvelles
dépendances et les limites de cette première étape.
