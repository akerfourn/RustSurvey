# Résoudre les options héritées de LimeSurvey

## Périmètre implémenté

Le résolveur reprend l'héritage de 30 options de `SurveysGroupsettings` dans
LimeSurvey 6.5.14, à partir du LSS et d'un contexte source explicite. Il couvre
les indicateurs oui/non, format A/G/S, informations de groupe, numéros/codes,
index des questions et délai de navigation. Il produit une valeur typée, son
état et son origine : questionnaire, groupe, parent ou groupe global.

Il ne remplace aucune valeur inconnue par la configuration RustSurvey. Le
contexte n'est pas une preuve de provenance d'installation : il doit venir du
futur manifeste de transfert, et son association à l'étude reste à qualifier. Le
résultat ne publie pas de campagne et ne modifie aucune réponse.

La résolution porte sur les options de groupes, pas sur tous les traitements du
formulaire. Thème effectif, dates et fuseau horaire, CAPTCHA, notifications,
ressources et plugins restent à compléter. Le réglage global `shownoanswer` peut
aussi forcer l'affichage indépendamment de l'option du questionnaire ; ce
forçage n'est pas appliqué par ce résolveur. Le
[profil d'exécution importé](import-execution.md) combine ce résultat avec des
faits source distincts pour qualifier une campagne neuve, dans un périmètre
strict. Le résolveur seul ne suffit pas à autoriser la collecte.

## Règles vérifiées

La référence est [`SurveysGroupsettings::getInstance` et
`shouldInherit`][source], au commit `ac3c44744e2c3769b3941c41e50b68419423ce67`.

- `I`, `inherit`, `-1`, valeur vide ou absente déclenchent l'héritage.
- La comparaison numérique souple reconnaît aussi des formes de `-1` comme
  `-1.0`. Les valeurs source ne sont pas réécrites.
- La chaîne `"0"` déclenche également l'héritage PHP pour ces options. Ainsi, un
  délai ou index nul dans l'étude peut retrouver une valeur de groupe.
- Les valeurs explicites restent prioritaires dans les groupes non nuls.
- Lorsqu'une étude est directement rattachée au groupe zéro, l'initialisation
  amont utilise les valeurs globales, même devant une valeur explicite.
- Un zéro numérique du dernier groupe global reste une valeur effective.

L'origine indique la couche qui a fourni la dernière valeur retenue. Une valeur
finale non admise reste `invalid`, avec valeur nulle dans le bilan. L'absence
d'un contexte nécessaire reste `unresolved`. Les diagnostics ne reproduisent
jamais les valeurs invalides reçues.

## Accueil hérité

`showwelcome=I` est désormais accepté par l'import LSS. Le modèle de lecture et
la définition importée conservent `showWelcome: null`, avec un avertissement.
Les valeurs absentes ou vides restent aussi inconnues. Auparavant, l'héritage
était refusé et une valeur absente était remplacée par `true`.

Pour les nouveaux imports, l'inspecteur, les aperçus publics et administrateur,
ainsi que l'historique, conservent cet état. L'aperçu signale la limite et ne
valide pas un parcours complet dont l'affichage de l'accueil est inconnu. Une
création native conserve son comportement local automatique lorsque son réglage
est nul. Les anciennes révisions SQL ne sont pas réécrites. Le bilan relit leur
original, même si leur ancienne projection avait déjà remplacé une absence par
une valeur locale.

## Utilisation

Le [bilan de collecte](imported-collection.md) affiche les 30 options dans
**Réglages héritables**, avec les valeurs directement résolubles depuis le LSS.
Le contexte peut maintenant être déposé dans **Contexte des réglages source**.
Choisir un fichier JSON, puis **Enregistrer et sélectionner ce contexte**.
Relancer ensuite **Vérifier les prérequis de collecte**. Le sélecteur permet
aussi de retrouver un contexte après redémarrage ou de revenir au LSS seul.

Chaque contexte est privé, immuable et lié à une révision. Un nouvel envoi ne
remplace aucun contexte existant et ne change pas un ancien bilan. Le fichier
original choisi se télécharge à l'octet près. Définition, aperçu et réponses ne
sont pas modifiés. Une nouvelle campagne qualifiée peut ensuite figer ce
contexte et ses faits d'exécution sans changer les révisions existantes.

La [décision 0011](decisions/0011-source-settings-contexts.md) décrit le
stockage SQL atomique de ces petits documents, la limite de 32 contextes par
révision et leur profil versionné. Aucune dépendance n'est ajoutée.

La bibliothèque expose `LssDocument::group_settings`. Le CLI écrit un fichier
nouveau, sans afficher de contexte ni de valeurs sur la sortie standard :

```sh
cargo run -p rustsurvey-limesurvey --locked -- group-settings \
  study.lss settings.json

cargo run -p rustsurvey-limesurvey --locked -- group-settings \
  study.lss resolved-settings.json source-context.json
```

Le contexte JSON est limité à 64 Kio, 64 groupes et 64 octets par valeur. Il
accepte seulement les 30 noms d'options sélectionnés. Les identifiants sont des
chaînes numériques canoniques ; groupes et ancêtres sont uniques et sans cycle.
Une identité d'étude ou de groupe contradictoire est refusée.

Extrait de forme, à compléter avec les options réelles de la source :

```json
{
  "schemaVersion": 1,
  "surveyId": "701",
  "surveyGroupId": "2",
  "groups": [
    {
      "id": "2",
      "parentId": null,
      "values": { "showwelcome": "N" }
    }
  ],
  "global": { "showwelcome": "Y" }
}
```

Le champ `complete` concerne uniquement la résolution des 30 options. Il ne
certifie ni les règles du formulaire, ni la continuité des participations.

## Extraire le contexte dans LimeSurvey

Le fichier
[RustSurveySettingsCommand.php](../tools/limesurvey/RustSurveySettingsCommand.php)
est une commande auxiliaire à placer dans `application/commands/` de
l'installation source. Elle exige LimeSurvey 6.5.14 et son schéma SQL 623.
Depuis cette installation, avec le véritable identifiant de l'étude et un
répertoire privé existant :

```sh
php application/commands/console.php rustsurveysettings 701 \
  /repertoire/prive/source-context.json
```

La commande utilise la configuration de l'installation source. Elle effectue des
lectures ciblées, sans modification de l'étude ni lecture des réponses ou
participants. Seules les 30 options sélectionnées sont exportées. La hiérarchie
est bornée à 64 groupes ; un cycle ou un ancêtre manquant produit un refus.

Le fichier doit être nouveau. Les valeurs ne sont pas imprimées ; sous POSIX, le
fichier est créé avec des droits 0600. Sous Windows, les droits du répertoire de
destination s'appliquent. Le serveur RustSurvey ne dépend pas de PHP.

LSS et contexte doivent représenter le même état source. La commande ne gèle pas
l'installation et ne certifie pas un instantané complet. La qualification et la
coupure suivent le [protocole de transfert](study-migration.md).

## API et garanties de persistance

Toutes les routes sont privées et sans cache :

| Méthode | Route                                                                          | Fonction                       |
| ------- | ------------------------------------------------------------------------------ | ------------------------------ |
| POST    | `/api/admin/studies/{id}/revisions/{revision}/settings-contexts`               | Déposer le JSON brut avec CSRF |
| GET     | `/api/admin/studies/{id}/revisions/{revision}/settings-contexts`               | Jusqu'à 32 métadonnées         |
| GET     | `/api/admin/source-settings-contexts/{id}`                                     | Métadonnées d'un instantané    |
| GET     | `/api/admin/source-settings-contexts/{id}/original.json`                       | Original exact                 |
| GET     | `/api/admin/studies/{id}/revisions/{revision}/collection-check?contextId={id}` | Bilan avec sélection explicite |

Le dépôt retourne 201 pour un nouveau fichier, 200 pour les mêmes octets déjà
enregistrés au même profil et à la même révision. Un contexte d'une autre étude
ou révision ne peut pas être sélectionné. Les noms de réglage dupliqués sont
refusés. Une valeur finale invalide peut être conservée pour diagnostic ; elle
ne devient pas une valeur exécutable.

Le résultat et le profil `limesurvey-group-settings-v1` sont conservés. Le bilan
vérifie à nouveau empreintes et résolution ; une différence produit un refus
explicite. La sélection n'est jamais remplacée par une autre en cas d'erreur. La
date de dépôt reste une métadonnée locale, distincte d'un instant de capture
source. La suppression et la rétention restent à compléter.

## Vérification dans la référence

`npm run test:limesurvey` crée deux groupes synthétiques dans sa base jetable,
modifie une étude synthétique et réexporte ses LSS ainsi que les contextes. Le
test Rust compare les 30 valeurs aux `oOptions` réellement obtenues dans
LimeSurvey : héritage sur deux niveaux, valeurs explicites, groupe zéro et
valeurs vides. Les modèles source sont relus après vidage du cache amont.

La commande console réelle et son refus d'écrasement sont aussi exécutés. Les
contextes extraits sont déposés dans RustSurvey, puis sélectionnés pour le
bilan. Les tests PostgreSQL/MariaDB couvrent la limite concurrente et un
original exact de 64 Kio. Le test HTTP vérifie la sélection après arrêt complet
du serveur.

Les autres tests couvrent contexte étranger, cycle, ancêtre manquant, clés
inconnues, valeurs invalides non divulguées, réexport d'un accueil hérité et
distinction entre paramètres natifs et importés. Aucun service distant n'entre
dans les tests courants. Voir la
[procédure de référence](limesurvey-reference.md).

[source]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/models/SurveysGroupsettings.php
