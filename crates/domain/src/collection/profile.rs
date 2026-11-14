use super::*;
use crate::{QuestionKind, rules};

pub(super) fn compile(
    definition: &Definition,
    extended: bool,
    defaults_enabled: bool,
) -> Result<Questionnaire, Vec<ValidationIssue>> {
    let mut issues = definition.validate();
    issues.extend(rules::validate(definition, None));
    if !issues.is_empty() {
        issues.truncate(100);
        return Err(issues);
    }
    let mut questions = Vec::new();
    for (group_index, group) in definition.groups.iter().enumerate() {
        let path = format!("groups[{group_index}]");
        if !group.randomization_group.is_empty() {
            issues.push(issue(
                &path,
                "Ordre aléatoire non pris en charge par la collecte.",
            ));
        }
        check_text(&group.title, &path, &mut issues);
        check_text(&group.description, &path, &mut issues);
        for translation in &group.translations {
            check_text(
                translation.text.as_deref().unwrap_or(""),
                &path,
                &mut issues,
            );
            check_text(
                translation.help.as_deref().unwrap_or(""),
                &path,
                &mut issues,
            );
        }
        for (index, question) in group.questions.iter().enumerate() {
            let path = format!("{path}.questions[{index}]");
            match prepare(
                question,
                &group.id,
                &group.relevance,
                &path,
                extended,
                defaults_enabled,
                definition,
            ) {
                Ok(question) => questions.push(question),
                Err(mut errors) => issues.append(&mut errors),
            }
        }
    }
    for text in [&definition.title, &definition.welcome, &definition.end] {
        check_text(text, "texts", &mut issues);
    }
    for translation in &definition.translations {
        for text in [&translation.title, &translation.welcome, &translation.end] {
            check_text(text.as_deref().unwrap_or(""), "translations", &mut issues);
        }
    }
    if questions.is_empty() {
        issues.push(issue(
            "groups",
            "La collecte nécessite au moins une question.",
        ));
    }
    if !issues.is_empty() {
        issues.truncate(100);
        return Err(issues);
    }
    let mut fields = BTreeMap::new();
    for (index, question) in questions.iter().enumerate() {
        for cell in &question.cells {
            fields.insert(cell.key.clone(), index);
        }
    }
    let order = order(&questions, &fields)?;
    if defaults_enabled {
        super::defaults::check_inactive_references(&questions, &fields)?;
    }
    super::filters::prepare(&mut questions)?;
    Ok(Questionnaire {
        definition: definition.clone(),
        questions,
        fields,
        order,
        defaults_enabled,
    })
}

fn check_text(text: &str, path: &str, issues: &mut Vec<ValidationIssue>) {
    if text.contains('{') {
        issues.push(issue(
            path,
            "Les expressions dans les textes ne sont pas encore prises en charge par la collecte.",
        ));
    }
}

fn prepare(
    question: &Question,
    group_id: &str,
    relevance: &str,
    path: &str,
    extended: bool,
    defaults_enabled: bool,
    definition: &Definition,
) -> Result<PreparedQuestion, Vec<ValidationIssue>> {
    let mut issues = Vec::new();
    if !defaults_enabled
        && question
            .defaults
            .as_ref()
            .is_some_and(|defaults| !defaults.values.is_empty())
    {
        issues.push(issue(path, "L’initialisation des valeurs par défaut n’est pas encore prise en charge par ce profil de collecte."));
    }
    if extended {
        forms::check_shape(question, path, &mut issues);
    } else {
        check_shape(question, path, &mut issues);
    }
    check_question_texts(question, path, &mut issues);
    let constraints = prepare_constraints(question, path, &mut issues, extended)?;
    if !issues.is_empty() {
        return Err(issues);
    }
    let cells = if extended {
        forms::cells(question, path)?
    } else {
        prepare_cells(question, path)?
    };
    let equation = prepare_equation(question, path)?;
    let exclusive = crate::exclusive::ExclusiveOptions::parse(question)
        .map_err(|message| vec![issue(path, message)])?;
    let defaults = if defaults_enabled {
        super::defaults::Defaults::compile(question, definition, path)?
    } else {
        super::defaults::Defaults::default()
    };
    let prepared = PreparedQuestion {
        source: question.clone(),
        group_id: group_id.to_owned(),
        relevance: vec![
            expression(relevance, path)?,
            expression(&question.relevance, path)?,
        ],
        constraints,
        equation,
        exclusive,
        cells,
        defaults,
        filters: if extended {
            forms::filters(question)
        } else {
            Vec::new()
        },
    };
    prepared.defaults.validate_literals(&prepared, definition)?;
    Ok(prepared)
}

