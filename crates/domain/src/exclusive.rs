//! Exclusive rows use source codes; answer keys remain an execution concern.
use crate::{Question, QuestionKind};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default)]
pub struct ExclusiveOptions {
    pub codes: BTreeSet<String>,
    pub automatic: bool,
}

impl ExclusiveOptions {
    pub fn parse(question: &Question) -> Result<Self, &'static str> {
        let mut source = "";
        let mut automatic = "";
        let mut seen = BTreeSet::new();
        for attribute in &question.attributes {
            if !["exclude_all_others", "exclude_all_others_auto"].contains(&attribute.name.as_str())
            {
                continue;
            }
            if !seen.insert(&attribute.name) {
                return Err("Une option d’exclusivité est déclarée plusieurs fois.");
            }
            if attribute
                .language
                .as_deref()
                .is_some_and(|language| !language.is_empty())
            {
                return Err("Les options d’exclusivité ne peuvent pas être localisées.");
            }
            if attribute.name == "exclude_all_others" {
                source = &attribute.value;
            } else {
                automatic = &attribute.value;
            }
        }
        if !["", "0", "1"].contains(&automatic) {
            return Err("La sélection exclusive automatique doit valoir 0 ou 1.");
        }
        let codes: BTreeSet<String> = source
            .split(';')
            .map(str::trim)
            .filter(|code| !code.is_empty())
            .map(str::to_owned)
            .collect();
        if codes.is_empty() && automatic != "1" {
            return Ok(Self::default());
        }
        use QuestionKind::*;
        if !matches!(
            question.kind,
            ArrayFive
                | ArrayTen
                | ArrayYesNo
                | ArrayIncrease
                | ArrayChoice
                | MultipleChoice
                | MultipleChoiceComment
                | MultipleNumbers
                | MultipleText
        ) {
            return Err("Ce type de question ne prend pas en charge les lignes exclusives.");
        }
        let multiple = matches!(question.kind, MultipleChoice | MultipleChoiceComment);
        if automatic == "1" && !multiple {
            return Err("La sélection exclusive automatique exige une question à choix multiples.");
        }
        for code in &codes {
            let other = multiple && question.other && code == "other";
            if !other
                && !question
                    .subquestions
                    .iter()
                    .any(|row| row.scale == 0 && &row.code == code)
            {
                return Err("Un code de ligne exclusive est inconnu.");
            }
        }
        Ok(Self {
            automatic: automatic == "1" && codes.len() == 1 && !source.trim().contains(';'),
            codes,
        })
    }
}
