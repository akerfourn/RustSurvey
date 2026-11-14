//! Bounded expression compilation, evaluation and definition validation.
mod evaluate;
mod functions;
mod settings;
mod syntax;
mod text_functions;
mod value;

use crate::{Definition, Question, QuestionKind, ValidationIssue};
use std::collections::{BTreeMap, BTreeSet};

pub use syntax::{Expression, references};
pub use value::{EvaluationBudget, MAX_STRING_UNITS, Scalar};

pub fn fields(question: &Question) -> Vec<String> {
    use QuestionKind::*;
    let rows: Vec<_> = question
        .subquestions
        .iter()
        .filter(|row| row.scale == 0)
        .collect();
    let columns: Vec<_> = question
        .subquestions
        .iter()
        .filter(|row| row.scale == 1)
        .collect();
    let mut fields = Vec::new();
    match question.kind {
        Information => {}
        NumberMatrix | TextMatrix => {
            if rows.len().saturating_mul(columns.len()) > 2000 {
                return vec![String::new(); 2001];
            }
            for row in &rows {
                for column in &columns {
                    fields.push(format!("{}_{}_{}", question.code, row.code, column.code));
                }
            }
        }
        Ranking => {
            let count = question
                .choices
                .iter()
                .filter(|choice| choice.scale == 0)
                .count();
            let maximum = question
                .attributes
                .iter()
                .find(|attribute| attribute.name == "max_subquestions")
                .and_then(|attribute| attribute.value.parse::<usize>().ok())
                .filter(|value| *value > 0)
                .unwrap_or(count);
            for index in 1..=count.min(maximum) {
                fields.push(format!("{}_{index}", question.code));
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
            for row in &rows {
                let key = format!("{}_{}", question.code, row.code);
                if question.kind == DualScale {
                    fields.push(format!("{key}_0"));
                    fields.push(format!("{key}_1"));
                } else {
                    fields.push(key.clone());
                }
                if question.kind == MultipleChoiceComment {
                    fields.push(format!("{key}comment"));
                }
            }
        }
        _ => {
            fields.push(question.code.clone());
        }
    }
    if question.kind == SingleChoiceComment {
        fields.push(format!("{}_comment", question.code));
    }
    if question.other {
        fields.push(format!("{}_other", question.code));
        if question.kind == MultipleChoiceComment {
            fields.push(format!("{}_othercomment", question.code));
        }
    }
    fields
}

pub fn validate(definition: &Definition, previous: Option<&Definition>) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let questions: Vec<_> = definition
        .groups
        .iter()
        .flat_map(|group| &group.questions)
        .collect();
    let previous_sources: BTreeMap<_, _> = previous
        .into_iter()
        .flat_map(|definition| definition.groups.iter().flat_map(|group| &group.questions))
        .map(|question| (question.code.as_str(), question))
        .collect();
    let mut variables = BTreeMap::new();
    for question in &questions {
        for field in fields(question) {
            variables.insert(field, question.id.as_str());
        }
    }
    let mut edges: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut changed = BTreeSet::new();
    for (group_index, group) in definition.groups.iter().enumerate() {
        let old_group = previous.and_then(|old| old.groups.iter().find(|old| old.id == group.id));
        let group_changed = old_group.is_none_or(|old| old.relevance != group.relevance);
        let group_path = format!("groups[{group_index}].relevance");
        let group_refs = check(
            &group.relevance,
            &group_path,
            group_changed,
            &variables,
            &mut issues,
        );
        for (index, question) in group.questions.iter().enumerate() {
            let old = old_group
                .and_then(|group| group.questions.iter().find(|old| old.id == question.id));
            let rule_changed = old.is_none_or(|old| old.relevance != question.relevance);
            if group_changed || rule_changed {
                changed.insert(question.id.as_str());
            }
            let path = format!("groups[{group_index}].questions[{index}].relevance");
            let mut dependencies = check(
                &question.relevance,
                &path,
                rule_changed,
                &variables,
                &mut issues,
            );
            dependencies.extend(group_refs.iter().copied());
            let question_path = format!("groups[{group_index}].questions[{index}]");
            settings::validate(question, old, &question_path, &mut issues);
            for (row_index, row) in question.subquestions.iter().enumerate() {
                let row_changed = old
                    .and_then(|old| old.subquestions.iter().find(|item| item.id == row.id))
                    .is_none_or(|old| old.relevance != row.relevance);
                if row_changed {
                    changed.insert(question.id.as_str());
                }
                dependencies.extend(check(
                    &row.relevance,
                    &format!("{question_path}.subquestions[{row_index}].relevance"),
                    row_changed,
                    &variables,
                    &mut issues,
                ));
            }
            for (attribute_index, attribute) in question.attributes.iter().enumerate() {
                let attribute_changed = old.is_none_or(|old| !old.attributes.contains(attribute));
                let attribute_path = format!("{question_path}.attributes[{attribute_index}].value");
                if is_constraint(&attribute.name) && !attribute.value.trim().is_empty() {
                    // Validation reads the final snapshot; self-references are valid and do not affect ordering.
                    check(
                        &attribute.value,
                        &attribute_path,
                        attribute_changed,
                        &variables,
                        &mut issues,
                    );
                }
                if attribute.name == "equation" && question.kind == QuestionKind::Equation {
                    if attribute_changed {
                        changed.insert(question.id.as_str());
                    }
                    dependencies.extend(check(
                        &attribute.value,
                        &attribute_path,
                        attribute_changed,
                        &variables,
                        &mut issues,
                    ));
                }
                if ["array_filter", "array_filter_exclude"].contains(&attribute.name.as_str()) {
                    if attribute_changed {
                        changed.insert(question.id.as_str());
                    }
                    for code in attribute
                        .value
                        .split(';')
                        .map(str::trim)
                        .filter(|code| !code.is_empty())
                    {
                        let source = questions.iter().find(|item| item.code == code);
                        let source_changed = match (previous_sources.get(code), source) {
                            (Some(old), Some(current)) => {
                                old.id != current.id || old.kind != current.kind
                            }
                            (None, None) => false,
                            _ => true,
                        };
                        if let Some(source) =
                            source.filter(|source| crate::filtering::supports_source(source.kind))
                        {
                            dependencies.insert(source.id.as_str());
                        } else if (attribute_changed || source_changed) && issues.len() < 100 {
                            issues.push(ValidationIssue {
                                path: attribute_path.clone(),
                                message: format!(
                                    "Le filtre {code} doit désigner une question à lignes multiples ou un classement."
                                ),
                            });
                        }
                    }
                }
            }
            if question.kind == QuestionKind::Equation
                && !question
                    .attributes
                    .iter()
                    .any(|item| item.name == "equation" && !item.value.is_empty())
            {
                let expression = question
                    .title
                    .trim()
                    .strip_prefix('{')
                    .and_then(|text| text.strip_suffix('}'))
                    .unwrap_or(question.title.trim());
                let equation_changed =
                    old.is_none_or(|old| old.title != question.title || old.kind != question.kind);
                if equation_changed {
                    changed.insert(question.id.as_str());
                }
                dependencies.extend(check(
                    expression,
                    &format!("{question_path}.title"),
                    equation_changed,
                    &variables,
                    &mut issues,
                ));
            }
            edges.insert(&question.id, dependencies);
        }
    }
    for id in changed {
        if reaches(id, id, &edges, &mut BTreeSet::new()) {
            issues.push(ValidationIssue {
                path: "groups".to_owned(),
                message: "Les conditions créent un cycle de dépendances.".to_owned(),
            });
            break;
        }
    }
    issues.truncate(100);
    issues
}

