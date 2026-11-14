use super::*;
use crate::{
    QuestionKind,
    rules::{EvaluationBudget, Scalar},
};

struct DefaultContext<'a> {
    language: &'a str,
    questions: Option<&'a BTreeSet<String>>,
    initialized: BTreeSet<String>,
}

pub(super) fn evaluate(
    runtime: &Questionnaire,
    answers: &Answers,
    submit: bool,
) -> Result<Snapshot, Vec<ValidationIssue>> {
    evaluate_context(
        runtime,
        answers,
        submit,
        &runtime.definition.language,
        None,
        true,
        &mut EvaluationBudget::default(),
    )
}

pub(super) fn evaluate_context(
    runtime: &Questionnaire,
    answers: &Answers,
    submit: bool,
    language: &str,
    initialize: Option<&BTreeSet<String>>,
    validate: bool,
    budget: &mut EvaluationBudget,
) -> Result<Snapshot, Vec<ValidationIssue>> {
    check_answers(runtime, answers)?;
    let mut effective = answers.clone();
    let mut relevant = BTreeSet::new();
    let mut active = BTreeSet::new();
    let mut visible_groups = BTreeSet::new();
    let mut selections = super::filters::SelectionCache::new();
    let mut defaults = DefaultContext {
        language,
        questions: initialize,
        initialized: BTreeSet::new(),
    };
    for index in &runtime.order {
        let question = &runtime.questions[*index];
        let result = apply(
            question,
            &mut effective,
            &mut active,
            budget,
            &mut selections,
            &mut defaults,
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
    defaults
        .initialized
        .retain(|key| effective.contains_key(key));
    let mut snapshot = Snapshot {
        answers: effective,
        relevant,
        groups,
        warnings: Vec::new(),
        initialized: defaults.initialized,
    };
    if !validate {
        return Ok(snapshot);
    }
    let mut errors = Vec::new();
    for question in &runtime.questions {
        if snapshot.relevant.contains(&question.source.id) {
            validation::validate(
                question,
                &active,
                &mut snapshot,
                submit,
                &runtime.definition,
                budget,
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
    selections: &mut super::filters::SelectionCache,
    defaults: &mut DefaultContext<'_>,
) -> Result<bool, &'static str> {
    for expression in &question.relevance {
        if !evaluate_rule(expression, answers, budget)?.truth() {
            for cell in &question.cells {
                answers.remove(&cell.key);
            }
            return Ok(false);
        }
    }
    if defaults
        .questions
        .is_some_and(|ids| ids.contains(&question.source.id))
    {
        defaults.initialized.extend(
            question
                .defaults
                .apply(answers, defaults.language, budget)?,
        );
    }
    if question.source.kind == QuestionKind::Language
        && answers
            .get(&question.source.code)
            .is_none_or(String::is_empty)
    {
        answers.insert(question.source.code.clone(), defaults.language.to_owned());
    }
    let rank_count = super::filters::apply_choices(question, answers, selections, budget)?;
    for (index, cell) in question.cells.iter().enumerate() {
        let mut enabled = true;
        if rank_count.is_some_and(|count| index >= count) {
            enabled = false;
        }
        for expression in &cell.relevance {
            if !evaluate_rule(expression, answers, budget)?.truth() {
                enabled = false;
            }
        }
        if let Some(row) = &cell.row {
            enabled &= super::filters::allows(&question.filters, row, answers, selections, budget)?;
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
        if enabled {
            active.insert(cell.key.clone());
        } else {
            answers.remove(&cell.key);
        }
    }
    super::exclusive::preserve_existing_selection(question, answers, active, &defaults.initialized);
    super::exclusive::apply(question, answers, active)?;
    super::exclusive::comments(question, answers, active);
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
