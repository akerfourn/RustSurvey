use super::{Answers, PreparedQuestion};
use std::collections::BTreeSet;

pub(super) fn preserve_existing_selection(
    question: &PreparedQuestion,
    answers: &mut Answers,
    active: &BTreeSet<String>,
    initialized: &BTreeSet<String>,
) {
    if initialized.is_empty() || question.exclusive.codes.is_empty() {
        return;
    }
    let selected = question.cells.iter().any(|cell| {
        !cell.comment
            && question.exclusive.codes.contains(code(cell))
            && active.contains(&cell.key)
            && !initialized.contains(&cell.key)
            && answers
                .get(&cell.key)
                .is_some_and(|value| !value.is_empty())
    });
    if !selected {
        return;
    }
    for cell in &question.cells {
        if initialized.contains(&cell.key) && question.exclusive.codes.contains(code(cell)) {
            answers.remove(&cell.key);
        }
    }
}

pub(super) fn apply(
    question: &PreparedQuestion,
    answers: &mut Answers,
    active: &mut BTreeSet<String>,
) -> Result<(), &'static str> {
    let options = &question.exclusive;
    if options.codes.is_empty() {
        return Ok(());
    }
    let primary: Vec<_> = question.cells.iter().filter(|cell| !cell.comment).collect();
    let selected: Vec<_> = primary
        .iter()
        .filter(|cell| {
            active.contains(&cell.key)
                && options.codes.contains(code(cell))
                && answers
                    .get(&cell.key)
                    .is_some_and(|value| !value.is_empty())
        })
        .collect();
    if selected.len() > 1 {
        return Err("Plusieurs lignes exclusives sont renseignées simultanément.");
    }
    let mut chosen = selected.first().map(|cell| cell.key.as_str());
    let mut automatic = false;
    if chosen.is_none() && options.automatic {
        let others: Vec<_> = primary
            .iter()
            .filter(|cell| !options.codes.contains(code(cell)))
            .collect();
        let all_answered = !others.is_empty()
            && others.iter().all(|cell| {
                !active.contains(&cell.key)
                    || answers
                        .get(&cell.key)
                        .is_some_and(|value| !value.is_empty())
            });
        if all_answered {
            chosen = primary
                .iter()
                .find(|cell| active.contains(&cell.key) && options.codes.contains(code(cell)))
                .map(|cell| cell.key.as_str());
            if let Some(key) = chosen {
                answers.insert(key.to_owned(), "Y".into());
                automatic = true;
            }
        }
    }
    let Some(chosen) = chosen else {
        return Ok(());
    };
    for cell in &question.cells {
        let selection = if cell.comment {
            cell.key.strip_suffix("comment").unwrap_or(&cell.key)
        } else {
            &cell.key
        };
        if selection != chosen {
            active.remove(&cell.key);
            answers.remove(&cell.key);
        } else if automatic && cell.comment {
            answers.remove(&cell.key);
        }
    }
    Ok(())
}

fn code(cell: &super::Cell) -> &str {
    cell.row.as_deref().unwrap_or("other")
}

pub(super) fn comments(
    question: &PreparedQuestion,
    answers: &mut Answers,
    active: &mut BTreeSet<String>,
) {
    if question.source.kind != crate::QuestionKind::MultipleChoiceComment {
        return;
    }
    for cell in question.cells.iter().filter(|cell| cell.comment) {
        let selection = cell.key.strip_suffix("comment").unwrap_or(&cell.key);
        let selected = answers
            .get(selection)
            .is_some_and(|value| !value.is_empty());
        let enabled = match super::attribute(&question.source, "commented_checkbox") {
            "" | "checked" => selected,
            "unchecked" => !selected,
            _ => true,
        };
        if !enabled {
            active.remove(&cell.key);
            answers.remove(&cell.key);
        }
    }
}
