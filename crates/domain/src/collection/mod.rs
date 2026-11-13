//! Stateless execution for the explicitly supported native collection profile.
mod profile;
mod snapshot;
mod validation;

use crate::{Definition, Question, ValidationIssue, rules::Expression};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub type Answers = BTreeMap<String, String>;
pub const PROFILE: &str = "native-simple-v1";
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
}

struct Cell {
    key: String,
    kind: CellKind,
    comment: bool,
    other: bool,
    relevance: Option<Expression>,
}

struct PreparedQuestion {
    source: Question,
    group_id: String,
    relevance: Vec<Expression>,
    constraints: BTreeMap<String, Expression>,
    equation: Option<Expression>,
    cells: Vec<Cell>,
}

pub struct Questionnaire {
    definition: Definition,
    questions: Vec<PreparedQuestion>,
    fields: BTreeMap<String, usize>,
    order: Vec<usize>,
}

impl Questionnaire {
    pub fn compile(definition: &Definition) -> Result<Self, Vec<ValidationIssue>> {
        profile::compile(definition)
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
