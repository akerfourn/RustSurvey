use super::*;
use crate::{Attribute, QuestionKind::*};

pub(super) fn has_rows(question: &Question) -> bool {
    matches!(
        question.kind,
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
            | DualScale
            | NumberMatrix
            | TextMatrix
    )
}

fn matrix(question: &Question) -> bool {
    matches!(question.kind, NumberMatrix | TextMatrix)
}

pub(super) fn checkbox(question: &Question) -> bool {
    question.kind == NumberMatrix && attribute(question, "multiflexible_checkbox") == "1"
}

pub(super) fn check_shape(question: &Question, path: &str, issues: &mut Vec<ValidationIssue>) {
    let rows = question
        .subquestions
        .iter()
        .filter(|row| row.scale == 0)
        .count();
    let columns = question
        .subquestions
        .iter()
        .filter(|row| row.scale == 1)
        .count();
    let choices = matches!(
        question.kind,
        SingleChoice
            | Dropdown
            | SingleChoiceComment
            | ArrayChoice
            | ArrayColumn
            | DualScale
            | Ranking
    );
    let dual = question.kind == DualScale;
    let invalid = (has_rows(question) && rows == 0)
        || (!has_rows(question) && !question.subquestions.is_empty())
        || (matrix(question) && columns == 0)
        || (!matrix(question) && columns != 0)
        || question.subquestions.iter().any(|row| row.scale > 1)
        || (choices && !question.choices.iter().any(|choice| choice.scale == 0))
        || (!choices && !question.choices.is_empty())
        || (dual && !question.choices.iter().any(|choice| choice.scale == 1))
        || question
            .choices
            .iter()
            .any(|choice| choice.scale > u32::from(dual));
    if invalid {
        issues.push(issue(
            path,
            "Axes ou échelles incompatibles avec le profil de collecte.",
        ));
    }
    if matches!(question.kind, FileUpload | Unsupported) {
        issues.push(issue(
            path,
            "Ce type de question n’est pas encore pris en charge par la collecte.",
        ));
    }
    if !question.randomization_group.is_empty()
        || !question.validation.is_empty()
        || (!question.theme.is_empty() && question.theme != "core")
        || !question.media.is_empty()
    {
        issues.push(issue(
            path,
            "Aléatoire, regex, thèmes et médias restent hors du profil de collecte.",
        ));
    }
    if question.other
        && !matches!(
            question.kind,
            SingleChoice | Dropdown | MultipleChoice | MultipleChoiceComment
        )
    {
        issues.push(issue(
            path,
            "Option autre incompatible avec le profil de collecte.",
        ));
    }
    if question.kind == NumberMatrix
        && !checkbox(question)
        && attribute(question, "input_boxes") != "1"
    {
        let count = crate::rules::fields(question).len();
        if let Ok((min, max, step)) = range(question, true) {
            let choices = ((max - min) / step + 1e-9).floor() + 1.0;
            if !choices.is_finite() || choices * count as f64 > 2000.0 {
                issues.push(issue(path, "Matrice limitée à 2000 choix affichés."));
            }
        }
    }
}

