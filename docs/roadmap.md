# Feuille de route proposée

L’ordre ci-dessous privilégie les preuves de compatibilité et le cadrage des
sessions avant une interface complète. Il ne constitue pas un calendrier.

## Étape 0 — Cadrage et squelette

Livrable du premier tour : README, consignes AGENTS, besoins, architecture,
contrat de compatibilité, stratégie de migrations et inventaire des outils.

Le squelette se limite à vérifier un démarrage Rust, une connexion de base et un
build TypeScript. Aucun modèle de questionnaire n’est figé.

## Étape 1 — Corpus et contrat du premier parcours

- Constituer des exports `.lss` anonymisés depuis LimeSurvey 6.5.14.
- Identifier les types de questions, expressions et options nécessaires.
- Préciser anonymat, participation, sauvegarde et reprise.
- Fixer une machine cible et des scénarios de charge réalistes.
- Définir ce que le premier import/export devra préserver et refuser.

Critère de sortie : un corpus versionné et un périmètre fonctionnel explicite,
avec des résultats attendus reproductibles.

## Étape 2 — Preuve d’échange et premier schéma

- Définir un modèle de domaine pour le sous-ensemble retenu.
- Implémenter import et export de structure avec diagnostics.
- Vérifier l’aller-retour avec une instance de référence 6.5.14 isolée.
- Introduire les premières migrations et leur matrice SQLite/MariaDB/PostgreSQL.
- Ajouter les contrôles automatisés adaptés au dépôt et à son futur hébergement.

Critère de sortie : les fonctions annoncées ont des tests, les limites sont
visibles et les trois bases conservent les mêmes données métier.

## Étape 3 — Premier questionnaire exploitable

- Afficher et remplir le sous-ensemble du corpus.
- Vérifier les règles pertinentes côté serveur.
- Enregistrer et reprendre une participation après redémarrage.
- Traiter les doubles envois et les modifications concurrentes.
- Contrôler les droits et les exigences d’anonymat retenues.

Critère de sortie : un parcours complet, testé et mesuré, avec des ressources
bornées. Comparer les mesures à la charge cible, sans extrapoler depuis le
simple point de contrôle HTTP du squelette.

## Étape 4 — Extension selon les usages

Prioriser ensuite l’édition, les autres types de questions, les archives `.lsa`,
les exports de réponses, les invitations et les outils d’exploitation. Chaque
extension doit préciser son impact sur la compatibilité, les données et le coût
serveur avant d’élargir le contrat.
