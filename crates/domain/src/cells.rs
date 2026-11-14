//! Stable cell coordinates; editorial codes are aliases, never cell identities.
use crate::{Question, QuestionKind, ValidationIssue};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Coordinate {
    Primary,
    Subquestion { id: String, scale: Option<u32> },
    Matrix { row: String, column: String },
    Rank { position: u32 },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Answer,
    Comment,
    Other,
    OtherComment,
    Calculation,
    Language,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Address {
    pub question_id: String,
    pub coordinate: Coordinate,
    pub role: Role,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub address: Address,
    pub runtime_key: String,
}

fn push(
    cells: &mut Vec<Cell>,
    question: &Question,
    coordinate: Coordinate,
    role: Role,
    key: String,
) {
    cells.push(Cell {
        address: Address {
            question_id: question.id.clone(),
            coordinate,
            role,
        },
        runtime_key: key,
    });
}

/// Enumerate cells of a validated question in runtime order, without external-format identifiers.
pub fn for_question(question: &Question) -> Result<Vec<Cell>, ValidationIssue> {
    use QuestionKind::*;
    let rows: Vec<_> = question
        .subquestions
        .iter()
        .filter(|item| item.scale == 0)
        .collect();
    let columns: Vec<_> = question
        .subquestions
        .iter()
        .filter(|item| item.scale == 1)
        .collect();
    let invalid = || ValidationIssue {
        path: question.id.clone(),
        message: "Cell layout is unsupported or exceeds 2000 cells.".into(),
    };
    if question.subquestions.len() > 200
        || question.choices.len() > 200
        || matches!(question.kind, FileUpload | Unsupported)
        || (matches!(question.kind, NumberMatrix | TextMatrix)
            && rows.len().saturating_mul(columns.len()) > 2000)
    {
        return Err(invalid());
    }
    let mut cells = Vec::new();
    match question.kind {
        Information => {}
        NumberMatrix | TextMatrix => {
            for row in &rows {
                for column in &columns {
                    push(
                        &mut cells,
                        question,
                        Coordinate::Matrix {
                            row: row.id.clone(),
                            column: column.id.clone(),
                        },
                        Role::Answer,
                        format!("{}_{}_{}", question.code, row.code, column.code),
                    );
                }
            }
        }
        Ranking => {
            let count = question
                .choices
                .iter()
                .filter(|item| item.scale == 0)
                .count();
            let maximum = question
                .attributes
                .iter()
                .find(|item| item.name == "max_subquestions")
                .and_then(|item| item.value.parse::<usize>().ok())
                .filter(|value| *value > 0)
                .unwrap_or(count);
            for position in 1..=count.min(maximum) {
                push(
                    &mut cells,
                    question,
                    Coordinate::Rank {
                        position: position as u32,
                    },
                    Role::Answer,
                    format!("{}_{position}", question.code),
                );
            }
        }
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
        | DualScale => {
            for row in rows {
                let key = format!("{}_{}", question.code, row.code);
                if question.kind == DualScale {
                    for scale in 0..=1 {
                        push(
                            &mut cells,
                            question,
                            Coordinate::Subquestion {
                                id: row.id.clone(),
                                scale: Some(scale),
                            },
                            Role::Answer,
                            format!("{key}_{scale}"),
                        );
                    }
                } else {
                    let coordinate = Coordinate::Subquestion {
                        id: row.id.clone(),
                        scale: None,
                    };
                    push(
                        &mut cells,
                        question,
                        coordinate.clone(),
                        Role::Answer,
                        key.clone(),
                    );
                    if question.kind == MultipleChoiceComment {
                        push(
                            &mut cells,
                            question,
                            coordinate,
                            Role::Comment,
                            format!("{key}comment"),
                        );
                    }
                }
            }
        }
        _ => {
            let role = match question.kind {
                Equation => Role::Calculation,
                Language => Role::Language,
                _ => Role::Answer,
            };
            push(
                &mut cells,
                question,
                Coordinate::Primary,
                role,
                question.code.clone(),
            );
        }
    }
    if question.kind == SingleChoiceComment {
        push(
            &mut cells,
            question,
            Coordinate::Primary,
            Role::Comment,
            format!("{}_comment", question.code),
        );
    }
    if question.other {
        push(
            &mut cells,
            question,
            Coordinate::Primary,
            Role::Other,
            format!("{}_other", question.code),
        );
        if question.kind == MultipleChoiceComment {
            push(
                &mut cells,
                question,
                Coordinate::Primary,
                Role::OtherComment,
                format!("{}_othercomment", question.code),
            );
        }
    }
    Ok(cells)
}
