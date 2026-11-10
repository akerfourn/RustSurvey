//! Bounded LimeSurvey XML inspection and lossless preservation of original files.

mod error;
mod model;
mod report;
mod structure;
mod xml;

pub use error::Error;
pub use model::{
    AnswerOption, Group, LocalizedGroup, LocalizedQuestion, LocalizedText, LocalizedTitle,
    Mandatory, Question, QuestionAttribute, QuestionFamily, SourceId, SourceMetadata, Subquestion,
    SurveySource,
};
pub use report::{Capability, Diagnostic, DiagnosticCode, Report, SectionSummary, Severity};

pub const TARGET_SCHEMA: u32 = 623;

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_depth: usize,
    pub max_elements: usize,
    pub max_attributes: usize,
    pub max_diagnostics: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 8 * 1024 * 1024,
            max_depth: 64,
            max_elements: 100_000,
            max_attributes: 32,
            max_diagnostics: 128,
        }
    }
}

pub struct LssDocument {
    xml: xml::Document,
    model: Option<SurveySource>,
    report: Report,
    limits: Limits,
}

impl LssDocument {
    pub fn parse(input: &[u8], limits: Limits) -> Result<Self, Error> {
        let xml = xml::Document::parse(input, limits)?;
        let schema = structure::schema(&xml)?;
        let mut report = Report::new(schema, limits.max_diagnostics);
        let model = structure::inspect(&xml, &mut report);
        if schema == TARGET_SCHEMA && !report.has_errors() {
            report.localized_title_edit = Capability::Available;
        }
        Ok(Self {
            xml,
            model,
            report,
            limits,
        })
    }

    pub fn source_model(&self) -> Option<&SurveySource> {
        self.model.as_ref()
    }

    pub fn report(&self) -> &Report {
        &self.report
    }

    /// Read common header metadata without claiming validation of historical schemas.
    pub fn metadata(&self) -> SourceMetadata {
        structure::metadata(&self.xml)
    }

    /// Return the input bytes exactly, including extensions, comments and whitespace.
    pub fn original_xml(&self) -> &[u8] {
        self.xml.source.as_bytes()
    }

    /// Change one localized title while leaving all other source bytes untouched.
    pub fn with_title(&self, language: &str, title: &str) -> Result<Self, Error> {
        if self.report.source_schema != TARGET_SCHEMA {
            return Err(Error::UnsupportedSchema(self.report.source_schema));
        }
        if self.report.has_errors() {
            return Err(Error::InvalidStructure);
        }
        let updated = structure::replace_title(&self.xml, language, title, self.limits)?;
        Self::parse(updated.as_bytes(), self.limits)
    }
}
