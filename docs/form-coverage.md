# Couverture des formulaires LimeSurvey

## Objectif et statut

L’objectif acquis est la parité des **formulaires natifs de LimeSurvey
6.5.14+240624**, schéma 623 : types, variantes de rendu, règles et parcours de
réponse. Les études déjà utilisées servent à prioriser et à vérifier ; elles ne
limitent pas cette cible fonctionnelle.

Ce document est un **audit et un plan de réalisation**, pas une annonce de
compatibilité complète. La base de comparaison est le commit `62e21e0`, examinée
le 11 novembre 2026, puis complétée par l’incrément local sans stockage du même
jour. Les états ci-dessous décrivent le moteur de formulaire. L’incrément
d’[édition native](authoring.md) ajoute ensuite le stockage SQL des définitions
et de l’administration, puis les sources LSS privées et les conditions éditables
de groupes/questions dans le format v2. Le format v3 ajoute les images natives ;
l’éditeur expose maintenant 28 types, paramètres et traductions, sans collecte
de réponses.

Les fonctions F5/F6 demandant un stockage des participations restent différées ;
les propositions correspondantes ci-dessous décrivent la cible future. Les
médias et fichiers suivent la
[décision de stockage objet](decisions/0002-object-storage.md). Le port, les
adaptateurs SQL/mémoire/fichiers et les images natives sont présents ; S3 et
téléversements participants restent à réaliser.

Le périmètre est le formulaire vu et rempli par le participant. L’éditeur
administrateur, les invitations, les statistiques et RemoteControl sont des
chantiers distincts. Une règle de quota, un droit de reprise ou une ressource
nécessaire au formulaire reste cependant dans l’analyse, même si sa réalisation
demande du serveur et du stockage. Aucun compte utilisateur n’est nécessaire
pour concevoir une participation anonyme durable.

Les extensions et thèmes personnalisés doivent être inventoriés par étude. Les
rendus livrés dans le tag sont recensés ci-dessous ; reproduire un comportement
d’extension demandera un adaptateur explicite. Le code PHP, JavaScript ou Twig
importé n’est jamais exécuté automatiquement.

## Référence et méthode

Référence amont immuable : commit LimeSurvey
`ac3c44744e2c3769b3941c41e50b68419423ce67`. Les sources consultées sont :

- le [registre des types][ls-types] : 29 types ;
- les [29 configurations de base][ls-renderers] et les [8 thèmes de question
  livrés][ls-question-themes] ;
- la [résolution des attributs][ls-attributes] et son [registre de
  repli][ls-fallback], qui ne suffit pas à inventorier les options modernes ;
- le [moteur d’expressions][ls-expression-core] : 81 fonctions enregistrées
  avant les ajouts de plugins ;
- le [gestionnaire des règles][ls-expression-manager], le [rendu][ls-qanda], les
  [options d’étude][ls-survey], la [navigation][ls-runtime] et les [fonctions de
  collecte][ls-frontend].

Le [catalogue détaillé](form-catalog.md) répertorie les noms d’attributs et les
fonctions. L’inventaire porte sur les déclarations présentes dans le tag, sans
chargement de plugin ni exécution de PHP. Les tests synthétiques du moteur
TypeScript couvrent les ajouts et les diagnostics décrits plus bas. Aucun
contenu du LSS privé n’est utilisé dans cette documentation.

Côté RustSurvey, la comparaison porte sur la
[projection Rust](../crates/limesurvey/src/structure/project.rs), son
[enrichissement](../crates/limesurvey/src/structure/details.rs), le
[moteur](../web/src/survey/engine.ts), les
[attributs](../web/src/survey/attributes.ts), les
[expressions](../web/src/survey/expressions.ts), la
[validation](../web/src/survey/validation.ts), les
[composants](../web/src/components/QuestionInput.tsx) et les
[contrôles de fin du viewer](../web/src/components/StudyViewer.tsx).

L’exécution comparative dans une instance 6.5.14 et la certification de chaque
combinaison restent à faire. Un nom reconnu par le parseur, une donnée conservée
et un rendu ressemblant ne sont pas des preuves d’équivalence.

