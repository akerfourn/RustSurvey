# Accès et participations multiples

## Réglages disponibles

Dans **Collecter des réponses**, choisir l’accès avant d’ouvrir la campagne. La
politique est figée avec la campagne ; les campagnes existantes conservent leur
accès public après la migration 0005.

| Choix                                 | Comportement                                                                       |
| ------------------------------------- | ---------------------------------------------------------------------------------- |
| Public, participations multiples      | Chaque démarrage crée une réponse indépendante.                                    |
| Public, limitation par navigateur     | Un cookie posé après envoi bloque les nouveaux démarrages et envois du navigateur. |
| Invitation, réponses indépendantes    | Le jeton autorise un nombre fixé de soumissions, ou un nombre illimité.            |
| Invitation avec reprise               | Le même lien retrouve un seul brouillon, puis sa consultation après envoi.         |
| Invitation avec reprise et correction | Le même lien retrouve une réponse modifiable, y compris après son envoi.           |

Pour créer les liens, ouvrir **Consulter les participations**, puis
**Invitations privées**. Chaque lien possède une limite et des dates de validité
facultatives. Copier le nouveau lien avant de quitter la page : son secret ne
sera pas réaffiché. La liste montre les compteurs et permet de révoquer une
invitation, même après fermeture de la campagne. Aucun courriel n’est envoyé.

Un lien avec reprise correspond toujours à une seule réponse. Sa correction
remplace cette réponse ; elle n’en crée pas une autre. **Modifier cette
réponse** déverrouille l’écran ; la première sauvegarde peut la remettre en
brouillon. Un nouvel envoi vérifie toutes les obligations et règles. Aucune
ancienne version des réponses n’est conservée dans cet incrément.

## Référence fonctionnelle LimeSurvey 6.5.14

Le relevé porte sur le commit de référence
`ac3c44744e2c3769b3941c41e50b68419423ce67`, en complément du [manuel des
paramètres participants][manual]. Il ne remplace pas un test de parcours dans
cette version.

- `usecookie` : hors questionnaire à jetons, un cookie `LS_<sid>_STATUS` marque
  une réponse terminée et refuse une nouvelle entrée. Il expire après 365 jours.
  Voir [le contrôle d’entrée][entry-cookie] et [la pose après
  soumission][completion].
- Les jetons portent `usesleft`, `completed`, `validfrom` et `validuntil`. Le
  parcours normal exige un jeton utilisable et non terminé. La soumission
  décrémente le compteur et termine le jeton selon les usages restants. Voir
  [les critères de validité][token] et [la soumission][submit].
- `tokenanswerspersistence` retrouve une réponse par jeton dans un questionnaire
  non anonymisé. `alloweditaftercompletion` permet soit de modifier cette
  réponse, soit d’en ajouter une lorsque la persistance est désactivée ou les
  réponses anonymisées. Le parcours éditable vérifie les dates du jeton, sans
  imposer un compteur positif. Voir [la sélection et reprise][entry-resume].

RustSurvey reprend ces choix fonctionnels avec des limites explicites :
`maxUses: 1`, un entier supérieur, ou `null` pour les réponses indépendantes
illimitées. La correction d’une réponse liée ne consomme pas d’usage
supplémentaire. Les instants sont des secondes Unix UTC ; la validité suit
`validFrom <= maintenant < validUntil`. La borne de fin est donc exclusive,
alors que le critère SQL LimeSurvey est inclusif. Cette différence doit être
convertie explicitement lors d’un futur transfert.

## Garantie des contrôles

Le cookie ne vérifie pas une personne. Effacement, navigation privée, autre
appareil ou requêtes simultanées sans cookie permettent de le contourner. Il ne
contient ni réponse ni secret de reprise et utilise `HttpOnly`, `SameSite=Lax`,
une durée de 365 jours et `Secure` selon la configuration HTTPS. Un lien privé
existant reste utilisable pour consulter sa réponse.

Une invitation limite les soumissions **par jeton**, même avec plusieurs
appareils ou des requêtes concurrentes. Elle ne prouve pas l’identité : son
détenteur peut la transmettre. Aucun contrôle par IP ni empreinte du navigateur
n’est ajouté. L’API vérifie les mêmes règles que l’interface.

