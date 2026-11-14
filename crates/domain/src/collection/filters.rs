use super::{Answers, CellKind, Filter, PreparedQuestion, issue};
use crate::{QuestionKind, ValidationIssue, rules::EvaluationBudget};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

struct SelectionCell {
    key: String,
    row: String,
    checkbox: bool,
}

#[derive(Default)]
pub(super) struct SelectionLayout {
    codes: BTreeSet<String>,
    cells: Vec<SelectionCell>,
    ranks: Vec<String>,
}

impl SelectionLayout {
    fn new(question: &PreparedQuestion) -> Self {
        let mut layout = Self::default();
        if question.source.kind == QuestionKind::Ranking {
            layout.codes.extend(
                question
                    .source
                    .choices
                    .iter()
                    .map(|choice| choice.code.clone()),
            );
            layout
                .ranks
                .extend(question.cells.iter().map(|cell| cell.key.clone()));
            return layout;
        }
        layout.codes.extend(
            question
                .source
                .subquestions
                .iter()
                .filter(|row| row.scale == 0)
                .map(|row| row.code.clone()),
        );
        for cell in &question.cells {
            if cell.comment {
                continue;
            }
            let Some(row) = &cell.row else {
                continue;
            };
            layout.codes.insert(row.clone());
            layout.cells.push(SelectionCell {
                key: cell.key.clone(),
                row: row.clone(),
                checkbox: cell.kind == CellKind::Checkbox,
            });
        }
        layout
    }

    fn selected(
        &self,
        answers: &Answers,
        budget: &mut EvaluationBudget,
    ) -> Result<BTreeSet<String>, &'static str> {
        let mut selected = BTreeSet::new();
        for cell in &self.cells {
            budget.charge(1)?;
            let value = answers.get(&cell.key).map_or("", String::as_str);
            if !value.is_empty() && (!cell.checkbox || value == "1") {
                selected.insert(cell.row.clone());
            }
        }
        for key in &self.ranks {
            budget.charge(1)?;
            if let Some(value) = answers.get(key).filter(|value| self.codes.contains(*value)) {
                selected.insert(value.clone());
            }
        }
        Ok(selected)
    }
}

pub(super) fn prepare(questions: &mut [PreparedQuestion]) -> Result<(), Vec<ValidationIssue>> {
    let needed: BTreeSet<_> = questions
        .iter()
        .flat_map(|question| question.filters.iter().map(|filter| filter.code.clone()))
        .collect();
    if needed.is_empty() {
        return Ok(());
    }
    let layouts: BTreeMap<_, _> = questions
        .iter()
        .filter(|question| {
            needed.contains(&question.source.code)
                && crate::filtering::supports_source(question.source.kind)
        })
        .map(|question| {
            (
                question.source.code.clone(),
                Arc::new(SelectionLayout::new(question)),
            )
        })
        .collect();
    let direct: BTreeMap<_, _> = questions
        .iter()
        .map(|question| (question.source.code.clone(), question.filters.clone()))
        .collect();
    for question in questions {
        let mut pending = question.filters.clone();
        let mut seen = BTreeSet::new();
        let mut expanded = Vec::new();
        while let Some(mut filter) = pending.pop() {
            if !seen.insert((filter.code.clone(), filter.exclude)) {
                continue;
            }
            filter.selections = layouts.get(&filter.code).cloned().ok_or_else(|| {
                vec![issue(
                    &question.source.id,
                    "Source de filtre inconnue ou sans lignes de réponse.",
                )]
            })?;
            if let Some(ancestors) = direct.get(&filter.code) {
                pending.extend(ancestors.iter().cloned());
            }
            expanded.push(filter);
        }
        question.filters = expanded;
    }
    Ok(())
}

pub(super) type SelectionCache = BTreeMap<String, BTreeSet<String>>;

pub(super) fn apply_choices(
    question: &PreparedQuestion,
    answers: &mut Answers,
    cache: &mut SelectionCache,
    budget: &mut EvaluationBudget,
) -> Result<Option<usize>, &'static str> {
    use QuestionKind::{Ranking, SingleChoice};
    if question.filters.is_empty() || !matches!(question.source.kind, SingleChoice | Ranking) {
        return Ok(None);
    }
    let mut allowed = BTreeSet::new();
    let mut codes = BTreeSet::new();
    for choice in &question.source.choices {
        codes.insert(choice.code.as_str());
        if allows(&question.filters, &choice.code, answers, cache, budget)? {
            allowed.insert(choice.code.as_str());
        }
    }
    if question.source.kind == SingleChoice {
        if let Some(value) = answers.get(&question.source.code) {
            let filtered = codes.contains(value.as_str()) && !allowed.contains(value.as_str());
            if filtered {
                answers.remove(&question.source.code);
            }
        }
        return Ok(None);
    }
    normalize_ranking(question, answers, &codes, &allowed, budget)?;
    Ok(Some(allowed.len()))
}

fn normalize_ranking(
    question: &PreparedQuestion,
    answers: &mut Answers,
    codes: &BTreeSet<&str>,
    allowed: &BTreeSet<&str>,
    budget: &mut EvaluationBudget,
) -> Result<(), &'static str> {
    let mut retained = Vec::new();
    let mut removed = false;
    let mut unique = BTreeSet::new();
    for cell in &question.cells {
        budget.charge(1)?;
        let Some(value) = answers.get(&cell.key).filter(|value| !value.is_empty()) else {
            continue;
        };
        if !codes.contains(value.as_str()) {
            return Err("Choix de réponse inconnu.");
        }
        if !unique.insert(value) {
            return Err("Chaque choix doit occuper une seule position du classement.");
        }
        if allowed.contains(value.as_str()) {
            retained.push(value.clone());
        } else {
            removed = true;
        }
    }
    if removed {
        for (index, cell) in question.cells.iter().enumerate() {
            if let Some(value) = retained.get(index) {
                answers.insert(cell.key.clone(), value.clone());
            } else {
                answers.remove(&cell.key);
            }
        }
    } else if question.cells.iter().skip(allowed.len()).any(|cell| {
        answers
            .get(&cell.key)
            .is_some_and(|value| !value.is_empty())
    }) {
        return Err("Le classement ne doit pas contenir de position vide entre deux choix.");
    }
    Ok(())
}

pub(super) fn allows(
    filters: &[Filter],
    row: &str,
    answers: &Answers,
    cache: &mut SelectionCache,
    budget: &mut EvaluationBudget,
) -> Result<bool, &'static str> {
    for filter in filters {
        budget.charge(1)?;
        if !filter.selections.codes.contains(row) {
            continue;
        }
        if !cache.contains_key(&filter.code) {
            cache.insert(
                filter.code.clone(),
                filter.selections.selected(answers, budget)?,
            );
        }
        if cache[&filter.code].contains(row) == filter.exclude {
            return Ok(false);
        }
    }
    Ok(true)
}