pub(super) fn accept_attribute(
    question: &Question,
    item: &Attribute,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) -> bool {
    let name = item.name.as_str();
    let boolean = (name == "use_dropdown"
        && matches!(
            question.kind,
            ArrayFive
                | ArrayTen
                | ArrayYesNo
                | ArrayIncrease
                | ArrayChoice
                | ArrayColumn
                | DualScale
        ))
        || (name == "input_boxes" && question.kind == NumberMatrix)
        || (name == "array_filter_style" && has_rows(question))
        || (["slider_layout", "slider_reset", "slider_showminmax"].contains(&name)
            && question.kind == MultipleNumbers)
        || (name == "multiflexible_checkbox" && question.kind == NumberMatrix);
    let literal = (["slider_min", "slider_max", "slider_accuracy"].contains(&name)
        && question.kind == MultipleNumbers)
        || ([
            "multiflexible_min",
            "multiflexible_max",
            "multiflexible_step",
        ]
        .contains(&name)
            && question.kind == NumberMatrix);
    let filter = ["array_filter", "array_filter_exclude"].contains(&name) && has_rows(question);
    let comment = question.kind == MultipleChoiceComment
        && ((name == "commented_checkbox"
            && ["checked", "unchecked", "allways"].contains(&item.value.as_str()))
            || (name == "commented_checkbox_auto" && item.value == "1"));
    let text = [
        "dualscale_headerA",
        "dualscale_headerB",
        "em_validation_sq_tip",
    ]
    .contains(&name);
    if !(boolean || literal || filter || comment || text) {
        return false;
    }
    if (boolean && !["0", "1"].contains(&item.value.as_str()))
        || (!text
            && item
                .language
                .as_deref()
                .is_some_and(|language| !language.is_empty()))
    {
        issues.push(issue(path, "Attribut d’interaction invalide."));
    }
    if literal
        && item
            .value
            .trim()
            .parse::<f64>()
            .ok()
            .is_none_or(|value| !value.is_finite())
    {
        issues.push(issue(path, "Borne numérique non littérale ou invalide."));
    }
    if text && item.value.contains('{') {
        issues.push(issue(
            path,
            "Les expressions dans les textes ne sont pas encore prises en charge par la collecte.",
        ));
    }
    true
}

pub(super) fn filters(question: &Question) -> Vec<Filter> {
    let mut filters = Vec::new();
    let mut seen = BTreeSet::new();
    for (name, exclude) in [("array_filter", false), ("array_filter_exclude", true)] {
        for code in attribute(question, name)
            .split(';')
            .map(str::trim)
            .filter(|code| !code.is_empty())
        {
            if !seen.insert((code, exclude)) {
                continue;
            }
            filters.push(Filter {
                code: code.to_owned(),
                exclude,
                selections: Arc::default(),
            });
        }
    }
    filters
}

fn kind(question: &Question) -> CellKind {
    if checkbox(question) {
        return CellKind::Checkbox;
    }
    match question.kind {
        Number | MultipleNumbers | NumberMatrix => CellKind::Number,
        Date => CellKind::Date,
        MultipleChoice | MultipleChoiceComment => CellKind::Multiple,
        SingleChoice | Dropdown | RatingFive | YesNo | Gender | SingleChoiceComment | Language
        | ArrayFive | ArrayTen | ArrayYesNo | ArrayIncrease | ArrayChoice | ArrayColumn
        | DualScale | Ranking => CellKind::Choice,
        _ if attribute(question, "numbers_only") == "1" => CellKind::Number,
        _ => CellKind::Text,
    }
}

