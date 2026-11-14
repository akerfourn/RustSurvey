# Stockage objet S3 privé

## Configuration

Le backend `s3` conserve les binaires hors du serveur Rust. Les définitions,
droits, références et l'index des objets restent en SQL. Avec un `DATABASE_URL`
PostgreSQL ou MariaDB explicite, ce mode ne demande aucun disque persistant au
processus Rust. SQLite reste adapté au développement local.

Configurer les variables privées du serveur, sans les transmettre au navigateur
:

```dotenv
OBJECT_STORE_BACKEND=s3
S3_ENDPOINT=https://s3.example.invalid
S3_BUCKET=rustsurvey-private
S3_REGION=us-east-1
S3_PREFIX=rustsurvey/v1
S3_ACCESS_KEY_ID=valeur-privee
S3_SECRET_ACCESS_KEY=valeur-privee
```

`S3_REGION` est explicite et doit correspondre au fournisseur. Un jeton
temporaire peut être fourni dans `S3_SESSION_TOKEN` ; omettre cette variable
lorsqu'elle est inutile. Les identifiants sont statiques pour la durée du
processus ; leur renouvellement exige un redémarrage. Il n'existe aucune lecture
automatique de métadonnées d'instance ou de fichier de profil AWS.

L'endpoint est une origine HTTPS, sans chemin, informations d'utilisateur,
requête ou fragment. Les requêtes utilisent l'adressage par chemin
`endpoint/bucket/prefix/key/digest`. Le préfixe utilise des segments ASCII
alphanumériques, `-` et `_`, séparés par `/` ; sa longueur maximale est 128. Les
noms de fichiers des participants ou administrateurs ne servent jamais de chemin
objet. TLS vérifie les certificats et le nom du serveur avec les racines du
système. Aucun réglage ne désactive cette vérification.

Pour une fixture locale seulement, `S3_ALLOW_HTTP_LOOPBACK=true` permet HTTP sur
une adresse IP de boucle locale littérale. Cette option n'autorise ni un hôte
distant ni un nom DNS. Redirections et proxys implicites sont désactivés.

## Confidentialité et droits

Créer un conteneur privé avec des accès limités au préfixe RustSurvey. Le compte
applicatif utilise `GetObject`, `PutObject` et `DeleteObject` sur ses objets ;
l'énumération et la création de conteneur ne font pas partie du parcours. Le
serveur ne rend aucun objet public et ne génère aucune URL présignée. Les médias
et originaux privés passent toujours par les contrôles HTTP RustSurvey.

Les erreurs n'incluent ni les corps d'erreur du fournisseur, ni les
identifiants, signatures, chemins privés ou secrets. Les en-têtes de signature
et de jeton sont marqués sensibles. Le signataire reçoit une empreinte
précalculée plutôt que le contenu des réponses.

Au démarrage, un petit objet de contrôle, avec une clé aléatoire neuve, vérifie
écriture, conflit, lecture et suppression. Son index et son contenu sont retirés
après le contrôle. Un échec arrête le démarrage avec un message borné. Une
coupure brutale ou un échec de suppression peut laisser cet objet sans
référence.

