# Décision 0011 — Contextes source immuables par révision

## Statut

Implémentée pour les 30 options héritables du profil
`limesurvey-group-settings-v1`. La collecte des imports et la continuité des
participations restent à réaliser.

## Décision

La migration 0008 ajoute `source_settings_contexts`. Un instantané associe
étude, révision de définition, empreinte du LSS original, octets JSON reçus,
empreinte du contexte, profil de résolution, résultat et date locale de dépôt.

Le document et son résultat sont immuables. Un fichier différent produit un
autre instantané. Les mêmes octets vers la même révision et le même profil
retrouvent le même instantané. Une nouvelle révision ne récupère pas
silencieusement les contextes d'une ancienne.

Le bilan choisit explicitement un identifiant de contexte. Sans identifiant, il
garde le LSS seul, même après un dépôt. Aucun pointeur mutable « dernier
contexte » ne change rétroactivement la signification d'un bilan. Les
définitions, campagnes, réponses et aperçus ne sont pas modifiés par un dépôt.

## Stockage et alternatives

| Option                           | Intérêt                              | Limite                                        |
| -------------------------------- | ------------------------------------ | --------------------------------------------- |
| Un contexte courant sur l'étude  | Accès simple                         | Changement implicite des bilans historiques   |
| Original objet et références SQL | Convient aux gros fichiers           | Réservation et réconciliation supplémentaires |
| Document SQL immuable            | Dépôt atomique et lien à la révision | Limite de volume explicite                    |

Le troisième choix est retenu pour un document structuré limité à 64 Kio.
L'original exact et le résultat tiennent dans la même transaction. Les colonnes
utilisent `LONGTEXT` sur MariaDB, `TEXT` ailleurs : 65 536 octets peuvent
dépasser la capacité d'un `TEXT` MariaDB. Les originaux LSR et médias continuent
à suivre leur port objet. Aucun accès disque métier, SDK ni service n'est
ajouté.

Une révision admet au plus 32 contextes. La transaction prend la ligne étude
comme point de sérialisation sans changer sa révision, puis vérifie doublon,
compte et insertion. Le dernier emplacement ne peut pas être attribué à deux
dépôts concurrents. Un réessai reste accepté après atteinte de la limite.

## Confidentialité et qualification

Toutes les routes exigent l'administrateur ; les dépôts exigent aussi le CSRF.
Les listes ne lisent ni originaux ni résultats. Les téléchargements retiennent
une place de lecture jusqu'à consommation ou abandon du corps. Le catalogue
public n'expose aucun contexte.

Le bilan vérifie les empreintes, la révision et le profil, puis compare la
résolution au résultat enregistré. Une divergence bloque le bilan choisi ; elle
ne fait pas revenir automatiquement au LSS seul. Un ancien original reste
téléchargeable si ses octets sont intacts, même si sa résolution doit être
requalifiée. Les noms de réglage JSON dupliqués sont refusés, y compris lorsque
la première valeur est nulle.

Le contexte est déclaré par l'administrateur. Il ne prouve pas l'identité
d'installation ou l'instant de coupure. La date du dépôt n'est pas une date
source. Le futur manifeste doit qualifier ces informations avant une bascule.

## Extraction et vérification

Une commande auxiliaire, exécutée dans l'installation LimeSurvey de référence,
lit seulement l'identité de l'étude, la chaîne des groupes et les 30 options.
Elle ne lit ni participants, réponses, adresses de notification ni secrets. Elle
borne la hiérarchie avant toute résolution récursive et écrit un fichier nouveau
privé, sans affichage des valeurs. PHP n'entre pas dans le serveur RustSurvey.

Les tests couvrent limites, concurrence, confidentialité, intégrité, réessai,
sélection explicite, révision historique et redémarrage. Le parcours est exécuté
sur SQLite, PostgreSQL et MariaDB. La référence compare l'extraction et les
options réelles, puis dépose les fichiers extraits dans RustSurvey.

Voir le [parcours et les routes](../source-settings.md).
