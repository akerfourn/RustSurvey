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
