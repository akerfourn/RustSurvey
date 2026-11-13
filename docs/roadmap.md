# Feuille de route proposée

L’ordre ci-dessous privilégie les preuves de compatibilité et le cadrage des
sessions avant une interface complète. Il ne constitue pas un calendrier.

La cible des formulaires couvre le natif de LimeSurvey 6.5.14, pas seulement le
profil d’un export de production. L’[audit de couverture](form-coverage.md)
définit les lots F0–F7 : capacités, cellules et types, expressions, règles,
rendus, collecte durable, fonctions avec état et certification. Ils précisent
les travaux des étapes 2 à 5 ci-dessous, avec leurs dépendances et critères de
sortie. Un incrément sans stockage couvre maintenant les 28 types de base hors
fichiers, 47 fonctions d’expression, les filtres M/P et la pertinence des
sous-questions. Les variantes restantes, les valeurs par défaut, les anciennes
conditions et la certification restent à réaliser ; le [viewer](viewer.md)
décrit le profil effectivement implémenté.

## Priorité du cycle actuel

Un [premier éditeur natif](authoring.md) et la persistance SQL des définitions
et de l’administration sont présents. Il permet de créer des questionnaires
publics avec 28 types de questions, des paramètres et un aperçu.
L’administration importe les LSS en SQL dans un format natif enrichi, conserve
l’original privé et permet d’éditer les conditions de groupes/questions. Le
parcours LSS conserve ses 28 types sans fichiers. Les réponses des aperçus
restent en mémoire dans le navigateur. La [collecte native](collection.md)
couvre maintenant 17 types sur des campagnes figées, avec validation Rust,
réponses SQL, soumission et lien privé de reprise.

F5 possède ainsi un premier profil natif. La suite doit étendre le moteur Rust
aux autres types et règles, vérifier la collecte des imports, puis les
transferts de ressources et de participations. Quotas, fichiers et autres
fonctions F6 restent différés. Les prochaines étapes d’édition pourront aussi
étendre les propriétés modifiables des imports. Les calculs/paramètres natifs
sont éditables ; le réexport LSS des seules conditions conserve les autres
octets et bloque les anciennes conditions contradictoires.

La migration des participations reste un objectif ultérieur. Les futurs médias
et pièces jointes suivront la
[décision de stockage objet](decisions/0002-object-storage.md) : port commun,
adaptateur S3 et adaptateur fichiers locaux, sans volume persistant requis pour
le déploiement S3. Les adaptateurs SQL/fichiers locaux et une médiathèque
d’images natives sont présents. L’historique restaurable, les traductions et
l’import/export JSON complètent l’édition. L’adaptateur S3 reste à faire.

## Étape 0 — Cadrage et squelette

Livrable du premier tour : README, consignes AGENTS, besoins, architecture,
contrat de compatibilité, stratégie de migrations et inventaire des outils.

Le squelette se limite à vérifier un démarrage Rust, une connexion de base et un
build TypeScript. La conception métier est documentée séparément ; aucune table
de questionnaire n’est encore créée.

## Étape 1 — Sources et architecture

- Étudier les exporteurs, importeurs, types et mécanismes de reprise du tag.
- Définir étude, révision, campagne, réponse, checkpoint et droits d’accès.
- Choisir le stockage commun et les correspondances d’identifiants source.
- Concevoir le transfert des études en cours et ses compléments nécessaires.
- Identifier le corpus amont et ses limites pour certifier le schéma 623.

Premier tour réalisé dans l’[audit](limesurvey-source-audit.md), le
[modèle interne](domain-model.md) et le [protocole](study-migration.md). Cette
conception ne dépend pas de la fourniture d’un export de production.

## Étape 2 — Référence et adaptateur d’échange

Premier incrément réalisé : bibliothèque et CLI `.lss`, conservation source,
contrôles structurels du schéma 623 et modification ciblée d’un titre. Les tests
synthétiques couvrent aussi les erreurs et limites XML. La génération de
fixtures de référence et les `FieldBinding` restent à réaliser.

- Préparer une instance de référence 6.5.14 isolée et des données synthétiques.
- Réexporter des cas amont pour obtenir des fixtures du schéma cible 623.
- Implémenter lecture XML bornée, conservation source et `FieldBinding`.
- Produire le rapport distinct de conservation, export, exécution et reprise.
- Vérifier l’aller-retour de structure et les références des réponses.
- Ajouter la CI et les contrôles adaptés au futur hébergement du dépôt.

Critère de sortie : le premier adaptateur préserve les cas annoncés, localise
les limites et ne transforme pas un import conservé en autorisation de collecte.

## Étape 3 — Persistance et parcours exécutable

Un premier [viewer local](viewer.md) est disponible : upload LSS, informations
de structure, rendu Preact et moteur partiel de pertinence/validation. Les
réponses de cet aperçu restent dans le navigateur. La collecte native réalise un
premier parcours durable indépendant, sans encore couvrir la migration des
participations ni la certification du profil LimeSurvey ci-dessous.

- Définitions et administration : premières tables et migrations testées sur les
  trois bases ; compléter ensuite campagnes et participations.
- Charger réponses complètes/partielles, participants et états disponibles.
- Implémenter les règles et le rendu Preact du profil couvert.
- Sauvegarder les blocs de réponses et le checkpoint de façon atomique.
- Tester redémarrage, répétition d’envoi, concurrence, quotas et anonymat.
- Mesurer mémoire, latence et requêtes SQL sur une charge documentée.

Critère de sortie : un parcours complet, testé et mesuré, avec progression
durable et ressources bornées. Les participations importées ne perdent ni valeur
ni droit d’accès dans le profil annoncé.

## Étape 4 — Preuve de migration d’étude en cours

- Compléter `.lsa`, réponses, participants et timings pour le profil testé.
- Traiter les ressources, paramètres effectifs et mécanismes de reprise requis.
- Fournir l’outil de transfert des compléments non couverts par `.lsa`.
- Répéter import en préparation, gel, instantané final et bascule de routage.
- Vérifier réponse neuve, reprise et transfert inverse après nouvelles
  écritures.
- Ajouter les cas de production anonymisés lorsqu’ils seront disponibles.

Critère de sortie : le périmètre complet demandé par une étude est validé, ou la
bascule est bloquée avec un diagnostic précis. Un sous-ensemble de questions
compatible ne suffit pas à annoncer une migration générale.

## Étape 5 — Parité des formulaires et outils complémentaires

Compléter les types, variantes de rendu et règles du
[catalogue des formulaires](form-catalog.md), puis vérifier leurs interactions
dans le corpus 6.5.14. La réussite d’une étude ne clôt pas cet objectif général.
Les lots de formulaire peuvent avancer dès les étapes précédentes ; cette étape
n’impose pas d’attendre la migration pour développer les types manquants.

L’extension de l’éditeur, les invitations et les outils d’exploitation restent
des chantiers complémentaires. Chaque extension précise son impact sur les
échanges, les données et le coût serveur. Les exports utilisateur affinent la
priorité des cas réels ; ils ne remplacent pas l’analyse des sources de
référence.
