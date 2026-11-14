# Valeurs par défaut : édition, collecte et échange

## Périmètre réalisé

L'adaptateur LSS lit `defaultvalues` et `defaultvalue_l10ns` au schéma 623. Son
modèle source conserve identifiant du défaut, question, sous-question
éventuelle, échelle, cible spéciale et valeurs par langue. Il ne convertit pas
les nombres lexicaux et ne réévalue aucune expression.

Une valeur vide reste une traduction présente. Une langue sans ligne reste
absente ; la langue principale n'est pas inventée comme repli. Le réglage
`same_default` distingue `0`, `1` et valeur indisponible. Une autre valeur
explicite produit un diagnostic.

Les contrôles couvrent identifiants, références, appartenance des
sous-questions, échelles numériques, doublons de cible ou de traduction et
langues déclarées. Une cible spéciale inconnue est conservée. Les originaux
restent réexportables à l'octet près, y compris après un diagnostic. Les
messages ne contiennent aucune valeur du questionnaire.

Les tables sont contrôlées structurellement ; un avertissement distingue cette
lecture de leur exécution. Les nouveaux imports persistants projettent les
cibles représentables dans la définition native v4. L'inspecteur temporaire et
les anciennes révisions non projetées gardent leur diagnostic. La collecte des
études importées reste soumise au [profil d'exécution](import-execution.md). La
continuité des anciennes participations est un chantier distinct différé.

## Projection des imports persistants

`project_question_defaults` relie question et sous-question par leurs identités
source archivées. Les codes seuls ne suffisent pas. Les 21 types du modèle
natif, le rôle Autre de L/!, les langues et les chaînes vides sont représentés.
Les valeurs et expressions ne sont ni évaluées ni converties en nombres. Un
`same_default` inconnu ne reçoit aucun mode implicite.

La projection est atomique par question. Une échelle, une cible spéciale, une
identité ou une taille non représentable empêche l'installation de tous ses
défauts ; les valeurs source restent dans l'original privé. Le rapport
`defaultProjection` expose le profil `source-defaults-v1`, les comptes, l'état
`complete` ou `partial` et des diagnostics sans contenu de réponse. Une
projection complète permet leur initialisation dans l'aperçu enregistré. Elle ne
neutralise ni les quotas, ni les autres contrôles de cet aperçu.

Les anciens imports ne sont pas réécrits. Leurs révisions historiques et
originaux restent inchangés. Le réexport chirurgical des conditions conserve
également les tables de défauts source. L'attribut d'équation constitué d'une
seule expression entre accolades est représenté sans son enveloppe dans le
modèle natif, après vérification syntaxique ; son texte source reste conservé.

Les corpus de référence vérifient aussi cette projection après les exports réels
LimeSurvey. Les thèmes et les autres réglages importés restent à qualifier : une
projection complète des défauts n'autorise pas la collecte.

## Configuration native et éditeur

La v4 ajoute `Question.defaults`, facultatif. Chaque entrée contient coordonnée
interne stable, rôle, langue et texte. Renommer un code ne change pas la cible.
Les v1/v2/v3 sont converties à la lecture ; les anciennes révisions SQL restent
inchangées. Un défaut déclaré dans une version antérieure à v4 est refusé.

Le panneau **Valeurs par défaut** expose les réponses principales de
S/T/U/N/D/L/!/5/Y/G/O, les lignes de M/P/Q/K/A/B/C/E/F/H et le texte Autre de
L/!. Classements, doubles échelles, matrices, commentaires et Autre M/P ne sont
pas des cibles natives de cet incrément. Une cible invalide est diagnostiquée.

Une case cochée avec un texte vide définit une valeur vide ; décochée, elle
laisse le défaut absent. Le mode partagé utilise la langue principale. Les
autres traductions restent conservées lorsqu'on change ce mode ou retire une
langue du parcours. L'export ne projette que les langues actives.

L'export LSS écrit `same_default`, `defaultvalues` et `defaultvalue_l10ns`, avec
les identifiants réattribués de ses questions et sous-questions. Les originaux
importés restent immuables et distincts de cette génération native.

## Initialisation à l'entrée d'une page

Une nouvelle campagne contenant des défauts sélectionne `native-defaults-v1`.
Sans présentation explicite dans l'API, elle reçoit le format groupe avec
progression. Les campagnes existantes gardent leur profil et leur révision.
L'aperçu natif utilise aussi une entrée explicite par groupe.

Rust et TypeScript initialisent seulement les champs absents des questions de la
page sortante et de la page demandée, plus les questions cachées précédentes
nécessaires au parcours. Les champs des pages futures attendent leur entrée. Une
clé présente, même `""`, n'est jamais remplacée. Un changement de langue ne
traduit donc pas rétroactivement les réponses. Sans défaut dans la langue
courante, aucune langue de repli n'est inventée.

Les pertinences, calculs, filtres et défauts partagent leur graphe de
dépendances. Les références entre cellules d'une question sont aussi ordonnées.
Les cellules inactives sont effacées avant stockage ; redevenues actives et
absentes, elles peuvent être initialisées à nouveau. Les évaluations
préparatoires ne doivent jamais effacer une réponse fournie avant qu'un défaut
source ait pu la rendre active. Seuls leurs défauts encore actifs sont repris
dans l'évaluation finale.

Une ligne exclusive explicitement choisie reste prioritaire sur un défaut d'une
autre ligne exclusive. Deux sélections exclusives fournies par le client restent
une erreur. Cette interaction possède des tests natifs, sans preuve de parité
exhaustive de toutes les combinaisons d'exclusivité amont.

Le serveur enregistre réponses et curseur atomiquement avec le reçu existant.
GET reste une lecture sans initialisation. Le navigateur prépare aussi la page
cible avant l'envoi ; un client API peut relire les réponses canoniques par GET.
Une répétition du même identifiant d'opération retrouve le même reçu.

La soumission initialise puis valide toutes les questions pertinentes : choisir
un curseur en début de questionnaire ne contourne pas les règles serveur.
L'appel générique `evaluate` refuse ce profil sans contexte ; utiliser
`enter_page` avec langue et présentation, ou le contrôle explicite des questions
`evaluate_initialized` pour une évaluation sans navigation.

## Expressions, limites et diagnostics

Les textes acceptent des fragments `{expression}` du moteur existant, sans
`eval`. Les accolades dans les chaînes citées sont respectées. Références,
fonctions inconnues, cycles et expressions non fermées bloquent le profil. Un
défaut littéral numérique ou de choix invalide bloque l'ouverture de la campagne
; un résultat dynamique est contrôlé avant toute sauvegarde.

Le contexte des valeurs inactives ExpressionScript n'est pas encore complet. Les
défauts utilisant `.NAOK`, ainsi que les règles lisant par `.NAOK` une question
possédant des défauts actifs, sont refusés explicitement.

Les limites par question sont 6 400 entrées et 4 096 unités UTF-16 par texte. Un
texte possède au plus 100 expressions ; son résultat est limité à 65 536 unités
UTF-16. Les évaluations Rust d'une transition partagent le même budget d'un
million d'unités, avec les règles et calculs. Le document final garde la limite
de réponses de 1 Mio. Aucune session participant n'est ajoutée en mémoire et
aucune dépendance externe n'est introduite.

## Vérification dans LimeSurvey 6.5.14

`examples/rules/defaults-reference.json` décrit 31 scénarios synthétiques.
`npm run test:limesurvey` importe un questionnaire indépendant par scénario,
crée les défauts et traductions, puis exporte un LSS réel. Le contrôleur de
référence suit l'affectation explicite de `specialtype` du contrôleur amont ; ce
champ n'appartient pas à ses attributs affectables en bloc.

Le moteur amont reçoit les langues et valeurs décrites. Le test exécute la
pertinence, les règles de sous-questions et `_ValidateQuestion`, puis compare la
valeur de session et, pour les cas concernés, les écritures `updatedValues`. Les
pertinences de la fixture de base sont remises à `1`, puis les conditions
propres au scénario sont ajoutées. Les valeurs source fournies sont contrôlées
dans le moteur. Le cache est remis à zéro entre étapes, comme dans la navigation
amont ; les sondes appellent la validation directement. Le lecteur Rust vérifie
chaque export : références réattribuées, échelles, cibles spéciales, traductions
exactes et `same_default`.

Les cas couvrent notamment :

- défauts littéraux et expressions simples de texte ou oui/non ;
- valeur absente, nulle, vide ou remplacée volontairement ;
- langue changée après initialisation et choix des défauts partagés ;
- absence de repli implicite pour un défaut localisé manquant ;
- zéro numérique, date ISO et codes de réponse avec zéro initial ;
- lignes de choix multiples, textes/nombres multiples et tableaux ;
- texte Autre d'une liste radio, Autre multiple et classement ;
- défaut numérique invalide, question non pertinente et ligne filtrée.

Ces contrôles portent sur le moteur PHP avec des entrées préparées. Ils ne
simulent pas un parcours complet dans le navigateur et ne certifient pas toutes
les questions, expressions ou options de défaut.

Le même lanceur importe aussi 29 configurations exportées depuis la v4 native,
compare leurs états d'initialisation et relit leur réexport : cibles, langues,
valeurs et `same_default`. Deux cibles ignorées dans la référence sont refusées
par le modèle natif. La configuration numérique invalide est exportable, mais
son exécution native est refusée ; conserver une structure n'autorise pas sa
collecte.

## Conséquences pour l'initialisation native

Le mécanisme amont teste la présence d'une valeur, pas seulement son contenu.
Une chaîne vide saisie volontairement doit rester vide au retour et à la
reprise. Un défaut partagé utilise la langue principale ; un défaut localisé
absent ne déclenche pas automatiquement ce repli. Une réponse déjà initialisée
ne doit pas être traduite rétroactivement au changement de langue.

Il faut aussi distinguer état d'évaluation et données persistées. Le moteur peut
remettre un défaut dans sa session pour une question ou ligne inactive, tout en
demandant d'écrire `NULL`. Copier toute sa session dans les réponses serait
incorrect. Les cas de pertinence et filtres vérifient cette différence avec
`deletenonvalues` activé.

Le défaut numérique invalide du corpus reste également dans l'état et la demande
d'écriture du moteur. Cette preuve ne garantit pas son stockage dans une colonne
numérique amont ; les contraintes et diagnostics natifs devront traiter ce cas
explicitement.

Les tests natifs Rust et TypeScript reprennent les valeurs persistables du
corpus, en distinguant explicitement une écriture nulle d'une valeur de session.
Ils ajoutent transitions de pages, effacement, dépendances, filtres activés par
un défaut et questions cachées. Les tests API, multi-bases et HTTP avec arrêt et
redémarrage vérifient la reprise et les reçus. Ces preuves ciblées ne certifient
pas toutes les interactions. Les cibles source restantes, NAOK et l'exécution
générale des études importées restent à réaliser.

## Références

- [Construction des champs et choix des défauts par langue][fields].
- [Initialisation et écritures du moteur d'expressions][manager].
- [Sauvegarde des défauts dans le contrôleur d'administration][controller].
- [Documentation des réponses par défaut][manual].

[fields]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/common_helper.php#L1404-L1443
[manager]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/expressions/em_manager_helper.php#L6556-L6610
[controller]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/controllers/admin/Database.php#L89-L145
[manual]: https://www.limesurvey.org/manual/Default_answers
