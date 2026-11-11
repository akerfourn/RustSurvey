# Analyse des sources LimeSurvey 6.5.14

## Périmètre et méthode

Analyse statique du tag `6.5.14+240624`, commit
`ac3c44744e2c3769b3941c41e50b68419423ce67`, réalisée le 10 novembre 2026. Le
schéma cible est 623. Les liens ci-dessous pointent vers ce commit, et non vers
la branche courante de LimeSurvey.

Les exporteurs, importeurs, modèles et exemples amont permettent de concevoir
RustSurvey sans export de production. Ils ne prouvent pas encore le résultat
d’un import exécuté dans LimeSurvey. Aucun code PHP n’a été exécuté et aucun
code ou fichier de données amont n’a été intégré au dépôt RustSurvey.

## Définition du questionnaire

`surveyGetXMLStructure` exporte les familles suivantes dans le `.lss` :

- groupes, questions, sous-questions et options de réponse ;
- traductions de ces objets et paramètres linguistiques de l’étude ;
- attributs de question et valeurs par défaut, y compris leurs traductions ;
- conditions, évaluations, quotas et membres de quotas ;
- paramètres du questionnaire, paramètres d’URL et réglages de plugins.

`surveyGetXMLData` ajoute langues et configurations de thèmes, dont une
configuration héritée. L’export de la ligne `surveys` exclut notamment
propriétaire, état actif et date de création. Il ne faut donc pas déduire l’état
opérationnel complet du seul `.lss`. [Source : exporteur][structure].

Le [modèle Question][question] distingue sous-questions, échelles et règles de
pertinence. Son caractère obligatoire possède trois valeurs `Y`, `S`, `N` : le
réduire à un booléen ferait perdre une distinction du produit source.

Le [registre des types][types] distingue notamment ces familles :

| Famille             | Codes                                       |
| ------------------- | ------------------------------------------- |
| Textes              | `S`, `T`, `U`                               |
| Choix               | `5`, `!`, `L`, `O`, `M`, `P`, `Y`, `G`      |
| Saisies             | `D`, `N`, `K`, `Q`, `I`                     |
| Tableaux            | `A`, `B`, `C`, `E`, `F`, `H`, `1`, `:`, `;` |
| Classement          | `R`                                         |
| Calcul et affichage | `*`, `X`                                    |
| Fichiers            | `\|`                                        |

Cette classification guide le modèle ; elle n’annonce pas leur implémentation.

## Réponses et correspondance des champs

`createFieldMap` associe les questions à des colonnes construites à partir des
identifiants du questionnaire, du groupe et de la question. Les tableaux,
commentaires, réponses « autre », classements et fichiers ajoutent leurs propres
coordonnées ou suffixes. Une simple correspondance code/valeur ne suffit pas.
[Source : construction des champs][fieldmap].

La même fonction décrit les métadonnées de réponse : identifiant, date de
soumission, dernière page, langue initiale et graine. Selon la configuration,
elle ajoute notamment jeton, dates, adresse IP et référent. La présence d’un
jeton dans les réponses dépend aussi de l’anonymat. [Source][fieldmap].

`buildXMLFromQuery` traite différemment une valeur SQL `NULL`, omise dans la
ligne XML, et une chaîne vide, écrite comme élément vide. Le nom des colonnes
numériques est adapté au XML ; certains caractères de noms sont également
transformés. L’importeur applique les transformations inverses et un
`FieldReMap`. [Sources : sérialisation][serialization], [import][import].

Cette sérialisation peut également modifier les valeurs avant leur transfert :
elle retire des caractères invalides en XML et remplace la séquence `]]>`. Pour
une fidélité aux valeurs de base au-delà du fichier reçu, ces cas demandent un
complément encodé de façon réversible. [Source][serialization].

## Contenu et limites de l’archive

`Export::exportarchive` assemble une structure `.lss`, des réponses `.lsr` si
l’étude est active, des participants `.lst` et éventuellement des timings
`.lsi`. Il ne joint ni table `saved_control`, ni sessions PHP, ni contenu des
fichiers téléversés. Ce constat porte sur cette fonction, sans supposer que tous
les autres chemins d’export sont équivalents. [Source][archive].

L’import `.lsa` de référence importe la structure, active le questionnaire, puis
charge réponses, participants et timings. RustSurvey devra au contraire garder
une étude importée en préparation jusqu’à la validation du transfert. C’est une
décision de conception, pas un comportement observé. [Source][import].

