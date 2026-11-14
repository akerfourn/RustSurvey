# Décision 0009 — Cellules stables et export des réponses

## Statut

Implémentée pour les coordonnées natives, l'échange LSR hors collecte et
l'export VV de campagnes natives fermées. Les transferts complets et l'import
SQL des réponses restent à implémenter.

## Choix

Le domaine décrit une cellule par l'identité de question, ses coordonnées typées
et son rôle. Les coordonnées distinguent réponse simple, sous-question/échelle,
matrice et position de classement. Les rôles séparent réponse, commentaire,
autre, calcul et langue. Un renommage de code ou un réordonnancement d'axes
conserve l'identité des cellules concernées.

L'adaptateur produit les `FieldBinding` entre ces adresses, les alias actuels du
moteur et les colonnes SGQA. Il utilise le même plan d'identifiants que l'export
LSS. Les correspondances natives sont vérifiées contre `createFieldMap` de
LimeSurvey 6.5.14. Les identités source des imports et leur validation contre
les fichiers reçus restent à compléter.

Le VV est retenu pour exporter les réponses natives vers LimeSurvey. Le LSR
reste nécessaire pour lire et écrire les parties standard d'une archive. Les
deux formats ne possèdent pas les mêmes garanties dans le runtime cible.

## Comparaison vérifiée

| Option         | Intérêt                                                  | Limite dans la référence 6.5.14         |
| -------------- | -------------------------------------------------------- | --------------------------------------- |
| LSR            | Format XML des réponses d'une archive LSA                | Import des textes avec altérations      |
| VV             | Correspondances par codes et import sans ces altérations | Ne transporte pas les droits de reprise |
| Paquet complet | Peut conserver état natif et compléments source          | Protocole à implémenter                 |

Dans [SurveyDynamic::insertRecords][insert], l'import LSR remplace les valeurs
égales à `null` par `null` et retire apostrophes et accents graves. Les tests
sur l'instance de référence confirment les effets sur les chaînes vides et les
textes. L'[import VV][import] utilise un autre chemin et restaure les marqueurs
de texte sans cette normalisation.

L'[export XML][export] remplace `]]>` dans ses CDATA ; les retours chariot
littéraux y subissent aussi la normalisation XML. Notre lecteur préserve ce que
le fichier reçu représente ; il ne devine pas une valeur antérieure.

Le corpus vérifie l'import VV dans une deuxième étude dont les identifiants ont
été réattribués. Il inclut deux langues, tableaux, doubles échelles,
commentaires, classements, brouillon, soumission et caractères sensibles au
format.

## Cohérence et sobriété

L'export exige une campagne fermée. La fermeture est déjà ordonnée avec les
écritures des participants ; aucune mutation des réponses ne peut continuer
pendant ce téléchargement. Le questionnaire et ses données restent liés à la
révision figée.

La lecture par clé `(run_id, id)` remplace une pagination profonde par offset.
L'index est ajouté par migration versionnée. Un seul document est décodé par
travail, hors du thread HTTP. Un canal borné et un budget de deux traitements
limitent les téléchargements concurrents. Le permis couvre aussi les travaux
bloquants après annulation du client.

La même migration ajoute `initial_language`, renseignée à la création puis
immuable. Les anciennes réponses gardent `NULL` dans cette colonne. Une étude
monolingue permet une déduction certaine ; un ancien parcours multilingue sans
cette information bloque l'export. La langue courante du checkpoint ne doit pas
être présentée comme une langue initiale connue.

Aucune dépendance externe n'est ajoutée. Le port de corps HTTP et les canaux
Tokio étaient déjà présents. Le domaine reste indépendant de HTTP, SQL et
LimeSurvey. Le formateur UTC natif est déterministe et ne consulte ni horloge ni
configuration locale.

Ce choix borne les ressources ; il ne prouve pas encore un gain CPU/RAM face à
LimeSurvey. Des mesures sur une charge définie restent nécessaires.

[insert]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/models/SurveyDynamic.php#L116-L129
[import]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/admin/import_helper.php#L2659-L2942
[export]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/export_helper.php#L762-L855
