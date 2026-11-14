# Préparer l'import de réponses LimeSurvey

## Parcours disponible

Une étude importée depuis un LSS peut recevoir des fichiers de réponses LSR du
schéma 623. Ils sont conservés dans une préparation privée liée à une révision
enregistrée. Cette préparation ne crée pas de campagne publique et ne remplace
aucune participation existante.

1. Importer le LSS dans l'administration, puis ouvrir **Modifier**.
2. Ouvrir **Importer des réponses LimeSurvey** et choisir un fichier `.lsr`.
3. Vérifier la révision affichée, puis sélectionner **Importer les réponses**.
4. Suivre la progression. Les chargements suivants sont lancés automatiquement
   tant que le panneau reste ouvert.
5. Après interruption, retrouver la préparation dans cette même étude et
   sélectionner **Reprendre la préparation**. Le fichier déjà conservé est
   utilisé ; il n'est pas nécessaire de l'envoyer à nouveau.
6. Consulter le bilan, l'original, les correspondances et les valeurs des
   lignes. L'état **Données conservées** décrit la fin de ce chargement privé.

Ces lignes historiques ne deviennent pas des participations exécutables. Leur
reprise et le transfert des droits restent différés selon le
[protocole de migration](study-migration.md). Le parcours distinct des
[nouvelles campagnes importées](import-execution.md) peut collecter de nouvelles
réponses après qualification, sans utiliser cette préparation privée.

## Garanties de conservation

L'original LSR est immuable, conservé par `ObjectStore` et téléchargeable à
l'octet près après authentification. Les adaptateurs SQL et fichiers locaux sont
utilisables ; le mode fichiers relit aussi les anciens originaux SQL.

Les lignes gardent les valeurs lexicales reçues : codes `01`, nombres, dates,
espaces, Unicode et contenus non interprétés. Aucune règle de pertinence ni
équation n'est réévaluée pendant ce chargement. Une ancienne réponse cachée ne
doit pas être effacée par le moteur courant lors de sa conservation.

Le schéma conserve toutes les colonnes déclarées. Les lignes JSON téléchargées
contiennent seulement les valeurs non nulles :

- une clé présente avec `""` représente une chaîne vide ;
- une colonne déclarée dans `mapping.json`, mais absente de la ligne JSON,
  représente `NULL` ;
- une colonne non déclarée reste absente, sans valeur inventée.

Les colonnes inconnues sont conservées et signalées. Une colonne identifiant
explicitement une autre étude produit un refus avant création de préparation.
Les correspondances des cellules utilisent les identifiants source archivés du
LSS et les identités natives, pas une égalité de codes ou de libellés.

Les identifiants de réponse numériques sont indexés sous une forme canonique,
mais leur représentation reçue reste dans les valeurs source : `001` n'est pas
remplacé par `1` dans la ligne. Un identifiant absent ou invalide est signalé.
Les doublons d'un fichier restent des lignes distinctes, identifiées par leur
position ; aucune ligne ne gagne arbitrairement contre une autre.

Le statut décrit le marqueur `submitdate` du fichier : valeur non nulle, `NULL`,
ou colonne absente. Les dates restent des chaînes source ; une date
conventionnelle n'est pas transformée en événement réel. L'heure de création de
la préparation est une métadonnée locale distincte.

## Réessai et périmètre de source

Réenvoyer les mêmes octets pour la même étude et la même révision retrouve la
même préparation. Les lignes déjà validées ne sont pas recréées. Un autre
fichier ou une autre révision crée une préparation séparée ; ces instantanés ne
sont pas fusionnés automatiquement.

La portée est qualifiée par l'étude cible, le LSS original et la révision. Le
numéro `sid` ne prouve pas l'identité d'une installation LimeSurvey. La
provenance complète et les transferts entre installations demanderont le
manifeste décrit dans le protocole ; elle n'est pas déduite d'un fichier seul.

Une reprise relit l'original privé et vérifie son empreinte ainsi que le schéma
de correspondances. Un changement de correspondances refuse la reprise plutôt
que réinterpréter des lignes déjà conservées. Une nouvelle tentative prend la
main au moyen d'un jeton d'écriture SQL ; une ancienne tentative reçoit un
conflit avant de pouvoir continuer à écrire.

## Conversion réversible des cellules

Chaque ligne propose **Examiner la conversion (JSON)**. Cette lecture projette
les valeurs dans les clés de cellules natives de la révision figée. Elle ne crée
ni réponse de collecte ni droit d'accès participant. Les états d'exécution et de
continuité restent respectivement `blocked` et `unavailable`.

Le document contient `answers`, les métadonnées source, les représentations
originales des dates converties et les diagnostics. Les identités archivées des
questions et axes déterminent les correspondances ; un code identique ne
remplace pas une identité source manquante. Le calcul et les règles du
questionnaire ne sont jamais exécutés pendant cette conversion : un ancien
résultat ou une réponse devenue inactive reste conservé. Les valeurs
historiquement invalides ne sont pas corrigées rétroactivement.