## Vue d’ensemble de l’existant

- 28 types disposent d’une saisie ou d’un résultat local de base. Les fichiers
  restent absents ; les variantes avancées des autres types restent partielles.
- 47 noms de fonction sur les 81 du registre amont sont implémentés. Signatures,
  conversions et sémantique restent à comparer à la référence.
- Le rendu utilise les contrôles Preact du projet ; les thèmes LimeSurvey ne
  sont pas interprétés. Le HTML est converti en texte.
- La pertinence couvre groupes, questions et sous-questions ; les filtres de
  lignes issus de M/P et les équations simples entrent dans le graphe.
- Les réponses sont des chaînes en mémoire ; il n’existe ni moteur de validation
  Rust des réponses, ni collecte, ni reprise durable.
- Le réexport original conserve les octets du LSS. La modification d’un titre
  est disponible ; la génération complète d’un questionnaire modifié ne l’est
  pas.

Ces décomptes décrivent des entrées de registres, **pas un pourcentage de
compatibilité**. Chaque type possède des variantes et des interactions. Zéro
blocage sur un LSS signifie seulement que les détecteurs actuels n’y ont trouvé
aucun blocage connu.

## Matrice des 29 types

« Base » signifie saisie locale disponible dans le profil du
[viewer](viewer.md), sans certifier toutes les options. « Partiel » signale une
restriction supplémentaire structurante. Les lots F0–F7 sont définis en fin de
document.

| Code | Type natif                           | État    | Principal travail restant                                 | Lot      |
| ---- | ------------------------------------ | ------- | --------------------------------------------------------- | -------- |
| `S`  | Texte court                          | Base    | Préfixe, suffixe, masque numérique, carte                 | F3/F4    |
| `T`  | Texte long                           | Base    | Dimensions, indicateurs et rendu riche                    | F4       |
| `U`  | Texte très long                      | Base    | Dimensions et limites natives                             | F3/F4    |
| `N`  | Nombre                               | Base    | Format local, préfixe/suffixe, variantes                  | F3/F4    |
| `D`  | Date/heure                           | Partiel | Heure, formats, bornes, menus et pas des minutes          | F3/F4    |
| `5`  | Choix de 1 à 5                       | Base    | Variantes de présentation et options communes             | F4       |
| `Y`  | Oui/non                              | Base    | Variantes et libellés d’interface localisés               | F4       |
| `G`  | Genre historique                     | Base    | Variantes et libellés d’interface localisés               | F4       |
| `L`  | Liste radio                          | Base    | Tri, filtres et thèmes de boutons/images                  | F1/F3/F4 |
| `!`  | Liste déroulante                     | Base    | Autre, catégories, recherche, préfixes                    | F1/F4    |
| `M`  | Choix multiples                      | Base    | Exclusivité, ordre et rendu autre avec case transitoire   | F1/F3    |
| `Q`  | Textes multiples                     | Base    | Lignes à la demande et contraintes par cellule            | F3/F4    |
| `K`  | Nombres multiples                    | Base    | Options avancées des curseurs et format numérique         | F3/F4    |
| `F`  | Tableau de choix par ligne           | Base    | Ordre, en-têtes répétés et largeur                        | F3/F4    |
| `H`  | Tableau de choix par colonne         | Base    | Options de rendu, largeur et ordre                        | F3/F4    |
| `:`  | Tableau numérique                    | Partiel | Bornes/pas dynamiques et variantes avancées               | F1/F3    |
| `X`  | Affichage sans réponse               | Base    | Texte enrichi, médias, minuterie et masquage              | F2/F4/F6 |
| `A`  | Tableau de choix 1 à 5               | Base    | Variantes avancées et corpus de référence                 | F1       |
| `B`  | Tableau de choix 1 à 10              | Base    | Variantes avancées et corpus de référence                 | F1       |
| `C`  | Tableau oui/non/incertain            | Base    | Traductions au-delà de français/anglais et corpus         | F1       |
| `E`  | Tableau augmenter/identique/diminuer | Base    | Traductions au-delà de français/anglais et corpus         | F1       |
| `1`  | Tableau à deux échelles              | Base    | Variantes de rendu et sémantique complète des contraintes | F1       |
| `;`  | Tableau de textes                    | Base    | Totaux affichés, variantes et corpus                      | F1/F3/F4 |
| `O`  | Liste avec commentaire               | Base    | Variantes de rendu et corpus                              | F1       |
| `P`  | Choix multiples avec commentaires    | Partiel | Commentaires sans effacement automatique et variantes     | F1/F3    |
| `R`  | Classement                           | Partiel | Filtres, limites natives et glisser-déposer               | F1/F3/F4 |
| `I`  | Changement de langue                 | Base    | Interface multilingue complète et corpus                  | F1/F4    |
| `*`  | Équation                             | Partiel | Affectations, équations composites/localisées et export   | F2       |
| `\|` | Téléversement de fichiers            | Absent  | Métadonnées, quotas de taille/nombre et stockage          | F6       |

