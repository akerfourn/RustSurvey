# Participants nombreux et requêtes espacées

## Objectif et résultat

Le [premier banc](performance.md) utilise jusqu'à huit clients qui sauvegardent
sans pause. Ce nombre décrit une concurrence de requêtes, pas une limite de
personnes pouvant remplir un questionnaire. Le nouveau banc distingue le nombre
de connexions ouvertes, la fréquence des écritures et les pointes synchronisées.

Sur le pilote Windows livré, 1 000 participants gardant chacun une connexion
ouverte et enregistrant toutes les 30 secondes fonctionnent sans refus pendant
la minute d'activité mesurée. La RAM résidente atteint 82,8 Mio en activité. Les
arrivées ou soumissions très groupées provoquent en revanche des refus
temporaires d'écriture. Ce résultat ne qualifie pas encore une ouverture massive
synchronisée.

## Reproduction

```sh
npm run bench:participants -- --package target/pilot/rustsurvey-win32-x64-Hjibmp
```

Le chemin désigne un paquet créé par `npm run package:pilot`, adapté à la
plateforme locale. Le banc contrôle les empreintes de tous ses fichiers puis
utilise son exécutable et ses ressources web sans les reconstruire. Il démarre
ses propres processus sur des ports locaux éphémères avec des bases SQLite et
objets fichiers temporaires. Il ne contacte aucun serveur existant.

Les valeurs par défaut sont `--clients 100,500,1000`, `--arrival-seconds 30`,
`--think-seconds 30`, `--cycles 2`, `--idle-seconds 5` et `--burst 1000`. Les
cohortes acceptent une à quatre tailles distinctes entre 1 et 5 000 ; les durées
d'arrivée et de réflexion vont de 1 à 60 secondes, les cycles de 1 à 4, la pause
de 1 à 30 secondes et la pointe de 0 à 5 000 participants. Une pointe de zéro
désactive ce scénario. Exemple de contrôle court du lanceur :

```sh
npm run bench:participants -- --package target/pilot/rustsurvey-win32-x64-Hjibmp --clients 10 --arrival-seconds 1 --think-seconds 1 --cycles 2 --idle-seconds 1 --burst 10
```

## Charge mesurée

Chaque cohorte utilise un processus et une base neufs. Le questionnaire public
contient les 28 types natifs sans fichiers, présentés sur une seule page avec le
profil `native-pages-v1`. Les identifiants et secrets synthétiques de reprise
sont distincts pour chaque participant.

1. Les arrivées sont réparties régulièrement sur 30 secondes. Chaque participant
   télécharge sans cache HTML, JavaScript et CSS, lit le questionnaire, puis
   crée sa réponse : cinq requêtes séquentielles, environ 253 Ko reçus au total.
2. Une pause de cinq secondes conserve les connexions HTTP/1.1 indépendantes. Le
   banc relève les sockets encore ouvertes, au lieu de confondre brouillons SQL
   et clients connectés.
3. Pendant 60 secondes, chaque participant sauvegarde puis soumet, avec 30
   secondes entre ses deux écritures. Les premiers envois sont répartis sur
   l'intervalle et leur ordre est permuté de façon déterministe. Le débit prévu
   est donc `participants / 30`, soit 33,3 écritures/s pour 1 000 personnes.
4. Chaque accusé est contrôlé. Toutes les réponses acceptées sont relues hors
   mesure, avec leur version, statut, dernière valeur et calcul serveur. Le
   processus est ensuite arrêté et redémarré ; chaque réponse est encore relue.

Les pauses n'envoient aucune requête. Aucun réessai immédiat ne masque les
refus. Si une écriture échoue, son opération reste en attente pour le prochain
créneau prévu du participant ; une réponse ne devient jamais soumise sur la
seule base d'une tentative. Les scénarios de pointe présentés ci-dessous
n'offrent qu'une tentative par participant.

Le rapport JSON sous `target/performance/` conserve les compteurs HTTP par type,
les latences des succès séparées de celles des refus, les octets transférés, le
retard de déclenchement, les connexions et les ressources du processus. Son état
devient `complete` seulement après les vérifications et redémarrages. Il ne
contient ni réponses ni secrets.

## Mesures du 14 novembre 2026

Hôte : Windows x64, Ryzen 7 7700X, 16 processeurs logiques, environ 63 Gio de
RAM, Node 24.15. Le paquet est compilé avec Rust 1.100.0, sans changer le
minimum 1.97.1 du projet. Empreinte SHA-256 de l'exécutable :

```text
673c9c2156e7cf57b70903ea79cb9835a092fd9539ab8075e24d0701d436b9bd
```

