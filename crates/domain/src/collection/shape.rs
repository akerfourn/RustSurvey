use super::*;
use crate::QuestionKind;

pub(super) fn answer_count(
    question: &PreparedQuestion,
    cells: &[&Cell],
    answers: &Answers,
    count: usize,
) -> usize {
    if forms::checkbox(&question.source) {
        return cells
            .iter()
            .filter(|cell| answers.get(&cell.key).is_some_and(|value| value == "1"))
            .count();
    }
    if question.source.kind != QuestionKind::DualScale {
        return count;
    }
    let mut complete_rows = BTreeMap::new();
    for cell in cells {
        let Some(row) = &cell.row else {
            continue;
        };
        let complete = complete_rows.entry(row).or_insert(true);
        if answers.get(&cell.key).is_none_or(String::is_empty) {
            *complete = false;
        }
    }
    complete_rows.values().filter(|complete| **complete).count()
}

pub(super) fn is_unanswered(
    question: &PreparedQuestion,
    cells: &[&Cell],
    answers: &Answers,
    missing: bool,
    empty: bool,
) -> bool {
    use QuestionKind::*;
    if matches!(question.source.kind, MultipleChoice | MultipleChoiceComment) {
        return empty;
    }
    if forms::checkbox(&question.source) {
        let mut rows = BTreeSet::new();
        let mut answered = BTreeSet::new();
        for cell in cells {
            if let Some(row) = &cell.row {
                rows.insert(row);
                if answers.get(&cell.key).is_some_and(|value| value == "1") {
                    answered.insert(row);
                }
            }
        }
        return !rows.is_subset(&answered);
    }
    if question.source.kind == Ranking
        && ["min_answers", "max_answers"]
            .iter()
            .any(|name| !["", "0"].contains(&attribute(&question.source, name)))
    {
        return empty;
    }
    missing
}

pub(super) fn validate_ranking(
    cells: &[&Cell],
    answers: &Answers,
    values: &[&str],
) -> Result<(), &'static str> {
    let unique: BTreeSet<_> = values.iter().copied().collect();
    if unique.len() != values.len() {
        return Err("Chaque choix doit occuper une seule position du classement.");
    }
    let mut blank = false;
    for cell in cells {
        if answers.get(&cell.key).is_none_or(String::is_empty) {
            blank = true;
        } else if blank {
            return Err("Le classement ne doit pas contenir de position vide entre deux choix.");
        }
    }
    Ok(())
}