Les familles A/B/C/E réutilisent le composant de choix par ligne avec leurs
domaines fixes. `1` conserve ses deux échelles. Le type I agit sur la langue
globale et le type `*` produit une valeur calculée. Les tests locaux ne
certifient pas encore leur équivalence générale dans LimeSurvey.

## Règles et interactions manquantes

### Cellules, commentaires et valeurs source

Le moteur local décrit maintenant les rôles réponse/commentaire/autre/calcul,
les positions de classement et les échelles. Les métadonnées de fichiers restent
différées. Les noms concaténés servent encore d’alias locaux ; une ambiguïté est
bloquante. Ils ne remplacent pas une identité interne stable.

Il faut réaliser les `FieldBinding` du [modèle interne](domain-model.md), avec
coordonnées et rôle typés, identités stables et alias d’expression distincts. Le
mapping vers les colonnes SGQA doit être vérifié pour chaque type et utilisé par
les imports/exports de réponses. Aujourd’hui, le viewer utilise ses noms locaux
; il n’offre pas cet échange.

La représentation source doit distinguer absent, `NULL`, vide, non coché et
valeur présente lorsque LimeSurvey les distingue. Conserver aussi codes à zéros
initiaux et représentation lexicale des nombres/dates. La carte de chaînes du
viewer ne suffit pas au stockage sans perte.

### Pertinence, filtres et initialisation

| Fonction                     | Existant                                     | À réaliser                                                   |
| ---------------------------- | -------------------------------------------- | ------------------------------------------------------------ |
| Pertinence groupe/question   | Graphe et effacement en cascade              | Vérifier la sémantique amont et les cas de navigation        |
| Pertinence de sous-question  | Visibilité et validité par cellule/ligne/axe | Compléter les sélecteurs et le corpus amont                  |
| Filtres inclusifs/exclusifs  | Sources M/P, lignes, styles 0/1              | Autres sources, choix simples et classement                  |
| Choix exclusifs              | Non gérés                                    | Exclusivité, auto-sélection, commentaire et autre            |
| Question cachée              | `hidden=1` distingue affichage et pertinence | Corpus, valeurs par défaut et collecte                       |
| Valeurs par défaut           | Section principale bloquante                 | Littéraux, expressions, traductions et `same_default`        |
| Anciennes conditions         | Section bloquante                            | Traduction des scénarios et rapprochement avec la pertinence |
| Préremplissage par URL       | Conservé, non projeté                        | Paramètres autorisés, typage et politique de priorité        |
| Aléatoire et ordre dépendant | Actif généralement bloquant                  | Groupes/questions/choix, `parent_order`, ordre réalisé       |

L’initialisation doit avoir une phase explicite. Une valeur par défaut ne doit
pas écraser une réponse au retour arrière ni être réappliquée après un
effacement sans preuve que la source le demande. Le masquage d’une question
calculée ne signifie pas suppression de son résultat. Le filtrage doit mettre à
jour les choix autorisés, l’obligatoire et les réponses devenues invalides.

### Validation

Présents dans le profil local : obligatoire strict/souple, nombre fini, entier,
date ISO, `min_num_value_n`, `max_num_value_n`, `min_answers`, `max_answers`,
`maximum_chars` et `em_validation_q` avec les expressions admises.

