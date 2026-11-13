use super::*;
use crate::{
    QuestionKind,
    rules::{EvaluationBudget, Scalar},
};

pub(super) fn evaluate(
    runtime: &Questionnaire,
    answers: &Answers,
    submit: bool,
) -> Result<Snapshot, Vec<ValidationIssue>> {
    check_answers(runtime, answers)?;
    let mut effective = answers.clone();
    let mut relevant = BTreeSet::new();
    let mut active = BTreeSet::new();
    let mut visible_groups = BTreeSet::new();
    let mut budget = EvaluationBudget::default();
    for index in &runtime.order {
        let question = &runtime.questions[*index];
        let result = apply(
            question,
            &mut effective,
            &mut active,
            &mut budget,
            &runtime.definition.language,
        );
        match result {
            Ok(true) => {
                relevant.insert(question.source.id.clone());
                if attribute(&question.source, "hidden") != "1" {
                    visible_groups.insert(question.group_id.clone());
                }
            }
            Ok(false) => {}
            Err(message) => return Err(vec![issue(&question.source.id, message)]),
        }
    }
    check_answers(runtime, &effective)?;
    let groups = runtime
        .definition
        .groups
        .iter()
        .filter(|group| visible_groups.contains(&group.id))
        .map(|group| group.id.clone())
        .collect();
    let mut snapshot = Snapshot {
        answers: effective,
        relevant,
        groups,
        warnings: Vec::new(),
    };
    let mut errors = Vec::new();
    for question in &runtime.questions {
        if snapshot.relevant.contains(&question.source.id) {
            validation::validate(
                question,
                &active,
                &mut snapshot,
                submit,
                &runtime.definition,
                &mut budget,
                &mut errors,
            );
        }
        if errors.len() >= 100 {
            break;
        }
    }
    if !errors.is_empty() {
        errors.truncate(100);
        return Err(errors);
    }
    Ok(snapshot)
}

fn check_answers(runtime: &Questionnaire, answers: &Answers) -> Result<(), Vec<ValidationIssue>> {
    let mut bytes = 0_usize;
    for (key, value) in answers {
        if !runtime.fields.contains_key(key) {
            return Err(vec![issue("answers", "Cellule de réponse inconnue.")]);
        }
        bytes = bytes.saturating_add(key.len()).saturating_add(value.len());
        if value.encode_utf16().count() > 65_536 || bytes > MAX_ANSWER_BYTES {
            return Err(vec![issue("answers", "Réponses trop volumineuses.")]);
        }
    }
    Ok(())
}

pub(super) fn evaluate_rule(
    expression: &Expression,
    answers: &Answers,
    budget: &mut EvaluationBudget,
) -> Result<Scalar, &'static str> {
    expression.evaluate_with_budget(
        &|name| Scalar::String(answers.get(variable(name)).cloned().unwrap_or_default()),
        budget,
    )
}

fn apply(
    question: &PreparedQuestion,
    answers: &mut Answers,
    active: &mut BTreeSet<String>,
    budget: &mut EvaluationBudget,
    language: &str,
) -> Result<bool, &'static str> {
    for expression in &question.relevance {
        if !evaluate_rule(expression, answers, budget)?.truth() {
            for cell in &question.cells {
                answers.remove(&cell.key);
            }
            return Ok(false);
        }
    }
    if question.source.kind == QuestionKind::Language
        && answers
            .get(&question.source.code)
            .is_none_or(String::is_empty)
    {
        answers.insert(question.source.code.clone(), language.to_owned());
    }
    for cell in &question.cells {
        let mut enabled = true;
        for expression in &cell.relevance {
            if !evaluate_rule(expression, answers, budget)?.truth() {
                enabled = false;
            }
        }
        if let Some(row) = &cell.row {
            for filter in &question.filters {
                budget.charge(1)?;
                let field = format!("{}_{row}", filter.code);
                if filter.selections.contains(&field)
                    && answers.get(&field).is_some_and(|value| value == "Y") == filter.exclude
                {
                    enabled = false;
                }
            }
        }
        if cell.other
            && matches!(
                question.source.kind,
                QuestionKind::SingleChoice | QuestionKind::Dropdown
            )
        {
            enabled &= answers
                .get(&question.source.code)
                .is_some_and(|value| value == "-oth-");
        }
        if cell.comment && question.source.kind == QuestionKind::MultipleChoiceComment {
            let selection = cell.key.strip_suffix("comment").unwrap_or(&cell.key);
            let selected = answers
                .get(selection)
                .is_some_and(|value| !value.is_empty());
            match attribute(&question.source, "commented_checkbox") {
                "" | "checked" => enabled &= selected,
                "unchecked" => enabled &= !selected,
                _ => {}
            }
        }
        if enabled {
            active.insert(cell.key.clone());
        } else {
            answers.remove(&cell.key);
        }
    }
    if let Some(expression) = &question.equation {
        let value = evaluate_rule(expression, answers, budget)?;
        let text = match value {
            Scalar::Boolean(value) => value.to_string(),
            value => value.text(),
        };
        answers.insert(question.source.code.clone(), text);
    }
    Ok(true)
}
