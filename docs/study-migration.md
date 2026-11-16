# Migration d’études en cours

## Exigence et portée

La continuité exacte d'une étude en cours est différée à la demande du porteur
du projet. Le protocole et ses exigences restent conservés pour cette étape
ultérieure. Ils ne bloquent pas la création et la configuration natives, les
nouvelles collectes ou les échanges de fichiers. L'ouverture d'une
[nouvelle campagne depuis un import](import-execution.md) reste distincte d'une
bascule avec anciennes participations.

L’échange standard des définitions et réponses reste dans le périmètre : LSS, VV
et LSR/LSA. Importer des réponses n’exige pas de recréer les droits de reprise,
cookies ou sessions de l’ancienne installation. Les états complets/partiels et
les informations utiles présentes dans les fichiers restent des données à
conserver ; ils ne constituent pas un droit d’accès.

Le protocole complet ci-dessous reste une cible ultérieure. L’extraction et la
conversion des métadonnées propres à LimeSurvey relèveront d’un outil externe ;
le backend et le frontend natifs n’accéderont pas à sa base ou à ses sessions.
Cette continuité historique n’est pas un préalable à la collecte RustSurvey.

La migration doit conserver les données et permettre à l’étude de continuer avec
le comportement attendu. Ce besoin fait partie de l’architecture dès maintenant,
même si toutes les fonctions LimeSurvey seront implémentées progressivement.

Le [modèle interne](domain-model.md) et l’[audit des sources 6.5.14][audit]
permettent de commencer sans export utilisateur. Le protocole ci-dessous est une
conception à implémenter et à vérifier, pas une migration déjà disponible.

La [préparation LSR](response-imports.md) réalise une première partie du
protocole : original privé, lignes conservées en SQL, correspondances des
cellules et chargement reprenable. Le
[bilan par révision](imported-collection.md) localise les prérequis d'exécution.
Une conversion réversible des cellules est maintenant disponible par ligne, sans
réévaluation des réponses historiques. La publication en réponses de collecte et
la continuité des participations restent à réaliser.

## Définir « sans perte »

Le contrat distingue quatre preuves :

1. **Conservation** : les données reçues et leurs états distincts restent
   accessibles ; aucun champ inconnu n’est supprimé silencieusement.
2. **Réexport** : les relations et valeurs peuvent être reproduites dans le
   format cible, y compris après les changements autorisés dans RustSurvey.
3. **Exécution** : questions, règles, accès et effets sur les réponses suivent
   le profil validé de LimeSurvey 6.5.14.
4. **Continuité** : les réponses existantes et les participations reprenables
   conservent leur état et leurs moyens d’accès après la bascule.

Archiver un fichier original prouve sa conservation, pas les trois autres
points. Une étude dont une fonction indispensable n’est pas exécutable peut être
conservée en préparation, mais ne peut pas être déclarée migrée.

La garantie porte sur les données effectivement fournies au transfert. Une
archive qui ne contient pas un fichier, un droit de reprise ou une valeur
effective ne permet pas de les recréer avec certitude. Le rapport doit les
réclamer comme éléments manquants avant la bascule, plutôt que diminuer
implicitement le périmètre de l’exigence.

## Profils d’échange

### Structure compatible

Le profil `.lss` porte la définition et les réglages représentables dans ce
format. Il ne transfère pas les réponses. Une réexportation transformée exige
que les alias et références concernés restent cohérents.

### Archive compatible

Le profil `.lsa` porte structure, réponses et parties optionnelles prévues par
le format. L’archive produite doit rester importable par LimeSurvey 6.5.14 sans
extension RustSurvey obligatoire pour ces parties standard.

Dans le code de cette version, l’archive standard ne transporte notamment pas
`saved_control` ni les fichiers téléversés. Elle ne représente donc pas à elle
seule toutes les garanties d’une étude en cours. [Constats][audit].

### Transfert complet d’étude

Un paquet de transfert ajoute aux fichiers standard un manifeste versionné et
les compléments nécessaires. Ce paquet est distinct d’un `.lsa` standard : le
simple import de celui-ci dans LimeSurvey n’appliquerait pas les compléments.

Le manifeste identifie l’installation source, l’étude, la version produit, le
schéma et l’instant de coupure. Il décrit les fichiers, leurs empreintes, les
nombres d’objets et les catégories demandées, présentes ou absentes.

