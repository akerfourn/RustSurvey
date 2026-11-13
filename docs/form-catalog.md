# Catalogue des options et expressions de formulaire

L’édition native expose maintenant les 28 types et leurs paramètres interprétés.
Les images natives passent par `ObjectStore` ; cela n’implémente ni les médias
embarqués dans le HTML LSS ni le type de téléversement participant `|`.

## Périmètre de l’inventaire

Les 47 fonctions du profil local ont aussi un évaluateur Rust et un corpus
commun de 87 cas. La [collecte native](collection.md) les utilise sur son profil
de 17 types. Les statuts ci-dessous restent ceux des variantes du viewer, sans
certification générale ni autorisation de collecte des imports.

Complément de l’[analyse de couverture](form-coverage.md), relevé le 11
novembre 2026. Référence LimeSurvey : `6.5.14+240624`, commit
`ac3c44744e2c3769b3941c41e50b68419423ce67`. État RustSurvey : incrément local
sans stockage après `62e21e0`. Les lots F0–F7 renvoient au plan de réalisation
de l’analyse.

L’inventaire des noms comprend :

- **133 attributs avancés distincts** dans les 29 configurations XML de base ;
- **130 noms** dans le registre PHP de repli, dont 3 absents des XML :
  `alphasort`, `question_template`, `thousands_separator` ;
- soit **136 noms** de base/repli, puis **17 noms supplémentaires** apportés par
  les 8 thèmes de question livrés ;
- **9 attributs généraux** déclarés par les configurations, dont
  `question_template` déjà compté ci-dessus : **161 noms distincts** au total ;
- **81 fonctions** dans le registre ExpressionScript, avant les plugins.

Ce ne sont ni 161 fonctionnalités indépendantes ni la liste de tous les champs
SQL de question/étude. Une option a des valeurs, des types applicables et des
interactions. Les options de collecte et les champs tels que
`question_theme_name` sont traités dans l’analyse principale ; ce dernier est
maintenant projeté pour contrôler la sélection de rendu. Les thèmes et plugins
personnalisés peuvent ajouter des noms à cet inventaire borné au tag.

## Méthode reproductible et sources

Lire les déclarations, sans exécuter PHP :

1. Dans les [configurations de base][base], relever
   `./attributes/attribute/name` et `./generalattributes/attribute` dans chaque
   `config.xml` ; dédupliquer les noms en conservant la casse.
2. Ajouter les clés affectées à `self::$attributes` par [le registre de
   repli][fallback]. La [résolution effective][resolver] utilise d’abord les
   configurations XML, puis ce repli.
3. Lire les mêmes chemins XML des [8 thèmes livrés][themes] ; distinguer les
   noms nouveaux des redéfinitions de `display_columns` et `input_size`.
4. Relever les clés de `RDP_ValidFunctions` dans le constructeur du [moteur
   d’expressions][expressions], avant l’événement d’extension.
5. Comparer au [registre local d’attributs][local-attributes], au
   [compilateur][local-expressions], à la [validation][local-validation] et aux
   [composants de saisie][local-input]. Les attributs généraux sont aussi lus
   par la [projection Rust][local-projection].

Les fichiers amont ont été consultés hors du dépôt. Ce catalogue ne copie ni
leur code d’implémentation ni les textes d’un export de production. Pour un
changement de cible, refaire ce relevé ; ne pas appliquer les chiffres à une
version LimeSurvey différente.

## Lecture des états

- **Partiel** : une interprétation existe, à compléter et à comparer à la
  référence. Ce n’est pas un statut de certification.
- **Toléré** : l’attribut passe sans bloquer mais son effet n’est pas rendu.
- **Non géré** : pas d’interprétation dédiée ; une valeur active produit un
  diagnostic. Une valeur `0` inconnue n’est plus supposée désactivée ; seules
  les valeurs neutres connues sont tolérées explicitement.
- **Durée active** : la minuterie bloque ; ses réglages secondaires sont tolérés
  sans durée, sans être implémentés.
- **Non projeté** : le champ reste dans le LSS conservé mais n’atteint pas le
  moteur local.

