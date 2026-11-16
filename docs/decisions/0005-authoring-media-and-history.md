# 0005 — Édition étendue, historique et médias natifs

## Décision du 13 novembre 2026

L’éditeur natif expose les 28 types sans téléversement participant déjà rendus
par le moteur. Un registre explicite décrit choix, sous-questions, colonnes,
double échelle et option « autre ». Les identifiants sont conservés lorsque
l’axe reste applicable ; une nouvelle dimension reçoit de nouveaux identifiants.
Les propriétés propres à un ancien type sont retirées lors du changement.

Les panneaux n’exposent que des options interprétées par le viewer : calculs,
contraintes, filtres, curseurs, matrices et textes de présentation. Rust vérifie
les règles des sous-questions, calculs et contraintes, ainsi que les bornes
littérales et les sources de filtres. Une validation peut lire sa propre réponse
sans créer un cycle d’affichage ; pertinence et calculs restent acycliques.
Cette validation de définition ne remplace pas un moteur Rust de réponses.
Sérialisation, validation et analyse des règles passent par `spawn_blocking`,
avec deux places partagées avec les traitements LSS. Le permis reste dans le
calcul jusqu’à sa fin même si la requête est annulée.

## Traductions et historique

Les langues et traductions natives s’éditent sans modifier les identifiants ou
les codes de choix. Un changement de langue principale sauvegarde les textes de
la précédente avant de charger la traduction choisie. Les traductions retirées
du parcours sont conservées. Les nouvelles traductions partent du texte de base.

L’historique privé est paginé par révision. Restaurer un instantané crée une
nouvelle révision avec contrôle de concurrence. Aucun document historique n’est
remplacé. Les règles déjà acceptées à l’import restent restaurables, même
lorsqu’elles dépassent le profil d’expression natif.

Un export JSON contient seulement la définition. Il peut être importé comme
nouvelle étude, avec validation serveur. Les champs inconnus sont transmis à
Rust pour rejet ; le client ne doit pas les supprimer avant l’envoi. Ce format
ne transporte ni images ni XML original ; il ne constitue pas une sauvegarde
complète d’un import.

## Format natif v3

Chaque question peut référencer au plus huit images par identifiant logique,
description alternative et langue optionnelle. La v3 explicite cet ajout. La
conversion v1 → v2 conserve les choix multiples ; v2 → v3 ajoute une liste de
médias vide. Une ancienne version contenant des références v3 est refusée. Les
anciennes révisions restent inchangées en SQL.

La migration 0003 ajoute objets, métadonnées de médias et associations aux
révisions. Ces dernières sont vérifiées et enregistrées dans la transaction de
sauvegarde. L’historique conserve donc les images nécessaires à une
restauration. La [décision 0002](0002-object-storage.md) décrit envois et
lectures.

La nouvelle crate locale `rustsurvey-object-store` dépend de SHA-256 déjà
présent. Elle n’ajoute aucune dépendance externe au graphe. `http-body`, déjà
transitif via Axum, devient direct pour conserver le permis de lecture dans un
corps HTTP découpé en blocs. Aucun SDK S3 ni service annexe n’est introduit.

## Réexport LSS ciblé

Les imports gardent leur structure protégée ; seules les pertinences de groupe
et question sont modifiables. Le réexport d’une révision remplace les champs
`grelevance` et `relevance` identifiés par leurs alias source, ajoute si besoin
leur déclaration et conserve les autres octets. Une analyse bornée du résultat
contrôle de nouveau sa structure. Le traitement reste hors du thread HTTP, avec
deux places partagées avec les imports.

Une section d’anciennes `conditions` non vide bloque une transformation de
pertinence, car l’importeur pourrait régénérer une règle contradictoire. Un
export sans modification reste identique. L’archive originale reste immuable et
accessible séparément. Les tests synthétiques ne certifient pas l’import
fonctionnel dans LimeSurvey 6.5.14 ; cette vérification reste à réaliser.

## Limites restantes

La [décision 0006](0006-drafts-and-publications.md) remplace depuis le 16
novembre le comportement de publication décrit ci-dessous : l’enregistrement
alimente un brouillon privé et une action explicite publie pour les nouveaux
arrivants, sans modifier les parcours déjà ouverts.

Publication distincte du brouillon, campagnes figées, moteur Rust d’évaluation,
collecte, reprise des participations et migration d’études en cours restent à
implémenter. Les modifications sont encore publiques dès leur sauvegarde ; les
réponses restent locales. S3, disque, nettoyage des objets, transfert des médias
LimeSurvey et téléversements participants restent des incréments séparés.
