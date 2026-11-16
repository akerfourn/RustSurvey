# 0006 — Brouillons et publication sans interruption

## Décision du 16 novembre 2026

Une étude possède un brouillon de travail et un historique de définitions
immuables. Enregistrer un brouillon conserve une révision privée. L’action
**Publier cette révision** rend cette révision accessible aux nouveaux
arrivants, sans arrêter l’étude. Une restauration alimente le brouillon ; elle
ne publie pas automatiquement l’ancien contenu.

Le lien public sans numéro de révision reste stable. Le serveur le résout vers
la publication de numéro le plus élevé. Une publication ne modifie ni les
définitions précédentes ni leurs références de médias.

## Contrat de participation

La révision est choisie au commencement d’une participation. Elle reste la même
pendant toute la saisie et lors d’une reprise ultérieure, même si une nouvelle
version a été publiée. Cette règle s’applique aux textes, choix, règles,
traductions et ressources.

La collecte du pilote conserve déjà cette référence dans la campagne de chaque
participation. Lire, valider, sauvegarder et reprendre utilisent cette
définition figée, sans suivre la dernière publication ni faire confiance à une
révision fournie dans une commande de réponse. Publier ne transforme pas les
anciennes réponses. Une fermeture d’accès est une action distincte.

Le catalogue de publications et les campagnes sont deux entrées distinctes. Le
pilotage automatique des nouvelles participations vers une nouvelle publication
dans une même campagne reste à harmoniser ; une publication seule ne modifie pas
les campagnes existantes. Le pilote ouvre explicitement une nouvelle campagne
pour une autre révision.

## Implémentation actuelle

La migration 0012 ajoute `study_publications`, liée aux définitions par la clé
composée étude/révision. Une publication conserve son titre et son instant de
publication. Les catalogues et lectures publiques n’utilisent pas le titre ou le
contenu du brouillon.

Le serveur ajoute une publication par une écriture SQL conditionnée par la
révision attendue du brouillon. Un brouillon devenu obsolète est refusé ; une
répétition de publication ne crée pas de doublon. La sélection par maximum de
révision évite qu’un réessai tardif remette une ancienne version à l’entrée. Il
n’y a aucune nouvelle dépendance ni changement du format natif v4.

Le viewer charge une seule définition et inscrit sa révision dans l’URL de cette
visite, sans recharger le composant de saisie. Les images utilisent des routes
portant la même révision. Les accès par numéro ne servent que des définitions
effectivement publiées et leurs propres médias. Le lien partagé depuis
l’administration reste celui de l’entrée, sans numéro de révision.

Les études déjà présentes étaient publiques avant cette migration. Leur seule
révision courante est donc enregistrée comme publication initiale, sans modifier
leurs documents historiques. Les nouvelles créations et importations restent
privées jusqu’à publication explicite.

## Limites et vérifications

Les réponses du viewer restent en mémoire dans l’onglet. Un rechargement
conserve la définition sélectionnée par l’URL mais efface encore la saisie.
Cette URL n’est pas un droit de reprise ni une participation persistante. Les
commandes de collecte du pilote utilisent la révision enregistrée dans leur
campagne et un droit de reprise distinct.

Les tests couvrent confidentialité des brouillons, titre du catalogue,
publication répétée et concurrente, écriture obsolète, anciennes définitions et
images après publication, et redémarrage du serveur. Les parcours SQL sont
également exercés par le lanceur de bases jetables. La collecte et sa reprise
possèdent leurs propres tests ; elles restent distinctes des aperçus publiés.
