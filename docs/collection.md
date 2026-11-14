# Collecte native et reprise

## Périmètre disponible

Une campagne native fige une révision enregistrée du questionnaire. Elle accepte
des participations sans compte, avec sauvegarde SQL, reprise privée et
soumission. Les [règles de participation](participation.md) permettent de
limiter les envois par navigateur ou invitation et d’autoriser certaines
corrections. L’éditeur et les liens `#/studies/...` restent des aperçus locaux ;
seule une campagne `#/runs/...` collecte des réponses.

Les campagnes créées par l'interface utilisent `native-pages-v1`, ou
`native-defaults-v1` lorsqu'elles contiennent des défauts. Elles couvrent les 28
types natifs sans fichiers et une présentation configurable :

| Famille  | Types                         |
| -------- | ----------------------------- |
| Textes   | S, T, U, Q                    |
| Saisies  | N, D, K avec ou sans curseurs |
| Choix    | L, !, 5, Y, G, M, O, P        |
| Tableaux | A, B, C, E, F, H, 1, :, ;     |
| Autres   | R, I, X, *                    |

Le profil couvre aussi les filtres de lignes, conditions des deux axes, matrices
de cases et leurs bornes/pas. Le thème natif `core` utilisé par l’éditeur est
admis.

Le [profil des valeurs par défaut](default-values.md) ajoute l'initialisation
explicite par page et langue sur 21 types. Il conserve une chaîne vide saisie
volontairement, initialise les pages futures à leur entrée et garde les anciens
profils inchangés. Le serveur valide et conserve les réponses canoniques avec
leur curseur ; le contexte NAOK reste explicitement limité.

Les [filtres en cascade](array-filters.md) acceptent 14 types de source, dont
les tableaux, champs multiples, classements et Autre. Les valeurs source sont
lues après normalisation. Les listes radio et classements peuvent aussi être
filtrés. Un corpus de 46 cas est comparé aux règles de lignes et de choix du
moteur LimeSurvey, sur les mêmes valeurs ; les cas de listes/classements
comparent aussi l'obligatoire. Cela ne certifie pas tous les parcours
participant.

Les [lignes exclusives](exclusive-options.md) des types A/B/C/E/F/M/P/K/Q et la
sélection automatique M/P sont également disponibles. Rust efface les cellules
désactivées avant de valider et de stocker la réponse. Plusieurs lignes
exclusives renseignées simultanément produisent une erreur sans écriture.

Les campagnes déjà enregistrées avec `native-simple-v1` conservent leurs 17
types et leur interpréteur. Le serveur sélectionne explicitement le profil
enregistré ; il ne convertit pas les campagnes ou révisions historiques.
`native-forms-v1` conserve aussi son parcours par groupe. Une création API sans
réglages de navigation continue d'utiliser ce profil.

Le profil couvre pertinence de groupe/question/sous-question, obligatoire strict
ou recommandé, option autre, commentaires O, codes lexicaux, équations, langues
et contraintes numériques, de longueur et de validation. Les expressions
utilisent les 47 fonctions du moteur local. Rust réévalue les règles et remplace
les résultats calculés fournis par le navigateur.

Les expressions dans les textes, regex, aléatoire, thèmes personnalisés, médias,
fichiers et quotas bloquent l’ouverture. Les attributs inconnus actifs, même de
valeur `0`, sont refusés. Une valeur vide reste sans effet. Les axes et échelles
sont contrôlés.

Les imports LimeSurvey disposent d'un [profil distinct](import-execution.md)
pour ouvrir des campagnes neuves après qualification des paramètres effectifs,
fonctions et correspondances. La reprise des anciennes participations est
différée. Ni les 28 types ni le corpus commun ne certifient l'équivalence
générale avec LimeSurvey 6.5.14.

## Présentation et reprise

Avant d'ouvrir une campagne, choisir **Un groupe par page**, **Une question par
page** ou **Tout sur une page** dans **Présentation de la campagne**. La case
**Afficher la progression** commande la barre et le compteur. Ces réglages sont
figés avec la campagne ; ils ne modifient pas les questions de sa révision.

Le retour aux pages précédentes est autorisé. Dans le nouveau profil, avancer,
revenir ou enregistrer sauvegarde le curseur et les réponses ensemble. La
reprise retrouve la question précise, même lorsque plusieurs questions partagent
un groupe. Une page masquée par les nouvelles réponses revient sur la première
page visible. Les calculs cachés continuent d'être évalués sans créer d'écran.

La validation de page ne porte que sur les questions affichées. L'envoi final
reste contrôlé sur toutes les réponses pertinentes par Rust. Un questionnaire
sans question pertinente conserve un écran avec le bouton d'envoi.

Les modes correspondent aux formats LimeSurvey G/S/A. Leur import et les
indicateurs de progression et de retour sont vérifiés dans la référence ; cela
ne certifie pas tous les parcours de formulaire. L'index, l'interdiction du
retour et les autres paramètres restent à compléter. Voir la
[décision 0012](decisions/0012-native-navigation.md).

## Essayer le parcours

1. Dans l’administration, créer un questionnaire natif du profil disponible.
2. Enregistrer puis ouvrir **Collecter des réponses**, choisir l’accès public ou
   sur invitation et sélectionner **Ouvrir une campagne sur la version
   enregistrée**.