Les lignes concernent le rendu et l’exécution, pas la conservation des octets
source. Les réglages statistiques/export sont recensés pour ne pas les perdre,
mais ne sont pas des fonctions de saisie à implémenter dans ce viewer.

## Attributs de base et de repli

`XML` désigne au moins une des 29 configurations natives ; `Repli` désigne un
nom présent seulement dans le registre PHP. Le lot est la destination proposée,
pas un travail terminé.

| Attribut                            | Source | État local        | Lot         |
| ----------------------------------- | ------ | ----------------- | ----------- |
| `allowed_filetypes`                 | XML    | Non géré          | F6          |
| `alphasort`                         | Repli  | Non géré          | F4          |
| `answer_order`                      | XML    | Non géré          | F4          |
| `answer_width`                      | XML    | Toléré            | F4          |
| `answer_width_bycolumn`             | XML    | Toléré            | F4          |
| `array_filter`                      | XML    | Partiel           | F3          |
| `array_filter_exclude`              | XML    | Partiel           | F3          |
| `array_filter_style`                | XML    | Partiel           | F3          |
| `assessment_value`                  | XML    | Non géré          | F2          |
| `category_separator`                | XML    | Non géré          | F4          |
| `choice_input_columns`              | XML    | Non géré          | F4          |
| `choice_title`                      | XML    | Non géré          | F4          |
| `commented_checkbox`                | XML    | Partiel           | F3          |
| `commented_checkbox_auto`           | XML    | Partiel           | F3          |
| `cssclass`                          | XML    | Toléré            | F4          |
| `date_format`                       | XML    | Non géré          | F4          |
| `date_max`                          | XML    | Non géré          | F3          |
| `date_min`                          | XML    | Non géré          | F3          |
| `display_columns`                   | XML    | Partiel           | F4          |
| `display_rows`                      | XML    | Partiel           | F4          |
| `display_type`                      | XML    | Non géré          | F4          |
| `dropdown_dates`                    | XML    | Non géré          | F4          |
| `dropdown_dates_minute_step`        | XML    | Non géré          | F4          |
| `dropdown_dates_month_style`        | XML    | Non géré          | F4          |
| `dropdown_prefix`                   | XML    | Non géré          | F4          |
| `dropdown_prepostfix`               | XML    | Non géré          | F4          |
| `dropdown_separators`               | XML    | Non géré          | F4          |
| `dropdown_size`                     | XML    | Non géré          | F4          |
| `dualscale_headerA`                 | XML    | Partiel           | F4          |
| `dualscale_headerB`                 | XML    | Partiel           | F4          |
| `em_validation_q`                   | XML    | Partiel           | F3          |
| `em_validation_q_tip`               | XML    | Partiel           | F3          |
| `em_validation_sq`                  | XML    | Non géré          | F3          |
| `em_validation_sq_tip`              | XML    | Non géré          | F3          |
| `equals_num_value`                  | XML    | Partiel           | F3          |
| `equation`                          | XML    | Partiel           | F2          |
| `exclude_all_others`                | XML    | Non géré          | F3          |
| `exclude_all_others_auto`           | XML    | Non géré          | F3          |
| `hidden`                            | XML    | Partiel           | F4          |
| `hide_tip`                          | XML    | Toléré            | F4          |
| `input_boxes`                       | XML    | Partiel           | F4          |
| `input_size`                        | XML    | Partiel           | F4          |
| `label_input_columns`               | XML    | Non géré          | F4          |
| `location_city`                     | XML    | Non géré          | F6          |
| `location_country`                  | XML    | Non géré          | F6          |
| `location_defaultcoordinates`       | XML    | Non géré          | F6          |
| `location_mapheight`                | XML    | Non géré          | F6          |
| `location_mapservice`               | XML    | Non géré          | F6          |
| `location_mapwidth`                 | XML    | Non géré          | F6          |
| `location_mapzoom`                  | XML    | Non géré          | F6          |
| `location_nodefaultfromip`          | XML    | Non géré          | F6          |
| `location_postal`                   | XML    | Non géré          | F6          |
| `location_state`                    | XML    | Non géré          | F6          |
| `max_answers`                       | XML    | Partiel           | F3          |
| `max_filesize`                      | XML    | Non géré          | F6          |
| `max_num_of_files`                  | XML    | Non géré          | F6          |
| `max_num_value`                     | XML    | Partiel           | F3          |
| `max_num_value_n`                   | XML    | Partiel           | F3          |
| `max_subquestions`                  | XML    | Partiel           | F3          |
| `maximum_chars`                     | XML    | Partiel           | F3          |
| `min_answers`                       | XML    | Partiel           | F3          |
| `min_num_of_files`                  | XML    | Non géré          | F6          |
| `min_num_value`                     | XML    | Partiel           | F3          |
| `min_num_value_n`                   | XML    | Partiel           | F3          |
| `multiflexible_checkbox`            | XML    | Partiel           | F1/F4       |
| `multiflexible_max`                 | XML    | Partiel           | F3          |
| `multiflexible_min`                 | XML    | Partiel           | F3          |
| `multiflexible_step`                | XML    | Partiel           | F3          |
| `num_value_int_only`                | XML    | Partiel           | F3          |
| `numbers_only`                      | XML    | Partiel           | F3          |
| `other_comment_mandatory`           | XML    | Partiel           | F3          |
| `other_numbers_only`                | XML    | Partiel           | F3          |
| `other_position`                    | XML    | Non géré          | F1/F4       |
| `other_position_code`               | XML    | Non géré          | F1/F4       |
| `other_replace_text`                | XML    | Non géré          | F1/F4       |
| `page_break`                        | XML    | Toléré            | F4          |
| `parent_order`                      | XML    | Non géré          | F3          |
| `placeholder`                       | XML    | Partiel           | F4          |
| `prefix`                            | XML    | Partiel           | F4          |
| `printable_help`                    | XML    | Non géré          | F4          |
| `public_statistics`                 | XML    | Toléré            | Hors saisie |
| `question_template`                 | Repli  | Base seulement    | F4          |
| `random_group`                      | XML    | Non géré          | F3          |
| `random_order`                      | XML    | Non géré          | F3          |
| `rank_title`                        | XML    | Non géré          | F4          |
| `repeat_headings`                   | XML    | Non géré          | F4          |
| `reverse`                           | XML    | Non géré          | F4          |
| `samechoiceheight`                  | XML    | Non géré          | F4          |
| `samelistheight`                    | XML    | Non géré          | F4          |
| `scale_export`                      | XML    | Toléré            | Hors saisie |
| `show_comment`                      | XML    | Non géré          | F6          |
| `show_grand_total`                  | XML    | Non géré          | F4          |
| `show_title`                        | XML    | Non géré          | F6          |
| `show_totals`                       | XML    | Non géré          | F4          |
| `showpopups`                        | XML    | Non géré          | F4          |
| `slider_accuracy`                   | XML    | Partiel           | F4          |
| `slider_custom_handle`              | XML    | Non géré          | F4          |
| `slider_default`                    | XML    | Non géré          | F4          |
| `slider_default_set`                | XML    | Non géré          | F4          |
| `slider_handle`                     | XML    | Non géré          | F4          |
| `slider_layout`                     | XML    | Partiel           | F4          |
| `slider_max`                        | XML    | Partiel           | F4          |
| `slider_middlestart`                | XML    | Non géré          | F4          |
| `slider_min`                        | XML    | Partiel           | F4          |
| `slider_orientation`                | XML    | Non géré          | F4          |
| `slider_rating`                     | XML    | Non géré          | F4          |
| `slider_reset`                      | XML    | Partiel           | F4          |
| `slider_reversed`                   | XML    | Non géré          | F4          |
| `slider_separator`                  | XML    | Non géré          | F4          |
| `slider_showminmax`                 | XML    | Partiel           | F4          |
| `statistics_graphtype`              | XML    | Toléré            | Hors saisie |
| `statistics_showgraph`              | XML    | Toléré            | Hors saisie |
| `statistics_showmap`                | XML    | Non géré          | Hors saisie |
| `suffix`                            | XML    | Partiel           | F4          |
| `text_input_columns`                | XML    | Non géré          | F4          |
| `text_input_width`                  | XML    | Toléré            | F4          |
| `thousands_separator`               | Repli  | Non géré          | F4          |
| `time_limit`                        | XML    | Durée active      | F6          |
| `time_limit_action`                 | XML    | Toléré sans durée | F6          |
| `time_limit_countdown_message`      | XML    | Toléré sans durée | F6          |
| `time_limit_disable_next`           | XML    | Toléré sans durée | F6          |
| `time_limit_disable_prev`           | XML    | Toléré sans durée | F6          |
| `time_limit_message`                | XML    | Toléré sans durée | F6          |
| `time_limit_message_delay`          | XML    | Toléré sans durée | F6          |
| `time_limit_message_style`          | XML    | Toléré sans durée | F6          |
| `time_limit_timer_style`            | XML    | Toléré sans durée | F6          |
| `time_limit_warning`                | XML    | Toléré sans durée | F6          |
| `time_limit_warning_2`              | XML    | Toléré sans durée | F6          |
| `time_limit_warning_2_display_time` | XML    | Toléré sans durée | F6          |
| `time_limit_warning_2_message`      | XML    | Toléré sans durée | F6          |
| `time_limit_warning_2_style`        | XML    | Toléré sans durée | F6          |
| `time_limit_warning_display_time`   | XML    | Toléré sans durée | F6          |
| `time_limit_warning_message`        | XML    | Toléré sans durée | F6          |
| `time_limit_warning_style`          | XML    | Toléré sans durée | F6          |
| `use_dropdown`                      | XML    | Partiel           | F4          |
| `value_range_allows_missing`        | XML    | Non géré          | F3          |

