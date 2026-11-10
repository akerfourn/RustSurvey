# Feuille de route proposée

L’ordre ci-dessous privilégie les preuves de compatibilité et le cadrage des
sessions avant une interface complète. Il ne constitue pas un calendrier.

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

- Préparer une instance de référence 6.5.14 isolée et des données synthétiques.
- Réexporter des cas amont pour obtenir des fixtures du schéma cible 623.
- Implémenter lecture XML bornée, conservation source et `FieldBinding`.
- Produire le rapport distinct de conservation, export, exécution et reprise.
- Vérifier l’aller-retour de structure et les références des réponses.
- Ajouter la CI et les contrôles adaptés au futur hébergement du dépôt.

Critère de sortie : le premier adaptateur préserve les cas annoncés, localise
les limites et ne transforme pas un import conservé en autorisation de collecte.

## Étape 3 — Persistance et parcours exécutable

- Introduire le schéma conçu et ses migrations sur les trois bases.
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

## Étape 5 — Extension selon les usages

Étendre les types et règles couverts, l’éditeur, les invitations et les outils
d’exploitation. Chaque extension précise son impact sur les échanges, les
données et le coût serveur. Les exports utilisateur affinent la priorité des cas
réels ; ils ne remplacent pas l’analyse des sources de référence.
