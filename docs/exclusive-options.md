# Lignes et choix exclusifs

## Réglages disponibles

Dans l'éditeur natif, ouvrir **Calculs, validation et présentation**, puis
renseigner **Codes des lignes exclusives**. Les codes sont séparés par `;`. Ils
désignent les sous-questions, par exemple `NONE`, et non leur libellé ou leur
identifiant technique.

L'exclusivité est disponible pour les types A/B/C/E/F/M/P/K/Q. Une ligne
exclusive renseignée désactive les autres lignes et efface leurs réponses. Les
lignes restent affichées. Pour un nombre, `0` est une valeur renseignée ; une
chaîne composée d'espaces l'est aussi. Effacer le choix exclusif réactive les
autres lignes, sans restaurer les réponses supprimées.

Les choix multiples M/P disposent aussi de **Sélectionner l'option exclusive
quand toutes les autres sont remplies**. Cette option correspond à
`exclude_all_others_auto=1`. Elle n'agit qu'avec un seul code exclusif et aucun
point-virgule dans la liste, comme dans la version de référence. Plusieurs codes
avec cette option conservent l'exclusivité manuelle, sans sélection automatique.

Le code `other` peut désigner l'option Autre d'un choix multiple si elle est
activée. Lorsqu'elle est une option ordinaire, son texte compte parmi les
réponses attendues par la sélection automatique. Les commentaires P ne comptent
pas comme des sélections. Ils suivent leur ligne et leur mode de présentation ;
une sélection automatique efface tous les commentaires de la question.

## Exécution et validation

Le moteur applique d'abord la pertinence et les filtres. Une ligne exclusive
devenue non pertinente ne peut pas désactiver les autres lignes. La sélection
automatique compte les lignes ordinaires pertinentes, et exige une ligne
exclusive pertinente. S'il existe des lignes ordinaires mais qu'elles sont
toutes non pertinentes, ce compte peut être nul et déclencher la sélection.

La normalisation précède les règles des questions dépendantes. Un tableau filtré
sur un choix multiple utilise ainsi ses sélections effectives. Les contrôles
obligatoires et de nombre de réponses restent appliqués aux cellules actives ;
le minimum n'est pas automatiquement ignoré devant un choix exclusif.

Le navigateur traite le clic sur une option exclusive comme une modification
atomique : il efface les autres options avant l'évaluation. Rust reçoit un
instantané, sans ordre de clic fiable. Si plusieurs lignes exclusives y sont
renseignées simultanément, il refuse la sauvegarde avec un diagnostic. Il ne
choisit pas arbitrairement la première valeur. Une seule ligne exclusive
renseignée provoque la même normalisation côté serveur, même si le client a
aussi envoyé des réponses ordinaires.

Les codes inconnus, types incompatibles, valeurs automatiques invalides,
attributs localisés ou dupliqués sont refusés lors d'une configuration native.
Les réglages inchangés d'un import restent conservés ; leur présence ne prouve
pas qu'une campagne importée soit exécutable. Le profil historique
`native-simple-v1` n'est pas étendu. Les profils natifs étendus utilisent les
mêmes règles Rust/TypeScript. Aucun schéma SQL ni dépendance n'est ajouté.

## Référence et preuves

L'analyse porte sur LimeSurvey 6.5.14+240624, commit
`ac3c44744e2c3769b3941c41e50b68419423ce67` :

- les configurations des questions et le [registre de repli][registry] déclarent
  `exclude_all_others` et limitent l'automatique à M/P ;
- le [gestionnaire d'expressions][manager] construit les règles de pertinence
  exclusive et le traitement client de la sélection automatique ;
- la [fonction `exprmgr_empty`][empty] distingue les chaînes vides de `0`.

Le corpus `examples/rules/exclusive.json` comprend 21 scénarios partagés par
Rust et TypeScript : neuf types, commentaires, Autre, filtres, pertinence,
automatique, effacement et entrée ambiguë. Des tests complètent le traitement du
dernier clic et le refus des attributs dupliqués.

Les tests HTTP vérifient normalisation Rust, erreur sans écriture partielle,
soumission et réessai idempotent. Le parcours PostgreSQL/MariaDB utilise aussi
ces réponses. Le test de stockage relit une réponse normalisée après arrêt
complet du serveur en modes objet SQL et fichiers locaux.

Le lanceur LimeSurvey importe réellement un LSS configurant les neuf types et
contrôle les attributs enregistrés. Cette preuve porte sur l'échange des
réglages. Les interactions complètes dans le navigateur LimeSurvey, notamment
avec les thèmes et les autres contraintes, restent à comparer. La collecte des
études importées demeure bloquée.

[registry]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/questionHelper.php#L336-L353
[manager]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/expressions/em_manager_helper.php#L1743-L1857
[empty]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/expressions/em_core_helper.php#L3200-L3206
