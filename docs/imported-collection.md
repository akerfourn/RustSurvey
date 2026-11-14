# Prérequis de collecte des études importées

## Bilan disponible

Dans l'administration d'une étude importée, ouvrir **Inventaire de compatibilité
et de migration**, puis **Vérifier les prérequis de collecte**. Le bilan porte
sur la révision enregistrée affichée ; une modification non enregistrée n'entre
pas dans ce contrôle.

Trois vérifications restent distinctes :

- `runtime` : les questions et règles passent ou non le compilateur actuel du
  profil natif `native-forms-v1`, ou `native-defaults-v1` si la révision porte
  des défauts projetés ;
- `mapping` : les cellules natives retrouvent ou non leurs identités source dans
  le LSS original ;
- `projection` : inventaire des sections, colonnes, extensions XML et réglages
  opérationnels qui demandent encore une interprétation.

Le résultat n'ouvre aucune campagne et ne transforme pas les réponses conservées
en participations. `execution` reste `blocked` et `continuity` reste
`unavailable`. L'absence de diagnostic local ne constitue pas une preuve
d'équivalence avec LimeSurvey. Les thèmes de base importés peuvent notamment
rester hors du profil natif, même si le viewer sait les reconnaître.

La projection des défauts des nouveaux imports persistants alimente leur modèle
natif et leur aperçu. Le bilan brut des sections source reste conservateur ; la
représentation des cellules ne qualifie pas à elle seule l'initialisation amont,
les thèmes et les réglages opérationnels.

Une demande d'ouverture sans configuration source retourne ces diagnostics avec
`422 imported_collection_unavailable`, sans créer de campagne. Le parcours
distinct [Ouvrir une nouvelle campagne](import-execution.md) fournit un contexte
explicite et les faits d'exécution, puis compile le profil
`limesurvey-import-v1`. Il peut ouvrir une collecte neuve qualifiée sans
transférer les réponses, sessions ou droits de reprise historiques. Le présent
inventaire reste conservateur et ne constitue pas le résultat de cette
qualification.

## Données examinées

Le bilan relit le LSS privé, vérifie son empreinte et charge explicitement la
révision demandée. Il ne réévalue aucune réponse et ne modifie aucun original,
révision ou fichier de préparation. Les conditions éditées sont contrôlées dans
la révision ; les autres données source sont examinées dans l'original.

L'inventaire compare les colonnes déclarées et rencontrées aux champs portés par
la projection actuelle. Un champ inconnu reste signalé même vide ou nul. Les
attributs visant des sous-questions sont signalés : leur table existe dans le
modèle de lecture, mais ces attributs ne sont pas projetés sur les axes natifs.
Les attributs et espaces de noms XML font aussi partie du bilan.

Une section connue sans ligne, comme `quota` ou `defaultvalues`, ne produit pas
de diagnostic de données actives. Une ligne avec un quota égal à zéro reste
présente et doit être interprétée. Une extension inconnue reste signalée même
vide. Le bilan ne reproduit aucune valeur source : seuls les noms techniques
bornés, positions et messages de contrôle sont retournés.

La liste des réglages dont la valeur effective doit être qualifiée couvre
notamment format, thème, anonymat, cookies, sauvegarde, persistance par jeton,
correction, dates, IP, référent, timings, retour arrière, sans-réponse, index,
délai, évaluations, dates d'accès, inscription et redirection. Une valeur
absente n'est jamais remplacée par un défaut RustSurvey. Cet inventaire
conservateur reste à compléter avec la résolution des paramètres hérités ; il ne
classe pas encore toutes les valeurs neutres ou compatibles.

Le panneau **Réglages héritables** ajoute la résolution de 30 options de
groupes, avec valeur typée et provenance. L'héritage se résout aussi via un
contexte source explicite dans le CLI ou déposé dans l'administration. Chaque
instantané est conservé par révision ; `?contextId={id}` le sélectionne pour un
bilan sans changer le LSS seul par défaut. Ce bilan ne traite pas les forçages
globaux ; le profil d'exécution distinct les vérifie dans ses faits source. Voir
[les règles et preuves](source-settings.md). Un accueil hérité ou absent reste
désormais inconnu dans les nouveaux imports et leurs aperçus.

## API et limites

`GET /api/admin/studies/{id}/revisions/{revision}/collection-check` exige une
session administrateur et retourne un document `schemaVersion: 1`, sans cache.
Une étude native ou une révision inexistante retourne 404. Le bilan expose la
révision, l'empreinte source, les états, le nombre de cellules et les compteurs
de projection. La liste combinée contient au plus 128 diagnostics, avec un
compteur des constats recueillis avant troncature. Les deux exigences globales
d'exécution et de continuité restent toujours affichées.

Le traitement partage les deux places de préparation existantes. Lecture de
l'original, compilation et analyse sont bornées par les limites du LSS et de la
définition. Compilation et analyse XML se font hors des tâches HTTP. Aucun cache
illimité, état participant en mémoire, service ou dépendance n'est ajouté.

Les tests vérifient confidentialité, valeurs non divulguées, quota nul, sections
vides, extensions XML, attributs de sous-question, troncature, conditions
corrigées et lecture d'une ancienne révision. Le corpus réel de la
[référence 6.5.14](limesurvey-reference.md) vérifie aussi les correspondances
des 52 cellules et détecte les colonnes non projetées du fichier amont.

## Exécution neuve et transfert ultérieur

Le [profil versionné](import-execution.md) applique les réglages effectifs et
qualifie un premier périmètre pour de nouvelles campagnes. Les autres fonctions
et leurs parcours restent à compléter. La migration des anciennes participations
est différée à la demande du porteur du projet. Sa future conversion devra
garder les états et valeurs historiques sans appliquer rétrospectivement les
règles actuelles. Les participants, ressources, droits de reprise et compléments
de transfert suivent le [protocole de migration](study-migration.md).

La [préparation des réponses LSR](response-imports.md) fournit la conservation
privée et les correspondances nécessaires à cette suite ; elle ne réduit pas
l'objectif de transfert à un archivage.

La conversion privée des cellules natives est désormais disponible par ligne,
avec restauration des valeurs source et sans application rétroactive des règles.
Elle ne choisit pas une langue de reprise, un curseur ou un droit d'accès et
n'autorise pas la collecte à elle seule.
