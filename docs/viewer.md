# Viewer et saisie locale

## Essayer le parcours

Démarrer le backend et Vite avec les commandes du [README](../README.md). Le
bouton **Essayer l’exemple** télécharge un LSS synthétique et utilise le même
endpoint d’analyse que l’upload d’un fichier local.

1. Renseigner un nom de projet, choisir Rust et indiquer la taille de l’équipe.
2. Passer au groupe suivant, choisir une expérience et un objectif.
3. Cocher « Autre objectif » pour faire apparaître la précision obligatoire.
4. Essayer les trois tableaux. Choisir « À l’aise » dans la colonne « Après »
   affiche une autre précision obligatoire.
5. Terminer par une note et valider la saisie locale.
6. Revenir au premier groupe et changer de technologie pour masquer le groupe
   Rust et effacer ses réponses.

Le changement de langue conserve la saisie. Le bouton de réinitialisation,
l’import d’une autre étude et le rechargement repartent de zéro. Le JSON
téléchargeable conserve les cellules locales ; ce format n’est pas un export de
réponses LimeSurvey et sa réimportation n’est pas encore implémentée.

## Types et règles couverts

| Types LSS | Saisie dans cet aperçu                        |
| --------- | --------------------------------------------- |
| S, T, U   | Texte court ou multiligne                     |
| N         | Nombre                                        |
| D         | Date calendaire ISO, sans heure               |
| L, !      | Choix simple, liste ou menu                   |
| 5, Y, G   | Note 1–5, oui/non, genre historique           |
| M         | Cases à cocher par sous-question              |
| Q, K      | Textes ou nombres multiples                   |
| F, H      | Tableau de choix par lignes/colonnes          |
| :         | Matrice à cases si `multiflexible_checkbox=1` |
| X         | Information sans réponse                      |

Les libellés, aides et choix utilisent la langue sélectionnée, puis la langue de
base en repli. Le HTML importé est converti en texte dans un élément `template`
inerte ; aucun HTML importé n’est inséré dans le DOM actif. La mise en forme,
les images et les scripts de thèmes ne sont pas reproduits.

Le moteur TypeScript prend en charge :

- pertinence de groupe et question, dépendances et effacement des réponses
  devenues masquées ; cycles et variables inconnues signalés ;
- obligatoire strict, obligatoire souple avec avertissement avant de continuer ;
- nombres finis, entier seul, limites `min_num_value_n`/`max_num_value_n` ;
- `min_answers`, `max_answers`, `maximum_chars` et `em_validation_q` ;
- substitutions de texte entre accolades avec les expressions prises en charge.

Les réponses F et H sont adressées par `CODE_sousquestion`. Les cellules d’une
matrice sont adressées par `CODE_ligne_colonne`, avec les deux axes source
`scale_id=0` et `scale_id=1`. H transpose l’affichage sans changer les réponses.
Les codes de choix sont conservés comme chaînes, y compris les zéros initiaux.

Une matrice conserve l’absence de saisie, la valeur cochée `1` et la valeur
décochée `0`. Effacer une question remet ses cellules à vide ; masquer la
question supprime toutes ses réponses. Une matrice obligatoire demande au moins
une case cochée par ligne ; une question F/H obligatoire demande une réponse par
sous-question. Le caractère facultatif reste respecté.

Le viewer restitue les textes d’accueil/fin dans la langue choisie. L’accueil
est présenté si `showwelcome` l’autorise et si un texte est présent. Modifier
des réponses après validation conserve la saisie, tandis que « Recommencer »
revient au début. Les libellés sont convertis en texte selon la même règle que
ceux des questions.

Les choix simples et multiples respectent `display_columns`, avec repli sur une
colonne sur petit écran. Les tableaux restent accessibles par défilement
horizontal. Les groupes possèdent une navigation en haut et en bas ; un contrôle
en échec ramène à la première question à corriger.

Les réglages de largeur (`input_size`) et les préférences d’édition
(`save_as_default`, `clear_default`) ne bloquent plus l’exécution. Les options
secondaires de minuterie restent sans effet quand aucune durée n’est définie.
Une minuterie active et les attributs d’exécution inconnus restent
diagnostiqués.

