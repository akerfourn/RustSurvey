//! Read model for the source format, separate from the future persistence model.

use serde::{Serialize, Serializer};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceId(pub u64);

impl Serialize for SourceId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: SourceId,
    pub order: u32,
    pub relevance: String,
    pub randomization_group: String,
    pub labels: Vec<LocalizedGroup>,
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
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub id: SourceId,
    pub group_id: SourceId,
    pub code: String,
    pub source_type: String,
    pub family: QuestionFamily,
    pub mandatory: Mandatory,
    pub order: u32,
    pub relevance: String,
    pub randomization_group: String,
    pub validation: String,
    pub other: bool,
    pub labels: Vec<LocalizedQuestion>,
    pub attributes: Vec<QuestionAttribute>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subquestion {
    pub id: SourceId,
    pub parent_id: SourceId,
    pub group_id: SourceId,
    pub code: String,
    pub scale: u32,
    pub order: u32,
    pub relevance: String,
    pub labels: Vec<LocalizedQuestion>,
}

#[derive(Debug, Serialize)]
pub struct LocalizedGroup {
    pub language: String,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LocalizedQuestion {
    pub language: String,
    pub text: Option<String>,
    pub help: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LocalizedText {
    pub language: String,
    pub text: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QuestionAttribute {
    pub name: String,
    pub value: String,
    pub language: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerOption {
    pub id: SourceId,
    pub question_id: SourceId,
    pub code: String,
    pub order: u32,
    pub scale: u32,
    pub labels: Vec<LocalizedText>,
}

#[derive(Debug, Serialize)]
pub struct LocalizedTitle {
    pub language: String,
    pub value: Option<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurveySource {
    pub survey_id: Option<SourceId>,
    pub base_language: Option<String>,
    pub languages: Vec<String>,
    pub titles: Vec<LocalizedTitle>,
    pub groups: Vec<Group>,
    pub questions: Vec<Question>,
    pub subquestions: Vec<Subquestion>,
    pub answers: Vec<AnswerOption>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMetadata {
    pub survey_id: Option<String>,
    pub base_language: Option<String>,
    pub languages: Vec<String>,
    pub titles: Vec<LocalizedTitle>,
}