Une clé native absente représente une cellule source nulle ; `""` reste une
valeur présente. Une colonne de cellule attendue absente refuse la conversion,
plutôt que produire un faux nul. Les colonnes supplémentaires restent dans les
métadonnées, avec diagnostic. Une langue initiale inconnue ou absente reste
exactement dans cet état ; elle n'est pas remplacée par la langue courante. Les
identifiants et marqueurs de date source ne sont pas réinterprétés.

La date calendaire stockée sous `YYYY-MM-DD 00:00:00` peut être présentée sous
`YYYY-MM-DD`, avec sa représentation originale conservée dans `originalValues`.
La bibliothèque `ResponseProjector::restore` la restitue tant que la valeur
native correspondante n'a pas changé. Une heure différente, ou une forme non
représentable par le champ date natif, reste intacte avec diagnostic ; elle
n'est pas tronquée silencieusement.

Le serveur vérifie les empreintes de l'original LSS et de la ligne SQL, puis
compare les correspondances enregistrées à celles de la révision. Un écart
refuse la conversion. Deux préparations au maximum compilent et convertissent
hors des tâches HTTP ; le téléchargement utilise les places de lecture
existantes. Les cellules natives gardent les limites de 1 Mio au total et 65 536
unités UTF-16 par valeur. Le document converti est limité à 16 Mio. Aucune
révision, ligne source ou campagne n'est modifiée par cette lecture.

Les tests couvrent les 28 types, tableaux et doubles échelles, commentaires,
valeurs vides/nulles, codes à zéro initial, dates, anciennes valeurs inactives,
calculs non réévalués et diagnostics sans réponse privée. Deux réponses et leurs
52 cellules réellement exportées par LimeSurvey 6.5.14 ont été projetées puis
restaurées à l'identique du fichier reçu. Cela ne reconstitue pas une donnée que
l'exporteur source avait déjà altérée et ne certifie pas encore une migration de
participations.

## Confidentialité et limites

Seul l'administrateur peut consulter ces préparations et leurs fichiers. Les
listes et bilans ne contiennent pas les valeurs de réponse. Les réponses, jetons
et autres champs privés reçus restent dans les originaux et les lignes privées.
Un jeton source n'est ni publié ni transformé automatiquement en droit de
reprise. Aucune association participant/réponse n'est déduite.

Le chargement est borné à 8 Mio de XML, 100 000 éléments/cellules source et deux
traitements concurrents. Le schéma et une ligne JSON peuvent occuper au plus 16
Mio après encodage. Une transaction porte sur 50 lignes ; une requête charge au
plus 500 lignes. Les listes contiennent au plus 50 préparations et 20 lignes par
page. Les téléchargements privés utilisent des fragments de 64 Kio et les quatre
places de lecture existantes.

Le parseur est relancé lors d'une reprise. Ce choix reste borné par le fichier
admis ; un index de fragments ou un parseur en flux devra être évalué pour les
gros transferts. Aucune mesure de gain CPU/RAM n'est annoncée pour cet import.

La suppression et la rétention restent à implémenter. Une préparation ne doit
pas être assimilée à une étude migrée ou à une autorisation de bascule.

## API

Toutes les routes sont privées. Les POST exigent aussi le client habituel et le
jeton CSRF.

| Méthode | Route                                                                  | Fonction              |
| ------- | ---------------------------------------------------------------------- | --------------------- |
| POST    | `/api/admin/studies/{id}/response-imports?revision=1`                  | Envoyer un LSR brut   |
| GET     | `/api/admin/studies/{id}/response-imports`                             | Préparations récentes |
| GET     | `/api/admin/response-imports/{id}`                                     | Progression et bilan  |
| POST    | `/api/admin/response-imports/{id}/continue`                            | Charger la suite      |
| GET     | `/api/admin/response-imports/{id}/original.lsr`                        | Original immuable     |
| GET     | `/api/admin/response-imports/{id}/mapping.json`                        | Colonnes et cellules  |
| GET     | `/api/admin/response-imports/{id}/records?from=0`                      | Liste de lignes       |
| GET     | `/api/admin/response-imports/{id}/records/{ordinal}/record.json`       | Valeurs non nulles    |
| GET     | `/api/admin/response-imports/{id}/records/{ordinal}/native-cells.json` | Cellules converties   |

Le premier envoi retourne 201 s'il est terminé, 202 s'il reste des lignes à
charger, ou 200 pour une préparation existante terminée. Les continuations
retournent leur progression courante. L'état `prepared` n'autorise pas la
collecte ; le bilan distingue conservation, correspondances, exécution et
continuité.

Les tests couvrent original exact, valeurs de plus de 64 Kio, réessais,
reprises, concurrence, colonnes inconnues ou absentes, identifiants dupliqués et
accès privés. Le parcours est exécuté sur SQLite, PostgreSQL et MariaDB, puis
avec les vrais fichiers de la [référence 6.5.14](limesurvey-reference.md). Le
test HTTP de stockage vérifie aussi l'arrêt complet du processus et les modes
SQL/fichiers. Les [limites du LSR amont](response-exchange.md) restent
applicables : une valeur déjà altérée par son exporteur ne peut pas être
reconstruite depuis le fichier reçu.
