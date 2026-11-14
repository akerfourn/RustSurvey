# Essayer la version pilote de RustSurvey

## Périmètre de cette livraison

Ce pilote permet de créer un questionnaire natif, ouvrir une campagne figée,
distribuer son lien, enregistrer et reprendre des réponses puis exporter LSS et
VV. Il utilise le binaire Rust et un client web compilé, sur la même origine.
Rust et Node ne sont pas nécessaires sur la machine qui exécute le paquet.

La collecte couvre les 28 types sans fichiers et les règles actuellement
qualifiées : pertinences, validations, filtres, exclusivités, calculs et valeurs
par défaut prises en charge. Une fonction incompatible bloque l'ouverture avec
un diagnostic. Le pilote n'annonce pas la parité complète avec LimeSurvey.

Les questionnaires et leurs aperçus sont publics. Les réponses, originaux
importés et téléchargements administratifs restent privés. L'installation
possède un seul compte administrateur. Les restrictions d'invitation portent sur
la participation ; elles ne rendent pas les définitions confidentielles.

Restent hors du pilote : fichiers participants, médias dans la collecte, quotas
et aléatoire avancés, expressions ou réglages non pris en charge, courriels
automatiques et transfert des anciennes participations. Les images de l'éditeur
et des aperçus sont disponibles séparément. Les imports LSS exécutables doivent
satisfaire leur profil explicite ; les LSR restent une préparation privée. La
reprise exacte des études en cours est différée.

## Démarrage du paquet local

Utiliser un paquet construit pour le même système et la même architecture. Sous
Linux, les bibliothèques système doivent être compatibles avec la machine de
construction. Le paquet contient un binaire natif ; il ne s'agit pas d'un
exécutable universel.

1. Copier `.env.example` en `.env` dans le dossier du paquet.
2. Renseigner `ADMIN_PASSWORD` avec un mot de passe privé d'au moins 12
   caractères. Il initialise seulement le premier compte.
3. Depuis ce dossier, lancer le serveur.

PowerShell :

```powershell
Copy-Item .env.example .env
# Renseigner ADMIN_PASSWORD dans .env avant de lancer le serveur.
.\rustsurvey-server.exe
```

Linux ou macOS :

```sh
cp .env.example .env
# Renseigner ADMIN_PASSWORD dans .env avant de lancer le serveur.
./rustsurvey-server
```

Ouvrir `http://127.0.0.1:3000/`, puis **Administration**. Le compte initial est
`admin`, avec le mot de passe choisi. `Ctrl+C` arrête le serveur. Les mêmes
questionnaires, réponses et sessions reviennent au prochain démarrage.

La configuration locale écoute seulement la boucle locale et autorise un cookie
HTTP local. Pour tester depuis un autre poste, utiliser un hébergement HTTPS
avec adresse d'écoute explicite et `ADMIN_COOKIE_SECURE=true`. Ne pas exposer
directement cette configuration locale sur Internet.

## Recette courte

1. Dans l'administration, importer la définition native
   `examples/collecte-simple.json`. L'exemple `28-types.json` permet ensuite
   d'explorer les autres contrôles.
2. Dans **Collecter des réponses**, choisir une présentation et **Public —
   limiter à un envoi par navigateur**, puis ouvrir la campagne sur la version
   enregistrée.
3. Ouvrir le lien de participation. Répondre Oui à `GATE`, saisir `24` pour
   `AGE`, choisir une réponse et écrire un court texte dans `NOTE`. Le calcul
   `SCORE` doit donner `25`.
4. Sauvegarder, conserver le lien privé de reprise puis arrêter le serveur. Le
   relancer, reprendre la réponse et la soumettre. Revenir au lien public avec
   le même navigateur doit signaler la limitation après cet envoi.
5. Dans l'administration, consulter les participations, fermer la campagne et
   télécharger les réponses VV et la structure LSS.

La limitation par cookie peut être contournée en changeant de navigateur. Pour
un contrôle serveur, ouvrir une autre campagne **Sur invitation — reprendre la
même réponse**, puis créer un lien limité à une utilisation. Les invitations
sont distribuées manuellement dans ce pilote. Les liens privés restent des
droits d'accès : ne pas les partager avec d'autres personnes.

## Données et hébergement

Le paquet local conserve SQLite et les objets dans `.data/`. Une sauvegarde
complète conserve ce dossier entier, serveur arrêté. Ne pas remplacer ce dossier
par celui d'un autre paquet. Les données ne sont jamais incluses dans la
commande de préparation d'un nouveau paquet.

Les trois pilotes SQLite, PostgreSQL et MariaDB sont compilés. Une installation
sans disque persistant utilise `DATABASE_URL` vers une base gérée et
`OBJECT_STORE_BACKEND=s3`, avec les paramètres S3 privés documentés dans le
dépôt. Les fichiers statiques du paquet sont des ressources du déploiement, sans
données d'étude. Leur racine est `STATIC_ASSETS_DIRECTORY=web`.

Le déploiement distant, TLS, sauvegardes restaurables et conditions de
l'hébergement doivent être vérifiés sur la cible choisie avant une collecte
réelle. Aucun compte Clever Cloud ou service HDS distant n'est qualifié par le
test local. Une seule instance doit appliquer les migrations au déploiement.

## Préparation et preuves

Depuis le dépôt avec ses outils de build installés :

```sh
npm run package:pilot
```

La commande construit le client et Rust en release, avec les trois pilotes SQL.
Elle crée un dossier neuf dans `target/pilot/`, sans écraser les précédents.
Elle copie seulement le binaire, le build web, ce guide, une configuration sans
mot de passe et deux exemples synthétiques. Aucun `.env`, objet, compte ou base
de travail n'est copié.

Avant d'annoncer la réussite, la commande exécute le paquet sur des données
temporaires : ressources web et cache, contrôles d'accès, campagnes natives et
importées qualifiées, invitations, réponses, reprises après arrêt complet,
LSS/VV et préparation LSR. `manifest.json` contient les empreintes des fichiers,
la plateforme, la révision Git et l'état des modifications locales. Il ne
constitue pas une signature de distribution.

La recette HTTP est automatisée ; le contrôle visuel et les clics dans un
navigateur restent à réaliser. Les tests locaux n'établissent pas une
compatibilité générale ni une aptitude de toutes les enquêtes à la production.