Selon l’étude, il ajoute :

- état de collecte, dates et paramètres effectifs, avec leur origine ;
- enregistrements de reprise et correspondances avec les réponses ;
- ressources, pièces jointes et chemins logiques correspondants ;
- états d’exécution persistés ou données permettant de les reconstruire ;
- dépendances de thème/plugin et fonctions externes nécessaires ;
- correspondances d’identifiants et contexte temporel des dates source.

Si l’exporteur standard normalise une valeur, une copie source encodée de façon
réversible doit pouvoir l’accompagner. Le format standard seul ne sera pas
annoncé fidèle à des valeurs qu’il ne représente pas. Le rapport distingue la
fidélité au fichier reçu et la fidélité aux données source.

L’extraction et la restauration de ces compléments demanderont un outil dédié.
Il restera limité aux données explicitement concernées par l’étude, avec un
accès autorisé ; aucun accès à la production n’est requis pour ce premier
travail de conception.

## Inventaire de migration

| Élément                   | Condition à vérifier                          |
| ------------------------- | --------------------------------------------- |
| Définition et traductions | Références et fonctions couvertes             |
| Réponses                  | Valeurs et états de soumission conservés      |
| Participants              | Jetons, attributs, usages et droits conservés |
| Reprises sauvegardées     | Secret vérifiable, réponse retrouvable        |
| Progression et aléatoire  | Ordre et curseur reconstituables              |
| Quotas et timings         | Valeurs et effets futurs cohérents            |
| Fichiers et ressources    | Contenu, empreintes et liens disponibles      |
| Paramètres hérités        | Valeurs effectives connues                    |
| Liens de participation    | Même étude et mêmes droits                    |

Le rapport distingue un élément réellement absent, non demandé, manquant ou
présent mais non pris en charge. « Non demandé » ne permet pas de certifier une
migration complète si l’étude en dépend.

Les comptes administrateurs appartiennent à l’installation cible. Les droits
d’exploitation sont affectés explicitement ; copier un identifiant de compte
source vers un compte cible ayant le même numéro serait incorrect.

## Identifiants, liens et participants

Les correspondances utilisent une identité source qualifiée par installation et
étude, puis le type d’objet et sa clé. Elles ne sont jamais établies par simple
égalité de libellé, de nom de question ou d’adresse courriel.

Les jetons et les identifiants de réponses importés conservent leurs alias
d’échange. Les états de participant ne sont pas réduits à un booléen ; leurs
marqueurs et compteurs doivent garder leur signification source.

La conservation des liens diffusés peut nécessiter le maintien du domaine ou un
adaptateur d’entrée limité aux anciennes URL de participation. Celui-ci traduit
l’identité de l’étude et vérifie les mêmes droits. Il ne s’agit pas de
reproduire RemoteControl ou toute l’API web de LimeSurvey.

Le routage ne doit pas exposer un jeton dans les journaux ou à un tiers. La
disponibilité du domaine et des routes figure parmi les préconditions
d’exploitation, pas parmi les informations déductibles d’un `.lss`.

## Reprise et anonymat

Trois états doivent être distingués :

- une réponse déjà persistée, partielle ou soumise ;
- un droit permettant de la retrouver après fermeture du navigateur ;
- des valeurs qui existent seulement dans une session ou un formulaire ouvert.

Les deux premiers peuvent être migrés lorsqu’ils sont disponibles et pris en
charge. Le troisième ne peut pas être inventé depuis un export de fichiers. Le
protocole doit faire sauvegarder les participants avant la coupure ou organiser
le drainage des sessions ; une session PHP n’est pas transplantée comme un
checkpoint RustSurvey.

Une graine source n’est exploitable que si le générateur et l’ordre de ses
appels sont compatibles. Sinon, il faut obtenir l’ordre réalisé ou annoncer le
blocage de reprise exacte. La dernière page doit être traduite selon le mode
d’affichage et l’ordre effectif, pas copiée dans un curseur natif.

L’anonymat est conservé : ne pas construire une association participant/réponse
par recoupement des lignes ou des dates. Les éventuelles informations propres au
mécanisme de reprise sont isolées de l’analyse des réponses et soumises à des
droits distincts. Le rapport décrit leurs implications sans prétendre qu’un
fichier de réponses dépourvu de jeton garantit tout l’anonymat.

## Import en préparation