fn check_shape(question: &Question, path: &str, issues: &mut Vec<ValidationIssue>) {
    use QuestionKind::*;
    let has_rows = matches!(
        question.kind,
        MultipleChoice | MultipleText | MultipleNumbers
    );
    let has_choices = matches!(question.kind, SingleChoice | Dropdown | SingleChoiceComment);
    if (has_rows && question.subquestions.is_empty())
        || (!has_rows && !question.subquestions.is_empty())
        || question.subquestions.iter().any(|row| row.scale != 0)
        || (has_choices && question.choices.is_empty())
        || (!has_choices && !question.choices.is_empty())
        || question.choices.iter().any(|choice| choice.scale != 0)
    {
        issues.push(issue(
            path,
            "Axes ou échelles incompatibles avec le profil de collecte.",
        ));
    }
    if !matches!(
        question.kind,
        ShortText
            | LongText
            | HugeText
            | Number
            | Date
            | SingleChoice
            | Dropdown
            | RatingFive
            | YesNo
            | Gender
            | MultipleChoice
            | MultipleText
            | MultipleNumbers
            | SingleChoiceComment
            | Information
            | Equation
            | Language
    ) {
        issues.push(issue(
            path,
            "Ce type de question n’est pas encore pris en charge par la collecte.",
        ));
    }
    if !question.randomization_group.is_empty()
        || !question.validation.is_empty()
        || !question.theme.is_empty()
        || !question.media.is_empty()
    {
        issues.push(issue(
            path,
            "Aléatoire, regex, thèmes et médias restent hors du profil de collecte.",
        ));
    }
    if question.other && !matches!(question.kind, SingleChoice | Dropdown | MultipleChoice) {
        issues.push(issue(
            path,
            "Option autre incompatible avec le profil de collecte.",
        ));
    }
}

fn check_question_texts(question: &Question, path: &str, issues: &mut Vec<ValidationIssue>) {
    use QuestionKind::Equation;
    if question.kind != Equation {
        check_text(&question.title, path, issues);
    }
    check_text(&question.help, path, issues);
    for translation in &question.translations {
        let text = translation.text.as_deref().unwrap_or("");
        if question.kind == Equation {
            if attribute(question, "equation").is_empty()
                && !text.is_empty()
                && text != question.title
            {
                issues.push(issue(
                    path,
                    "Les équations localisées restent hors du profil de collecte.",
                ));
            }
        } else {
            check_text(text, path, issues);
        }
        check_text(translation.help.as_deref().unwrap_or(""), path, issues);
    }
    for choice in question.choices.iter().chain(&question.subquestions) {
        check_text(&choice.label, path, issues);
        check_text(&choice.help, path, issues);
        for translation in &choice.translations {
            check_text(translation.text.as_deref().unwrap_or(""), path, issues);
            check_text(translation.help.as_deref().unwrap_or(""), path, issues);
        }
    }
}

fn prepare_constraints(
    question: &Question,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
    extended: bool,
) -> Result<BTreeMap<String, Expression>, Vec<ValidationIssue>> {
    use QuestionKind::Equation;
    let mut constraints = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for item in &question.attributes {
        if !seen.insert((&item.name, &item.language)) {
            issues.push(issue(path, "Attribut déclaré plusieurs fois."));
        }
        if item.value.is_empty() {
            continue;
        }
        if rules::is_constraint(&item.name) {
            if item
                .language
                .as_deref()
                .is_some_and(|language| !language.is_empty())
            {
                issues.push(issue(path, "Contrainte localisée non prise en charge."));
                continue;
            }
            if item.name == "maximum_chars" && item.value.trim() == "0" {
                continue;
            }
            constraints.insert(item.name.clone(), expression(&item.value, path)?);
        } else if [
            "hidden",
            "numbers_only",
            "num_value_int_only",
            "other_comment_mandatory",
            "other_numbers_only",
        ]
        .contains(&item.name.as_str())
        {
            if !["0", "1"].contains(&item.value.as_str())
                || item
                    .language
                    .as_deref()
                    .is_some_and(|language| !language.is_empty())
            {
                issues.push(issue(path, "Attribut d’interaction invalide."));
            }
        } else if item.name == "equation" && question.kind == Equation {
            if item
                .language
                .as_deref()
                .is_some_and(|language| !language.is_empty())
            {
                issues.push(issue(path, "Équation localisée non prise en charge."));
            }
        } else if [
            "display_rows",
            "input_size",
            "display_columns",
            "prefix",
            "suffix",
            "placeholder",
            "other_replace_text",
            "em_validation_q_tip",
        ]
        .contains(&item.name.as_str())
        {
            check_text(&item.value, path, issues);
        } else if extended && forms::accept_attribute(question, item, path, issues) {
            continue;
        } else {
            issues.push(issue(
                path,
                format!("Attribut hors du profil de collecte : {}.", item.name),
            ));
        }
    }
    Ok(constraints)
}