Restent notamment :

- expressions régulières `preg` et `regexMatch` ; leur dialecte et leurs
  garanties de coût doivent être vérifiés avant de choisir un moteur ;
- validation de sous-question `em_validation_sq` et messages localisés
  `em_validation_q_tip`/`em_validation_sq_tip` ;
- minimum, maximum et égalité de sommes, valeurs manquantes autorisées,
  contraintes dépendant d’autres réponses ;
- nombres seuls pour texte/autre, séparateurs locaux et format des dates ;
- bornes/pas des matrices et curseurs, minima/maxima de dates et fichiers ;
- contraintes du classement, des commentaires et de l’option autre ;
- interactions de l’obligatoire avec chaque type, filtre, échelle et valeur «
  sans réponse », ainsi que la confirmation du mode souple.

Chaque contrainte doit avoir un domaine d’application typé. Le moteur applique
les contraintes aux rôles et types de cellules pris en charge ; leur présence
dans son registre ne prouve pas leur équivalence pour toutes les familles.

### ExpressionScript, substitutions et scores

Le registre local contient 47 noms ; les 34 autres sont listés dans le
[catalogue](form-catalog.md). Les signatures usuelles de `if` et `round`, les
comparaisons de chaînes et les alias `eq`, `ne`, `lt`, `le`, `gt`, `ge` sont
maintenant admis. Des bornes explicites limitent les résultats numériques et
textuels.

Les conversions et variantes de signatures restent partielles : notamment `trim`
à un argument, `strrev` limité à ASCII, précision d’arrondi bornée et
comparaisons à confronter à la référence. Les fonctions de date, aléatoire,
regex et plusieurs conversions/formatages restent à faire.

Manquent aussi :

- les alias SGQA/`INSERTANS`, les variables réservées et `TOKEN:...` ;
- les sélecteurs `self` et `that`, développés en listes de variables selon la
  question et les sous-questions visées ;
- les suffixes `.shown`, `.code`, `.value`, `.valueNAOK`, `.relevanceStatus` et
  les attributs statiques des variables ; `.NAOK` est actuellement retiré du
  nom, sans modèle complet de pertinence des variables ;
- les équations persistables, affectations et calculs composites/localisés ;
- la sémantique complète des substitutions et attributs de variable ; les textes
  de groupe, options et sous-questions sont déjà substitués localement ;
- les évaluations : `assessment_value` des choix, scores de groupe/global,
  règles et textes de la section `assessments`.

Concevoir un contexte d’évaluation typé, un AST borné, les lectures/écritures
autorisées et un graphe incluant calculs, filtres, défauts et validations. Les
faits de temps/aléatoire ont un cycle de vie explicite ; une réévaluation de
rendu ne doit pas relancer un tirage. Le serveur devra réévaluer les décisions
de confiance. Le [modèle interne](domain-model.md) prévoit un corpus partagé
Rust/TypeScript ; WASM reste une option à décider, pas une nouvelle dépendance
acquise. Aucun `eval` ni exécution de PHP n’est nécessaire.

## Rendus et parcours à compléter

| Domaine                  | Situation actuelle                                  | Cible                                                             |
| ------------------------ | --------------------------------------------------- | ----------------------------------------------------------------- |
| Pagination               | Toujours par groupe                                 | Formats A/G/S, options héritées, sauts et retour autorisé         |
| Début/fin                | Accueil/fin en texte                                | Description, politique de données, consentement, URL de fin       |
| Informations de parcours | Compteur et navigation propres au viewer            | Affichage des groupes, numéros/codes, index, progression          |
| Sans réponse             | Effacement local et option sur tableaux facultatifs | Politique effective `shownoanswer` et comportement par type       |
| Texte                    | Conversion en texte                                 | HTML sûr, liens, listes, emphase et substitutions échappées       |
| Ressources               | Aucune image ni pièce jointe rendue                 | Images, audio/vidéo, liens et ressources de thème nécessaires     |
| Disposition              | Colonnes et tableaux simples                        | Largeurs, lignes, préfixes/suffixes, en-têtes, masquage des aides |
| Contrôles                | Champs, radios, menus, cases                        | Curseurs, notes, menus de tableaux et catégories                  |
| Langues                  | Libellés importés, langue de base en repli          | Interface traduite, formats locaux, droite-à-gauche, type I       |
| Impression               | Non gérée                                           | Questionnaire et réponses imprimables, sauts de page              |
| Accessibilité            | Bases présentes, DOM non vérifié                    | Clavier, focus, erreurs, lecteurs d’écran et petits écrans        |

