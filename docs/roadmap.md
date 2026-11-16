# Feuille de route proposée

L'ordre ci-dessous privilégie les parcours utilisables et leurs preuves de
compatibilité. La continuité exacte des études en cours est différée à la
demande du porteur du projet ; elle ne bloque pas les nouvelles collectes. Cette
feuille de route ne constitue pas un calendrier.

La cible des formulaires couvre le natif de LimeSurvey 6.5.14, pas seulement le
profil d’un export de production. L’[audit de couverture](form-coverage.md)
définit les lots F0–F7 : capacités, cellules et types, expressions, règles,
rendus, collecte durable, fonctions avec état et certification. Ils précisent
les travaux des étapes 2 à 5 ci-dessous, avec leurs dépendances et critères de
sortie. Un incrément sans stockage couvre maintenant les 28 types de base hors
fichiers, 47 fonctions d’expression, les filtres M/P et la pertinence des
sous-questions. Les variantes restantes, les cibles de défauts non projetées,
les anciennes conditions et la certification restent à réaliser ; le
[viewer](viewer.md) décrit le profil effectivement implémenté.

## Priorité du cycle actuel

Après le point d'étape du 14 novembre, privilégier les jalons complets et leurs
preuves observables, sans réduire l'objectif de parité :

Le porteur du projet a ensuite demandé de rester sur une **version pilote**. Le
lot courant est donc borné : distribution du client et de l'API depuis Rust,
paquet natif, recette HTTP complète et guide d'essai. Le
[guide du pilote](pilot.md) distingue les fonctions disponibles et exclues. Les
extensions ci-dessous reprennent après ce jalon et les retours de recette.

1. Consolider création, configuration, collecte et restrictions de participation
   dans un parcours natif utilisable.
2. Permettre les nouvelles campagnes sur des imports qualifiés et les échanges
   de définitions et de réponses, sans transférer les anciennes participations.
3. Compléter fichiers et fonctions métier restantes du catalogue natif ; mesurer
   et préparer le déploiement du nouvel adaptateur S3.
4. Mesurer CPU/RAM et comparer à LimeSurvey sur une charge documentée.

Le parcours des défauts possède maintenant son format v4 et son profil
`native-defaults-v1`, avec corpus commun Rust/TypeScript, API et reprise. La
projection des défauts des nouveaux imports persistants est disponible ; les
autres cibles et le contexte NAOK restent à compléter. Le projet possède aussi
un [banc CPU/RAM local](performance.md), avec rapports versionnés et comparaison
de l'attente des écritures sur le même binaire. Les mesures comparatives de
LimeSurvey restent à réaliser ; aucun ratio de gain global n'est annoncé. La
migration pilote et ses droits de reprise restent un chantier ultérieur.