fn prepare_cells(question: &Question, path: &str) -> Result<Vec<Cell>, Vec<ValidationIssue>> {
    use QuestionKind::*;
    let mut cells = Vec::new();
    let mut kind = match question.kind {
        Number | MultipleNumbers => CellKind::Number,
        Date => CellKind::Date,
        SingleChoice | Dropdown | RatingFive | YesNo | Gender | SingleChoiceComment | Language => {
            CellKind::Choice
        }
        MultipleChoice => CellKind::Multiple,
        _ => CellKind::Text,
    };
    if matches!(
        question.kind,
        ShortText | LongText | HugeText | MultipleText
    ) && attribute(question, "numbers_only") == "1"
    {
        kind = CellKind::Number;
    }
    if matches!(
        question.kind,
        MultipleChoice | MultipleText | MultipleNumbers
    ) {
        for row in &question.subquestions {
            let mut cell = Cell::new(format!("{}_{}", question.code, row.code), kind);
            cell.relevance
                .push(Arc::new(expression(&row.relevance, path)?));
            cells.push(cell);
        }
    } else if question.kind != Information {
        cells.push(Cell::new(question.code.clone(), kind));
    }
    if question.kind == SingleChoiceComment {
        let mut cell = Cell::new(format!("{}_comment", question.code), CellKind::Text);
        cell.comment = true;
        cells.push(cell);
    }
    if question.other {
        let kind = if attribute(question, "other_numbers_only") == "1" {
            CellKind::Number
        } else {
            CellKind::Text
        };
        let mut cell = Cell::new(format!("{}_other", question.code), kind);
        cell.other = true;
        cells.push(cell);
    }
    Ok(cells)
}

fn prepare_equation(
    question: &Question,
    path: &str,
) -> Result<Option<Expression>, Vec<ValidationIssue>> {
    use QuestionKind::Equation;
    let equation = if question.kind == Equation {
        let source = attribute(question, "equation");
        let source = if source.is_empty() {
            question
                .title
                .trim()
                .strip_prefix('{')
                .and_then(|text| text.strip_suffix('}'))
                .unwrap_or(&question.title)
        } else {
            source
        };
        Some(expression(source, path)?)
    } else {
        None
    };
    Ok(equation)
}

pub(super) fn expression(source: &str, path: &str) -> Result<Expression, Vec<ValidationIssue>> {
    let source = if source.trim().is_empty() {
        "1"
    } else {
        source
    };
    Expression::compile(source).map_err(|message| vec![issue(path, message)])
}

fn order(
    questions: &[PreparedQuestion],
    fields: &BTreeMap<String, usize>,
) -> Result<Vec<usize>, Vec<ValidationIssue>> {
    let mut dependencies = Vec::new();
    for (question_index, question) in questions.iter().enumerate() {
        let mut required = BTreeSet::new();
        let expressions = question
            .relevance
            .iter()
            .chain(question.equation.iter())
            .chain(
                question
                    .cells
                    .iter()
                    .flat_map(|cell| cell.relevance.iter().map(Arc::as_ref)),
            );
        for expression in expressions {
            for name in &expression.variables {
                let index = fields.get(variable(name)).ok_or_else(|| {
                    vec![issue(
                        &question.source.id,
                        "Variable inconnue dans une règle de collecte.",
                    )]
                })?;
                required.insert(*index);
            }
        }
        for filter in &question.filters {
            if let Some(index) = questions
                .iter()
                .position(|source| source.source.code == filter.code)
            {
                required.insert(index);
            }
        }
        for name in &question.defaults.variables {
            let index = fields.get(variable(name)).ok_or_else(|| {
                vec![issue(
                    &question.source.id,
                    "Variable inconnue dans une valeur par défaut.",
                )]
            })?;
            if *index != question_index {
                required.insert(*index);
            }
        }
        dependencies.push(required);
    }
    let mut order = Vec::new();
    let mut done = BTreeSet::new();
    while order.len() < questions.len() {
        let next = dependencies
            .iter()
            .enumerate()
            .find(|(index, required)| !done.contains(index) && required.is_subset(&done));
        let Some((index, _)) = next else {
            return Err(vec![issue(
                "groups",
                "Cycle de dépendances dans la collecte.",
            )]);
        };
        done.insert(index);
        order.push(index);
    }
    Ok(order)
}
