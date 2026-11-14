# Décision 0013 — Attente bornée des écritures participant

## Constat mesuré

Le premier banc HTTP local du 14 novembre 2026 conserve deux écritures actives.
Avec huit clients sans pause ni réessai, l'ancien `try_acquire` refuse environ
95 % des sauvegardes. À un ou deux clients, les mêmes charges passent sans
refus. Ce constat concerne le profil mesuré, pas une capacité de production.
Voir [le protocole et les résultats](../performance.md).

## Décision

Conserver deux écritures actives et ajouter au plus huit requêtes participant en
attente, pendant 250 ms au maximum. L'attente précède la lecture du corps et
l'exécution métier. Une file pleine ou un délai expiré retourne toujours
`503 write_busy`, sans modification des réponses ni des compteurs.

`COLLECTION_WRITE_QUEUE_CAPACITY` accepte 0 à 32, avec 8 par défaut. Zéro
reproduit le rejet immédiat pour une comparaison avec le même binaire. La
capacité d'écriture active, le pool SQL et les deux évaluations lourdes ne sont
pas augmentés. L'administration conserve son admission immédiate et partage les
deux écritures actives ; elle n'occupe pas la file participant.

| Option              | Intérêt                                | Limite                                  |
| ------------------- | -------------------------------------- | --------------------------------------- |
| Refus immédiat      | Très peu de mémoire en surcharge       | Refuse aussi les courtes pointes        |
| Attente sans limite | Absorbe une pointe                     | Nombre et durée des requêtes non bornés |
| Attente bornée      | Absorbe une courte pointe avec plafond | Peut encore refuser une charge soutenue |

La troisième option ne crée aucune session participant en mémoire. Les permis
sont partagés entre les clones du serveur et libérés à l'admission, à
l'expiration ou à l'annulation de la requête. Le nombre de participations SQL
n'augmente pas la taille de cette file.

Les contrôles d'accès, limites des corps, transactions, versions et reçus
idempotents restent exécutés. Une attente ne transforme jamais un refus métier
en succès. Le délai global HTTP de quinze secondes continue d'englober la
requête. Aucun schéma SQL, profil de campagne ou format d'échange ne change.

## Dépendances et vérifications

Aucune dépendance de production n'est ajoutée. L'option Tokio `test-util` est
activée uniquement comme dépendance de développement du serveur, pour avancer
une horloge virtuelle dans les tests du délai. Elle ne change pas le binaire
release.

Les tests déterministes vérifient admission après libération, capacité maximale,
annulation, expiration sans fuite de permis et file désactivée. Le banc HTTP
relit les valeurs et les versions après chaque phase, puis après redémarrage du
processus. Les rapports comparent file 0 et file 8 à charge et binaire
identiques, en séparant sauvegardes et refus.