Créer ou sauvegarder un brouillon ne consomme pas d’usage. Un envoi invalide,
une transaction annulée ou un conflit de version n’en consomme pas non plus. Le
dernier usage ne peut être attribué qu’à une seule soumission. Les autres
brouillons restent consultables, mais ne peuvent plus être enregistrés avec le
jeton épuisé. Le réessai d’un envoi déjà accepté rend son accusé sans décompter
une seconde fois.

## Reprise, secrets et confidentialité

Le secret d’invitation contient 256 bits aléatoires. SQL ne garde que son
empreinte SHA-256. Le lien utilise un fragment `#/runs/.../invite/...`, absent
des requêtes HTTP ; l’API le reçoit dans `X-Participation-Token`. Aucun secret
n’est enregistré dans `localStorage`, les métadonnées de liste ou un export de
réponses. HTTPS reste nécessaire hors développement local.

**Réponses indépendantes** : aucune clé SQL ne relie invitation et réponse. Un
second secret protège chaque réponse. Le lien privé de reprise contient les deux
droits ; le droit d’invitation reste nécessaire pour sauvegarder ou envoyer.
Révoquer l’invitation bloque ces écritures, sans supprimer le droit distinct de
consulter une réponse déjà créée. Un autre jeton valide peut autoriser son
envoi, sans construire une association persistante.

**Reprise par invitation** : SQL relie explicitement invitation et réponse. Le
jeton sert aussi de droit de reprise. Sa révocation ou son expiration bloque
lectures et écritures du participant ; l’administrateur conserve ses accès. Ce
mode ne convient pas à une promesse de réponses non reliées aux invitations.

L’absence de clé dans le premier mode ne certifie pas l’anonymat complet. Dates
de réponse, journaux d’infrastructure et modalités de distribution des liens
doivent aussi être étudiés. Les listes nominatives, attributs invités,
inscription publique, envois de courriels et anonymisation temporelle LimeSurvey
restent hors de cet incrément. La collecte des imports reste bloquée ; aucun
réglage source n’est converti silencieusement.

## API et stockage

La création de campagne accepte `participationPolicy` :

```json
{
  "expectedRevision": 1,
  "participationPolicy": {
    "mode": "invitation",
    "persistence": true,
    "allowEditAfterSubmit": false
  }
}
```

Les autres modes sont `{ "mode": "open" }` et `{ "mode": "browserOnce" }`. Une
politique absente conserve le mode `open`. L’édition après envoi exige
`persistence: true`.

| Méthode  | Route                                             | Accès                            |
| -------- | ------------------------------------------------- | -------------------------------- |
| POST/GET | `/api/admin/runs/{id}/invitations`                | Administrateur ; CSRF pour créer |
| POST     | `/api/admin/runs/{runId}/invitations/{id}/revoke` | Administrateur et CSRF           |

La création reçoit `id` UUID, `token` hexadécimal de 64 caractères, `maxUses`
(défaut 1, `null` pour illimité), `validFrom` et `validUntil` facultatifs. Le
navigateur conserve la même commande lors d’un réessai après perte de connexion.
Réutiliser l’UUID avec un autre contenu produit un conflit.

La liste est paginée par 50 invitations, avec au plus 10 000 par campagne. Une
limite finie doit être comprise entre 1 et 100 000. Le corps de création est
limité à 4 Kio. Les opérations utilisent les limites et droits habituels de la
[collecte](collection.md).

Les démarrages, décomptes et révocations utilisent la même ligne de campagne
pour ordonner leurs transactions. Quota, réponse et reçu idempotent sont
enregistrés ensemble. Aucun service supplémentaire ni état participant attaché à
un processus n’est nécessaire. Ce choix privilégie la cohérence ; son coût reste
à mesurer sur une charge documentée.

Les tests couvrent cookies, droits, quotas, concurrence, dates, révocation,
corrections et migrations SQLite. `npm run test:storage` vérifie aussi les
invitations, compteurs et reprises après arrêt complet du serveur. Le parcours
optionnel multi-bases doit encore être exécuté sur PostgreSQL/MariaDB jetables.

[manual]: https://www.limesurvey.org/manual/Surveys_-_introduction
[entry-cookie]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/controllers/survey/SurveyIndex.php#L405-L420
[completion]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/SurveyRuntimeHelper.php#L1295-L1301
[token]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/models/Token.php#L452-L468
[submit]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/helpers/frontend_helper.php#L365-L416
[entry-resume]:
  https://github.com/LimeSurvey/LimeSurvey/blob/ac3c44744e2c3769b3941c41e50b68419423ce67/application/controllers/survey/SurveyIndex.php#L572-L623