Le [rapport des cohortes et de la pointe d'arrivée][paced] utilise exactement le
paquet livré. Ce binaire diffère de celui du premier banc ; on ne peut pas
attribuer leur écart de mémoire au seul nombre de connexions.

Mesures pendant la minute de sauvegarde/soumission :

| Participants connectés | Écritures acceptées | Écritures/s | p95 en ms | CPU, % d'un cœur | Pic RAM, Mio |
| ---------------------- | ------------------- | ----------- | --------- | ---------------- | ------------ |
| 100                    | 200 / 200           | 3,33        | 9,34      | 0,68             | 22,73        |
| 500                    | 1 000 / 1 000       | 16,67       | 4,17      | 5,31             | 50,12        |
| 1 000                  | 2 000 / 2 000       | 33,33       | 4,08      | 11,25            | 82,78        |

Les 1 650 arrivées réparties et les 3 300 écritures réussissent sans refus. Les
1 650 réponses sont soumises, puis vérifiées avant et après redémarrage. Les
connexions restent toutes ouvertes à la fin des pauses et de l'activité. Le
retard de déclenchement p95 est inférieur à 14 ms pendant l'activité.

Pendant les arrivées, le CPU moyen vaut respectivement 1,47 %, 6,16 % et 11,51 %
d'un cœur ; le pic RAM observé vaut 22,50, 49,55 et 82,12 Mio. Pendant les
pauses de cinq secondes, aucun temps CPU supplémentaire n'est relevé à la
résolution du compteur. Cela ne prouve pas un coût strictement nul.

Le pic RAM système sur toute la vie des processus vaut respectivement 32,02,
50,12 et 82,78 Mio. Il inclut le démarrage et l'authentification administrateur.
Le résultat antérieur sous 30 Mio ne constituait donc pas un plafond général. Un
Mio représente 1 048 576 octets ; 82,78 Mio représentent environ 86,80 Mo.

## Pointes synchronisées

Deux charges distinctes rendent la saturation visible, sans réessai :

| Cas                                       | Résultat utile        | Refus d'écriture | Pic RAM observé, Mio |
| ----------------------------------------- | --------------------- | ---------------- | -------------------- |
| 1 000 arrivées déclenchées ensemble       | 15 réponses créées    | 985              | 92,08                |
| 1 000 soumissions prévues sur une seconde | 314 réponses soumises | 686              | 82,79                |

Pour les arrivées groupées, les 3 000 téléchargements statiques et les 1 000
lectures du questionnaire réussissent. Les refus concernent la création des
réponses. Les 15 créations acceptées sont relues après redémarrage.

Le [rapport des soumissions groupées][grouped] utilise la commande suivante :

```sh
npm run bench:participants -- --package target/pilot/rustsurvey-win32-x64-Hjibmp --clients 1000 --arrival-seconds 30 --think-seconds 1 --cycles 1 --idle-seconds 1 --burst 0
```

Les 1 000 participants entrent d'abord sans refus. Les envois prévus sur une
seconde se terminent en 1,05 seconde : environ 299 succès/s, p95 des succès
45,06 ms, CPU moyen 62,50 % d'un cœur. Les 686 autres réponses restent des
brouillons de version zéro ; les 1 000 états sont vérifiés après redémarrage.

Tous les refus de ces deux pointes sont `503 write_busy`. Le serveur garde deux
écritures actives et huit places d'attente selon la
[décision 0013](decisions/0013-bounded-write-admission.md). Cette protection
borne la charge admise ; elle ne garantit pas l'acceptation d'une vague de
clics. Avant un pilote avec arrivées massives synchronisées, il reste à
qualifier la reprise après ces refus et l'absorption des pointes.

## Limites d'interprétation

- Il s'agit d'une exécution courte par cohorte, avec répartition régulière,
  complétée par deux pointes. Ce n'est pas une endurance de plusieurs heures.
- Le client Node utilise une connexion par personne et télécharge les vrais
  fichiers, sans exécuter le navigateur. Le modèle initial simplifie le parcours
  web et omet notamment la relecture de reprise après navigation. Il ne mesure
  ni rendu, ni saisie, ni mémoire du navigateur.
- Client et serveur partagent le même ordinateur, en HTTP local. HTTPS, reverse
  proxy, latence réseau, débit Internet et cache navigateur restent à qualifier.
  Le CPU du client et de l'auxiliaire est exclu des compteurs serveur, mais leur
  activité peut influencer la mesure.
- La RAM est celle du processus serveur. Les buffers TCP du noyau et les autres
  services de l'hôte ne sont pas intégralement compris dans ce chiffre. Les pics
  échantillonnés toutes les 100 ms peuvent manquer un maximum instantané ; le
  rapport conserve aussi le pic système sur la vie du processus.
- La campagne est publique, sans cookie de restriction ni invitation. Les
  médias, pièces jointes, S3, PostgreSQL/MariaDB et les questionnaires plus
  lourds ne sont pas mesurés ici. Aucun ratio face à LimeSurvey ni capacité
  garantie d'un hébergement n'en est déduit.

[paced]: measurements/2026-11-14-paced-participants.json
[grouped]: measurements/2026-11-14-grouped-submissions.json
