# 0003 — Définitions SQL et administration initiale

## Décision

Un unique administrateur protège les écritures. Les questionnaires sont publics
et les réponses restent locales. SQLite fichier est le défaut ; SQLite mémoire
sert aux tests et aux essais volatils. PostgreSQL et MariaDB utilisent le même
accès aux données SeaORM/SeaQuery, sans opérateurs JSON propres à un moteur.

Le domaine natif versionné est distinct de la projection LSS. Chaque sauvegarde
conserve un document immuable, avec mise à jour conditionnelle de la révision
courante. Les prochaines extensions de document devront prévoir leur conversion
explicite. L’historique ne remplace pas une sauvegarde de la base.

## Alternatives

| Option                | Avantage                                 | Coût                                         |
| --------------------- | ---------------------------------------- | -------------------------------------------- |
| Session opaque en SQL | Révocation et reprise du serveur simples | Lecture SQL par commande protégée            |
| Jeton signé autonome  | Évite une lecture de session             | Révocation, clés et droits périmés à traiter |
| Session en mémoire    | Mise en route courte                     | Perd les connexions au redémarrage           |

La session SQL est retenue. Aucun état par visiteur public n’est créé. Redis,
JWT, fournisseur d’identité et framework d’authentification ne sont pas requis
pour ce périmètre. L’administration reste volontairement limitée à un compte.

## Dépendances ajoutées

- `argon2` 0.6, options `alloc` et `password-hash`, défauts désactivés : calcul
  Argon2id et vérification du format PHC. Paramètres par défaut de cette version
  : mémoire de 19 Mio, deux passages et parallélisme de un. Ce minimum suit la
  [recommandation OWASP][passwords]. L’exécution est isolée avec
  `spawn_blocking` et un sémaphore conserve la borne même en cas d’annulation.
- `getrandom` 0.4 : entropie du système pour sels, sessions, CSRF et UUID. Le
  temps et l’entropie sont injectables pour les tests déterministes.
- `sha2` 0.11, défauts désactivés : empreintes SHA-256 des secrets de session à
  forte entropie. Il n’est jamais utilisé pour protéger un mot de passe.
- `subtle` 2.6, défauts désactivés : comparaison du jeton CSRF en temps constant
  pour des chaînes de même longueur.
- `uuid` 1.29, seule option `std` : construction et validation des identifiants
  UUIDv4, avec les octets aléatoires déjà fournis par `getrandom`.
- Option `query` d’Axum : lecture typée de la pagination publique.
- `dotenvy` 0.15, défauts désactivés : lecture du `.env` du répertoire courant.
  Cette crate était déjà une dépendance transitive de SQLx. Son [itérateur de
  lecture][dotenv] est utilisé pour construire une configuration propre au
  serveur, sans mutation de l’environnement du processus, sans code `unsafe`
  dans le projet et sans exposer les lignes fautives dans les erreurs.
  L’environnement existant reste prioritaire. Le fichier d’exemple contient
  seulement des identifiants publics de test et le `.env` réel reste ignoré.

Les versions exactes sont verrouillées par `Cargo.lock`. Aucune dépendance
client supplémentaire n’est ajoutée. Les cookies sont écrits directement avec un
nom et des attributs constants ; leur seule valeur variable est hexadécimale.

## Limites assumées

Les sauvegardes sont immédiatement publiques, sans état de publication. L’import
LSS n’est pas réduit au modèle des huit types éditables. Sa conservation et sa
conversion demandent un incrément distinct.

La gestion de plusieurs administrateurs, la réinitialisation du mot de passe,
les reçus d’idempotence, les campagnes figées et les réponses persistantes
restent à réaliser. Le déploiement multi-instance devra coordonner migrations et
limites de concurrence. Les médias restent soumis à la décision 0002 sur le
stockage objet.

[passwords]:
  https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
[dotenv]: https://docs.rs/dotenvy/0.15.7/dotenvy/fn.from_read_iter.html
