use super::*;
use crate::{
    Mandatory, QuestionKind,
    rules::{EvaluationBudget, Scalar},
};

pub(super) fn validate(
    question: &PreparedQuestion,
    active: &BTreeSet<String>,
    snapshot: &mut Snapshot,
    submit: bool,
    definition: &Definition,
    budget: &mut EvaluationBudget,
    errors: &mut Vec<ValidationIssue>,
) {
    if matches!(
        question.source.kind,
        QuestionKind::Information | QuestionKind::Equation
    ) {
        return;
    }
    let cells: Vec<_> = question
        .cells
        .iter()
        .filter(|cell| active.contains(&cell.key))
        .collect();
    let mut values = Vec::new();
    let mut text_values = Vec::new();
    let mut numbers = Vec::new();
    let mut unanswered = false;
    for cell in &cells {
        let value = snapshot.answers.get(&cell.key).map_or("", String::as_str);
        if !cell.comment && !cell.other && value.is_empty() {
            unanswered = true;
        }
        if value.is_empty() {
            continue;
        }
        text_values.push(value);
        if !cell.comment {
            values.push(value);
        }
        if let Err(message) = validate_value(question, cell, value, definition) {
            errors.push(issue(&cell.key, message));
        }
        if cell.kind == CellKind::Number
            && let Ok(number) = Scalar::String(value.to_owned()).numeric()
        {
            numbers.push(number);
        }
    }
    let answer_count =
        super::shape::answer_count(question, &cells, &snapshot.answers, values.len());
    unanswered = super::shape::is_unanswered(
        question,
        &cells,
        &snapshot.answers,
        unanswered,
        values.is_empty(),
    );
    if question.source.kind == QuestionKind::Ranking
        && let Err(message) = super::shape::validate_ranking(&cells, &snapshot.answers, &values)
    {
        errors.push(issue(&question.source.id, message));
    }
    if submit && unanswered {
        match question.source.mandatory {
            Mandatory::Strict => errors.push(issue(
                &question.source.id,
                "Cette question est obligatoire.",
            )),
            Mandatory::Soft => snapshot.warnings.push(issue(
                &question.source.id,
                "Il est recommandé de répondre à cette question.",
            )),
            Mandatory::No => {}
        }
    }
    if submit
        && question.source.other
        && matches!(
            question.source.kind,
            QuestionKind::SingleChoice | QuestionKind::Dropdown
        )
        && snapshot
            .answers
            .get(&question.source.code)
            .is_some_and(|value| value == "-oth-")
        && attribute(&question.source, "other_comment_mandatory") != "0"
        && snapshot
            .answers
            .get(&format!("{}_other", question.source.code))
            .is_none_or(String::is_empty)
    {
        errors.push(issue(&question.source.id, "Précisez la réponse « autre »."));
    }
    if !submit {
        return;
    }
    for (name, expression) in &question.constraints {
        let result =
            snapshot::evaluate_rule(expression, &snapshot.answers, budget).and_then(|result| {
                if name == "maximum_chars" {
                    check_constraint(name, &result, &text_values, &numbers, answer_count)
                } else {
                    check_constraint(name, &result, &values, &numbers, answer_count)
                }
            });
        if let Err(message) = result {
            errors.push(issue(&question.source.id, message));
        }
    }
}

pub(super) fn validate_value(
    question: &PreparedQuestion,
    cell: &Cell,
    value: &str,
    definition: &Definition,
) -> Result<(), &'static str> {
    match cell.kind {
        CellKind::Number => {
            let number = Scalar::String(value.to_owned())
                .numeric()
                .map_err(|_| "Un nombre valide est attendu.")?;
            if attribute(&question.source, "num_value_int_only") == "1" && number.fract() != 0.0 {
                return Err("Un nombre entier est attendu.");
            }
            forms::validate_number(question, number)?;
        }
        CellKind::Date if !is_date(value) => return Err("Une date valide est attendue."),
        CellKind::Multiple if value != "Y" => return Err("Choix de réponse inconnu."),
        CellKind::Checkbox if !["0", "1"].contains(&value) => {
            return Err("Choix de réponse inconnu.");
        }
        CellKind::Choice => {
            let allowed = match question.source.kind {
                QuestionKind::RatingFive | QuestionKind::ArrayFive => {
                    ["1", "2", "3", "4", "5"].contains(&value)
                }
                QuestionKind::ArrayTen => {
                    ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"].contains(&value)
                }
                QuestionKind::ArrayYesNo => ["Y", "U", "N"].contains(&value),
                QuestionKind::ArrayIncrease => ["I", "S", "D"].contains(&value),
                QuestionKind::YesNo => ["Y", "N"].contains(&value),
                QuestionKind::Gender => ["F", "M"].contains(&value),
                QuestionKind::Language => {
                    value == definition.language
                        || definition
                            .languages
                            .iter()
                            .any(|language| language == value)
                }
                _ => {
                    question
                        .source
                        .choices
                        .iter()
                        .any(|choice| choice.scale == cell.scale && choice.code == value)
                        || (question.source.other && value == "-oth-")
                }
            };
            if !allowed {
                return Err("Choix de réponse inconnu.");
            }
        }
        _ => {}
    }
    Ok(())
}

fn check_constraint(
    name: &str,
    result: &Scalar,
    values: &[&str],
    numbers: &[f64],
    answer_count: usize,
) -> Result<(), &'static str> {
    if name == "em_validation_q" {
        if !result.truth() {
            return Err("La règle de validation de cette question n’est pas satisfaite.");
        }
        return Ok(());
    }
    let limit = result.numeric()?;
    let invalid = match name {
        "min_answers" => (answer_count as f64) < limit,
        "max_answers" => answer_count as f64 > limit,
        "maximum_chars" => values
            .iter()
            .any(|value| value.chars().count() as f64 > limit),
        "min_num_value_n" => numbers.iter().any(|value| *value < limit),
        "max_num_value_n" => numbers.iter().any(|value| *value > limit),
        "min_num_value" if !numbers.is_empty() => numbers.iter().sum::<f64>() < limit,
        "max_num_value" if !numbers.is_empty() => numbers.iter().sum::<f64>() > limit,
        "equals_num_value" if !numbers.is_empty() => {
            (numbers.iter().sum::<f64>() - limit).abs() > 1e-9
        }
        _ => false,
    };
    if invalid {
        return Err("La réponse ne respecte pas les contraintes de la question.");
    }
    Ok(())
}

fn is_date(value: &str) -> bool {
    if value.len() != 10 || !value.is_ascii() || &value[4..5] != "-" || &value[7..8] != "-" {
        return false;
    }
    if value
        .bytes()
        .enumerate()
        .any(|(index, byte)| index != 4 && index != 7 && !byte.is_ascii_digit())
    {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        value[..4].parse::<u32>(),
        value[5..7].parse::<u32>(),
        value[8..].parse::<u32>(),
    ) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let maximum = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    (1..=maximum).contains(&day)
}
