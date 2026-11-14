# Décision 0016 — Distribution du client et paquet pilote

## Décision

Le pilote sert le build web et les API depuis le même processus Rust et la même
origine. `STATIC_ASSETS_DIRECTORY` est facultatif ; absent, il conserve le
serveur API seul utilisé avec Vite en développement. Une valeur explicite
invalide arrête le démarrage avant l'ouverture de la base.

Le serveur charge `index.html` et les ressources reconnues de `assets/` dans un
instantané partagé. Aucune route HTTP n'ouvre un fichier. Seuls `/`,
`/index.html` et les clés précises chargées existent ; les autres chemins
retournent 404, y compris les API inconnues. Les routes API gardent leurs
contrôles d'accès et leurs propres corps de réponse.

## Limites et sécurité

- Index UTF-8 compilé, limité à 512 Kio, avec script et ressources présents.
- Au plus 128 entrées, fichiers de 4 Mio maximum et 8 Mio au total.
- Répertoires et fichiers réguliers ; liens symboliques refusés.
- Ressources JS, CSS, WOFF2, PNG, SVG ou ICO aux noms ASCII bornés.
- Aucun fichier caché, carte source, JSON de données ou accès de répertoire.

Le chargement et le hachage passent hors des tâches HTTP. Les corps `Bytes`
partagent les mêmes allocations ; une requête ne copie pas tout le bundle. Le
cache est immutable pour les noms portant une empreinte ; l'index et les autres
noms sont revalidés. ETag, HEAD et réponses 304 évitent les retransferts. Un
redémarrage charge un nouveau build cohérent. La compression peut être assurée
en amont ; elle n'ajoute aucun travail CPU à ce serveur dans ce lot.

La politique CSP autorise les scripts de la même origine, sans code inline ni
évaluation dynamique. Les styles inline restent admis pour les contrôles Preact
existants. Images locales, données passives et blobs couvrent les aperçus. Les
connexions réseau restent sur la même origine ; frames, plugins et soumissions
externes sont refusés. `nosniff`, absence de référent et politique de
permissions complètent les réponses. TLS et HSTS relèvent du frontal de
déploiement.

Les fichiers statiques sont des artefacts publics de confiance du déploiement.
Ce répertoire ne doit contenir ni données métier ni envois utilisateur. Le
serveur ne doit pas avoir à modifier les ressources publiques à l'exécution.

## Alternatives et coût

| Option                             | Intérêt                              | Limite                                                         |
| ---------------------------------- | ------------------------------------ | -------------------------------------------------------------- |
| Frontal statique séparé            | Cache et compression dédiés          | Configuration supplémentaire pour le pilote                    |
| Lecture du disque à chaque requête | Peu de mémoire permanente            | I/O et contrôle des chemins sur le parcours HTTP               |
| Instantané public borné            | Un processus, aucune I/O par requête | Copie permanente du petit build, redémarrage pour le remplacer |

L'instantané est retenu pour le pilote. Il n'ajoute ni dépendance, ni service,
ni nouvelle migration. Le plafond est une borne de mémoire des ressources, sans
prétendre borner à lui seul la mémoire totale du processus. Aucun gain CPU/RAM
comparatif n'est annoncé par ce choix.

## Paquet vérifié

`npm run package:pilot` construit le client et Rust en release avec les trois
pilotes SQL. Il prépare un dossier neuf sous `target/pilot/`, sans reprendre les
données locales. Binaire, build public, configuration sans mot de passe, guide
et exemples synthétiques constituent le livrable.

Le binaire copié est ensuite lancé sur des données temporaires. La recette
vérifie ressources web, cache, API, droits, participations, invitations,
réessais, redémarrages et échanges. Le manifeste n'est écrit qu'après réussite ;
il porte les empreintes des fichiers, la plateforme et l'état Git. Ce n'est pas
une signature de distribution ni un paquet universel pour tous les systèmes.

Les tests isolés contrôlent en plus limites de taille, build incomplet, fichier
modifié après chargement, méthodes interdites, traversées et absence de données
privées. Le contrôle visuel du navigateur et la qualification de l'hébergeur
restent distincts. Voir le [guide du pilote](../pilot.md).