1. Inventorier le paquet et vérifier son intégrité avant toute publication.
2. Analyser structure, références, colonnes et dépendances du profil cible.
3. Produire un rapport par objet : conservable, exportable, exécutable,
   reprenable, ou bloqué avec une raison localisée.
4. Charger dans une campagne de préparation invisible aux répondants, par lots
   et transactions bornés.
5. Réconcilier nombres de réponses, états, participants, usages et quotas.
6. Tester le réexport et les parcours représentatifs en environnement isolé.
7. Autoriser l’ouverture uniquement après vérification du périmètre demandé.

Un gros transfert ne doit pas immobiliser une base dans une transaction unique.
Les lots sont repris avec un état de transfert persistant ; la publication
finale change un état ou une référence dans une transaction courte. Un lot
partiel n’est jamais présenté comme une étude opérationnelle.

Une nouvelle tentative du même paquet vers la même cible ne crée pas de
doublons. Elle utilise l’identité du transfert, les empreintes et les clés
source. Un autre instantané est une autre version de transfert ; les conflits ne
sont pas remplacés silencieusement.

## Bascule retenue pour la première version

Le premier protocole privilégie une fenêtre de gel contrôlée et un instantané
final cohérent. La synchronisation continue entre deux systèmes acceptant
simultanément des réponses est un sujet distinct, non nécessaire au premier
parcours de migration.

1. Réaliser l’analyse et une répétition sur copie pendant que la source reste
   seule autorité pour les écritures.
2. Arrêter les nouvelles entrées et drainer les écritures en cours selon une
   procédure vérifiable.
3. Capturer l’instantané final, les compléments et les paramètres effectifs à la
   même frontière de cohérence.
4. Charger et vérifier la campagne cible encore fermée.
5. Basculer le routage, puis ouvrir RustSurvey ; la source reste fermée.
6. Vérifier une réponse neuve et une reprise avant de terminer l’opération.

Les modifications temporaires faites pour geler la source ne doivent pas
remplacer ses réglages fonctionnels dans le manifeste. Un export effectué
pendant des écritures concurrentes ne sera pas présumé cohérent simplement parce
qu’il est accepté par un parseur.

Avant toute écriture dans RustSurvey, le retour à la source utilise la copie
figée et le routage précédent. Après de nouvelles réponses ou consommations de
jetons, le retour exige un transfert inverse vérifié de ces changements. Une
restauration ancienne ferait perdre les écritures reçues depuis la bascule.

## Échanges après modification

Une étude marquée compatible 6.5.14 ne doit accepter que des modifications
représentables dans son profil. Une fonction native sans équivalent dans ce
profil bloque l’export compatible ; elle n’est pas supprimée du fichier.

L’export courant combine le modèle interne et les données source encore
pertinentes. Une archive d’origine n’est jamais renvoyée comme état courant
après suppression ou modification de réponses. Conservation des originaux,
suppression et durée de rétention doivent rester cohérentes.

Un export complet distingue les fichiers standards et les compléments à
restaurer avec l’outil dédié. Une nouvelle importation dans RustSurvey doit
conserver tout l’état natif du transfert. Le retour dans LimeSurvey dépend aussi
des possibilités de restauration de son runtime ; le rapport doit préciser ce
périmètre au lieu de traiter `.lsa` comme un format universel.

## Preuves attendues

Le futur corpus couvre au minimum :

- conservation de `NULL`, vide, zéro, Unicode et codes à zéros initiaux ;
- colonnes SGQA, échelles, commentaires, « autre » et classements ;
- réponses soumises et partielles, marqueurs historiques et dates ambiguës ;
- jetons valides/épuisés et droit de reprise distinct de la liste des invités ;
- anonymat avec et sans reprise, sans association d’identité ajoutée ;
- navigation, ordre aléatoire, pertinence et effacement des valeurs ;
- fichiers absents, options héritées et fonction externe non disponible ;
- interruption d’import, réessai et absence de publication partielle ;
- bascule, nouvelle réponse, puis transfert inverse vérifié.

Comparer les données normalisées et les parcours aux résultats obtenus dans
LimeSurvey 6.5.14. Réexécuter les cas de persistance et migrations sur SQLite,
MariaDB et PostgreSQL. Les parseurs et compilateurs de pilotes seuls ne
constituent pas une preuve d’équivalence fonctionnelle.

[audit]: limesurvey-source-audit.md
