//! Bounded syntax/reference validation. Evaluation remains in the client until collection exists.
mod syntax;

use crate::{Definition, Question, QuestionKind, ValidationIssue};
use std::collections::{BTreeMap, BTreeSet};

pub use syntax::references;

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
            // Existing subquestion rules and equations participate in cycle detection, even when read-only.
            for row in &question.subquestions {
                dependencies.extend(check(&row.relevance, &path, false, &variables, &mut issues));
            }
            for attribute in &question.attributes {
                if attribute.name == "equation" || attribute.name == "em_validation_q" {
                    dependencies.extend(check(
                        &attribute.value,
                        &path,
                        false,
                        &variables,
                        &mut issues,
                    ));
                }
                if ["array_filter", "array_filter_exclude"].contains(&attribute.name.as_str()) {
                    for code in attribute.value.split(';').map(str::trim) {
                        if let Some(source) = questions.iter().find(|item| item.code == code) {
                            dependencies.insert(source.id.as_str());
                        }
                    }
                }
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