La grammaire bornée accepte nombres, chaînes, booléens, parenthèses,
comparaisons, arithmétique et opérateurs booléens `and`, `or`, `not`, `&&`,
`||`, `!`. Les fonctions disponibles sont `if`, `is_empty`, `sum`, `count`,
`min`, `max`, `strlen`, `abs` et `round` à un argument. Les variables utilisent
les codes de questions, `CODE_sousquestion` et le suffixe `.NAOK`.

C’est un moteur d’aperçu partiel : conversions et fonctions doivent encore être
confrontées à un corpus exécuté dans LimeSurvey 6.5.14. Les comparaisons de
chaînes, les règles sur les valeurs vides et les variantes de fonctions ne sont
pas une garantie de sémantique ExpressionScript complète.

Les autres variantes de tableaux, classements, fichiers, équations, champs «
autre » natifs, expressions régulières, filtres et pertinence de sous-questions,
ordres aléatoires et attributs actifs inconnus produisent des limites
explicites. Les questions concernées sont consultables avec saisie désactivée ;
la validation finale locale reste désactivée. Les conditions historiques,
valeurs par défaut et quotas présents bloquent également cette validation. Le
viewer impose sa navigation par groupe : il ne reproduit pas tous les paramètres
de collecte.

Une erreur de structure désactive toute saisie. Un schéma différent de 623
permet seulement l’inspection des métadonnées et du rapport ; aucune conversion
implicite de schéma n’est effectuée.

Le programme client est borné à 2 000 questions, 2 000 cellules par question, 2
000 choix affichés par tableau F/H et 20 000 cellules pour l’ensemble de
l’étude. Les produits d’axes sont vérifiés avant leur allocation ; un
dépassement permet seulement l’inspection et ne rend pas un grand tableau
désactivé.

## Frontière HTTP

`POST /api/surveys/inspect` reçoit directement les octets XML dans le corps,
avec `Content-Type: application/xml`, `text/xml` ou `application/octet-stream`.
Pas de multipart : le fichier est le corps de la requête.

La réponse JSON contient `metadata`, `study` et `report`. Les identifiants
source sont des chaînes pour conserver les entiers 64 bits dans JavaScript.
`study` vaut `null` pour un schéma sans projection. Un XML lisible mais
comportant des erreurs de structure produit HTTP 200 et un rapport avec
`errors > 0` ; le client doit respecter ce blocage.

Les erreurs utilisent `{ "error": { "code": "…", "message": "…" } }` :

| HTTP | Cas                                        |
| ---- | ------------------------------------------ |
| 408  | Corps non reçu en 15 secondes              |
| 413  | Plus de 8 Mio                              |
| 415  | Type de contenu non pris en charge         |
| 422  | XML/LSS illisible ou limites XML atteintes |
| 503  | Deux analyses déjà en cours                |

Le serveur borne le corps avant l’analyse et conserve au plus deux traitements
simultanés, lecture du corps comprise. L’analyse XML et la sérialisation sont
déportées dans `spawn_blocking`. Aucun fichier, questionnaire ou état de saisie
n’est conservé après la requête. Les réponses d’analyse utilisent `no-store`.
Les limites XML existantes restent applicables.

`GET /api/surveys/example.lss` sert uniquement la fixture synthétique
versionnée.

## Vérifications et suite

`npm test` exécute les tests purs du moteur avec le lanceur natif Node. Le mode
TypeScript utilise `--experimental-strip-types`, disponible sur Node 22.15 de
cette machine ; aucun framework de test supplémentaire n’est ajouté. Les tests
Rust exercent la projection, les messages localisés et la frontière HTTP sans
réseau ni base externe. Les fixtures synthétiques couvrent F/H, matrices,
dépendances, cellules facultatives/obligatoires et limites. Les exports locaux
de `.surveys-tests/` restent ignorés et hors des tests versionnés.

Le parcours HTTP local a été vérifié avec Vite, le backend, le décodeur JSON et
le moteur réels. Le navigateur pilotable étant indisponible dans la session de
développement, le contrôle visuel et les interactions DOM restent à vérifier.

La collecte durable exigera la validation des réponses et des transitions côté
Rust, ainsi que les tables et migrations correspondantes. La validation locale
actuelle n’autorise aucune écriture serveur et ne remplace pas ces contrôles.