pub(super) fn cells(question: &Question, path: &str) -> Result<Vec<Cell>, Vec<ValidationIssue>> {
    let mut cells = Vec::new();
    let mut axes = BTreeMap::new();
    for row in &question.subquestions {
        axes.insert(
            row.id.clone(),
            Arc::new(profile::expression(&row.relevance, path)?),
        );
    }
    if has_rows(question) {
        for row in question.subquestions.iter().filter(|row| row.scale == 0) {
            let key = format!("{}_{}", question.code, row.code);
            let mut cell = Cell::new(key.clone(), kind(question));
            cell.row = Some(row.code.clone());
            cell.relevance.push(axes[&row.id].clone());
            if matrix(question) {
                for column in question
                    .subquestions
                    .iter()
                    .filter(|column| column.scale == 1)
                {
                    let mut item = Cell::new(format!("{key}_{}", column.code), cell.kind);
                    item.row = cell.row.clone();
                    item.relevance = vec![axes[&row.id].clone(), axes[&column.id].clone()];
                    cells.push(item);
                }
            } else if question.kind == DualScale {
                for scale in 0..=1 {
                    let mut item = Cell::new(format!("{key}_{scale}"), CellKind::Choice);
                    item.row = cell.row.clone();
                    item.relevance = cell.relevance.clone();
                    item.scale = scale;
                    cells.push(item);
                }
            } else {
                cells.push(cell);
                if question.kind == MultipleChoiceComment {
                    let mut comment = Cell::new(format!("{key}comment"), CellKind::Text);
                    comment.comment = true;
                    comment.row = Some(row.code.clone());
                    comment.relevance.push(axes[&row.id].clone());
                    cells.push(comment);
                }
            }
        }
    } else if question.kind != Information {
        for key in crate::rules::fields(question) {
            if key == format!("{}_comment", question.code) && question.kind == SingleChoiceComment {
                continue;
            }
            if question.other && key == format!("{}_other", question.code) {
                continue;
            }
            cells.push(Cell::new(key, kind(question)));
        }
    }
    if question.kind == SingleChoiceComment {
        let mut cell = Cell::new(format!("{}_comment", question.code), CellKind::Text);
        cell.comment = true;
        cells.push(cell);
    }
    if question.other {
        let mut cell_kind = CellKind::Text;
        if attribute(question, "other_numbers_only") == "1" {
            cell_kind = CellKind::Number;
        }
        let mut cell = Cell::new(format!("{}_other", question.code), cell_kind);
        cell.other = true;
        cells.push(cell);
        if question.kind == MultipleChoiceComment {
            let mut comment = Cell::new(format!("{}_othercomment", question.code), CellKind::Text);
            comment.comment = true;
            cells.push(comment);
        }
    }
    Ok(cells)
}

fn number(question: &Question, name: &str, default: f64) -> Result<f64, &'static str> {
    let source = attribute(question, name);
    if source.is_empty() {
        return Ok(default);
    }
    source
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or("Borne numérique invalide.")
}

pub(super) fn range(question: &Question, matrix: bool) -> Result<(f64, f64, f64), &'static str> {
    let (min, max, step) = if matrix {
        (
            number(question, "multiflexible_min", 1.0)?,
            number(question, "multiflexible_max", 10.0)?,
            number(question, "multiflexible_step", 1.0)?,
        )
    } else {
        (
            number(question, "slider_min", 0.0)?,
            number(question, "slider_max", 100.0)?,
            number(question, "slider_accuracy", 1.0)?,
        )
    };
    if max < min || step <= 0.0 {
        return Err("Intervalle numérique invalide.");
    }
    Ok((min, max, step))
}

pub(super) fn validate_number(question: &PreparedQuestion, value: f64) -> Result<(), &'static str> {
    let source = &question.source;
    let slider = source.kind == MultipleNumbers && attribute(source, "slider_layout") == "1";
    let matrix_menu = source.kind == NumberMatrix && attribute(source, "input_boxes") != "1";
    if slider || matrix_menu {
        let (min, max, step) = range(source, matrix_menu)?;
        let position = (value - min) / step;
        if value < min
            || value > max
            || !position.is_finite()
            || (position - position.round()).abs() > 1e-7
        {
            return Err("La valeur ne respecte pas les bornes ou le pas.");
        }
    } else if matches!(source.kind, NumberMatrix | MultipleNumbers) {
        for (setting, constraint, minimum) in if source.kind == NumberMatrix {
            [
                ("multiflexible_min", "", true),
                ("multiflexible_max", "", false),
            ]
        } else {
            [
                ("slider_min", "min_num_value_n", true),
                ("slider_max", "max_num_value_n", false),
            ]
        } {
            if attribute(source, setting).is_empty()
                || question.constraints.contains_key(constraint)
            {
                continue;
            }
            let limit = number(source, setting, 0.0)?;
            if (minimum && value < limit) || (!minimum && value > limit) {
                return Err("La valeur ne respecte pas les bornes numériques.");
            }
        }
    }
    Ok(())
}