Les trois formats de réponse sont A, G et S ; la valeur I signifie héritage, pas
un quatrième écran. L’impression est une présentation distincte. La provenance
des réglages hérités doit être conservée et leur valeur effective résolue avant
publication.

Les 8 thèmes de question fournis ajoutent ces variantes :

| Thème                         | Type | Comportement à reproduire                        |
| ----------------------------- | ---- | ------------------------------------------------ |
| `bootstrap_buttons`           | L    | Choix sous forme de boutons                      |
| `bootstrap_buttons_multi`     | M    | Boutons à choix multiples                        |
| `bootstrap_dropdown`          | !    | Menu avec recherche et réglages visuels          |
| `image_select-listradio`      | L    | Choix unique illustré                            |
| `image_select-multiplechoice` | M    | Choix multiples illustrés                        |
| `inputondemand`               | Q    | Ajout de lignes parmi les sous-questions prévues |
| `ranking_advanced`            | R    | Classement et présentations/options associées    |
| `browserdetect`               | S    | Valeur issue des informations du navigateur      |

`question_theme_name` est maintenant projeté et les noms des 29 rendus de base
sont reconnus avec leur type. Les variantes supplémentaires non implémentées
bloquent la saisie. Leurs paramètres effectifs et leur rendu Preact restent à
réaliser. L’apparence des thèmes d’étude Vanilla/Fruity/Bootswatch et leurs
variantes est un sujet distinct du contrat des réponses ; les écrans et
fonctions qu’ils activent doivent être recensés. Une reproduction visuelle
exacte n’est pas démontrée par cet inventaire.

Les paramètres à projeter incluent notamment `format`, `allowprev`,
`questionindex`, `navigationdelay`, `showwelcome`, `showprogress`,
`showxquestions`, `showgroupinfo`, `showqnumcode`, `shownoanswer`,
`printanswers`, `autoredirect` et l’URL de fin localisée. Les formats
`surveyls_dateformat` et `surveyls_numberformat` concernent aussi les valeurs,
pas seulement la présentation. `allowsave`, `tokenanswerspersistence`,
`alloweditaftercompletion` et les réglages d’accès rejoignent la collecte
durable. Le retour de l’écran de fin à l’édition est aujourd’hui une fonction du
viewer, sans interprétation de l’autorisation source.

Pour les contenus riches, conserver la source, filtrer un sous-ensemble HTML et
valider les URL/ressources. Séparer le texte d’une expression de son résultat
échappé ; ne pas transformer une réponse en HTML exécutable. Le choix d’un
assainisseur et de sa dépendance devra être justifié à l’implémentation.

Les cartes/localisations liées à S demandent un composant dédié. Coordonnées,
permissions du navigateur, géocodage éventuel et fournisseurs de cartes sont des
dépendances explicites. Ni un import ni un aperçu ne doit déclencher un service
externe automatiquement. La détection du navigateur n’autorise pas à faire
confiance à cette valeur côté serveur.

## Rapport de capacités : corrections et limites

L’incrément sans stockage corrige plusieurs lacunes de l’audit initial :

1. Les noms de rendus de base sont distingués des thèmes supplémentaires ;
   ceux-ci ne sont plus assimilés silencieusement au rendu de base.
2. Un attribut inconnu valant `0` est diagnostiqué. Seules les valeurs neutres
   connues sont tolérées explicitement.
3. Les expressions de tous les textes et langues sont préanalysées ; les erreurs
   d’évaluation des textes affichables rejoignent le blocage global.
4. `input_size`, `display_rows`, les préfixes/suffixes/placeholders sont
   appliqués. Plusieurs différences cosmétiques sont signalées séparément.

