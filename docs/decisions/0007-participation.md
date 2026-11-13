# Décision 0007 — Contrôle des participations multiples

## Statut

Retenue pour les campagnes natives. Les choix, le relevé des sources 6.5.14 et
les limites sont décrits dans [Accès et participations](../participation.md). La
migration de participants LimeSurvey reste à implémenter et à vérifier.

## Décisions

- Figer une politique typée par campagne : public, cookie, invitations avec ou
  sans reprise liée et correction après soumission.
- Conserver le mode public des campagnes existantes via une migration
  versionnée. Ne pas modifier les documents ou révisions historiques.
- Consommer un usage lors de la première soumission valide, dans la transaction
  de la réponse et de son reçu. La même réponse liée corrigée ne consomme pas
  d’usage supplémentaire.
- Sérialiser décomptes, ouverture, fermeture et révocation par la ligne campagne
  déjà utilisée pour les écritures. Le dernier usage possède un seul gagnant.
- Garder une limite indépendante par jeton, facultativement illimitée, au lieu
  de déduire implicitement son effet d’un compteur épuisé et d’un booléen.
- Distinguer le droit d’écrire avec une invitation du droit de lire une réponse
  indépendante. Ne stocker un lien invitation/réponse que si la reprise liée a
  été choisie explicitement.
- Utiliser les primitives déjà présentes : Web Crypto, SHA-256, SQL et dates UTC
  injectables. Aucune nouvelle dépendance ou service n’est ajouté.

## Conséquences

Le cookie est une commodité contournable, sans preuve d’identité. Les jetons
apportent un décompte serveur atomique, mais restent transmissibles. La reprise
liée permet des corrections explicites et leur révocation ; elle ajoute une
association incompatible avec des réponses promises non reliées.

Les réponses indépendantes demandent deux secrets dans leur lien privé pour
reprendre les écritures. Le serveur n’offre pas de récupération des secrets
perdus. Les listes ne réaffichent pas les liens d’invitation.

Les transactions d’une campagne restent sérialisées ; les coûts CPU/RAM et le
débit ne sont pas annoncés améliorés sans mesure. La portabilité réelle exige
des tests sur les trois bases et la compatibilité LimeSurvey des parcours dans
la version cible.
