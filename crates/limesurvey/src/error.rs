use std::{error::Error as StdError, fmt};

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    LimitExceeded {
        resource: &'static str,
        limit: usize,
    },
    InvalidUtf8,
    InvalidXml {
        offset: u64,
    },
    InvalidXmlCharacter,
    UnsupportedEncoding,
    DtdForbidden,
    ProcessingInstructionForbidden,
    NotSurveyDocument,
    InvalidMetadata(&'static str),
    InvalidStructure,
    UnsupportedSchema(u32),
    TitleNotFound,
    InvalidRelevanceEdit,
    LegacyConditionsPreventEdit,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => formatter.write_str("all parser limits must be positive"),
            Self::LimitExceeded { resource, limit } => {
                write!(
                    formatter,
                    "{resource} exceeds the configured limit of {limit}"
                )
            }
            Self::InvalidUtf8 => formatter.write_str("the survey must be valid UTF-8"),
            Self::InvalidXml { offset } => write!(formatter, "invalid XML at byte {offset}"),
            Self::InvalidXmlCharacter => formatter.write_str("invalid XML 1.0 character"),
            Self::UnsupportedEncoding => {
                formatter.write_str("only XML 1.0 with UTF-8 encoding is supported")
            }
            Self::DtdForbidden => {
                formatter.write_str("XML document type declarations are forbidden")
            }
            Self::ProcessingInstructionForbidden => {
                formatter.write_str("XML processing instructions are not supported")
            }
            Self::NotSurveyDocument => formatter.write_str("expected a LimeSurvey Survey document"),
            Self::InvalidMetadata(field) => write!(formatter, "invalid or missing {field}"),
            Self::InvalidStructure => {
                formatter.write_str("survey structure contains errors; inspect its diagnostics")
            }
            Self::UnsupportedSchema(version) => {
                write!(
                    formatter,
                    "schema {version} cannot be edited; expected schema 623"
                )
            }
            Self::TitleNotFound => {
                formatter.write_str("expected exactly one survey title for the requested language")
            }
            Self::InvalidRelevanceEdit => formatter
                .write_str("expected unique existing group/question targets for relevance edits"),
            Self::LegacyConditionsPreventEdit => {
                formatter.write_str("legacy conditions prevent a transformed relevance export")
            }
        }
    }
}

impl StdError for Error {}
