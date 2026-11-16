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

Le contrat visé est un échange de fichiers importables et réexportables,
permettant aussi la migration d’études en cours sans perte de données et avec le
comportement attendu. La reproduction de RemoteControl ou de la base interne
LimeSurvey n’est pas une exigence.

Quatre preuves sont distinctes : conservation des données reçues, réexport
fidèle, exécution équivalente et continuité des participations. Le
[protocole de migration](study-migration.md) définit ces garanties. Un fichier
importable ou archivé ne prouve pas que ses règles et reprises sont exécutables.

**Première implémentation `.lss` disponible** dans
[`crates/limesurvey`](../crates/limesurvey/README.md) : lecture bornée, modèle
source du schéma 623, diagnostics, export original et édition d’un titre. Un
[viewer avec moteur local partiel](viewer.md) permet la saisie dans le
navigateur. La collecte native possède maintenant validation Rust, réponses SQL,
reprise et invitations. Un [profil importé](import-execution.md) ouvre des
campagnes neuves qualifiées. Les archives et la continuité des anciennes
participations restent à implémenter ; cette continuité est différée à la
demande du porteur du projet.

L'export LSS des créations natives est disponible depuis leur historique et le
CLI. Un [aller-retour réel dans LimeSurvey 6.5.14](limesurvey-reference.md)
vérifie la structure et les cellules des 28 types sans fichiers en deux langues.
Il vérifie aussi l'import de deux réponses VV dans deux études dont les
identifiants diffèrent. L'[échange des réponses](response-exchange.md) distingue
ces garanties des transferts complets et des limites du LSR amont. Ce corpus ne
certifie pas tous les réglages ni l'exécution des formulaires.

La parité des formulaires natifs est la cible, au-delà des seuls exports déjà
essayés. L’[audit de couverture](form-coverage.md) et le
[catalogue des options](form-catalog.md) détaillent les manques et leur plan de
réalisation. Les diagnostics actuels ne constituent pas encore un contrôle
exhaustif de toutes les dépendances d’une étude.

## Formats et priorités proposées

| Format         | Nature              | Priorité proposée          |
| -------------- | ------------------- | -------------------------- |
| `.lss`         | Structure XML       | Première cible             |
| `.lsa`         | Archive ZIP         | Échange structure/réponses |
| `.lsr`         | Réponses XML        | Requis, à implémenter      |
| VV             | Réponses tabulées   | Import/export requis       |
| `.lsg`, `.lsq` | Groupe/question XML | Ultérieure                 |
| TSV            | Structure tabulée   | Selon usage réel           |
| CSV/réponses   | Résultats           | Export pour analyse        |

Le [manuel officiel][manual] distingue la structure `.lss`, sans réponses, de
l’archive `.lsa`. Commencer par `.lss` fixe l’ordre de développement de
l’adaptateur, sans limiter le modèle aux structures seules.

L’échange standard des réponses n’exige pas de recréer les anciens droits de
reprise ou sessions LimeSurvey. L’outil externe de transfert complet reste un
chantier ultérieur, distinct de cette compatibilité de fichiers.

## Points établis dans le code 6.5.14

La fonction `surveyGetXMLData` de l’[exporteur de structure][export-helper]
produit un document XML UTF-8 avec une racine `document`, un `LimeSurveyDocType`
de valeur `Survey`, un `DBVersion` et les langues.

La structure comprend des sections pour les groupes, questions, sous-questions,
réponses proposées, traductions, attributs, conditions, quotas et paramètres du
questionnaire. Leur structure peut être déduite du code de cette version ; le
corpus vérifie ensuite les variantes absentes, nulles et vides. Le
[relevé des sources](limesurvey-source-audit.md) détaille cet inventaire.

Le [contrôleur d’export][export-controller] construit les archives `.lsa` avec
un `.lss` et, selon les données disponibles, des fichiers de réponses `.lsr`, de
participants `.lst` et de chronométrage `.lsi`. Il ne faut pas assimiler ce
conteneur à une sauvegarde exhaustive de toute l’installation. Les reprises
`saved_control`, les fichiers téléversés et les paramètres opérationnels
manquants font l’objet d’un complément de transfert explicite.

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
- Préserver réponses soumises et partielles, participants, usages, marqueurs
  historiques, timings et données de reprise effectivement fournis.
- Distinguer les champs absents, nuls et vides, les codes et nombres lexicaux.
- Conserver les éléments source et des correspondances d’identifiants stables,
  selon le [modèle interne](domain-model.md).
- Garder l’étude en préparation si des données manquent ou si une fonction
  nécessaire à son comportement n’est pas prise en charge.

L’égalité recherchée est sémantique : les identifiants réattribués et les
différences de mise en forme XML doivent être distingués des pertes réelles.

## Corpus et vérification

Les sources et le corpus amont permettent de concevoir et de commencer les tests
sans export utilisateur. L’[audit](limesurvey-source-audit.md) relève 65
structures XML et plusieurs archives examinées dans le tag. Les structures
inspectées annoncent des schémas plus anciens que 623 ; leur présence dans ce
tag ne les transforme pas en exports de 6.5.14.

Pour certifier la cible, créer ou réexporter les fixtures avec **la version
6.5.14**. Aucun fichier amont n’a été copié dans le dépôt. Consigner provenance,
licence, options et résultat attendu. Les exports de production anonymisés
affineront plus tard les usages, sans bloquer l’architecture actuelle.

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
