# Mesures CPU et mémoire

## Périmètre

`npm run bench` construit le serveur Rust optimisé avec le lockfile, puis mesure
des requêtes HTTP réelles sur SQLite fichier. Il n'appelle aucun serveur
existant : chaque scénario démarre son propre processus sur une adresse locale
éphémère, dans un répertoire temporaire avec données et accès synthétiques. La
base et ce répertoire sont supprimés après arrêt du processus.

Le banc n'est pas une mesure comparative de LimeSurvey. Il ne certifie ni les
déploiements HDS, ni PostgreSQL/MariaDB, ni S3, ni tous les questionnaires. Il
donne une première référence reproductible pour orienter les choix du serveur.
Les résultats locaux et leurs limites figurent plus bas.

Le [banc de participants espacés](performance-participants.md) complète cette
mesure : 100, 500 et 1 000 connexions indépendantes avec pauses utilisateur,
téléchargement du client web et pointes d'arrivée ou de soumission. Il utilise
le paquet pilote livré, sans reconstruction.

## Exécution

```sh
npm run bench
npm run bench -- --queue 0
```

La seconde commande reproduit le rejet immédiat des écritures occupées. Les deux
modes conservent deux écritures actives. La
[décision 0013](decisions/0013-bounded-write-admission.md) décrit la file courte
ajoutée après le premier constat de surcharge.

Contrôle court du lanceur, sans interprétation de performance :

```sh
npm run bench -- --operations 20 --sessions 10 --repeats 1 --concurrency 1,2,8
```

Options bornées : `--operations` de 1 à 1000 par phase, `--sessions` de 0 à
10000, `--repeats` de 1 à 10, `--queue` de 0 à 32. `--concurrency` accepte
jusqu'à quatre valeurs distinctes de 1 à 32. Le nombre d'opérations par réponse,
échauffement compris, reste inférieur au plafond des reçus journaliers. Les
identifiants de réponses et d'opérations synthétiques sont déterministes. Une
phase dépassant une minute échoue ; chaque requête possède aussi son délai de
quinze secondes. Le lanceur attend les requêtes déjà parties avant de nettoyer
son serveur.

Les mesures processus supportent Windows via un auxiliaire PowerShell local et
Linux via `/proc` et `getconf CLK_TCK`. L'auxiliaire est lancé sans fenêtre et
ne change aucune configuration système. Le parcours Linux possède un test de
lecture des compteurs ; son banc complet reste à exécuter sur un hôte Linux. Les
autres plateformes produisent une erreur explicite.

## Charges et contrôles

Trois questionnaires publics utilisent une présentation sur une page :

| Nom        | Définition                            | Modification répétée |
| ---------- | ------------------------------------- | -------------------- |
| `small`    | Sept questions, conditions et calcul  | Note texte           |
| `forms`    | Les 28 types natifs sans fichiers     | Texte très long      |
| `defaults` | Les mêmes 28 types avec trois défauts | Texte très long      |

Les définitions proviennent des corpus `collection.json` et
`collection-forms.json`. Le troisième scénario ajoute des défauts à S/T/Q, dont
une expression. Il utilise `native-defaults-v1` ; les autres utilisent
`native-pages-v1`. L'empreinte de chaque définition est enregistrée.

Pour chaque scénario, un processus et une base neufs sont utilisés. Après la
création et l'authentification, la mémoire est relevée avec zéro, cent puis
mille participations supplémentaires ouvertes, selon l'option choisie. Cette
phase mesure aussi la création des participations, avec deux clients.

Les sauvegardes utilisent par défaut trois répétitions, chacune avec 1, 2 puis 8
clients, et 1000 tentatives par phase. Chaque client possède sa propre réponse
et effectue dix sauvegardes d'échauffement, exclues des mesures. Les réponses
sont renouvelées entre phases ; la base conserve cependant les lignes et reçus
précédents. Il s'agit d'une charge fermée sans pause utilisateur, sans réessai
automatique et avec un client Node sur le même hôte.