`display_columns` règle la disposition, avec une limite locale de 12 colonnes et
un repli mobile. `multiflexible_checkbox` n’active que la variante à cases du
type `:`. Les contraintes ne sont exécutées qu’avec une expression admise et,
pour les attributs de contrainte, sans variante de langue. Le catalogue ne
présume pas qu’elles ont la sémantique native pour chaque famille de question.

Les 6 noms présents dans les XML mais absents du repli sont `cssclass`,
`hidden`, `other_position`, `other_position_code`, `show_grand_total` et
`show_totals`. C’est pourquoi l’inventaire du seul helper PHP serait incomplet.

Le registre local tolère aussi quelques noms absents de cet inventaire amont,
comme `hide_title`, `hide_help` et `chart_type`. Cela ne prouve pas qu’ils
appartiennent au contrat natif de 6.5.14.

## Attributs généraux déclarés dans les XML

Ces noms décrivent des propriétés générales de question ou d’édition ; leur
emplacement n’est pas forcément une ligne de `question_attributes`.

| Nom                 | État local          | Travail restant                                              |
| ------------------- | ------------------- | ------------------------------------------------------------ |
| `clear_default`     | Toléré              | Préférence d’édition, à distinguer du défaut de réponse      |
| `encrypted`         | Non projeté         | Protection du stockage et échange, F5/F6                     |
| `gid`               | Structure de base   | Références de groupe et mapping source, F0/F1                |
| `mandatory`         | Partiel             | N/S/Y selon chaque type et filtre, F3                        |
| `other`             | Partiel             | Choix simples/multiples, variantes restantes du rendu, F1/F3 |
| `preg`              | Bloquant si présent | Dialecte de regex et validation, F3                          |
| `question_template` | Base seulement      | Rendus de base admis, variantes supplémentaires bloquées, F4 |
| `relevance`         | Partiel             | Sous-questions et sémantique complète, F2/F3                 |
| `save_as_default`   | Toléré              | Préférence d’édition, à distinguer du défaut de réponse      |