L'architecture permet un service S3 géré ; elle ne qualifie aucune offre HDS. Le
choix du contrat, de la région et de la protection au repos appartient au
déploiement. Pour Cellar, reprendre l'endpoint et les accès fournis par
[Clever Cloud](https://www.clever.cloud/developers/doc/deploy/storage/cellar/),
puis vérifier cette installation. Aucun compte distant n'a été utilisé dans les
essais du dépôt.

## Immutabilité et reprise

La migration 0011 ajoute `s3_objects` : clé opaque unique, empreinte du lieu de
stockage, SHA-256 et taille. Aucun binaire n'entre dans cette table.

1. Vérifier que la clé logique n'est pas publiée.
2. Calculer SHA-256 hors des tâches HTTP.
3. Envoyer le contenu à `prefix/key/sha256` avec signature SigV4.
4. Publier sa correspondance par une insertion SQL unique.

Deux contenus différents pour une même clé possèdent des chemins physiques
différents. Une seule insertion SQL gagne ; l'autre écriture ne change pas le
contenu publié. Deux écritures identiques peuvent partager le même chemin sans
changer ses octets. Cette garantie ne dépend pas de `If-None-Match`, que Garage
2.5.0 ignore dans le test réel.

L'en-tête conditionnel est tout de même envoyé. Si le fournisseur répond 412, le
contenu existant est relu et vérifié avant publication. Après une écriture S3
réussie suivie d'un échec SQL, un nouvel essai peut ainsi terminer la
publication. Un contenu perdu, corrompu ou associé à un autre emplacement
produit une erreur, sans lecture de secours qui masquerait le problème.

Un échec SQL ou une concurrence perdue peut laisser un contenu non publié. Aucun
nettoyage automatique n'est ajouté : supprimer aveuglément ce contenu pourrait
toucher une publication concurrente identique. Un futur collecteur devra
vérifier l'index, les références et une durée de rétention.

## Limites de ressources

Le contrat reste limité à 8 Mio par objet, avec quatre opérations actives et
sans attente non bornée. Le client HTTP est partagé, limité à quatre connexions
inactives. Connexion : cinq secondes ; lecture inactive : quinze secondes ;
requête complète : trente secondes. Les erreurs remontent sans relance
automatique cachée ; les parcours applicatifs possèdent leur propre réessai.

Les lectures vérifient taille, format et SHA-256 contre l'index SQL avant de
rendre le contenu. Une plage demandée lit et vérifie l'objet entier avant de le
découper, comme les adaptateurs SQL et fichiers. Il ne s'agit pas encore d'un
téléchargement S3 par plages ni d'un transfert de gros fichiers en flux.

Aucun état participant ou cache de binaires ne croît avec le nombre de sessions.
Le coût CPU/RAM de S3 n'a pas encore été mesuré ; les anciens rapports natifs ne
constituent pas une mesure de cet adaptateur.

## Données existantes et sauvegarde

Le mode S3 conserve la lecture des anciens objets SQL lorsqu'aucune
correspondance S3 n'existe pour leur clé. Les nouveaux objets vont dans S3. Une
erreur S3, un index incohérent ou une corruption ne déclenchent pas ce repli.

Les objets locaux ne sont pas copiés automatiquement. Passer du mode fichiers à
S3 exige une copie explicite avec publication de l'index ; l'outil de copie
reste à réaliser. Ne pas alterner les adaptateurs d'écriture entre plusieurs
instances de la même installation. Changer endpoint, conteneur ou préfixe ne
constitue pas une migration : l'empreinte du lieu protège contre un changement
silencieux.

Une sauvegarde cohérente conserve SQL et les contenus S3 correspondants, y
compris l'historique et les envois en attente. Garder les réglages d'emplacement
avec cette sauvegarde ; les identifiants peuvent être renouvelés séparément.

## Vérification locale reproductible

Préparer explicitement l'image de référence :

```sh
docker pull dxflrs/garage:v2.5.0@sha256:677e5b758e4425a325b811c425c026bf35a4bf42f7ddbaa773de3f4fd779a5a5
npm run test:s3
```

Le lanceur ne télécharge aucune image. Il crée un conteneur à ressources
bornées, son réseau propre et des données synthétiques en mémoire. Seul le port
S3 est publié sur la boucle locale. Les contrôles et la configuration sont
retirés à la sortie. Garage est un outil de test, sans dépendance de production.

Le test réel vérifie signature correcte et incorrecte, contrat commun, SQL
historique, accès aux images, restauration, originaux LSR et arrêt complet du
processus Rust avant relecture et répétition d'envoi. Les tests isolés ajoutent
concurrence, panne de publication SQL, corruption, bornes, redirection et
indisponibilité. La [décision 0015](decisions/0015-s3-indexed-content.md)
détaille les alternatives et les dépendances.
