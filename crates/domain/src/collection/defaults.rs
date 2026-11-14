use super::{Answers, PreparedQuestion, issue, variable};
use crate::{
    Definition, Question, ValidationIssue,
    defaults::template::Template,
    rules::{EvaluationBudget, Scalar},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct Defaults {
    fields: BTreeMap<String, BTreeMap<String, Template>>,
    order: Vec<String>,
    pub variables: BTreeSet<String>,
    base_language: String,
    shared: bool,
}

impl Defaults {
    pub fn compile(
        question: &Question,
        definition: &Definition,
        path: &str,
    ) -> Result<Self, Vec<ValidationIssue>> {
        let mut prepared = Self {
            base_language: definition.language.clone(),
            ..Self::default()
        };
        let Some(defaults) = &question.defaults else {
            return Ok(prepared);
        };
        prepared.shared = defaults.same_for_all_languages;
        let addresses: BTreeMap<_, _> = crate::cells::for_question(question)
            .map_err(|issue| vec![issue])?
            .into_iter()
            .map(|cell| {
                (
                    (cell.address.coordinate, cell.address.role),
                    cell.runtime_key,
                )
            })
            .collect();
        let mut languages: BTreeSet<_> = definition.languages.iter().map(String::as_str).collect();
        languages.insert(&definition.language);
        for (index, value) in defaults.values.iter().enumerate() {
            if !languages.contains(value.language.as_str())
                || (prepared.shared && value.language != definition.language)
            {
                continue;
            }
            let path = format!("{path}.defaults.values[{index}]");
            let template =
                Template::compile(&value.value).map_err(|message| vec![issue(&path, message)])?;
            if template
                .variables
                .iter()
                .any(|name| name.ends_with(".NAOK"))
            {
                return Err(vec![issue(
                    &path,
                    "Les défauts utilisant NAOK nécessitent encore un contexte de valeurs inactives vérifié.",
                )]);
            }
            let key = addresses
                .get(&(value.coordinate.clone(), value.role))
                .ok_or_else(|| vec![issue(&path, "Cellule de défaut inconnue.")])?
                .clone();
            prepared
                .variables
                .extend(template.variables.iter().cloned());
            prepared
                .fields
                .entry(key)
                .or_default()
                .insert(value.language.clone(), template);
        }
        prepared.order =
            field_order(&prepared.fields).map_err(|message| vec![issue(path, message)])?;
        Ok(prepared)
    }

    pub fn validate_literals(
        &self,
        question: &PreparedQuestion,
        definition: &Definition,
    ) -> Result<(), Vec<ValidationIssue>> {
        for (key, localized) in &self.fields {
            let cell = question
                .cells
                .iter()
                .find(|cell| cell.key == *key)
                .ok_or_else(|| vec![issue(key, "Cellule de défaut inconnue.")])?;
            for template in localized.values() {
                if let Some(value) = template.literal().filter(|value| !value.is_empty()) {
                    super::validation::validate_value(question, cell, value, definition)
                        .map_err(|message| vec![issue(key, message)])?;
                }
            }
        }
        Ok(())
    }

    pub fn apply(
        &self,
        answers: &mut Answers,
        language: &str,
        budget: &mut EvaluationBudget,
    ) -> Result<Vec<String>, &'static str> {
        let language = if self.shared {
            self.base_language.as_str()
        } else {
            language
        };
        let mut initialized = Vec::new();
        for key in &self.order {
            if answers.contains_key(key) {
                continue;
            }
            let Some(template) = self.fields[key].get(language) else {
                continue;
            };
            let value = template.render(
                &|name| Scalar::String(answers.get(variable(name)).cloned().unwrap_or_default()),
                budget,
            )?;
            answers.insert(key.clone(), value);
            initialized.push(key.clone());
        }
        Ok(initialized)
    }
}

pub(super) fn check_inactive_references(
    questions: &[PreparedQuestion],
    fields: &BTreeMap<String, usize>,
) -> Result<(), Vec<ValidationIssue>> {
    for question in questions {
        let expressions = question
            .relevance
            .iter()
            .chain(question.equation.iter())
            .chain(question.constraints.values())
            .chain(
                question
                    .cells
                    .iter()
                    .flat_map(|cell| cell.relevance.iter().map(std::sync::Arc::as_ref)),
            );
        for name in expressions.flat_map(|expression| &expression.variables) {
            if name.ends_with(".NAOK")
                && fields
                    .get(variable(name))
                    .is_some_and(|index| !questions[*index].defaults.fields.is_empty())
            {
                return Err(vec![issue(
                    &question.source.id,
                    "Une référence NAOK à une question avec défaut nécessite encore un contexte de valeurs inactives vérifié.",
                )]);
            }
        }
    }
    Ok(())
}

fn field_order(
    fields: &BTreeMap<String, BTreeMap<String, Template>>,
) -> Result<Vec<String>, &'static str> {
    let dependencies: BTreeMap<_, BTreeSet<_>> = fields
        .iter()
        .map(|(key, localized)| {
            let required = localized
                .values()
                .flat_map(|template| &template.variables)
                .map(|name| variable(name).to_owned())
                .filter(|name| name != key && fields.contains_key(name))
                .collect();
            (key.clone(), required)
        })
        .collect();
    let mut done = BTreeSet::new();
    let mut order = Vec::new();
    while order.len() < fields.len() {
        let next = dependencies
            .iter()
            .find(|(key, required)| !done.contains(*key) && required.is_subset(&done));
        let Some((key, _)) = next else {
            return Err("Cycle entre valeurs par défaut d’une question.");
        };
        done.insert(key.clone());
        order.push(key.clone());
    }
    Ok(order)
}
