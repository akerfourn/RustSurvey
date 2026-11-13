//! Stateless execution for versioned native collection profiles.
mod forms;
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
pub const MAX_ANSWER_BYTES: usize = 1024 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub answers: Answers,
    pub relevant: BTreeSet<String>,
    pub groups: Vec<String>,
    pub warnings: Vec<ValidationIssue>,
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

struct Filter {
    code: String,
    exclude: bool,
    selections: Arc<BTreeSet<String>>,
}

struct PreparedQuestion {
    source: Question,
    group_id: String,
    relevance: Vec<Expression>,
    constraints: BTreeMap<String, Expression>,
    equation: Option<Expression>,
    cells: Vec<Cell>,
    filters: Vec<Filter>,
}

pub struct Questionnaire {
    definition: Definition,
    questions: Vec<PreparedQuestion>,
    fields: BTreeMap<String, usize>,
    order: Vec<usize>,
}

impl Questionnaire {
    pub fn compile(definition: &Definition) -> Result<Self, Vec<ValidationIssue>> {
        profile::compile(definition, false)
    }

    pub fn compile_forms(definition: &Definition) -> Result<Self, Vec<ValidationIssue>> {
        profile::compile(definition, true)
    }

    pub fn compile_profile(
        definition: &Definition,
        profile: &str,
    ) -> Result<Self, Vec<ValidationIssue>> {
        match profile {
            PROFILE => Self::compile(definition),
            FORMS_PROFILE => Self::compile_forms(definition),
            _ => Err(vec![issue("profile", "Profil de collecte inconnu.")]),
        }
    }

    pub fn evaluate(
        &self,
        answers: &Answers,
        submit: bool,
    ) -> Result<Snapshot, Vec<ValidationIssue>> {
        snapshot::evaluate(self, answers, submit)
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
