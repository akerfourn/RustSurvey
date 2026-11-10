# Compatibilité des fichiers LimeSurvey

## Référence précise

La cible est **LimeSurvey 6.5.14**, tag **`6.5.14+240624`**, publié le 24 juin
2024 selon les [releases officielles][releases]. Le commit associé au tag est
`ac3c44744e2c3769b3941c41e50b68419423ce67`.

Le [fichier de version de ce commit][version] fixe `versionnumber` à `6.5.14` et
`dbversionnumber` à **623**. Ce numéro décrit le schéma LimeSurvey : ce n’est ni
la version de RustSurvey ni un identifiant unique de version produit.

Les sources de cette version priment sur le manuel courant pour les détails du
format. Celui-ci évolue avec LimeSurvey et peut décrire des versions
ultérieures.

## Sens de « compatible »

Le contrat visé est un échange de fichiers importables et réexportables. La
reproduction de RemoteControl, des routes web ou de la base LimeSurvey n’est pas
une exigence.

Deux niveaux doivent être vérifiés séparément : la conservation de la structure
et l’exécution équivalente du questionnaire. Un fichier importable ne garantit
pas que toutes ses expressions ou options sont exécutables.

**Aucun import/export n’est encore implémenté.**

## Formats et priorités proposées

| Format         | Nature              | Priorité proposée         |
| -------------- | ------------------- | ------------------------- |
| `.lss`         | Structure XML       | Première cible            |
| `.lsa`         | Archive ZIP         | Après cadrage des données |
| `.lsg`, `.lsq` | Groupe/question XML | Ultérieure                |
| TSV            | Structure tabulée   | Selon usage réel          |
| CSV/réponses   | Résultats           | Périmètre à définir       |

Le [manuel officiel][manual] distingue la structure `.lss`, sans réponses, de
l’archive `.lsa`. Le choix de commencer par `.lss` est une décision de périmètre
proposée pour RustSurvey.

## Points établis dans le code 6.5.14

La fonction `surveyGetXMLData` de l’[exporteur de structure][export-helper]
produit un document XML UTF-8 avec une racine `document`, un `LimeSurveyDocType`
de valeur `Survey`, un `DBVersion` et les langues.

La structure comprend des sections pour les groupes, questions, sous-questions,
réponses proposées, traductions, attributs, conditions, quotas et paramètres du
questionnaire. La liste exacte des champs devra être figée avec le corpus de
test, y compris les variantes absentes/vides.

Le [contrôleur d’export][export-controller] construit les archives `.lsa` avec
un `.lss` et, selon les données disponibles, des fichiers de réponses `.lsr`, de
participants `.lst` et de chronométrage `.lsi`. Il ne faut pas assimiler ce
conteneur à une sauvegarde exhaustive de toute l’installation.

L’[importeur de cette version][import-helper] est la référence pour vérifier ce
qu’un fichier produit par RustSurvey devra permettre de réimporter.

## Contrat à construire

- Préserver codes, ordre, langues, textes, choix, sous-questions et références.
- Remapper les identifiants internes en conservant leurs relations.
- Vérifier la sémantique des conditions et expressions prises en charge.
- Distinguer une option inconnue conservable d’une fonction indispensable à
  l’exécution ; refuser cette dernière si elle n’est pas implémentée.
- Produire des diagnostics localisés plutôt qu’une conversion silencieuse.
- Ne pas annoncer une préservation des thèmes ou plugins sans support dédié.
- Définir le traitement des champs inconnus avant de promettre un aller-retour
  sans perte. Leur simple conservation brute ne garantit pas la sémantique.

L’égalité recherchée est sémantique : les identifiants réattribués et les
différences de mise en forme XML doivent être distingués des pertes réelles.

## Corpus et vérification

Obtenir ou créer des fixtures anonymisées avec **la version 6.5.14**. Il n’y en
a pas encore dans le dépôt. Consigner leur provenance, les options utilisées et
le résultat attendu ; ne jamais inclure réponses ou jetons réels.

Le corpus initial devrait couvrir texte, nombres, choix simple/multiple,
sous-questions, champs obligatoires, deux langues, conditions et expressions
effectivement présentes en production. Ajouter aussi des cas volontairement non
pris en charge pour vérifier les diagnostics.

Pour chaque sous-ensemble annoncé :

1. Exporter une fixture depuis LimeSurvey 6.5.14.
2. L’importer dans RustSurvey et vérifier son modèle normalisé.
3. L’exporter depuis RustSurvey et l’importer dans LimeSurvey 6.5.14.
4. Comparer la structure et les parcours de réponse attendus.
5. Documenter les écarts et conserver un test de régression déterministe.

Les imports devront borner taille, profondeur et durée de traitement. Refuser
les entités XML externes et les accès réseau ; pour les futures archives, borner
la décompression et empêcher les sorties du répertoire d’extraction. Les textes
riches importés devront avoir une politique de rendu sûre.

[releases]: https://community.limesurvey.org/releases/
[version]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/config/version.php
[manual]: https://www.limesurvey.org/manual/Display/Export_survey
[export-helper]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/export_helper.php
[export-controller]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/controllers/admin/Export.php
[import-helper]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/admin/import_helper.php
