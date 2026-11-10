# Fixtures de compatibilité

`minimal-623.lss` est une fixture synthétique écrite pour RustSurvey à partir de
la structure documentée du format XML. Elle n’est pas un export certifié par une
instance LimeSurvey. Aucun fichier ni donnée amont n’a été copié.

Elle couvre deux langues, deux questions, une sous-question, des options à codes
conservant leurs zéros initiaux, l’obligatoire souple, un champ vide, un champ
absent et une extension opaque avec espace de noms.

Les variantes invalides sont construites explicitement par les tests. La
certification de l’échange avec une instance 6.5.14 reste distincte de ces tests
unitaires. Toute mise à jour de la fixture doit être intentionnelle et relue ;
elle n’est jamais régénérée automatiquement.
