//! Native questionnaire definitions, independent of SQL, HTTP and LimeSurvey.
use serde::{Deserialize, Serialize};
mod validation;

pub const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;
pub const MAX_QUESTIONS: usize = 500;
pub const MAX_ANSWER_CELLS: usize = 20000;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Definition {
    pub schema_version: u32,
    pub title: String,
    pub language: String,
    pub welcome: String,
    pub end: String,
    pub groups: Vec<Group>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Group {
    pub id: String,
    pub title: String,
    pub description: String,
    pub questions: Vec<Question>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Question {
    pub id: String,
    pub code: String,
    pub title: String,
    pub help: String,
    pub kind: QuestionKind,
    pub mandatory: Mandatory,
    pub choices: Vec<Choice>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuestionKind {
    ShortText,
    LongText,
    Number,
    Date,
    SingleChoice,
    MultipleChoice,
    YesNo,
    Information,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mandatory {
    No,
    Soft,
    Strict,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Choice {
    pub id: String,
    pub code: String,
    pub label: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ValidationIssue {
    pub path: String,
    pub message: String,
}

impl Definition {
    pub fn validate(&self) -> Vec<ValidationIssue> {
        validation::validate(self)
    }
}