L’exporteur des réponses et participants passe par le déchiffrement des modèles.
Une archive peut donc contenir des valeurs en clair même si la base source est
chiffrée. Le chiffrement de la base ne suffit pas à protéger le fichier de
transfert. [Source][serialization].

## Reprise, héritage et exécution

`SavedControl` contient le lien à la réponse, un identifiant de reprise, un
vérificateur de secret et une étape sauvegardée. `loadanswers` s’en sert pour
rétablir la progression. Conserver seulement les réponses ne conserve donc pas
nécessairement le moyen de les reprendre. [Modèle][saved], [chargement][load].

L’exécution conserve aussi des informations dans la session PHP. La
randomisation utilise une graine et construit des correspondances de groupes et
questions ; une reprise fidèle doit reproduire cet ordre, pas seulement
réutiliser une graine dans un autre générateur. [Source][randomization].

Les règles de pertinence peuvent modifier les valeurs enregistrées lorsque des
champs cessent d’être pertinents. Leur effet dépasse le simple affichage. La
validation, les équations et la navigation sont liées dans le moteur. [Source :
moteur d’expressions][expressions].

Enfin, des options d’étude sont héritées d’un groupe ou des valeurs par défaut.
Une valeur brute « hériter » ne suffit pas à retrouver sa valeur effective sur
une autre installation. Le transfert devra expliciter ces valeurs et leur
provenance. [Source : résolution des options][options].

## Corpus amont disponible

La lecture XML des 65 fichiers `.lss` à la racine de `tests/data/surveys` a
réussi avec le parseur standard Python. Leurs révisions de schéma vont de 164 à
617 : aucune de ces structures n’annonce 623. Appartenir au tag 6.5.14 ne
signifie donc pas avoir été exporté par 6.5.14. [Dossier de
référence][fixtures].

Exemples examinés, utilisables pour préparer les scénarios :

| Fichier ou sujet du corpus                          | Schéma lu |
| --------------------------------------------------- | --------- |
| `limesurvey_survey_OrderTestSurvey.lss`             | 617       |
| `limesurvey_survey_MandatorySoftMultiPage.lss`      | 496       |
| `limesurvey_survey_376789_quotas.lss`               | 493       |
| `limesurvey_survey_allinoneCondition.lss`           | 485       |
| `limesurvey_survey_expressionAnswerOptionsTest.lss` | 477       |
| `survey_archive_2_basic_responses.lsa`              | 489       |
| `survey_export_responses_with_tokens.lsa`           | 495       |
| `survey_archive_getFileUploadTest.lsa`              | 486       |

Les trois archives ont été inspectées comme conteneurs ZIP et XML, sans
extraction ni affichage des valeurs de réponses ou des participants.

Ces observations fournissent un corpus de départ. Pour certifier la cible, il
faudra importer les cas retenus dans une instance de référence 6.5.14, les
réexporter et conserver les résultats attendus. Documenter la provenance et la
licence avant toute intégration de fixtures amont ; ne pas copier
automatiquement l’ensemble du dossier de tests.

## Conséquences de conception

L’[audit des formulaires du 11 novembre 2026](form-coverage.md) complète ce
premier relevé : 29 types, configurations et variantes de rendu, attributs,
fonctions ExpressionScript et écarts du moteur local. Le
[catalogue associé](form-catalog.md) distingue les déclarations XML, le registre
de repli et les ajouts des thèmes de question livrés. Il fixe une liste de
travaux, y compris pour les fonctions nécessitant une persistance.

Le [modèle interne](domain-model.md) sépare définition, campagne, réponse,
droits de reprise et provenance. Le [protocole de transfert](study-migration.md)
ajoute à l’archive standard les informations nécessaires à une étude en cours.
Les éléments absents de la source ne sont jamais reconstitués par supposition.

[structure]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/export_helper.php#L857-L1047
[question]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/models/Question.php
[types]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/models/QuestionType.php
[fieldmap]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/common_helper.php#L1294-L1990
[serialization]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/export_helper.php#L762-L852
[import]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/admin/import_helper.php
[archive]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/controllers/admin/Export.php#L1072-L1118
[saved]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/models/SavedControl.php
[load]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/frontend_helper.php#L23-L110
[randomization]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/frontend_helper.php#L1031-L1200
[expressions]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/expressions/em_manager_helper.php
[options]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/models/Survey.php#L2133-L2160
[fixtures]:
  https://github.com/LimeSurvey/LimeSurvey/tree/ac3c44744e2c3769b3941c41e50b68419423ce67/tests/data/surveys
