# Collecte native et reprise

## Périmètre disponible

Une campagne native fige une révision enregistrée du questionnaire. Elle accepte
des participations sans compte, avec sauvegarde SQL, reprise privée et
soumission définitive. L’éditeur et les liens `#/studies/...` restent des
aperçus locaux ; seule une campagne `#/runs/...` collecte des réponses.

Le profil `native-simple-v1` admet 17 types :

| Famille | Types                    |
| ------- | ------------------------ |
| Textes  | S, T, U                  |
| Saisies | N, D, Q, K sans curseurs |
| Choix   | L, !, 5, Y, G, M, O      |
| Autres  | I, X, *                  |

Le profil couvre pertinence de groupe/question/sous-question, obligatoire strict
ou recommandé, option autre, commentaires O, codes lexicaux, équations, langues
et contraintes numériques, de longueur et de validation. Les expressions
utilisent les 47 fonctions du moteur local. Rust réévalue les règles et remplace
les résultats calculés fournis par le navigateur.

Les tableaux, classements, commentaires P, curseurs, filtres, expressions dans
les textes, regex, aléatoire, thèmes, médias, fichiers et quotas bloquent
l’ouverture. Les attributs inconnus actifs, même de valeur `0`, sont refusés.
Une valeur vide reste sans effet. Les axes et échelles sont contrôlés.

Les imports LimeSurvey ne peuvent pas ouvrir de campagne dans cet incrément.
Leur collecte demandera un profil vérifié, les paramètres effectifs et les
correspondances des réponses. Ni les 17 types ni le corpus commun ne certifient
l’équivalence avec LimeSurvey 6.5.14.

## Essayer le parcours

1. Dans l’administration, créer un questionnaire natif du profil disponible.
2. Enregistrer puis ouvrir **Collecter des réponses** et sélectionner **Ouvrir
   une campagne sur la version enregistrée**.
3. Ouvrir le lien public de participation et commencer le questionnaire.
4. Conserver le lien privé de reprise affiché. Chaque passage au groupe suivant
   sauvegarde la progression ; le bouton d’enregistrement permet aussi de
   sauvegarder au milieu d’un groupe.
5. Recharger le lien privé, ou l’ouvrir dans un autre navigateur, pour retrouver
   les réponses, la langue et le groupe enregistrés.
6. Envoyer les réponses sur la dernière page. Une réponse soumise reste
   consultable mais ne peut plus être modifiée.
7. Dans l’administration, consulter les participations de la campagne et
   télécharger leurs réponses JSON. Les brouillons sont aussi accessibles.

Fermer une campagne refuse les nouvelles participations et écritures. Les
réponses enregistrées restent lisibles par leurs détenteurs et par
l’administration. Une modification du questionnaire ne modifie aucune campagne
existante. Ouvrir une autre campagne pour utiliser une autre révision.

Le JSON téléchargé est natif : il ne s’agit pas d’un export de réponses
LimeSurvey ni d’une archive `.lsa`.

## Droits de reprise

Le navigateur crée un UUID et un secret aléatoire de 256 bits avec Web Crypto.
Le secret est transmis dans `Authorization: Bearer ...`, jamais dans un chemin
HTTP ou une chaîne de requête. SQL conserve seulement son empreinte SHA-256.
Chaque lecture et écriture vérifie ce droit ; le compte administrateur n’est pas
un droit de reprise participant.

Le lien privé place le secret dans le fragment `#/responses/...`, qui n’est pas
envoyé au serveur HTTP. Le navigateur et les favoris peuvent conserver ce lien :
son détenteur peut lire la participation. Aucun secret n’est ajouté à
`localStorage`. Perdre le lien empêche de retrouver soi-même la participation ;
il n’existe pas encore de récupération ou révocation de ce droit. HTTPS reste
requis hors du développement local.

Les écritures exigent aussi l’en-tête de client habituel. Aucun CORS ouvert,
cookie participant ou association avec une identité invitée n’est ajouté. Cela
ne constitue pas une certification de l’anonymat d’une étude migrée.

## Validation et garanties SQL

Un brouillon vérifie types, domaines, dates, cellules connues et limites de
taille. Il peut rester incomplet ou ne pas satisfaire les contraintes finales.
Une erreur d’évaluation d’expression refuse l’enregistrement. La soumission
contrôle en plus les obligations, précisions autre et contraintes de toutes les
questions pertinentes. L’obligatoire recommandé produit un avertissement.

Le serveur efface les réponses non pertinentes avant d’évaluer leurs
dépendances. Il recalcule les équations, valide le checkpoint puis enregistre
réponses, langue et curseur dans une même transaction. Un groupe devenu caché
est remplacé par le premier groupe visible. Les cellules inconnues sont
refusées, sans recopier leurs valeurs dans les erreurs.

