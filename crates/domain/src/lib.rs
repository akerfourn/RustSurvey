//! Native questionnaire definitions, independent of SQL, HTTP and LimeSurvey.
use serde::{Deserialize, Serialize};
pub mod collection;
pub mod participation;
pub mod rules;
mod validation;

pub const MAX_DOCUMENT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_QUESTIONS: usize = 500;
pub const MAX_ANSWER_CELLS: usize = 20000;
pub const SCHEMA_VERSION: u32 = 3;

pub fn always() -> String {
    "1".to_owned()
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceReference {
    pub id: String,
    pub kind: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Translation {
    pub language: String,
    pub text: Option<String>,
    pub help: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SurveyTranslation {
    pub language: String,
    pub title: Option<String>,
    pub welcome: Option<String>,
    pub end: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Attribute {
    pub name: String,
    pub value: String,
    pub language: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionMedia {
    pub id: String,
    pub alt: String,
    pub language: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Definition {
    pub schema_version: u32,
    pub title: String,
    pub language: String,
    pub welcome: String,
    pub end: String,
    pub groups: Vec<Group>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub translations: Vec<SurveyTranslation>,
    #[serde(default)]
    pub show_welcome: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Group {
    pub id: String,
    pub title: String,
    pub description: String,
    pub questions: Vec<Question>,
    #[serde(default = "always")]
    pub relevance: String,
    #[serde(default)]
    pub translations: Vec<Translation>,
    #[serde(default)]
    pub randomization_group: String,
    #[serde(default)]
    pub source: Option<SourceReference>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Question {
    pub id: String,
    pub code: String,
    pub title: String,
    pub help: String,
    pub kind: QuestionKind,
    pub mandatory: Mandatory,
    pub choices: Vec<Choice>,
    #[serde(default = "always")]
    pub relevance: String,
    #[serde(default)]
    pub translations: Vec<Translation>,
    #[serde(default)]
    pub subquestions: Vec<Choice>,
    #[serde(default)]
    pub attributes: Vec<Attribute>,
    #[serde(default)]
    pub other: bool,
    #[serde(default)]
    pub validation: String,
    #[serde(default)]
    pub randomization_group: String,
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub source: Option<SourceReference>,
    #[serde(default)]
    pub media: Vec<QuestionMedia>,
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
    HugeText,
    Dropdown,
    RatingFive,
    Gender,
    SingleChoiceComment,
    MultipleChoiceComment,
    MultipleText,
    MultipleNumbers,
    ArrayFive,
    ArrayTen,
    ArrayYesNo,
    ArrayIncrease,
    ArrayChoice,
    ArrayColumn,
    DualScale,
    NumberMatrix,
    TextMatrix,
    Ranking,
    Language,
    Equation,
    FileUpload,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mandatory {
    No,
    Soft,
    Strict,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Choice {
    pub id: String,
    pub code: String,
    pub label: String,
    #[serde(default)]
    pub help: String,
    #[serde(default)]
    pub scale: u32,
    #[serde(default = "always")]
    pub relevance: String,
    #[serde(default)]
    pub translations: Vec<Translation>,
    #[serde(default)]
    pub source: Option<SourceReference>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ValidationIssue {
    pub path: String,
    pub message: String,
}

impl Definition {
    pub fn upgrade(&mut self) -> Result<(), &'static str> {
        if self.schema_version < 3
            && self
                .groups
                .iter()
                .flat_map(|group| &group.questions)
                .any(|question| !question.media.is_empty())
        {
            return Err("Les références de médias nécessitent le format natif v3.");
        }
        if self.schema_version == 1 {
            if self
                .groups
                .iter()
                .flat_map(|group| &group.questions)
                .any(|question| {
                    question.kind == QuestionKind::MultipleChoice
                        && !question.choices.is_empty()
                        && !question.subquestions.is_empty()
                })
            {
                return Err("Définition v1 ambiguë : choix et sous-questions simultanés.");
            }
            for question in self
                .groups
                .iter_mut()
                .flat_map(|group| &mut group.questions)
            {
                if question.kind == QuestionKind::MultipleChoice && question.subquestions.is_empty()
                {
                    question.subquestions = std::mem::take(&mut question.choices);
                }
            }
            self.schema_version = 2;
        }
        if self.schema_version == 2 {
            self.schema_version = SCHEMA_VERSION;
        }
        if self.schema_version != SCHEMA_VERSION {
            return Err("Version de définition non prise en charge.");
        }
        Ok(())
    }

    pub fn validate(&self) -> Vec<ValidationIssue> {
        if self.schema_version == 1 || self.schema_version == 2 {
            let mut definition = self.clone();
            if let Err(message) = definition.upgrade() {
                return vec![ValidationIssue {
                    path: "schemaVersion".to_owned(),
                    message: message.to_owned(),
                }];
            }
            return validation::validate(&definition);
        }
        validation::validate(self)
    }

    pub fn without_conditions(&self) -> Self {
        let mut definition = self.clone();
        for group in &mut definition.groups {
            group.relevance = always();
            for question in &mut group.questions {
                question.relevance = always();
            }
        }
        definition
    }
}
