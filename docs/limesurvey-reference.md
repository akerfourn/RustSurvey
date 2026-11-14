# Vérification avec LimeSurvey 6.5.14

## Preuve réalisée

Le 14 novembre 2026, le lanceur `npm run test:limesurvey` a exporté une
définition native synthétique, importé son LSS dans LimeSurvey, puis réexporté
ce questionnaire avec les fonctions amont. Le test Rust compare ensuite les
structures, en autorisant le changement des identifiants techniques.

Le corpus couvre les 28 types sans fichiers, deux langues, groupes, textes et
aides, axes et doubles échelles, choix, obligatoire, pertinences, paramètres et
équation. Les codes, ordres et valeurs comparés doivent rester identiques.
LimeSurvey renomme les codes réservés comme `LANG` : l'export natif les refuse
désormais avec un diagnostic, sans réécrire les règles de l'étude.

Le test compare aussi les cellules à `createFieldMap`, active le questionnaire
et importe deux réponses, dont un brouillon. Il vérifie les données en SQL puis
leur réexport LSR. Un second import du LSS réattribue les identifiants ; un VV
utilisant les codes y retrouve les mêmes réponses, sans identifiants source.

Ce corpus vérifie les altérations du chemin LSR amont et la conservation des
valeurs par VV. Voir l'[échange des réponses](response-exchange.md). Il ne
certifie pas les parcours de formulaire, toutes les options, les fichiers,
invitations ni la migration des reprises. Ce corpus structurel ne suffit pas à
qualifier une campagne importée ; le parcours distinct ci-dessous le fait dans
un profil borné.

Le même lanceur transmet ensuite les vrais LSS/LSR réexportés à l'API
RustSurvey. La [préparation des réponses](response-imports.md) vérifie la
correspondance des 52 cellules, l'original LSR exact et les valeurs de chaque
ligne. Le [bilan de collecte](imported-collection.md) détecte les colonnes amont
non projetées ; il conserve le blocage d'exécution. Ce contrôle d'import ne
simule pas une reprise participant et ne certifie pas les parcours.

Quatre cas supplémentaires comparent les 30
[options de groupes](source-settings.md) aux valeurs réelles de
`Survey::oOptions` : deux niveaux d'héritage, valeurs explicites, groupe zéro et
valeurs vides. Les contextes, LSS et résultats sont générés dans la base
jetable, puis supprimés avec les autres artefacts.

Trois LSS supplémentaires exportent les présentations natives groupe, question
et tout sur une page. Après leur import réel, le lanceur compare les options
effectives `format` G/S/A, `showprogress` Y/N et `allowprev=Y`. Cette
vérification porte sur les réglages importés ; elle ne simule pas encore les
parcours de réponse dans le navigateur LimeSurvey.

Un autre LSS configure les lignes exclusives des neuf types déclarés dans la
référence et l'automatique M/P. Le test compare les attributs réellement
enregistrés après import. Le corpus comportemental Rust/TypeScript reste
distinct de cette vérification d'échange ; il ne simule pas encore les clics
dans LimeSurvey.

Le lanceur compare aussi 46 cas de [filtrage](array-filters.md) au moteur PHP
amont. Chaque structure est importée dans la base jetable ; les valeurs
normalisées sont liées aux vrais noms de champs. Le gestionnaire calcule les
règles de lignes et de choix après initialisation de la pertinence. Les
résultats couvrent les 14 types de source, Autre, les commentaires, les ancêtres
et les listes radio/classements cibles. Ces deux cibles vérifient aussi
l'obligatoire sur leurs réponses normalisées, liées au moteur et à sa session.
Ils vérifient cette frontière avec les mêmes entrées, sans simuler un parcours
complet ni certifier la normalisation amont des sources.

Les [valeurs par défaut](default-values.md) possèdent 31 scénarios de référence
: cibles, langues, présence, effacement, expressions et écritures des réponses
inactives. Chaque LSS réellement exporté est relu par Rust. Le test distingue la
valeur de session des écritures demandées par le moteur. Il importe également 29
configurations LSS natives v4 et relit leurs réexports. Les mêmes valeurs
persistables alimentent les tests Rust/TypeScript ; le stockage final amont de
chaque valeur et tous les parcours participant ne sont pas certifiés.