`randomization_group`, `same_default`, les valeurs d’évaluation des réponses et
les paramètres de l’étude ne sont pas rendus exhaustifs par ce tableau : ils
figurent dans l’analyse et doivent entrer dans le rapport de capacités F0.

## Options supplémentaires des thèmes livrés

Les variantes natives et les options déjà listées restent nécessaires à ces
thèmes. Tous les noms suivants sont actuellement non gérés ; lot F4, avec F6 si
une ressource ou une collecte durable est nécessaire.

| Attribut            | Thème concerné                                          |
| ------------------- | ------------------------------------------------------- |
| `add_platform_info` | `browserdetect`                                         |
| `addlineicon`       | `inputondemand`                                         |
| `autoaddnewline`    | `inputondemand`                                         |
| `button_size`       | `bootstrap_buttons`, `bootstrap_buttons_multi`          |
| `crop_or_resize`    | `image_select-listradio`, `image_select-multiplechoice` |
| `fix_height`        | `image_select-listradio`, `image_select-multiplechoice` |
| `fix_width`         | `image_select-listradio`, `image_select-multiplechoice` |
| `horizontal_scroll` | `image_select-listradio`, `image_select-multiplechoice` |
| `keep_aspect`       | `image_select-listradio`, `image_select-multiplechoice` |
| `max_buttons_row`   | `bootstrap_buttons`, `bootstrap_buttons_multi`          |
| `only_pull`         | `ranking_advanced`                                      |
| `show_handle`       | `ranking_advanced`                                      |
| `show_number`       | `ranking_advanced`                                      |
| `show_search`       | `bootstrap_dropdown`                                    |
| `show_tick`         | `bootstrap_dropdown`                                    |
| `visualize`         | `ranking_advanced`                                      |
| `width_entry`       | `bootstrap_dropdown`                                    |

