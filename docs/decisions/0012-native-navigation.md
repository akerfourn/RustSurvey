# Décision 0012 — Présentation figée des campagnes natives

## Statut

Implémentée pour les campagnes `native-pages-v1` : un groupe par page, une
question par page ou tout le questionnaire sur une page, avec progression
facultative. Les 28 types et les règles du profil `native-forms-v1` restent
communs. Les études LimeSurvey importées restent bloquées en collecte.

## Décision et alternatives

La définition v3 décrit les questions et leurs cellules. La campagne fige sa
présentation, comme sa politique de participation :

```json
{
  "navigation": {
    "format": "question",
    "showProgress": false
  }
}
```

| Option                          | Intérêt                   | Limite                                  |
| ------------------------------- | ------------------------- | --------------------------------------- |
| Modifier la définition v3       | Un seul document          | Change un contrat existant              |
| Nouvelle définition v4          | Version explicite         | Conversion sans besoin sur les cellules |
| Réglages figés dans la campagne | Isole le parcours exécuté | Deux exports de structure à distinguer  |

Le troisième choix évite de modifier les révisions historiques. La migration
0009 ajoute une colonne texte nullable `runs.navigation`. Les anciennes lignes
gardent `NULL`, leur profil et leur comportement par groupe. Une création API
sans réglages conserve `native-forms-v1` ; l'interface envoie explicitement ses
choix et crée `native-pages-v1`. Aucun repli ne masque un profil inconnu ou une
combinaison profil/réglages incohérente.

Les réponses gardent les mêmes cellules, sans stockage par écran. Aucune
dépendance externe ni service supplémentaire n'est ajouté. La construction des
pages parcourt les groupes et questions ; la navigation ne garde pas de session
en mémoire côté serveur. Les mesures CPU/RAM comparées restent à réaliser.

## Curseur et validation

Le checkpoint ajoute `pageId` pour le nouveau profil : `g:<identifiant>` pour un
groupe, `q:<identifiant>` pour une question, `all` pour l'écran unique. Les
codes éditoriaux des questions ne servent pas de curseur. `groupId` reste
présent et doit correspondre à la page ; il est nul pour `all`.

Le serveur vérifie format, existence et correspondance du curseur. Une page
devenue non pertinente est remplacée par la première page visible. Il ne
supprime pas les calculs cachés : ils continuent d'être évalués, sans créer de
page. Si aucune question n'est visible, les modes groupe/question possèdent une
page `empty` permettant l'envoi ; le mode tout conserve `all`.

Réponses et curseur sont sauvegardés dans la même transaction. L'accusé et les
réessais idempotents incluent la page normalisée. L'absence de `pageId` reste
omise lors de la sérialisation des anciens profils, pour conserver notamment les
empreintes de commandes déjà reçues. Un ancien profil refuse un curseur de page
non nul.

Le navigateur valide les questions de la page avant de continuer. La validation
finale Rust porte toujours sur toutes les questions pertinentes, quel que soit
le curseur demandé. Le retour arrière est autorisé ; dans le nouveau profil, il
sauvegarde aussi les réponses et la page de destination. Un écran d'accueil
éventuel reste distinct du curseur des questions.

## Échange et preuves

L'export privé `GET /api/admin/runs/{id}/definition.lss` utilise la définition
figée et la présentation de cette campagne. L'export d'une simple révision garde
les valeurs natives par défaut : groupe et progression affichée. Les deux
chemins écrivent explicitement `allowprev=Y`.

La bibliothèque expose `export_native_with_navigation`. Le CLI accepte un
fichier JSON facultatif, borné à 1 Kio, après l'identifiant d'échange. Formats
et champs inconnus ou incomplets sont refusés avant création du fichier.

Le 14 novembre 2026, les vérifications couvrent :

- huit scénarios de pages communs aux moteurs Rust et TypeScript ;
- validation par question et contrôle final de tout le questionnaire ;
- sauvegarde, masquage, réessai, envoi et export des trois formats ;
- migration SQLite conservant anciens checkpoints et reçus ;
- migration 0009 et parcours sur PostgreSQL 17 et MariaDB 11.4 jetables ;
- reprise HTTP après arrêt complet, en modes objet SQL et fichiers locaux ;
- import réel des trois LSS dans LimeSurvey 6.5.14 et comparaison des options
  effectives `format`, `showprogress` et `allowprev`.

L'import réel prouve ces réglages, sans certifier tous les parcours participant.
Les contrôles de rendu au clavier et avec lecteur d'écran restent à compléter.
L'interdiction du retour, l'index, les délais, les ordres aléatoires et les
réglages effectifs d'études importées restent à implémenter. Les contextes
source ne sont pas appliqués automatiquement à ces campagnes natives.
