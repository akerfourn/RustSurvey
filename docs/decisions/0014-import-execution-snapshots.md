# Décision 0014 — Instantané d'exécution des nouvelles campagnes importées

## Décision

Une révision importée reste immuable et conserve ses références source. Le
compilateur `limesurvey-import-v1` vérifie configuration, correspondances et
fonctions avant de produire un instantané natif pour une nouvelle campagne. Il
ne publie aucune préparation de réponses historiques.

La migration 0010 ajoute un document privé nullable sur `runs`. Il contient la
définition exécutable, les profils, paramètres d'accès et de navigation, les
empreintes du LSS et du contexte choisi et les faits source déclarés. Les
anciennes campagnes conservent une valeur nulle et leur parcours existant.

La représentation dérivée utilise les profils natifs versionnés. Le rendu de
base est normalisé ; les réglages effectifs sont figés. Éditer le questionnaire
ou déposer un autre contexte ne change pas une campagne ouverte.

## Alternatives et coût

| Option                                   | Intérêt                           | Limite                                                              |
| ---------------------------------------- | --------------------------------- | ------------------------------------------------------------------- |
| Recalcul depuis le LSS à chaque écriture | Peu de stockage dérivé            | XML, résolution et couplage au contexte sur le parcours participant |
| Réécrire la révision importée            | Réutilisation directe             | Change l'historique et confond source conservée et exécution        |
| Instantané SQL par campagne              | Lecture autonome et version figée | Copie bornée de la définition par campagne                          |

L'instantané SQL est retenu. Il est limité à 8 Mio, utilise `LONGTEXT` sur
MariaDB et `TEXT` sur SQLite/PostgreSQL. Aucun volume disque, nouveau service ou
dépendance n'est nécessaire. Les listes et verrous ne lisent pas ce gros
document ; son décodage utilise les deux places de préparation existantes.

## Configuration et limites

L'ouverture choisit explicitement un contexte lié à la révision et des faits
source. Les empreintes sont vérifiées avant compilation. Une demande contenant
des accès ou une navigation différents des valeurs qualifiées est refusée. Les
faits ne sont pas une attestation d'installation ni un instant de coupure.

Les rendus adaptés et options admises sont documentés dans le
[profil d'exécution](../import-execution.md). Les horodatages VV exigent UTC et
un décalage nul. Le compilateur ne choisit pas un fuseau local pour une source
inconnue. Les premiers instantanés sans ce complément conservent leur exécution
figée à la lecture ; ils ne servent pas de preuve pour de nouvelles ouvertures.

La reprise exacte d'une étude en cours est différée à la demande du porteur du
projet. Le protocole reste conservé, sans bloquer les créations, nouvelles
collectes et échanges de fichiers. Ni une campagne neuve ni l'archivage d'un LSR
ne doivent être présentés comme une migration de participations.