L’existence du thème est identifiée par ses métadonnées et son
`question_theme_name`, pas uniquement par la présence de ces attributs. Une
option absente peut prendre un défaut du thème. L’analyse de ce défaut et de
l’héritage fait partie de F0/F4.

## Fonctions ExpressionScript

Les noms sont ceux du registre amont, y compris la casse de `regexMatch`. Le
compilateur local normalise les noms admis ; le simple partage d’un nom ne
prouve ni les mêmes arguments ni le même résultat.

| Fonction                  | État local | Lot      |
| ------------------------- | ---------- | -------- |
| `abs`                     | Partiel    | F2       |
| `acos`                    | Partiel    | F2       |
| `addslashes`              | Non géré   | F2       |
| `asin`                    | Partiel    | F2       |
| `atan`                    | Partiel    | F2       |
| `atan2`                   | Partiel    | F2       |
| `ceil`                    | Partiel    | F2       |
| `checkdate`               | Non géré   | F2       |
| `convert_value`           | Non géré   | F2       |
| `cos`                     | Partiel    | F2       |
| `count`                   | Partiel    | F2       |
| `countif`                 | Partiel    | F2       |
| `countifop`               | Partiel    | F2       |
| `date`                    | Non géré   | F2       |
| `exp`                     | Partiel    | F2       |
| `fixnum`                  | Non géré   | F2       |
| `floatval`                | Non géré   | F2       |
| `floor`                   | Partiel    | F2       |
| `gmdate`                  | Non géré   | F2       |
| `html_entity_decode`      | Non géré   | F2       |
| `htmlentities`            | Non géré   | F2       |
| `htmlspecialchars`        | Non géré   | F2       |
| `htmlspecialchars_decode` | Non géré   | F2       |
| `idate`                   | Non géré   | F2       |
| `if`                      | Partiel    | F2       |
| `implode`                 | Partiel    | F2       |
| `intval`                  | Non géré   | F2       |
| `is_empty`                | Partiel    | F2       |
| `is_float`                | Non géré   | F2       |
| `is_int`                  | Non géré   | F2       |
| `is_nan`                  | Non géré   | F2       |
| `is_null`                 | Partiel    | F2       |
| `is_numeric`              | Partiel    | F2       |
| `is_string`               | Partiel    | F2       |
| `join`                    | Partiel    | F2       |
| `list`                    | Partiel    | F2       |
| `listifop`                | Non géré   | F2       |
| `log`                     | Partiel    | F2       |
| `ltrim`                   | Partiel    | F2       |
| `max`                     | Partiel    | F2       |
| `min`                     | Partiel    | F2       |
| `mktime`                  | Non géré   | F2       |
| `nl2br`                   | Non géré   | F2       |
| `number_format`           | Non géré   | F2       |
| `pi`                      | Partiel    | F2       |
| `pow`                     | Partiel    | F2       |
| `quoted_printable_decode` | Non géré   | F2       |
| `quoted_printable_encode` | Non géré   | F2       |
| `quotemeta`               | Non géré   | F2       |
| `rand`                    | Non géré   | F2/F5/F6 |
| `regexMatch`              | Non géré   | F2/F3    |
| `round`                   | Partiel    | F2       |
| `rtrim`                   | Partiel    | F2       |
| `sin`                     | Partiel    | F2       |
| `sprintf`                 | Non géré   | F2       |
| `sqrt`                    | Partiel    | F2       |
| `stddev`                  | Non géré   | F2       |
| `str_pad`                 | Non géré   | F2       |
| `str_repeat`              | Partiel    | F2       |
| `str_replace`             | Partiel    | F2       |
| `strcasecmp`              | Partiel    | F2       |
| `strcmp`                  | Partiel    | F2       |
| `strip_tags`              | Non géré   | F2       |
| `stripos`                 | Partiel    | F2       |
| `stripslashes`            | Non géré   | F2       |
| `stristr`                 | Non géré   | F2       |
| `strlen`                  | Partiel    | F2       |
| `strpos`                  | Partiel    | F2       |
| `strrev`                  | Partiel    | F2       |
| `strstr`                  | Partiel    | F2       |
| `strtolower`              | Partiel    | F2       |
| `strtotime`               | Non géré   | F2       |
| `strtoupper`              | Partiel    | F2       |
| `substr`                  | Partiel    | F2       |
| `sum`                     | Partiel    | F2       |
| `sumifop`                 | Partiel    | F2       |
| `tan`                     | Partiel    | F2       |
| `time`                    | Non géré   | F2/F5/F6 |
| `trim`                    | Partiel    | F2       |
| `ucwords`                 | Non géré   | F2       |
| `unique`                  | Partiel    | F2       |

