//! Versioned defaults tied to stable cells, independently of execution and exchange aliases.
pub mod template;
use crate::{
    Question, QuestionKind, ValidationIssue,
    cells::{Coordinate, Role},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MAX_DEFAULT_VALUES: usize = 6400;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionDefaults {
    pub same_for_all_languages: bool,
    pub values: Vec<DefaultValue>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefaultValue {
    pub coordinate: Coordinate,
    pub role: Role,
    pub language: String,
    pub value: String,
}

pub fn supports_cell(question: &Question, coordinate: &Coordinate, role: Role) -> bool {
    use QuestionKind::*;
    match (coordinate, role) {
        (Coordinate::Primary, Role::Answer) => matches!(
            question.kind,
            ShortText
                | LongText
                | HugeText
                | Number
                | Date
                | SingleChoice
                | Dropdown
                | RatingFive
                | YesNo
                | Gender
                | SingleChoiceComment
        ),
        (Coordinate::Primary, Role::Other) => {
            question.other && matches!(question.kind, SingleChoice | Dropdown)
        }
        (Coordinate::Subquestion { id, scale: None }, Role::Answer) => {
            matches!(
                question.kind,
                MultipleChoice
                    | MultipleChoiceComment
                    | MultipleText
                    | MultipleNumbers
                    | ArrayFive
                    | ArrayTen
                    | ArrayYesNo
                    | ArrayIncrease
                    | ArrayChoice
                    | ArrayColumn
            ) && question
                .subquestions
                .iter()
                .any(|row| row.id == *id && row.scale == 0)
        }
        _ => false,
    }
}

pub(crate) fn validate(question: &Question, path: &str) -> Vec<ValidationIssue> {
    let Some(defaults) = &question.defaults else {
        return Vec::new();
    };
    let mut issues = Vec::new();
    if defaults.values.len() > MAX_DEFAULT_VALUES {
        issues.push(issue(
            format!("{path}.defaults"),
            "Maximum de 6400 valeurs par défaut par question.",
        ));
    }
    let mut seen = BTreeSet::new();
    let mut languages = BTreeSet::new();
    for (index, value) in defaults.values.iter().take(MAX_DEFAULT_VALUES).enumerate() {
        let path = format!("{path}.defaults.values[{index}]");
        if !supports_cell(question, &value.coordinate, value.role) {
            issues.push(issue(
                path.clone(),
                "Cellule incompatible avec les valeurs par défaut prises en charge.",
            ));
        }
        if !seen.insert((&value.coordinate, value.role, &value.language)) {
            issues.push(issue(
                path.clone(),
                "Valeur par défaut dupliquée pour cette cellule et cette langue.",
            ));
        }
        languages.insert(&value.language);
        if !crate::validation::valid_language(&value.language)
            || value.value.encode_utf16().count() > 4096
            || value.value.contains('\0')
        {
            issues.push(issue(
                path,
                "Langue ou texte de valeur par défaut invalide (maximum 4096 unités UTF-16).",
            ));
        }
        if issues.len() >= 100 {
            break;
        }
    }
    if languages.len() > 32 {
        issues.push(issue(
            format!("{path}.defaults"),
            "Maximum de 32 langues de valeurs par défaut.",
        ));
    }
    issues.truncate(100);
    issues
}

fn issue(path: String, message: &str) -> ValidationIssue {
    ValidationIssue {
        path,
        message: message.to_owned(),
    }
}
