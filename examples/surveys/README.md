# Exemple de questionnaire

`demo.lss` est une fixture synthétique rédigée pour RustSurvey, sans données de
production ni copie de questionnaire amont. Elle utilise le schéma 623 et les
tables de traduction de la cible LimeSurvey 6.5.14.

Elle couvre douze questions, quatre groupes, deux langues, des choix simples et
multiples, un nombre borné, une pertinence de groupe, des questions
conditionnelles et des textes avec substitutions. Ses identifiants sont
arbitraires et fixes.

Le groupe « Les tableaux » couvre F par lignes, H par colonnes et `:` à cases à
cocher. Il contient une condition sur une cellule H. Les cellules sont
facultatives et les codes F `01` et `02` permettent de vérifier leur
conservation. Les textes d’accueil et de fin sont localisés. Les attributs de
largeur, préférences d’édition et minuterie inactive exercent les cas non
bloquants.

L’API sert exactement ce fichier et les tests Rust vérifient sa projection. Son
import et son comportement dans une instance LimeSurvey 6.5.14 restent à valider
; elle ne constitue pas une fixture de certification amont.

## Démonstration des 28 types sans fichiers

`controls.lss` est une seconde fixture synthétique rédigée pour RustSurvey, sans
contenu de production ni copie de questionnaire amont. Elle regroupe les 28
types autres que le téléversement : 28 questions, 3 groupes, 2 langues et 52
cellules locales. Le bouton **Essayer les 28 types** la charge via
`GET /api/surveys/controls.lss` ; le fichier peut aussi être sélectionné à la
main.

Elle exerce les tableaux fixes et à deux échelles, matrices texte/numériques,
commentaires, autre, classement, langue, curseurs et une équation simple. Le
choix `PICK` filtre `FILTERED` ; le critère B dépend aussi de `GATE`. Le
résultat `TOTAL` vaut `sum(NUMBER, 2)` et se recalcule après modification du
nombre.

Toutes les questions sont facultatives ; la précision autre devient requise si
cette option est choisie. Les tests contrôlent la structure via Rust et l’API.
Le parcours HTTP et le moteur réels ont aussi été exercés, y compris filtrage,
classement invalide et recalcul. Le rendu DOM et l’import dans une instance
LimeSurvey 6.5.14 restent à vérifier ; cette fixture ne certifie pas
l’équivalence générale des types.