Le rapport ne constitue pas encore un inventaire complet :

- les formats et autres paramètres d’étude ne sont pas tous projetés ;
- les attributs des sous-questions ne sont pas enrichis comme ceux des parents ;
- `assessments`, `survey_url_parameters`, `plugin_settings`, `themes` et
  `themes_inherited` sont conservés avec avertissement, sans analyse exhaustive
  des dépendances ;
- la conversion en texte retire mise en forme, médias et code importé. Leurs
  interactions nécessaires demandent un diagnostic et une adaptation dédiés ;
- les options avancées admises ou tolérées ne sont pas toutes certifiées par une
  exécution comparative dans LimeSurvey.

Le futur rapport doit inventorier **avant affichage**, pour chaque emplacement
source et chaque langue : type, variante, attribut effectif, expression,
ressource, section et dépendance externe. États distincts proposés : conservé,
exécutable, rendu équivalent, dépendant du serveur/stockage, non pris en charge
et non vérifié. Séparer dégradation de présentation et altération des réponses,
sans jamais les confondre avec une certification.

## Fonctions demandant du stockage ou une autorité serveur

**Implémentation différée pour le cycle actuel.** Ces fonctions restent
inventoriées pour conserver la cible de parité et préparer leur réalisation.

Tout ce qui précède peut en grande partie être simulé localement. La collecte
réelle exige aussi un moteur Rust de validation et de transition : un résultat
validé dans le navigateur n’est jamais une écriture de confiance.

Les noms de tables de collecte ci-dessous sont **conceptuels**. Les premières
migrations concernent seulement définitions et administration. Réutiliser le
[modèle interne](domain-model.md) et la [stratégie SQL](database.md), plutôt
qu’une table de réponses par étude.

| Sujet                       | État durable nécessaire                            | Proposition de réalisation                                                       |
| --------------------------- | -------------------------------------------------- | -------------------------------------------------------------------------------- |
| Sauvegarde/reprise          | Révision, réponses, curseur, langue, droit d’accès | `responses`, `answer_blocks`, checkpoint et secret opaque vérifiable             |
| Ordres aléatoires           | Ordres réalisés et faits de tirage                 | Checkpoint versionné ; les tirages d’une reprise ne changent pas                 |
| Temps et minuteries         | Début, échéance, actions, faits temporels          | Horloge injectable, règles de pause/rechargement vérifiées, autorité serveur     |
| Timings                     | Durées/événements utiles par écran                 | Stockage borné, sémantique de retour/reprise et correspondance `.lsi`            |
| Quotas                      | État des réponses, limites et compteurs            | Mise à jour transactionnelle portable, contrôle de concurrence et réconciliation |
| Participants/préremplissage | Jetons, attributs autorisés, usages                | `participants` et accès séparés des réponses anonymes                            |
| Fichiers                    | Contenu, métadonnées, propriété et état            | Métadonnées SQL, fichiers privés derrière une abstraction de stockage            |
| Ressources du formulaire    | Manifeste, contenu, liens, empreintes              | Ressources versionnées avec la définition ; réécriture des liens source          |
| Calculs/scores              | Révision du moteur, entrées et faits nécessaires   | Recalcul serveur, résultats typés selon les règles d’échange                     |
| Consentement et fin         | Accord requis, état de soumission et effets        | Transition atomique, vérification serveur et redirection contrôlée               |
| Chiffrement demandé         | Valeurs protégées et format d’échange              | Identifier `encrypted`, politique de clés séparées et export maîtrisé            |

### Écritures et sessions

Conserver une définition immuable par campagne. Une commande porte une version
attendue et un identifiant d’opération. Dans une transaction courte : vérifier
l’accès, réévaluer les règles sur l’état serveur, mettre à jour les blocs,
effacer les cellules devenues invalides, avancer le checkpoint et traiter les
effets de soumission. Un conflit ou un réessai ne doit pas doubler quota,
fichier associé ou consommation de jeton.

La mise à jour conditionnelle de version et les transactions évitent de dépendre
d’un verrou spécifique à PostgreSQL. Tester concurrence et migrations sur
SQLite, MariaDB et PostgreSQL. Les documents JSON versionnés restent validés en
Rust et stockés en texte avec une capacité adaptée au moteur.

