//! Source defaults mapped to native cells without evaluating their values.
use crate::{Question, SurveySource};
use rustsurvey_domain::{
    Question as NativeQuestion, ValidationIssue,
    cells::{Coordinate, Role},
    defaults::{DefaultValue, MAX_DEFAULT_VALUES, QuestionDefaults, supports_cell},
};

/// Projects a whole question atomically; unsupported targets never produce partial defaults.
pub fn project_question_defaults(
    model: &SurveySource,
    source: &Question,
    target: &NativeQuestion,
) -> Result<Option<QuestionDefaults>, Vec<ValidationIssue>> {
    let rows: Vec<_> = model
        .default_values
        .iter()
        .filter(|value| value.question_id == source.id)
        .collect();
    if rows.is_empty() {
        return Ok(None);
    }
    let path = format!("source.questions.{}.defaults", source.id.0);
    if crate::native::layout::question_type(target.kind) != Some(source.source_type.as_str())
        || target.code != source.code
        || target.other != source.other
        || target.source.as_ref().is_none_or(|identity| {
            identity.id != source.id.0.to_string()
                || identity.kind.as_deref() != Some(source.source_type.as_str())
        })
    {
        return Err(vec![issue(
            &path,
            "Identité source de la question absente ou différente.",
        )]);
    }
    let Some(shared) = source.same_default else {
        return Err(vec![issue(
            &path,
            "Le réglage source same_default est absent ; aucun mode implicite n'est choisi.",
        )]);
    };
    let mut values = Vec::new();
    let mut issues = Vec::new();
    for row in rows {
        if issues.len() >= 100 {
            issues.truncate(100);
            break;
        }
        let row_path = format!("{path}.{}", row.id.0);
        let Some((coordinate, role)) = coordinate(model, source, target, row) else {
            issues.push(issue(
                &row_path,
                "Cible, échelle ou identité source du défaut non prise en charge.",
            ));
            continue;
        };
        for localized in &row.values {
            if values.len() == MAX_DEFAULT_VALUES {
                issues.push(issue(
                    &path,
                    "Maximum de 6400 valeurs par défaut par question.",
                ));
                issues.truncate(100);
                return Err(issues);
            }
            if localized.value.encode_utf16().count() > 4096 || localized.value.contains('\0') {
                issues.push(issue(
                    &row_path,
                    "Texte de défaut non représentable (maximum 4096 unités UTF-16).",
                ));
                continue;
            }
            values.push(DefaultValue {
                coordinate: coordinate.clone(),
                role,
                language: localized.language.clone(),
                value: localized.value.clone(),
            });
        }
        if issues.len() >= 100 {
            issues.truncate(100);
            break;
        }
    }
    if !issues.is_empty() {
        issues.truncate(100);
        return Err(issues);
    }
    Ok(Some(QuestionDefaults {
        same_for_all_languages: shared,
        values,
    }))
}

fn coordinate(
    model: &SurveySource,
    source: &Question,
    target: &NativeQuestion,
    value: &crate::DefaultValue,
) -> Option<(Coordinate, Role)> {
    if value.scale != 0 {
        return None;
    }
    let role = match value.special_type.as_str() {
        "" => Role::Answer,
        "other" if value.subquestion_id.is_none() => Role::Other,
        _ => return None,
    };
    let coordinate = match value.subquestion_id {
        None => Coordinate::Primary,
        Some(id) => {
            let row = model
                .subquestions
                .iter()
                .find(|row| row.id == id && row.parent_id == source.id && row.scale == 0)?;
            let candidates: Vec<_> = target
                .subquestions
                .iter()
                .filter(|candidate| {
                    candidate
                        .source
                        .as_ref()
                        .is_some_and(|identity| identity.id == id.0.to_string())
                })
                .collect();
            let [candidate] = candidates.as_slice() else {
                return None;
            };
            if candidate.scale != 0 || candidate.code != row.code {
                return None;
            }
            Coordinate::Subquestion {
                id: candidate.id.clone(),
                scale: None,
            }
        }
    };
    supports_cell(target, &coordinate, role).then_some((coordinate, role))
}

fn issue(path: &str, message: &str) -> ValidationIssue {
    ValidationIssue {
        path: path.into(),
        message: message.into(),
    }
}
