# Décision 0008 — Étendre la collecte aux 28 types natifs

## Statut

Implémentée pour les créations natives, sans certification des imports
LimeSurvey. Complète la [décision 0006](0006-native-collection.md).

## Décision

Les nouvelles campagnes utilisent `native-forms-v1`. `native-simple-v1` reste un
profil distinct sélectionné depuis la campagne enregistrée. Aucun changement de
version d’une définition, réécriture historique ou nouvelle migration SQL n’est
nécessaire pour ajouter ce profil.

Réutiliser le parseur et l’évaluateur bornés. Ajouter les cellules des tableaux,
doubles échelles, matrices, classements et commentaires multiples. Les
expressions d’axes sont partagées en mémoire, sans être recompilées pour chaque
cellule de matrice. Le domaine reste indépendant de HTTP, SQL et LimeSurvey.
Aucune dépendance n’est ajoutée.

Le serveur interprète les filtres M/P, les conditions des deux axes, les modes
de commentaire P, les domaines par échelle, les curseurs et les bornes/pas des
matrices. Il efface les cellules inactives avant les dépendances suivantes. Une
matrice de cases compte les valeurs `1` et vérifie chaque ligne obligatoire. Une
double échelle compte les lignes complètes pour les contraintes de nombre. Un
classement refuse doublons et trous, y compris en brouillon.

Les bornes et pas s’appliquent aux listes numériques ; les saisies libres des
matrices n’ont que leurs bornes explicites. Le nombre de choix affichés d’une
matrice est plafonné à 2000. La limite globale de réponse est contrôlée avant et
après les calculs, pour que leurs résultats ne contournent pas le plafond de
stockage. Le thème natif `core` de l’éditeur est autorisé.

## Vérifications et limites

Un corpus commun décrit 21 parcours et les attentes des deux moteurs. Les tests
HTTP enregistrent les 28 types avec le document produit par le modèle de
l’éditeur, vérifient refus des réponses invalides, calculs et réessais. Ils
vérifient aussi qu’une ancienne campagne utilise toujours son ancien profil.

Les attributs non reconnus restent bloquants ; `max_subquestions`, les variantes
de présentation, médias, fichiers, aléatoire, quotas et expressions dans les
textes ne sont pas annoncés pris en charge. Les tests internes ne remplacent pas
des fixtures exécutées dans LimeSurvey 6.5.14. Les fonctionnalités compatibles
avec un `.lss` conservé ne suffisent toujours pas à ouvrir une collecte
importée.