Chaque écriture porte `expectedVersion` et un `operationId` UUID canonique. Une
mise à jour conditionnelle garantit un seul gagnant. Un réessai de la même
opération renvoie le même accusé ; le même identifiant avec un autre contenu est
refusé. Les reçus ne sont lus qu’après vérification du secret. La création d’une
participation est aussi répétable avec le même UUID, secret, campagne et langue
initiale.

Les reçus sont conservés 24 heures, avec au plus 1024 opérations par réponse
dans cette fenêtre. Au-delà, la sauvegarde est refusée jusqu’à expiration. Les
reçus expirés de la réponse sont nettoyés lors de son prochain enregistrement.
Un ancien réessai hors fenêtre peut produire un conflit ; il faut relire la
réponse avant de continuer.

Une mise à jour de la ligne campagne ordonne les écritures et sa fermeture.
Cette première garantie sérialise les transactions d’une campagne ; son coût
reste à mesurer avant toute optimisation. Aucun verrou en mémoire attaché à une
participation ni aucune session participant par processus n’est nécessaire.

## Limites de ressources

- Corps HTTP : 2 Mio, ou 4 Kio pour créer une participation ; chaînes de réponse
  : 65 536 unités UTF-16 par cellule.
- Somme des clés et valeurs de réponse : 1 Mio avant stockage.
- Limites structurelles de l’éditeur : 500 questions et 20 000 cellules.
- Expressions : 4096 unités UTF-16, 512 jetons et profondeur de parseur 32.
- Budget partagé d’un million d’unités par évaluation du questionnaire.
- Deux préparations/évaluations/imports et deux écritures par processus, lecture
  et décodage des corps d’écriture compris.
- Délai HTTP serveur : 15 secondes ; client de collecte : 20 secondes.
- Listes administrateur : 50 campagnes récentes et 20 participations par page.

Les réponses et leur checkpoint sont, pour ce premier profil borné, un document
JSON sur la ligne `responses`. Les blocs par groupe restent une évolution du
stockage, sans modification de la notion de cellule publique. Les métadonnées de
liste ne chargent pas les réponses ; chaque téléchargement administrateur porte
sur une seule participation.

## API

| Méthode  | Route                                                  | Accès                                            |
| -------- | ------------------------------------------------------ | ------------------------------------------------ |
| POST/GET | `/api/admin/studies/{id}/runs`                         | Admin ; création avec CSRF et `expectedRevision` |
| POST     | `/api/admin/runs/{id}/close`                           | Admin et CSRF                                    |
| GET      | `/api/runs/{id}`                                       | Public ; révision figée, jamais de réponses      |
| POST     | `/api/runs/{id}/responses`                             | Secret et client ; `responseId`, `language`      |
| GET/PUT  | `/api/responses/{id}`                                  | Secret ; écriture avec en-tête de client         |
| GET      | `/api/admin/runs/{id}/responses?offset=0`              | Admin ; métadonnées paginées                     |
| GET      | `/api/admin/runs/{runId}/responses/{id}/response.json` | Admin ; téléchargement privé                     |

Le corps PUT contient `expectedVersion`, `operationId`, `answers`,
`checkpoint: { language, groupId }` et `submit`. L’accusé ne contient pas les
réponses : `id`, `version`, `status`, `checkpoint`, `updatedAt`, `warnings`. GET
fournit l’état enregistré et la définition figée. Toutes ces réponses HTTP
utilisent `Cache-Control: no-store`.

## Vérifications et suite

Le corpus `examples/rules/evaluation.json` compare 87 cas entre Rust et
TypeScript. `collection.json` compare calculs, visibilité, effacement des
réponses, codes, avertissements et erreurs sur dix parcours synthétiques. Ce
sont des comparaisons internes, pas des fixtures certifiées LimeSurvey.

Les tests HTTP couvrent accès privés, révision figée, doublons, concurrence,
soumission, fermeture, migration, limites et reprise SQLite fichier. Le test
`npm run test:storage` vérifie aussi les réponses et reçus avec un vrai serveur
arrêté puis redémarré, y compris après reprise d’une ancienne base racine. Le
test multi-bases optionnel inclut le parcours de collecte ; PostgreSQL et
MariaDB doivent être exécutés sur des bases jetables pour valider la
migration 0004.

Restent notamment : étendre le moteur aux 28 types du viewer, vérifier les
imports, `.lsa`, participants, transferts de reprises, blocs de réponses,
révocation, suppression/rétention, quotas, fichiers, exploitation et mesures
CPU/RAM comparées à LimeSurvey.
