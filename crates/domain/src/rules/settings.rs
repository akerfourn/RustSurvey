use crate::{Question, QuestionKind, ValidationIssue};

fn value<'a>(question: &'a Question, name: &str, default: &'a str) -> &'a str {
    question
        .attributes
        .iter()
        .find(|item| item.name == name && item.language.as_deref().is_none_or(str::is_empty))
        .map(|item| item.value.as_str())
        .filter(|value| !value.is_empty())
        .unwrap_or(default)
}

fn number(source: &str) -> Option<f64> {
    source
        .trim_matches(super::syntax::is_whitespace)
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

pub(super) fn validate(
    question: &Question,
    previous: Option<&Question>,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    for (index, item) in question.attributes.iter().enumerate() {
        if item.value.is_empty() || previous.is_some_and(|old| old.attributes.contains(item)) {
            continue;
        }
        let boolean = matches!(
            item.name.as_str(),
            "hidden"
                | "numbers_only"
                | "num_value_int_only"
                | "use_dropdown"
                | "input_boxes"
                | "other_comment_mandatory"
                | "other_numbers_only"
                | "array_filter_style"
                | "slider_layout"
                | "slider_reset"
                | "slider_showminmax"
                | "multiflexible_checkbox"
        );
        let literal = matches!(
            item.name.as_str(),
            "slider_min"
                | "slider_max"
                | "slider_accuracy"
                | "multiflexible_min"
                | "multiflexible_max"
                | "multiflexible_step"
                | "display_rows"
        );
        let invalid = (boolean && !["0", "1"].contains(&item.value.as_str()))
            || (literal && number(&item.value).is_none());
        if invalid && issues.len() < 100 {
            issues.push(ValidationIssue {
                path: format!("{path}.attributes[{index}].value"),
                message: "Valeur de réglage invalide.".to_owned(),
            });
        }
        if (boolean || literal || super::is_constraint(&item.name))
            && item
                .language
                .as_deref()
                .is_some_and(|language| !language.is_empty())
            && issues.len() < 100
        {
            issues.push(ValidationIssue {
                path: format!("{path}.attributes[{index}].language"),
                message: "Ce réglage ne peut pas dépendre de la langue.".to_owned(),
            });
        }
    }
    if previous
        .is_some_and(|old| old.attributes == question.attributes && old.kind == question.kind)
    {
        return;
    }
    let range = if question.kind == QuestionKind::MultipleNumbers
        && value(question, "slider_layout", "0") == "1"
    {
        Some((
            "slider_min",
            "slider_max",
            "slider_accuracy",
            "0",
            "100",
            "1",
        ))
    } else if question.kind == QuestionKind::NumberMatrix
        && value(question, "multiflexible_checkbox", "0") != "1"
    {
        if value(question, "input_boxes", "0") == "1" {
            let min = number(value(question, "multiflexible_min", ""));
            let max = number(value(question, "multiflexible_max", ""));
            if min.zip(max).is_some_and(|(min, max)| min > max) {
                issues.push(ValidationIssue {
                    path: format!("{path}.attributes"),
                    message: "Intervalle invalide : maximum inférieur au minimum.".to_owned(),
                });
            }
            return;
        }
        Some((
            "multiflexible_min",
            "multiflexible_max",
            "multiflexible_step",
            "1",
            "10",
            "1",
        ))
    } else {
        None
    };
    if let Some((minimum, maximum, step, min_default, max_default, step_default)) = range {
        let min = number(value(question, minimum, min_default));
        let max = number(value(question, maximum, max_default));
        let step = number(value(question, step, step_default));
        let mut invalid_step = step.is_some_and(|step| step <= 0.0);
        if question.kind == QuestionKind::NumberMatrix {
            invalid_step = step.is_some_and(|step| step < 1e-10);
        }
        if min.zip(max).is_some_and(|(min, max)| max < min) || invalid_step {
            issues.push(ValidationIssue {
                path: format!("{path}.attributes"),
                message:
                    "Intervalle invalide : maximum ≥ minimum et pas strictement positif requis."
                        .to_owned(),
            });
        }
    }
}