pub(crate) fn is_constraint(name: &str) -> bool {
    matches!(
        name,
        "min_num_value_n"
            | "max_num_value_n"
            | "min_answers"
            | "max_answers"
            | "maximum_chars"
            | "em_validation_q"
            | "min_num_value"
            | "max_num_value"
            | "equals_num_value"
    )
}

fn check<'a>(
    source: &str,
    path: &str,
    strict: bool,
    variables: &BTreeMap<String, &'a str>,
    issues: &mut Vec<ValidationIssue>,
) -> BTreeSet<&'a str> {
    let mut dependencies = BTreeSet::new();
    let source = if source.trim_matches(syntax::is_whitespace).is_empty() {
        "1"
    } else {
        source
    };
    match references(source) {
        Ok(names) => {
            for name in names {
                let key = name.strip_suffix(".NAOK").unwrap_or(&name);
                if let Some(id) = variables.get(key) {
                    dependencies.insert(*id);
                } else if strict && issues.len() < 100 {
                    issues.push(ValidationIssue {
                        path: path.to_owned(),
                        message: format!("Variable inconnue : {name}."),
                    });
                }
            }
        }
        Err(message) if strict && issues.len() < 100 => issues.push(ValidationIssue {
            path: path.to_owned(),
            message: message.to_owned(),
        }),
        Err(_) => {}
    }
    dependencies
}

fn reaches<'a>(
    start: &'a str,
    node: &'a str,
    edges: &BTreeMap<&'a str, BTreeSet<&'a str>>,
    visited: &mut BTreeSet<&'a str>,
) -> bool {
    if !visited.insert(node) {
        return false;
    }
    edges.get(node).is_some_and(|dependencies| {
        dependencies
            .iter()
            .any(|dependency| *dependency == start || reaches(start, dependency, edges, visited))
    })
}
