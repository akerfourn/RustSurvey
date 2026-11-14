# Décision 0010 — Préparer les réponses avant publication

## Statut

Implémentée pour les fichiers LSR du schéma 623 associés à un LSS déjà importé.
La conversion en participations exécutables et reprenables reste à réaliser.

## Décision

La migration 0007 crée `response_imports` et `response_import_records`, sans
modifier les réponses natives. Une préparation fige étude, révision, empreinte
du LSS, original LSR, schéma de colonnes et correspondances de cellules.

Les valeurs source sont conservées comme chaînes. Le schéma partagé porte les
colonnes, et chaque ligne stocke seulement ses valeurs non nulles. Cette
représentation conserve `NULL`, vide et absence de colonne, sans répéter un
grand dictionnaire de valeurs nulles dans chaque ligne. Les données inconnues ne
deviennent pas exécutables du seul fait d'avoir été conservées.

Les binaires passent par le port objet existant. SQL garde les métadonnées et
les lignes préparées. Aucune dépendance externe n'est ajoutée et aucun accès au
disque n'entre dans la préparation métier.

## Alternatives

| Option                            | Intérêt                                      | Limite                                                        |
| --------------------------------- | -------------------------------------------- | ------------------------------------------------------------- |
| Charger dans les réponses natives | Parcours immédiatement commun                | Risque de modifier règles, états et droits non vérifiés       |
| Archiver seulement le fichier     | Conservation minimale                        | Pas de correspondances ni de chargement reprenable des lignes |
| Préparation séparée               | Conservation, bilan et progression contrôlés | Publication et continuité restent une étape explicite         |

La troisième option est retenue comme étape du protocole complet. Elle ne réduit
pas l'objectif de migration aux fichiers : elle empêche de présenter une
conversion incomplète comme une étude opérationnelle.

## Reprise et concurrence

Les mêmes octets vers la même révision identifient une même préparation.
L'original est écrit après réservation SQL et avant chargement des lignes. Un
échec laisse une intention retrouvable ; un nouvel envoi identique peut réparer
un original manquant.

Une tentative obtient un jeton d'écriture en SQL. Chaque transaction vérifie ce
jeton et la position attendue, puis insère au plus 50 lignes et avance le
compteur dans la même transaction. Une nouvelle tentative remplace le jeton ;
l'ancienne ne peut plus écrire. Aucun verrou attaché au processus, bail basé sur
l'horloge ou traitement en arrière-plan permanent n'est nécessaire.

Une requête traite au plus 500 lignes. La suite utilise l'original conservé, y
compris après redémarrage. La publication d'une campagne reste absente de cette
transition. Les dates locales du chargement ne remplacent jamais les dates
source des réponses.

## Identités et limites

Les `FieldBinding` importés utilisent les alias source du LSS et les identités
natives. Un code identique sans identité source correspondante ne suffit pas. La
qualification d'installation devra venir du futur manifeste de transfert. Deux
fichiers différents restent des instantanés séparés : leurs identifiants de
réponse ne sont pas fusionnés automatiquement.

Les valeurs sont conservées avant toute validation d'exécution. Les types
inconnus, colonnes non projetées et états incomplets produisent des diagnostics.
Le rapport distingue explicitement exécution non vérifiée et continuité non
disponible. Les jetons présents restent privés et n'accordent aucun accès.

Le recalcul du plan pendant une reprise privilégie une implémentation bornée et
vérifiable pour des fichiers de 8 Mio. Son coût devra être mesuré avant
d'étendre les volumes ; le stockage indexé des fragments est une évolution
possible. Les sauvegardes de participation ne dépendent pas de ce format de
préparation.

La [procédure](../response-imports.md) décrit l'interface, les routes et les
vérifications réalisées, ainsi que les travaux de publication encore requis.