Le viewer actuel réévalue un instantané global après saisie. Étendre le graphe
puis évaluer les dépendances affectées permettra de borner le coût des grands
formulaires ; mesurer cette évolution avant d’annoncer un gain. Les limites
d’aperçu actuelles, dont 2 000 questions, 2 000 cellules par question et 20 000
cellules par étude, sont détaillées dans le [viewer](viewer.md). Leur
dépassement est une limite de ressources, à distinguer d’un type non pris en
charge ; ne pas simplement supprimer ces bornes pour annoncer une couverture
plus large.

Une participation ouverte ne garde pas de session résidente attachée à un
processus. Un cache borné de définitions peut être partagé ; la reprise doit
fonctionner après redémarrage et changement de processus. Ni Redis ni compte
administrateur ne sont un préalable.

### Pièces jointes et ressources

Le port de stockage objet est présent avec adaptateurs SQL, mémoire et fichiers.
Un premier parcours d’images natives contrôle propriétaire, taille, signature,
dimensions et association à une révision. L’adaptateur S3 et les pièces jointes
des participants restent à réaliser ; voir la
[décision 0002](decisions/0002-object-storage.md). Le déploiement S3 ne devra
pas dépendre d’un volume disque persistant.

Prévoir un protocole temporaire → prêt → associé et une collecte des fichiers
orphelins : le stockage objet et SQL ont des cycles d’écriture distincts. Les
noms source restent des métadonnées, jamais des chemins d’écriture. Un fichier
masqué, effacé ou remplacé suit une politique explicite de rétention et de
réexport. Tester les interruptions entre téléversement et sauvegarde.

Le LSS ne fournit pas tous les médias ; une archive LSA ne contient pas non plus
systématiquement les fichiers téléversés. Les compléments du
[protocole de transfert](study-migration.md) restent nécessaires. Une ressource
manquante doit être signalée avant ouverture.

### Quotas, temps et accès

Un quota global ne se simule pas correctement avec les seules réponses de
l’onglet. Reproduire d’abord le moment de son évaluation, ses règles de
comptage, ses actions et ses messages ; l’admission finale est atomique côté
serveur. Pour une étude importée, réconcilier les compteurs avec ses réponses
sans rescanner toutes les réponses à chaque interaction.

Les minuteries demandent une décision vérifiée sur retour arrière, fermeture,
rechargement et reprise. Stocker des instants/faits utiles plutôt qu’une boucle
serveur par participant. Les fonctions temporelles et aléatoires du moteur ne
doivent pas diverger entre validation client et serveur.

Les accès par jeton, cookies de participation, CAPTCHA, dates d’ouverture et
d’expiration peuvent modifier le parcours. Les paramètres sont à importer et à
diagnostiquer même avant leur implémentation. Les variables privées de
participant ne sont pas toutes transmises au client. La collecte d’IP, référent,
données de navigateur et localisation suit les options effectives et les
garanties d’anonymat de l’étude, sans ajouter de suivi implicitement.

## Lots de réalisation proposés

Les identifiants ci-dessous servent à suivre ce document ; ce ne sont pas des
tickets créés dans un service externe. Chaque lot doit livrer du code, des
fixtures synthétiques et des tests de comportement. Les données de production
restent hors Git.