Chaque accusé vérifie identité, version et curseur. Le banc relit ensuite les
réponses pour vérifier la dernière valeur et le calcul serveur, y compris un
calcul envoyé initialement comme falsifié. Après les phases, il redémarre le
processus et vérifie encore des réponses sauvegardées et une participation
ouverte sans sauvegarde. Une erreur métier ou HTTP inattendue fait échouer le
banc, plutôt que compter comme débit utile.

## Lecture des métriques

Le temps CPU est le cumul utilisateur et noyau du seul processus serveur. Son
delta est séparé de la durée murale. `cpuPercentOfOneCore` utilise 100 % pour un
cœur logique occupé ; ce n'est pas le pourcentage global du gestionnaire des
tâches. `cpuMsPerSuccess` inclut le coût des refus de la phase : une charge avec
beaucoup de refus ne peut pas être comparée comme un travail réussi identique.

Le débit compte seulement les succès. Les statuts et codes de refus restent dans
le rapport. Les percentiles des succès et ceux des refus sont séparés, pour
qu'un rejet rapide n'améliore pas artificiellement la latence affichée. La durée
client comprend préparation, transport local et décodage de l'accusé.

La mémoire résidente est échantillonnée environ toutes les 100 ms, avec un
relevé avant et après chaque phase. Le rapport distingue ce maximum observé du
pic résident fourni par le système sur la vie du processus, qui inclut son
démarrage et les hachages d'authentification. Les octets privés engagés Windows
ne sont pas assimilés à la RAM résidente ; cette métrique est nulle sur Linux.

Les compteurs CPU sont quantifiés par le système. Une phase très courte peut
donc afficher zéro sans signifier un traitement gratuit. L'échantillonnage ne
capture pas tous les pics instantanés. Le client, l'auxiliaire de mesure et les
autres programmes du bureau peuvent influencer les latences de cet hôte ; leur
CPU n'est pas ajouté au compteur du serveur.

Les champs Windows suivent les propriétés [temps CPU][cpu], [mémoire
résidente][rss] et [octets privés][private] de `System.Diagnostics.Process`.
Linux utilise les compteurs utilisateur/noyau et `VmRSS`/`VmHWM` décrits dans la
[documentation du noyau][proc].

## Rapports et reproductibilité

Un rapport JSON neuf est créé sous `target/performance/`. Il contient date,
architecture, modèle CPU, mémoire physique totale, versions Node/Rust, empreinte
du binaire, HEAD et état modifié du dépôt, options, résultats de chaque phase et
contrôles de redémarrage. Il ne contient ni mot de passe, ni secret de reprise,
ni réponse, ni chemin temporaire.

Le rapport ne remplace pas une révision Git reproductible : le dépôt peut avoir
des modifications non commitées, ce qui est indiqué. Comparer les empreintes du
binaire, des définitions et les options avant d'attribuer un écart au code.
Garder le même hôte, compiler en release et éviter d'autres charges lourdes. Ne
pas mélanger les résultats Windows et ceux de conteneurs Linux pour annoncer un
ratio entre RustSurvey et LimeSurvey.

## Résultats du 14 novembre 2026

Les rapports [file désactivée][immediate] et [file de huit places][bounded]
utilisent le même binaire optimisé, les mêmes empreintes de définition et les
mêmes options, sauf `queue`. Hôte : Windows x64, AMD Ryzen 7 7700X, seize
processeurs logiques, Node 24.15 et compilateur Rust 1.100.0. Cette version du
compilateur décrit la mesure ; le minimum du projet reste 1.97.1.

Les deux séries comportent chacune 27 000 tentatives de sauvegarde. Le tableau
indique les refus sur les 3 000 tentatives de chaque couple charge/clients. Les
autres colonnes sont les médianes des trois phases avec la file de huit places.
Le percentile affiché est donc une médiane de p95, pas un percentile recalculé
sur toutes les requêtes.