Ces exports réels vérifient maintenant aussi la projection vers les cellules de
défauts natives, leurs langues, le mode partagé et les refus de cibles non
représentables. Cette preuve ne qualifie pas les autres réglages d'une collecte
importée.

Les deux réponses et 52 cellules du LSR réel sont également projetées dans les
clés natives puis restaurées au contenu du fichier reçu. Les tests API relisent
la révision, vérifient les empreintes et contrôlent les représentations de date
conservées. Ils ne recalculent pas les valeurs historiques et n'ouvrent aucune
campagne importée. Voir
[la conversion](response-imports.md#conversion-réversible-des-cellules).

## Nouvelle collecte sur un import qualifié

Un cas distinct exporte un LSS réellement produit par LimeSurvey avec les
réglages admis par `limesurvey-import-v1`. Deux commandes de lecture extraient
les 30 options héritables et les faits d'exécution bornés, dont le fuseau UTC et
le décalage nul. La référence compare leurs sorties à ses modèles complets.
L'auxiliaire d'exécution est aussi lancé dans un processus PHP neuf et refuse
l'écrasement du fichier obtenu. Voir le [parcours](import-execution.md).

Le lanceur transmet ce LSS et ces deux documents à RustSurvey. Le test API ouvre
une campagne, saisit les 28 types, recrée l'état serveur sur la même base,
reprend la réponse, répète une écriture, soumet puis exporte VV et LSS. La
campagne reste sur sa révision figée après une édition du questionnaire.

Ce test ne simule pas les clics dans le navigateur LimeSurvey. Il ne reprend
aucune participation ancienne et ne certifie pas les options hors profil. La
continuité exacte des études en cours est différée à la demande du porteur du
projet.

## Environnement isolé

L'image de test contient PHP 8.2 et le code amont du tag `6.5.14+240624`, dont
le commit est vérifié pendant sa construction :
`ac3c44744e2c3769b3941c41e50b68419423ce67`. Le numéro de version et le schéma
623 sont aussi contrôlés pendant le test. Le code amont et ses licences restent
dans l'image de référence ; ce n'est pas une dépendance de production.

Les téléchargements et la construction sont explicites :

```sh
docker pull mariadb:11.4
docker build -f tests/limesurvey-reference/Dockerfile \
  -t rustsurvey-limesurvey-reference:6.5.14 .
npm run test:limesurvey
```

Le contexte de construction est filtré par `Dockerfile.dockerignore` : seuls les
scripts de référence et l'extracteur sont inclus, sans `.data/`, `.env` ni les
autres fichiers du dépôt. La construction utilise GitHub et les dépôts
Debian/PHP. Le lanceur n'effectue aucun téléchargement. Il crée son réseau
Docker interne et ses deux conteneurs uniques ; aucun port n'est publié et
aucune base existante n'est utilisée. MariaDB utilise un système de fichiers
temporaire limité à 384 Mio. Les conteneurs ont des limites de mémoire, CPU et
processus. Seules des données et des identifiants synthétiques y sont chargés.

Le montage contient uniquement le répertoire temporaire créé pour ce test. Son
chemin réel est contrôlé avant suppression. Les conteneurs et le réseau créés
sont retirés à la sortie, y compris en cas d'erreur. Une interruption brutale du
processus ou du moteur Docker peut exiger leur nettoyage manuel ; leurs noms
commencent par `rustsurvey-reference-` et incluent un UUID.

## Choix de l'export natif

La bibliothèque `rustsurvey-limesurvey` dépend du crate local
`rustsurvey-domain` pour lire le format natif versionné et réutiliser sa
validation. Aucune dépendance externe n'est ajoutée ; le domaine ne dépend pas
de LimeSurvey. Le XML est généré de manière déterministe, échappé puis relu avec
les mêmes limites que les imports.

L'export d'une création native est distinct du réexport ciblé d'un LSS importé.
Les médias, thèmes spécifiques, traductions inactives et paramètres non
représentables bloquent l'export plutôt que disparaître. Le LSS ne porte pas la
politique de participation ni les réponses. L'export d'une campagne inclut ses
réglages de présentation. L'original d'un import reste privé et immuable.

Le [contrat de compatibilité](limesurvey-compatibility.md) et le
[protocole de transfert](study-migration.md) définissent les preuves restant à
apporter.