Au total : 47 noms partiellement présents et 34 non gérés. Les extensions de
plugin peuvent enregistrer d’autres fonctions ; leur prise en charge exige un
inventaire et un adaptateur explicites.

Écarts déjà identifiés parmi les noms présents :

- `if` accepte maintenant 2 ou 3 arguments, avec branches paresseuses ;
- `round` accepte une précision, bornée localement à 15 chiffres ;
- `min`/`max` et les comparaisons demandent une sémantique de chaînes et de
  conversions plus complète ;
- `count`, `sum`, `is_empty`, `strlen`, `abs` et les arrondis doivent être
  confrontés aux valeurs vides, aux types et à l’Unicode de la référence.

Le noyau reconnaît également des suffixes de variable : `code`, `NAOK`,
`relevanceStatus`, `shown`, `valueNAOK`, `value`, ainsi que des attributs
statiques comme `qid`/`gid` et les textes de question. Le viewer ne fait
actuellement qu’une résolution des cellules connues et retire `.NAOK`. Les alias
SGQA, `INSERTANS`, variables réservées et `TOKEN:...` restent à réaliser avec
les règles d’accès correspondantes.

Le gestionnaire amont développe aussi les sélecteurs `self` et `that` en listes
de variables. Ce mécanisme est distinct d’une fonction enregistrée ; il manque
également au compilateur local, avec les effets de pertinence par cellule.

Les comparateurs textuels `eq`, `ne`, `lt`, `le`, `gt`, `ge` sont maintenant
acceptés. Les affectations restent des travaux F2 distincts des 81 noms de
fonction. Ni ce comptage ni celui des attributs ne remplace la vérification des
parcours et des effets sur les réponses.

[base]:
  https://github.com/LimeSurvey/LimeSurvey/tree/ac3c44744e2c3769b3941c41e50b68419423ce67/application/views/survey/questions/answer
[fallback]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/questionHelper.php
[resolver]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/models/QuestionAttribute.php#L399-L436
[themes]:
  https://github.com/LimeSurvey/LimeSurvey/tree/ac3c44744e2c3769b3941c41e50b68419423ce67/themes/question
[expressions]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/expressions/em_core_helper.php#L197-L280
[local-attributes]: ../web/src/survey/attributes.ts
[local-expressions]: ../web/src/survey/expressions.ts
[local-validation]: ../web/src/survey/validation.ts
[local-input]: ../web/src/components/QuestionInput.tsx
[local-projection]: ../crates/limesurvey/src/structure/project.rs
