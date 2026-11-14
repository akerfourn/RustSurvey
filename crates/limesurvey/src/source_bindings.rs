use crate::{
    Error, FieldBinding, NativeExportError, SourceId, SurveySource,
    native::{bindings, layout::question_type},
};
use rustsurvey_domain::{Definition, Question, SourceReference, ValidationIssue};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBindings {
    pub survey_id: String,
    pub fields: Vec<FieldBinding>,
    pub ignored_columns: Vec<String>,
    pub issues: Vec<ValidationIssue>,
    pub issue_count: usize,
}

fn alias(source: Option<&SourceReference>) -> Option<SourceId> {
    source
        .and_then(|source| source.id.parse().ok())
        .map(SourceId)
}

fn matches(question: &Question, source: &crate::Question, survey: &SurveySource) -> bool {
    if source.code != question.code
        || question_type(question.kind) != Some(source.source_type.as_str())
        || source.other != question.other
    {
        return false;
    }
    let rows: Vec<_> = survey
        .subquestions
        .iter()
        .filter(|row| row.parent_id == source.id)
        .collect();
    let answers: Vec<_> = survey
        .answers
        .iter()
        .filter(|answer| answer.question_id == source.id)
        .collect();
    rows.len() == question.subquestions.len()
        && answers.len() == question.choices.len()
        && question.subquestions.iter().all(|item| {
            rows.iter().any(|row| {
                Some(row.id) == alias(item.source.as_ref())
                    && row.code == item.code
                    && row.scale == item.scale
            })
        })
        && question.choices.iter().all(|item| {
            answers.iter().any(|answer| {
                Some(answer.id) == alias(item.source.as_ref())
                    && answer.code == item.code
                    && answer.scale == item.scale
            })
        })
}

/// Bind an imported definition by its preserved source identities, never by coincident codes.
pub fn source_bindings(
    definition: &Definition,
    source: &SurveySource,
) -> Result<SourceBindings, Error> {
    let survey_id = source
        .survey_id
        .ok_or(Error::InvalidMetadata("source survey ID"))?;
    let mut result = SourceBindings {
        survey_id: survey_id.0.to_string(),
        fields: Vec::new(),
        ignored_columns: Vec::new(),
        issues: Vec::new(),
        issue_count: 0,
    };
    let mut seen = BTreeSet::new();
    for group in &definition.groups {
        let group_id = alias(group.source.as_ref());
        for question in &group.questions {
            let original = source.questions.iter().find(|item| {
                Some(item.id) == alias(question.source.as_ref()) && Some(item.group_id) == group_id
            });
            let Some(original) = original
                .filter(|original| matches(question, original, source) && seen.insert(original.id))
            else {
                result.issues.push(ValidationIssue {
                    path: question.id.clone(),
                    message: "Source identities or cell structure do not match the archived LSS."
                        .into(),
                });
                continue;
            };
            if original.source_type == "X" {
                result.ignored_columns.push(format!(
                    "{}X{}X{}",
                    survey_id.0, original.group_id.0, original.id.0
                ));
            }
            match bindings::question_bindings(
                question,
                survey_id.0,
                original.group_id.0,
                original.id.0,
            ) {
                Ok(fields) => result.fields.extend(fields),
                Err(NativeExportError::Unsupported(issues)) => result.issues.extend(issues),
                Err(_) => result.issues.push(ValidationIssue {
                    path: question.id.clone(),
                    message: "Source cell coordinates are invalid.".into(),
                }),
            }
        }
    }
    if seen.len() != source.questions.len() {
        result.issues.push(ValidationIssue {
            path: "questions".into(),
            message: "Some archived source questions do not have a unique native identity.".into(),
        });
    }
    if let Err(NativeExportError::Unsupported(issues)) = bindings::unique(&result.fields) {
        result.issues.extend(issues);
        result.fields.clear();
    }
    result.issue_count = result.issues.len();
    result.issues.truncate(128);
    Ok(result)
}