| Lot | Contenu                                                                        | Dépendances                                        | Critère de sortie                                                                         |
| --- | ------------------------------------------------------------------------------ | -------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| F0  | Projection et registre de capacités complet                                    | Audit actuel                                       | Options, thèmes et textes inventoriés ; aucun inconnu actif ignoré silencieusement        |
| F1  | Cellules typées et types A/B/C/E/1/;/O/P/R/I, variante numérique de : et autre | F0, `FieldBinding`                                 | Rendu local, domaines de valeurs, mapping des cellules et cas facultatif/obligatoire      |
| F2  | ExpressionScript, équations, alias, substitutions et scores                    | F0, cellules typées                                | 81 fonctions inventoriées avec signatures ; comportements testés ou diagnostic explicite  |
| F3  | Filtres, défauts, anciennes conditions, validations et ordre                   | F1/F2                                              | Chaînes de dépendances cohérentes, réponses effacées/initialisées au bon moment           |
| F4  | Rendus et parcours natifs, 8 thèmes de question, HTML sûr et langues           | F1/F2, règles de F3                                | Modes A/G/S, contrôles, textes et interactions vérifiés au clavier et sur petit écran     |
| F5  | Moteur Rust, premières migrations, collecte et reprise                         | Cellules et sémantique stabilisées du profil livré | Validation serveur, atomicité, idempotence, reprise après redémarrage sur les trois bases |
| F6  | Fichiers, ressources, quotas, temps et paramètres d’accès                      | F5 ; rendu local possible avant                    | Stockage/réexport fidèle, concurrence, pannes et compléments de migration testés          |
| F7  | Certification des formulaires 6.5.14                                           | Transversal, finalisé après F1–F6                  | Corpus exécuté dans les deux moteurs et aller-retour source/cible documenté               |

F1 possède maintenant une base locale pour les types sans fichiers. F0, F2, F3
et F4 ont progressé avec les diagnostics, expressions, filtres et contrôles.
Leurs variantes restantes et le corpus de référence constituent la suite du
travail sans stockage. F2/F3 doivent ensuite précéder toute annonce de règles
générales compatibles. F4 avance sur les rendus autonomes ; ses fonctions
nécessitant des médias importés restent à compléter. Le port objet et les images
natives couvrent une première partie de F6. Collecte, reprise, fichiers
participants, quotas et minuteries de F5/F6 restent à réaliser. F7 peut préparer
le corpus et comparer les profils locaux dès maintenant ; la parité complète
reste à vérifier après les fonctions différées.

## Preuves à obtenir pour chaque capacité

1. Créer une fixture synthétique dans LimeSurvey 6.5.14, puis exporter le schéma
   623 ; consigner options effectives, provenance et résultat attendu.
2. Tester le rendu et les valeurs autorisées : vide/`NULL`/zéro, codes lexicaux,
   obligatoire N/S/Y, langues, options et échelles concernées.
3. Comparer les parcours : avant/arrière, changement de langue, filtres,
   modifications en amont, calculs, masquage, défauts et soumission.
4. Exécuter le même corpus sémantique en TypeScript et Rust, avec temps et
   aléatoire injectés. Toute exception reçoit un diagnostic et un test.
5. Réexporter dans LimeSurvey : structure, réponses, fichiers et états utiles
   restent cohérents. Le JSON local actuel n’est pas cet export.
6. Pour le durable : interruption, réessai, conflit, redémarrage, quotas et
   migration avec données sur les trois bases.
7. Associer l’état de capacité à ses tests et aux variantes réellement vérifiées
   ; une ligne « implémentée » ne devient « compatible » qu’après comparaison à
   la référence.

Le corpus doit aussi combiner les fonctions : tableau filtré obligatoire avec
défaut, commentaire exclusif, classement filtré après retour arrière, équation
cachée utilisée dans un score, minuteur après reprise, quota concurrent et
fichier remplacé puis masqué. Une fixture par type sans ces interactions ne
suffit pas à établir la parité des formulaires.

[ls-types]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/models/QuestionType.php
[ls-renderers]:
  https://github.com/LimeSurvey/LimeSurvey/tree/ac3c44744e2c3769b3941c41e50b68419423ce67/application/views/survey/questions/answer
[ls-question-themes]:
  https://github.com/LimeSurvey/LimeSurvey/tree/ac3c44744e2c3769b3941c41e50b68419423ce67/themes/question
[ls-attributes]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/models/QuestionAttribute.php#L399-L436
[ls-fallback]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/questionHelper.php
[ls-expression-core]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/expressions/em_core_helper.php#L197-L280
[ls-expression-manager]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/expressions/em_manager_helper.php
[ls-qanda]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/qanda_helper.php
[ls-survey]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/models/Survey.php
[ls-runtime]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/SurveyRuntimeHelper.php
[ls-frontend]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/frontend_helper.php
