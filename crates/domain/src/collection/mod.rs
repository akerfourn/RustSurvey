//! Stateless execution for versioned native collection profiles.
mod defaults;
mod exclusive;
mod filters;
mod forms;
mod initialization;
mod profile;
mod shape;
mod snapshot;
mod validation;

use crate::{Definition, Question, ValidationIssue, rules::Expression};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub type Answers = BTreeMap<String, String>;
pub const PROFILE: &str = "native-simple-v1";
pub const FORMS_PROFILE: &str = "native-forms-v1";
pub const PAGES_PROFILE: &str = "native-pages-v1";
pub const DEFAULTS_PROFILE: &str = "native-defaults-v1";
pub const MAX_ANSWER_BYTES: usize = 1024 * 1024;
pub use initialization::PageCursor;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub answers: Answers,
    pub relevant: BTreeSet<String>,
    pub groups: Vec<String>,
    pub warnings: Vec<ValidationIssue>,
    #[serde(skip)]
    initialized: BTreeSet<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum CellKind {
    Text,
    Number,
    Date,
    Choice,
    Multiple,
    Checkbox,
}

struct Cell {
    key: String,
    kind: CellKind,
    comment: bool,
    other: bool,
    relevance: Vec<Arc<Expression>>,
    row: Option<String>,
    scale: u32,
}

impl Cell {
    fn new(key: String, kind: CellKind) -> Self {
        Self {
            key,
            kind,
            comment: false,
            other: false,
            relevance: Vec::new(),
            row: None,
            scale: 0,
        }
    }
}

#[derive(Clone)]
struct Filter {
    code: String,
    exclude: bool,
    selections: Arc<filters::SelectionLayout>,
}

struct PreparedQuestion {
    source: Question,
    group_id: String,
    relevance: Vec<Expression>,
    constraints: BTreeMap<String, Expression>,
    equation: Option<Expression>,
    cells: Vec<Cell>,
    filters: Vec<Filter>,
    exclusive: crate::exclusive::ExclusiveOptions,
    defaults: defaults::Defaults,
}

pub struct Questionnaire {
    definition: Definition,
    questions: Vec<PreparedQuestion>,
    fields: BTreeMap<String, usize>,
    order: Vec<usize>,
    defaults_enabled: bool,
}

impl Questionnaire {
    pub fn compile(definition: &Definition) -> Result<Self, Vec<ValidationIssue>> {
        profile::compile(definition, false, false)
    }

    pub fn compile_forms(definition: &Definition) -> Result<Self, Vec<ValidationIssue>> {
        profile::compile(definition, true, false)
    }

    pub fn compile_defaults(definition: &Definition) -> Result<Self, Vec<ValidationIssue>> {
        profile::compile(definition, true, true)
    }

    pub fn compile_profile(
        definition: &Definition,
        profile: &str,
    ) -> Result<Self, Vec<ValidationIssue>> {
        match profile {
            PROFILE => Self::compile(definition),
            FORMS_PROFILE | PAGES_PROFILE => Self::compile_forms(definition),
            DEFAULTS_PROFILE => Self::compile_defaults(definition),
            _ => Err(vec![issue("profile", "Profil de collecte inconnu.")]),
        }
    }

    pub fn evaluate(
        &self,
        answers: &Answers,
        submit: bool,
    ) -> Result<Snapshot, Vec<ValidationIssue>> {
        if self.defaults_enabled {
            return Err(vec![issue(
                "profile",
                "Ce profil exige une langue et une entrée de page explicites.",
            )]);
        }
        snapshot::evaluate(self, answers, submit)
    }

    pub fn evaluate_initialized(
        &self,
        answers: &Answers,
        submit: bool,
        language: &str,
        questions: &BTreeSet<String>,
    ) -> Result<Snapshot, Vec<ValidationIssue>> {
        self.check_initialization(language)?;
        if questions.iter().any(|id| {
            !self
                .questions
                .iter()
                .any(|question| question.source.id == *id)
        }) {
            return Err(vec![issue(
                "initialization",
                "Question d’initialisation inconnue.",
            )]);
        }
        snapshot::evaluate_context(
            self,
            answers,
            submit,
            language,
            Some(questions),
            true,
            &mut crate::rules::EvaluationBudget::default(),
        )
    }

    fn check_initialization(&self, language: &str) -> Result<(), Vec<ValidationIssue>> {
        if !self.defaults_enabled {
            return Err(vec![issue(
                "profile",
                "Ce profil ne permet pas l’initialisation des défauts.",
            )]);
        }
        if language != self.definition.language
            && !self
                .definition
                .languages
                .iter()
                .any(|value| value == language)
        {
            return Err(vec![issue(
                "checkpoint.language",
                "Langue d’initialisation inconnue.",
            )]);
        }
        Ok(())
    }
}

fn issue(path: impl Into<String>, message: impl Into<String>) -> ValidationIssue {
    ValidationIssue {
        path: path.into(),
        message: message.into(),
    }
}

fn attribute<'a>(question: &'a Question, name: &str) -> &'a str {
    question
        .attributes
        .iter()
        .find(|item| item.name == name && item.language.as_deref().is_none_or(str::is_empty))
        .map_or("", |item| item.value.as_str())
}

fn variable(name: &str) -> &str {
    name.strip_suffix(".NAOK").unwrap_or(name)
}
