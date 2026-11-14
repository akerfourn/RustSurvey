# Décision 0015 — Contenus S3 et publication SQL immuable

## Problème et décision

Le port objet exige une création sans écrasement. L'API S3 propose une écriture
conditionnelle, mais sa disponibilité varie selon les implémentations. Le test
réel sur Garage 2.5.0 a montré que son PUT ignore `If-None-Match`. Une
vérification HEAD suivie d'un PUT laisserait aussi une concurrence entre ces
opérations.

L'adaptateur place donc le contenu à `prefix/opaque-key/sha256` puis publie une
correspondance SQL par insertion unique. L'index spécifique S3 est un port
ciblé, implémenté par SeaORM dans le serveur ; le domaine ne dépend ni de S3, ni
de SQL. Les quatre adaptateurs conservent le même contrat `ObjectStore`.

Une seule publication gagne. Une écriture concurrente différente utilise un
autre chemin ; une écriture identique conserve les mêmes octets. L'index reste
la source des correspondances, sans énumération de conteneur. Le lieu de
stockage est identifié par une empreinte d'endpoint, conteneur et préfixe ; les
secrets et la région n'entrent pas dans cette identité.

La migration 0011 crée quatre colonnes scalaires portables. Les métadonnées
restent dans la base déjà nécessaire à l'application. Aucun service de verrou,
volume de fichiers ni processus supplémentaire n'est requis en production.

## Alternatives

| Option                                 | Intérêt                      | Limite                                                     |
| -------------------------------------- | ---------------------------- | ---------------------------------------------------------- |
| PUT conditionnel seul                  | Peu de métadonnées locales   | Fonction absente de certains S3 compatibles                |
| Vérifier puis écrire le même chemin    | Simple                       | Concurrence et écrasement possibles                        |
| Chemins par contenu et publication SQL | Immutabilité locale portable | Une correspondance SQL et des contenus orphelins possibles |

La troisième option est retenue. Un échec après le PUT laisse un contenu
reprenable ; un nouvel essai vérifie les données présentes avant publication si
le fournisseur refuse la réécriture. Aucun nettoyage automatique d'orphelin
n'est fait sur le chemin d'écriture, pour protéger les publications
concurrentes.

## Dépendances et coût

- `reqwest` 0.13, défauts désactivés et `rustls-no-provider` : HTTP asynchrone
  borné, pool partagé et certificats système. JSON, compression, cookies,
  HTTP/2, HTTP/3 et proxys système ne sont pas activés.
- `rustls` 0.23, défauts désactivés, `ring`, `std` et `tls12` : réutilise le
  fournisseur cryptographique déjà présent pour les pilotes SQL.
- `aws-sigv4`, défauts désactivés, `sign-http` et `http1`, plus
  `aws-credential-types` : signature fournie par AWS, sans SDK S3 complet,
  résolution d'endpoint, découverte de compte ni chaîne de fournisseurs.

Le lockfile reçoit le graphe nécessaire à ces ajouts ; la version minimale du
workspace reste Rust 1.97.1. Certaines dépendances transitives concernent les
racines de confiance propres aux différents systèmes. Le signataire reçoit
seulement une empreinte précalculée ; aucun corps privé ne lui est confié.

Une écriture ajoute une vérification et une insertion SQL. Une lecture charge
une petite correspondance et son objet borné. Le contenu complet est vérifié
avant extraction d'une plage. Quatre opérations et les délais du client bornent
l'usage ; aucun gain CPU/RAM n'est revendiqué sans mesure.

## Vérification et limites

Le [guide S3](../s3-storage.md) décrit configuration, confidentialité, reprise,
sauvegarde et tests. Le serveur contrôle au démarrage une clé aléatoire neuve,
puis retire ce contrôle. Les autres instances ne voient aucune référence métier
vers cet objet. Un échec arrête le démarrage.

Garage vérifie réellement SigV4 et le contrat, y compris le comportement sans
PUT conditionnel. Les API et le processus HTTP vérifient les images et LSR sur
le même stockage. Les trois moteurs SQL vérifient la publication concurrente.
Les essais locaux ne qualifient pas un compte Cellar ou une offre HDS distante.
La copie d'anciens objets fichiers, les gros fichiers en flux, le renouvellement
automatique d'accès temporaires et le collecteur d'orphelins restent à réaliser.

Références primaires : [écritures conditionnelles AWS][conditional], [signataire
AWS][signer], [options reqwest][reqwest] et [guide Garage][garage].

[conditional]:
  https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html
[signer]: https://docs.rs/aws-sigv4/1.6.5/aws_sigv4/http_request/index.html
[reqwest]: https://docs.rs/reqwest/0.13.5/reqwest/struct.ClientBuilder.html
[garage]:
  https://github.com/deuxfleurs-org/garage/blob/main-v2/doc/book/quick-start/_index.md
