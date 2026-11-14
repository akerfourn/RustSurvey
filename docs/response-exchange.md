# Échange des réponses LimeSurvey

L'import administrateur de LSR dispose maintenant d'une
[préparation privée et reprenable](response-imports.md). Les valeurs reçues et
leurs correspondances sont conservées en SQL ; leur conversion en participations
natives reste à réaliser. Le [bilan de collecte](imported-collection.md)
localise séparément les prérequis d'exécution de chaque révision importée.

## Export depuis une campagne native

1. Dans l'administration, ouvrir **Collecter des réponses**, puis **Consulter
   les participations**.
2. Fermer la campagne pour figer ses réponses.
3. Télécharger **Exporter la structure LSS** et **Exporter toutes les réponses
   (VV LimeSurvey)**.
4. Dans LimeSurvey 6.5.14, importer le LSS, activer le questionnaire, puis
   importer les réponses VV en conservant leur état de soumission.

L'export utilise la révision de la campagne, même si le questionnaire a depuis
été modifié. Les codes de questions permettent de retrouver les colonnes après
réattribution des identifiants par LimeSurvey. Les brouillons restent des
brouillons ; les soumissions portent leur dernière date d'envoi native en UTC.
La configuration temporelle de la cible doit donc interpréter ces dates en UTC.

Le LSS téléchargé depuis une campagne porte aussi son format groupe/question/
tout, la progression et le retour autorisé. Le téléchargement d'une simple
révision garde les valeurs natives par défaut. La politique d'invitation et les
moyens de reprise restent exclus de cette structure.

La migration 0006 conserve la langue initiale des nouvelles participations
séparément de leur langue courante. Pour une ancienne réponse, une définition
monolingue permet de retrouver cette langue sans ambiguïté. Si plusieurs langues
étaient possibles et que la langue initiale n'est pas connue, l'export VV est
refusé avec `initial_language_unavailable`. Le JSON natif conserve alors
explicitement cette absence ; aucune valeur historique n'est inventée.

Les chaînes vides restent distinctes des valeurs nulles. Espaces, apostrophes,
accents graves, Unicode, retours à la ligne et marqueurs VV littéraux sont
conservés dans les cas vérifiés. LimeSurvey peut normaliser la représentation
SQL des nombres et dates, par exemple `3` en `3.0000000000`.

Cet échange porte sur les données. Les UUID natifs, invitations, secrets,
checkpoint, ordre de navigation et ressources ne sont pas transférés. Les
identifiants numériques sont attribués par LimeSurvey. Réimporter le même VV
ajoute des réponses : l'idempotence et la continuité nécessiteront le protocole
de [transfert d'étude](study-migration.md).

Les restrictions du [profil d'export natif](authoring.md) s'appliquent : médias,
thèmes spécifiques et données non représentables bloquent l'échange.

Une [nouvelle campagne importée qualifiée](import-execution.md) exporte aussi
ses réponses en VV, avec ses correspondances source figées. Son LSS conserve les
identifiants et données de l'original, avec les conditions de la révision ; les
réglages hérités ne sont pas matérialisés dans ce fichier. Une langue initiale
inconnue bloque l'export même en monolingue. Ce parcours ne transfère aucune
participation historique.

## API et ressources

`GET /api/admin/runs/{id}/responses.vv` exige une session administrateur. Une
campagne ouverte produit `409 export_requires_closed_run`. Les réponses sont
privées et sans cache ; les secrets de participation ne sont jamais exportés.

Le serveur parcourt les identifiants par lots de 50, avec l'index `(run_id, id)`
de la migration 0006. Chaque document est ensuite lu et encodé séparément. Le
canal de téléchargement contient au plus un élément ; le client reçoit des
fragments de 64 Kio. Décodage JSON et encodage VV utilisent les travaux
bloquants avec le même budget de deux traitements que les imports.

Les buffers d'un document et d'une ligne sont limités à 8 Mio. La durée maximale
de production est de cinq minutes ; l'annulation du téléchargement arrête le
producteur. Une erreur, une interruption ou une limite atteinte fait échouer le
flux HTTP, plutôt que terminer normalement un fichier incomplet. Un export
interrompu doit être repris depuis le début ; un transfert reprenable reste à
réaliser pour les gros volumes.

## Lecture et écriture LSR hors collecte

L'adaptateur lit et écrit le XML de réponses du schéma 623. Il conserve les
colonnes déclarées, y compris inconnues, leurs valeurs lexicales et les champs
nuls ou vides. Une colonne absente n'est pas inventée. Le résultat JSON est une
table source ; cette lecture n'ajoute aucune participation en SQL.

```sh
cargo run -p rustsurvey-limesurvey --locked -- import-responses \
  responses.lsr responses.json
cargo run -p rustsurvey-limesurvey --locked -- export-responses \
  responses.json responses.lsr
cargo run -p rustsurvey-limesurvey --locked -- export-bindings \
  definition.json bindings.json 701
cargo run -p rustsurvey-limesurvey --locked -- export-vv \
  responses.json aliases.json responses.vv
```

Les fichiers de sortie doivent être nouveaux. `aliases.json` associe les noms de
colonnes de la table à leurs codes VV ; les colonnes sans alias conservent leur
nom. La bibliothèque expose aussi `VvEncoder` pour produire les en-têtes une
fois et traiter les lignes séparément.

Sur Unix, les fichiers produits par le CLI sont lisibles et modifiables
uniquement par leur propriétaire (`0600`). Les autres systèmes conservent les
droits d'accès du répertoire de destination.

Une table JSON utilise `languages`, `columns` et `rows`. Chaque ligne est un
objet dont les valeurs sont des chaînes ou `null`. Les nombres restent des
chaînes pour préserver leurs codes et leur représentation source.

Le parseur borne aussi le produit lignes × colonnes : un petit XML composé de
nombreuses lignes nulles ne doit pas créer un document énorme en mémoire. Les
sections inconnues, champs non déclarés, doublons, ambiguïtés et versions hors
623 sont refusés explicitement.

## Limites de la version de référence

La [préparation LSR](response-imports.md#conversion-réversible-des-cellules)
expose aussi une conversion réversible vers les clés de cellules natives. Elle
conserve les métadonnées et les représentations originales nécessaires au
retour, sans recalculer les réponses historiques ni créer de participations. La
publication et les droits de reprise restent à compléter.

Les tests réels ont confirmé des altérations dans le chemin LSR de LimeSurvey
6.5.14 : l'importeur convertit les chaînes vides en `NULL` et retire apostrophes
et accents graves. Son exporteur remplace `]]>` et écrit les retours chariot
dans des CDATA dont XML normalise les fins de ligne. L'export VV de RustSurvey
évite ces altérations dans les cas testés. Voir la
[décision 0009](decisions/0009-response-exchange.md).

Un fichier déjà altéré par sa source ne permet pas de reconstruire les valeurs
originales. Le protocole de migration devra conserver les compléments
nécessaires ; une archive LSA seule ne prouve pas un transfert sans perte.

Le [test de référence](limesurvey-reference.md) vérifie les colonnes et deux
réponses synthétiques sur deux études, avec des identifiants différents. Les
tests HTTP vérifient aussi pagination SQL, révision figée, confidentialité,
interruption et annulation du flux. Cela ne certifie pas tous les comportements
de formulaire ni les migrations en cours.