3. Ouvrir le lien public de participation et commencer le questionnaire.
4. Conserver le lien privé de reprise affiché. Chaque passage au groupe suivant
   sauvegarde la progression ; le bouton d’enregistrement permet aussi de
   sauvegarder au milieu d’un groupe.
5. Recharger le lien privé, ou l’ouvrir dans un autre navigateur, pour retrouver
   les réponses, la langue et le groupe enregistrés.
6. Envoyer les réponses sur la dernière page. Une réponse soumise reste
   consultable. Sa modification exige une invitation avec reprise et correction
   explicitement autorisées lors de l’ouverture de la campagne.
7. Dans l’administration, consulter les participations de la campagne et
   télécharger leurs réponses JSON. Les brouillons sont aussi accessibles.

Fermer une campagne refuse les nouvelles participations et écritures. Les
réponses enregistrées restent lisibles par leurs détenteurs et par
l’administration. Une modification du questionnaire ne modifie aucune campagne
existante. Ouvrir une autre campagne pour utiliser une autre révision.

Le JSON téléchargé conserve l'état natif. Après fermeture de la campagne,
**Exporter toutes les réponses (VV LimeSurvey)** et **Exporter la structure
LSS** permettent un premier [échange des données](response-exchange.md). Les
invitations, ressources et moyens de reprise ne font pas partie de ces deux
fichiers ; ils ne constituent pas une archive complète d'étude.

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
il n’existe pas encore de récupération. La révocation d’une invitation avec
reprise bloque aussi ce droit lié ; le droit distinct de lire une réponse
indépendante n’est pas révoqué dans cet incrément. HTTPS reste requis hors du
développement local.

Les écritures exigent aussi l’en-tête de client habituel. Les invitations et le
cookie facultatif sont décrits dans [leur périmètre](participation.md). Aucun
CORS ouvert n’est ajouté. Cela ne constitue pas une certification de l’anonymat
d’une étude migrée.

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
- Somme des clés et valeurs de réponse : 1 Mio, calculs serveur compris.
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

La route privée `GET /api/admin/runs/{id}/responses.vv` exporte progressivement
les réponses d'une campagne fermée. La
[décision 0009](decisions/0009-response-exchange.md) décrit ses correspondances,
limites et garanties. La langue initiale des nouvelles participations est
conservée séparément du checkpoint courant.

`GET /api/admin/runs/{id}/definition.lss` exporte la structure figée avec le
format de cette campagne. Il accepte le même `surveyId` facultatif que l'export
d'une révision, qui garde une présentation par groupe par défaut.

La création accepte `navigation: { format, showProgress }`, avec `format` parmi
`group`, `question`, `all`. Les deux champs sont requis si l'objet est fourni.
La réponse de campagne contient `navigation`, nul pour les anciens profils.

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
`checkpoint: { language, groupId, pageId? }` et `submit`. `pageId` appartient au
profil `native-pages-v1` ou `native-defaults-v1` ; un curseur de groupe/question
doit porter le bon `groupId`, nul pour `all` et `empty`. L’accusé ne contient
pas les réponses : `id`, `version`, `status`, `checkpoint`, `updatedAt`,
`warnings`. GET fournit l’état enregistré et la définition figée. Toutes ces
réponses HTTP utilisent `Cache-Control: no-store`.

## Vérifications et suite

Les écritures participant conservent deux places actives. Une courte pointe peut
attendre dans une file de huit requêtes, pendant 250 ms au maximum.
`COLLECTION_WRITE_QUEUE_CAPACITY` accepte 0 à 32 ; zéro conserve le rejet
immédiat. Une file pleine ou un délai expiré retourne `503 write_busy`. Cette
attente précède la lecture du corps et ne modifie aucune règle de collecte. Voir
[la décision](decisions/0013-bounded-write-admission.md) et le
[protocole CPU/RAM](performance.md).

Le corpus `examples/rules/evaluation.json` compare 87 cas entre Rust et
TypeScript. `collection.json` compare calculs, visibilité, effacement des
réponses, codes, avertissements et erreurs sur dix parcours synthétiques. Ce
corpus est complété par `collection-forms.json` : 21 parcours sur les 28 types,
filtres, axes, curseurs, classements et commentaires. Ce sont des comparaisons
internes, pas des fixtures certifiées LimeSurvey.

`exclusive.json` ajoute 21 scénarios partagés sur les lignes exclusives,
commentaires, option Autre, pertinence, filtres et sélection automatique. Les
tests HTTP, multi-bases et de redémarrage couvrent leur normalisation durable.

Les tests HTTP couvrent accès privés, révision figée, doublons, concurrence,
soumission, fermeture, migration, limites et reprise SQLite fichier. Le test
`npm run test:storage` vérifie aussi les réponses et reçus avec un vrai serveur
arrêté puis redémarré, y compris après reprise d’une ancienne base racine. Le
test multi-bases optionnel inclut le parcours de collecte ; PostgreSQL et
MariaDB ont été exécutés le 14 novembre 2026 sur des bases jetables pour valider
les migrations 0004 et 0005, les invitations et les 28 types.

Restent notamment : vérifier les imports, `.lsa`, participants nominatifs,
transferts de reprises, blocs de réponses, révocation des droits indépendants,
suppression/rétention, quotas, fichiers, exploitation et mesures CPU/RAM
comparées à LimeSurvey.
