//! Read model for the source format, separate from the future persistence model.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct SourceId(pub u64);

#[derive(Debug, Serialize)]
pub struct Group {
    pub id: SourceId,
    pub order: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionFamily {
    Text,
    Choice,
    Input,
    Array,
    Ranking,
    Calculation,
    Display,
    File,
    Unknown,
}

impl QuestionFamily {
    pub fn from_code(code: &str) -> Self {
        match code {
            "S" | "T" | "U" => Self::Text,
            "5" | "!" | "L" | "O" | "M" | "P" | "Y" | "G" => Self::Choice,
            "D" | "N" | "K" | "Q" | "I" => Self::Input,
            "A" | "B" | "C" | "E" | "F" | "H" | "1" | ":" | ";" => Self::Array,
            "R" => Self::Ranking,
            "*" => Self::Calculation,
            "X" => Self::Display,
            "|" => Self::File,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mandatory {
    No,
    Soft,
    Strict,
}

#[derive(Debug, Serialize)]
pub struct Question {
    pub id: SourceId,
    pub group_id: SourceId,
    pub code: String,
    pub source_type: String,
    pub family: QuestionFamily,
    pub mandatory: Mandatory,
    pub order: u32,
}

#[derive(Debug, Serialize)]
pub struct Subquestion {
    pub id: SourceId,
    pub parent_id: SourceId,
    pub group_id: SourceId,
    pub code: String,
    pub scale: u32,
}

#[derive(Debug, Serialize)]
pub struct LocalizedTitle {
    pub language: String,
    pub value: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct SurveySource {
    pub survey_id: Option<SourceId>,
    pub base_language: Option<String>,
    pub languages: Vec<String>,
    pub titles: Vec<LocalizedTitle>,
    pub groups: Vec<Group>,
    pub questions: Vec<Question>,
    pub subquestions: Vec<Subquestion>,
}
