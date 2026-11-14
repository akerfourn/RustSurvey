# Nouvelle collecte d'un questionnaire importé

## Parcours disponible

Une étude importée peut ouvrir une nouvelle campagne si ses questions, règles et
réglages correspondent au profil `limesurvey-import-v1`. Les réponses de cette
campagne suivent ensuite les API natives : validation Rust, sauvegarde SQL,
curseur, soumission, reprise privée et reçus de réessai.

Ce parcours ne transfère pas les anciennes réponses ou invitations. La reprise
exacte d'une étude LimeSurvey en cours est différée, à la demande du porteur du
projet. La compatibilité des fichiers reste un objectif actif distinct.

Dans l'administration :

1. Importer un LSS, puis ouvrir **Modifier**.
2. Enregistrer les éventuelles modifications de conditions.
3. Dans **Collecter des réponses**, déposer ou choisir explicitement un
   **Contexte des réglages source** pour cette révision.
4. Choisir le fichier des **Paramètres d'exécution source**.
5. Sélectionner **Ouvrir une nouvelle campagne sur la version enregistrée**.
6. Utiliser le lien de participation ou créer les nouvelles invitations si le
   mode d'accès source les exige.

Les diagnostics bloquants restent localisés et n'exposent pas les valeurs des
réponses. Les réglages d'accès et de présentation proviennent de la source ; ils
ne sont pas remplacés par les choix par défaut du panneau natif.

## Produire les deux documents source

Copier `RustSurveySettingsCommand.php` et `RustSurveyRuntimeCommand.php`, depuis
`tools/limesurvey/`, dans `application/commands/` de l'installation source.
Depuis sa racine, avec le véritable identifiant d'étude et un répertoire privé
existant :

```sh
php application/commands/console.php rustsurveysettings 701 \
  /repertoire/prive/source-context.json
php application/commands/console.php rustsurveyruntime 701 \
  /repertoire/prive/source-runtime.json
```

Les commandes exigent LimeSurvey 6.5.14, schéma 623. Elles n'ouvrent aucune
campagne et ne modifient pas les données source. La première lit les 30 options
de groupes ; son périmètre est inchangé. La seconde lit seulement quatre
colonnes du questionnaire, les réglages globaux nécessaires, l'existence de la
table des participants et le nombre d'extensions actives. Elle ne lit aucune
ligne de participant ni de réponse.

Les fichiers doivent être nouveaux ; les auxiliaires refusent l'écrasement. Ils
n'affichent pas les valeurs et créent les fichiers privés en `0600` sous POSIX.
Windows conserve les droits du répertoire de destination.

Ces documents doivent correspondre au même état source que le LSS. Ils sont
déclarés par l'administrateur ; ils ne prouvent ni l'identité d'installation ni
une frontière de bascule. Le manifeste de migration reste un chantier séparé.

## Profil pris en charge

La résolution des 30 options doit être complète. Le compilateur compare les
cellules aux identités archivées et vérifie les défauts déjà projetés dans la
révision. Il conserve l'original et produit une définition d'exécution où seuls
les rendus de base connus deviennent le rendu natif `core`. L'accueil,
présentation et progression utilisent leurs valeurs source effectives.

| Sujet                                                      | Valeurs admises dans ce profil                                      |
| ---------------------------------------------------------- | ------------------------------------------------------------------- |
| Questions                                                  | Les 28 types sans fichiers, dans le périmètre natif vérifié         |
| Présentation                                               | Groupe, question ou tout sur une page ; progression oui/non         |
| Accès public                                               | Participations multiples ou limitation par cookie                   |
| Invitations nouvelles                                      | Réponses indépendantes ou liées, correction d'une réponse liée      |
| Reprise native                                             | Autorisée ; le réglage source `allowsave` doit être actif           |
| Horodatage                                                 | Actif, UTC explicite et décalage source nul                         |
| Navigation                                                 | Retour autorisé, sans index ni délai                                |
| Sans réponse                                               | Actif, y compris après le forçage global source                     |
| Valeurs inactives                                          | Effacement activé côté source                                       |
| Thèmes                                                     | `vanilla` et rendus de questions de base ; interface native adaptée |
| Notifications, IP, référent, timings, évaluations          | Désactivés                                                          |
| CAPTCHA, politique d'acceptation, suivi Analytics          | Explicitement désactivés                                            |
| Dates d'accès et destinataires de notification             | Colonnes déclarées, sans valeur active                              |
| Scripts, modules, extensions, quotas et ressources actives | Refusés s'ils dépassent le profil                                   |

Les autres options ne sont pas supposées neutres. Un champ ou une section
inconnus, même vides, restent bloquants. Les contenus de courriel, légendes
d'attributs et autres métadonnées reconnues mais inactives restent dans
l'original. Un réglage demandé sans fonction correspondante est diagnostiqué.

Les différences de présentation du navigateur ne reproduisent pas les thèmes
PHP. Aucun script embarqué n'est exécuté. Le filtrage cible du tableau H reste
refusé car la référence ne l'exécute pas. Les limites NAOK, aléatoire, regex,
expressions et variantes restantes suivent le périmètre des moteurs natifs.

## Instantané figé et ressources

La migration SQL 0010 ajoute `runs.execution_document`, nullable. Une campagne
importée stocke une fois sa définition d'exécution, le profil, les empreintes
source et contexte, le contexte sélectionné et les faits déclarés. Les révisions
et originaux existants ne sont pas modifiés.

Les listes et transactions d'écriture lisent seulement les métadonnées de
campagne. La lecture d'un instantané est limitée aux deux places de préparation
et son décodage se fait hors des tâches HTTP. Il n'existe aucun cache de
définitions ou de participants croissant avec les sessions ouvertes.

Une demande d'ouverture est limitée à 64 Kio, l'instantané à 8 Mio. Les premiers
instantanés sans métadonnées de fuseau restent lisibles ; une nouvelle ouverture
exige désormais le fuseau et le décalage explicites. Une campagne enregistrée
n'est pas requalifiée contre une source courante à chaque lecture.

## Échanges et vérifications

Une campagne fermée exporte ses réponses en VV avec les correspondances source.
Une langue initiale inconnue bloque cet export, même pour un questionnaire à une
seule langue. Le LSS de campagne conserve les identifiants et données de
l'original, avec les conditions de sa révision figée. Il ne matérialise pas les
réglages hérités d'une autre installation ; les contextes restent nécessaires.
Le changement d'identifiant d'échange des créations natives ne s'y applique pas.

La référence réelle importe un LSS généré par LimeSurvey, extrait ses deux
documents puis exécute les API RustSurvey : ouverture, saisie des 28 types,
recréation de l'état serveur, reprise, soumission et exports. Les tests courants
ajoutent défauts, révision éditée après ouverture, refus d'override et
instantané illisible sans divulgation. Les essais externes vérifient également
les trois bases et un arrêt puis une relance du processus HTTP en modes SQL et
fichiers.

Ces preuves portent sur ce profil. Elles ne certifient pas toute la parité
LimeSurvey, les ressources de production ou le transfert des anciennes reprises.