| Charge       | Clients | Refus file 0 | Refus file 8 | Sauvegardes/s file 8 | p95 en ms | CPU ms/succès |
| ------------ | ------- | ------------ | ------------ | -------------------- | --------- | ------------- |
| Court        | 1       | 0            | 0            | 255,9                | 4,90      | 2,906         |
| Court        | 2       | 0            | 0            | 251,6                | 34,77     | 2,484         |
| Court        | 8       | 2 834        | 0            | 280,5                | 53,53     | 2,375         |
| 28 types     | 1       | 0            | 0            | 203,3                | 5,92      | 3,438         |
| 28 types     | 2       | 0            | 0            | 198,3                | 49,66     | 3,219         |
| 28 types     | 8       | 2 856        | 0            | 232,2                | 58,44     | 3,000         |
| Avec défauts | 1       | 0            | 0            | 193,6                | 6,58      | 3,609         |
| Avec défauts | 2       | 0            | 0            | 195,9                | 50,35     | 3,297         |
| Avec défauts | 8       | 2 871        | 0            | 224,6                | 62,82     | 3,328         |

La file courte absorbe cette pointe à huit clients sans augmenter les deux
écritures actives : 9 000 sauvegardes réussissent, contre 439 sans attente. Le
temps CPU des requêtes refusées et les quantités de travail accepté diffèrent
donc fortement ; ce n'est pas une preuve d'accélération générale du moteur. Les
réponses et versions ont été relues après chaque phase et après arrêt et
redémarrage des six processus de scénario.

Mémoire résidente en Mio, avec la file de huit places :

| Charge       | 0 participation | 100   | 1 000 | Pic observé pendant les écritures | Pic sur la vie du processus |
| ------------ | --------------- | ----- | ----- | --------------------------------- | --------------------------- |
| Court        | 10,45           | 11,89 | 12,02 | 14,38                             | 28,25                       |
| 28 types     | 10,64           | 12,44 | 12,63 | 15,24                             | 28,32                       |
| Avec défauts | 10,74           | 12,39 | 12,79 | 15,35                             | 28,33                       |

Les participations de ce tableau sont des brouillons SQL créés puis laissés sans
requête active. Ce ne sont pas mille clients HTTP simultanés. Le pic sur la vie
du processus inclut démarrage et authentification ; il ne faut pas le remplacer
par le seul relevé au repos pour dimensionner un déploiement.

La dispersion reste visible dans les rapports bruts. Par exemple, le scénario
avec défauts à un client varie de 116,6 à 201,4 sauvegardes/s dans la série avec
attente, avec un p95 de 6,01 à 20,82 ms. L'origine de ce ralentissement n'est
pas établie par ce banc. Il mesure un bureau réel et une base qui grossit au fil
des phases, pas une machine isolée. Aucun gain CPU de quelques pourcents, ratio
face à LimeSurvey ou budget d'hébergement n'est déduit de ces chiffres.

## Suite de la mesure

Une comparaison LimeSurvey demandera des charges métier équivalentes, le même
hôte d'exécution, les ressources de PHP et de la base, des réponses vérifiées,
ainsi que les mêmes garanties de sauvegarde et de reprise. Les fichiers
statiques sont inclus dans le banc de participants ; HTTPS, les pièces jointes,
les gros volumes et les questionnaires complexes restent à mesurer. La référence
locale ne permet pas encore d'annoncer un gain global face à LimeSurvey.

[cpu]:
  https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.process.totalprocessortime?view=net-10.0
[rss]:
  https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.process.workingset64
[private]:
  https://learn.microsoft.com/en-us/dotNet/api/system.diagnostics.process.privatememorysize64?view=netframework-4.8.1
[proc]: https://docs.kernel.org/filesystems/proc.html
[immediate]: measurements/2026-11-14-write-queue-immediate.json
[bounded]: measurements/2026-11-14-write-queue-bounded.json
