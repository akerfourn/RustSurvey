# Exemple de questionnaire

`demo.lss` est une fixture synthétique rédigée pour RustSurvey, sans données de
production ni copie de questionnaire amont. Elle utilise le schéma 623 et les
tables de traduction de la cible LimeSurvey 6.5.14.

Elle couvre huit questions, trois groupes, deux langues, des choix simples et
multiples, un nombre borné, une pertinence de groupe, une question
conditionnelle et un texte avec substitutions. Ses identifiants sont arbitraires
et fixes.

L’API sert exactement ce fichier et les tests Rust vérifient sa projection. Son
import et son comportement dans une instance LimeSurvey 6.5.14 restent à valider
; elle ne constitue pas une fixture de certification amont.
