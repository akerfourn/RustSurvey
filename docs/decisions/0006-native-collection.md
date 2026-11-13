# 0006 — Première collecte native durable

## Décision

Cette décision décrit `native-simple-v1`, conservé pour les campagnes
existantes. Les [règles de participation](0007-participation.md) et le
[profil de 28 types](0008-collection-forms.md) la complètent.

Livrer un profil de collecte explicite avant d’annoncer une exécution générale
des études importées. Réutiliser le modèle natif v3, le parseur Rust existant,
SeaQuery et les migrations. Aucune dépendance n’est ajoutée.

Le [profil de collecte](../collection.md) couvre 17 types et le sous-ensemble
commun d’expressions. Les fonctions hors profil bloquent l’ouverture d’une
campagne. Le viewer local continue à couvrir ses 28 types sans imposer leur
collecte. Les données LimeSurvey restent conservables sans être exécutables.

## Moteur partagé par comportement

Le parseur Rust construit désormais un arbre réutilisable pour la validation de
définition et l’évaluation. Les 47 fonctions du moteur TypeScript sont
exécutables côté Rust avec limites de taille, contrôles des résultats non finis
et budget commun au questionnaire. Les corpus JSON versionnés vérifient les deux
implémentations sans réseau ni horloge réelle. Ils ne remplacent pas une
comparaison avec les comportements de la version source.

Le domaine reste indépendant de HTTP, SQL et LimeSurvey. WASM n’est pas
nécessaire à ce premier profil.

## Révisions et réponses

Une campagne référence une révision immuable avec clé étrangère composée.
Ouverture et fermeture sont des actions administrateur explicites. Modifier la
définition courante ne change pas une campagne existante.

Une réponse porte version, état, empreinte du droit de reprise et document borné
contenant réponses et checkpoint. Une seule ligne JSON est retenue pour ce
premier profil ; la fragmentation par groupes logiques reste possible via une
migration ultérieure. Aucun opérateur JSON SQL n’est utilisé.

Une transaction vérifie la version attendue, conserve l’état et ajoute un reçu
d’opération. Les reçus de 24 heures, plafonnés à 1024 par participation, bornent
le coût des réessais. Ils stockent seulement un accusé, sans copie des réponses.
Une mise à jour réelle de la ligne campagne ordonne les sauvegardes avec la
fermeture, y compris sur MariaDB où une mise à jour sans changement ne garantit
pas le même compteur de lignes modifiées.

Cette première solution privilégie la correction ; elle sérialise les
transactions d’une campagne. Mesurer ce coût avant de remplacer ce mécanisme.
Connexions et tâches restent bornées ; aucun état participant permanent n’est
ajouté en mémoire.

## Droit de reprise

Le client génère UUID et secret de 256 bits avec Web Crypto avant la création.
Il peut répéter une création après perte de l’accusé sans que le serveur
conserve le secret en clair. SQL stocke son empreinte ; les requêtes utilisent
un en-tête Bearer et le lien de reprise un fragment d’URL.

Compte administrateur et droit participant sont distincts. Les exports privés
ont leurs propres routes. Aucun compte participant, identité invitée, jeton
source LimeSurvey ou secret dans `localStorage` n’est introduit. Révocation et
politiques de rétention restent des travaux explicites.

## Validation

Les tests du domaine et de l’API exercent les comportements annoncés, y compris
base fichier fermée puis rouverte. Le test externe optionnel inclut la collecte.
Une compilation des pilotes ne prouve pas la portabilité du nouveau schéma :
exécuter ce test sur chaque moteur jetable. La parité LimeSurvey et le gain
CPU/RAM ne sont pas annoncés.
