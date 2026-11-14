use super::{NativeExportError, layout::Layout};
use rustsurvey_domain::{
    Question, ValidationIssue,
    cells::{Address, Coordinate, Role},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FieldBinding {
    pub address: Address,
    pub runtime_key: String,
    pub column: String,
}

fn code<'a>(question: &'a Question, id: &str) -> Result<&'a str, NativeExportError> {
    question
        .subquestions
        .iter()
        .find(|row| row.id == id)
        .map(|row| row.code.as_str())
        .ok_or_else(|| crate::Error::InvalidStructure.into())
}

pub(super) fn build(layout: &Layout<'_>) -> Result<Vec<FieldBinding>, NativeExportError> {
    let mut result = Vec::new();
    for question in &layout.questions {
        result.extend(question_bindings(
            question.source,
            layout.survey_id,
            question.group_id,
            question.id,
        )?);
    }
    unique(&result)?;
    Ok(result)
}

pub(crate) fn unique(bindings: &[FieldBinding]) -> Result<(), NativeExportError> {
    let mut columns = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for binding in bindings {
        if !columns.insert(&binding.column) || !keys.insert(&binding.runtime_key) {
            return Err(NativeExportError::Unsupported(vec![ValidationIssue {
                path: binding.address.question_id.clone(),
                message: "Ambiguous response column or expression alias.".into(),
            }]));
        }
    }
    Ok(())
}

pub(crate) fn question_bindings(
    question: &Question,
    survey: u64,
    group: u64,
    id: u64,
) -> Result<Vec<FieldBinding>, NativeExportError> {
    let mut result = Vec::new();
    let base = format!("{survey}X{group}X{id}");
    let cells = rustsurvey_domain::cells::for_question(question)
        .map_err(|issue| NativeExportError::Unsupported(vec![issue]))?;
    for cell in cells {
        let mut column = base.clone();
        match &cell.address.coordinate {
            Coordinate::Primary => {}
            Coordinate::Subquestion { id, scale } => {
                column.push_str(code(question, id)?);
                if let Some(scale) = scale {
                    column.push_str(&format!("#{scale}"));
                }
            }
            Coordinate::Matrix { row, column: other } => {
                column.push_str(code(question, row)?);
                column.push('_');
                column.push_str(code(question, other)?);
            }
            Coordinate::Rank { position } => column.push_str(&position.to_string()),
        }
        match cell.address.role {
            Role::Comment => column.push_str("comment"),
            Role::Other => column.push_str("other"),
            Role::OtherComment => column.push_str("othercomment"),
            _ => {}
        }
        result.push(FieldBinding {
            address: cell.address,
            runtime_key: cell.runtime_key,
            column,
        });
    }
    Ok(result)
}