Le 14 novembre 2026, l'export LSS des créations natives et un premier
aller-retour structurel réel dans la version cible complètent l'échange de
définitions. Voir la [preuve et ses limites](limesurvey-reference.md). Les
coordonnées natives et l'export VV sont maintenant vérifiés dans la référence.
Les correspondances des cellules importées et la conservation SQL des réponses
sont disponibles dans la [préparation LSR](response-imports.md). Son chargement
reprend après interruption ou redémarrage. Le
[bilan des prérequis](imported-collection.md) localise les données non projetées
de chaque révision. Le [profil d'exécution importé](import-execution.md)
applique désormais un contexte explicitement choisi et les faits d'exécution
nécessaires à une nouvelle campagne. Les configurations hors profil restent à
compléter. La conversion réversible des cellules historiques est disponible par
ligne ; leur publication et le transfert des droits de reprise restent différés.
Voir l'[échange des réponses](response-exchange.md).

Le résolveur de groupes couvre désormais 30 options et leur provenance. Quatre
scénarios réels vérifient héritage, valeurs explicites, groupe zéro et valeurs
vides. Le premier profil accepte le thème de base et qualifie les forçages
globaux ; il exige des dates d'accès, CAPTCHA et fonctions annexes inactifs.
Leur exécution générale reste à compléter.

Les contextes source sont maintenant extraits par une commande de lecture et
conservés en SQL par révision. Leur sélection reste explicite et ne modifie pas
les bilans historiques. Le profil d'exécution les applique à une campagne neuve
dans son périmètre strict, sans modifier les révisions importées.

Les campagnes natives possèdent maintenant les trois formats A/G/S, une
progression facultative et un curseur stable de page. L'import des réglages est
vérifié dans LimeSurvey ; les anciens profils sont conservés. Cette première
application de paramètres de présentation est réutilisée par le profil importé.
L'index, le retour configurable et les délais restent à compléter.

Un [premier éditeur natif](authoring.md) et la persistance SQL des définitions
et de l’administration sont présents. Il permet de créer des questionnaires
publics avec 28 types de questions, des paramètres et un aperçu.
L’administration importe les LSS en SQL dans un format natif enrichi, conserve
l’original privé et permet d’éditer les conditions de groupes/questions. Le
parcours LSS conserve ses 28 types sans fichiers. Les réponses des aperçus
restent en mémoire dans le navigateur. La [collecte native](collection.md)
couvre maintenant 28 types sur des campagnes figées, avec validation Rust,
réponses SQL, soumission et lien privé de reprise. Les
[règles de participation](participation.md) permettent maintenant de limiter les
envois par navigateur ou invitation, de reprendre et de corriger une réponse
selon la politique de campagne.

F5 possède ainsi un profil natif étendu et un premier profil d'import
exécutable. La suite doit compléter les variantes, règles, fichiers et stockage
objet. Quotas et autres fonctions F6 restent à réaliser. Les transferts des
anciennes participations sont différés. Les prochaines étapes d'édition pourront
étendre les propriétés modifiables des imports. Les calculs/paramètres natifs
sont éditables ; le réexport LSS des seules conditions conserve les autres
octets et bloque les anciennes conditions contradictoires.

La migration des participations reste un objectif ultérieur. Les futurs médias
et pièces jointes suivront la
[décision de stockage objet](decisions/0002-object-storage.md) : port commun,
adaptateur S3 et adaptateur fichiers locaux, sans volume persistant requis pour
le déploiement S3. Les adaptateurs SQL/fichiers locaux et une médiathèque
d’images natives sont présents. L’historique restaurable, les traductions et
l'import/export JSON complètent l'édition. L'[adaptateur S3](s3-storage.md)
possède maintenant son index SQL, son contrat et un parcours HTTP avec
redémarrage sur un fournisseur local réel. Les fichiers participants, la copie
des objets locaux et le déploiement distant restent à réaliser.

Les lignes exclusives des neuf types concernés et l'automatique M/P sont
maintenant disponibles dans l'éditeur et la collecte native. Les commentaires,
Autre et filtres possèdent un corpus commun ; les attributs sont importés dans
la référence. La comparaison complète des parcours reste nécessaire. La prise en
charge des autres cibles de défauts, les anciennes conditions et les ordres
aléatoires restent des travaux importants avant l'exécution générale des
imports.

La [lecture source des défauts](default-values.md) conserve maintenant cibles,
échelles, traductions et `same_default`. Un corpus de 31 scénarios compare
l'initialisation et les écritures demandées par le moteur de référence. La v4
native ajoute configuration, éditeur et export LSS de 21 types ; 29
configurations sont réimportées dans LimeSurvey. La collecte initialise les
pages sans remplacer une clé présente, même vide. Les nouveaux imports
persistants projettent leurs défauts représentables, avec diagnostic par
question ; NAOK et les autres interactions restent à compléter.

Les [filtres de lignes](array-filters.md) disposent de 14 types de source, du
code Autre et des restrictions des ancêtres. Les listes radio et classements
cibles sont aussi disponibles. Un corpus de 46 cas est comparé aux règles du
moteur PHP de référence sur les mêmes valeurs normalisées, avec contrôle de
l'obligatoire des listes/classements. Le tableau H peut servir de source ; son
filtrage cible natif bloque l'export LSS et son aperçu importé signale l'écart,
car la référence n'exécute pas cette fonction.

Le workflow de brouillon et publication est présent : une publication change la
version des nouveaux arrivants sans interrompre les parcours déjà ouverts.
Chaque future participation persistante devra conserver sa révision lors des
sauvegardes et reprises. La reprise des données historiques LimeSurvey relève
d’un outil externe ultérieur ; l’échange standard VV/LSR/LSA reste requis.

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
durable et ressources bornées. La conservation des participations importées
relève de l'étape 4 différée, sans bloquer les campagnes neuves.

## Étape 4 — Preuve de migration d’étude en cours

Cette étape est différée à la demande du porteur du projet. Le protocole reste
une exigence pour une future bascule, sans conditionner les étapes 3 et 5.

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
