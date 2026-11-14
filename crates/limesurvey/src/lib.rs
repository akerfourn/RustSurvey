//! Bounded LimeSurvey XML inspection and lossless preservation of original files.

mod default_projection;
mod error;
mod execution;
mod import_execution;
mod model;
mod native;
mod report;
mod response_projection;
mod responses;
mod settings;
mod source_bindings;
mod structure;
mod vv;
mod xml;
mod xml_write;

pub use response_projection::{PreservedSourceValue, ProjectedResponse, ResponseProjector};
pub use responses::{ResponseRow, ResponseTable};
pub use settings::{
    GROUP_SETTINGS_PROFILE, GroupSettingsReport, MAX_SETTINGS_CONTEXT_BYTES, ResolvedSetting,
    SettingOrigin, SettingValue, SettingsContext, SettingsGroup,
};
pub use source_bindings::{SourceBindings, source_bindings};
pub use vv::VvEncoder;

pub use default_projection::project_question_defaults;
pub use error::Error;
pub use execution::ProjectionGaps;
pub use import_execution::{ExecutableImport, IMPORT_EXECUTION_PROFILE, SourceRuntimeFacts};
pub use model::{
    AnswerOption, DefaultValue, Group, LocalizedDefaultValue, LocalizedGroup, LocalizedQuestion,
    LocalizedSurveyMessages, LocalizedText, LocalizedTitle, Mandatory, Question, QuestionAttribute,
    QuestionFamily, SourceId, SourceMetadata, Subquestion, SurveySource,
};
pub use native::{
    FieldBinding, NativeExport, NativeExportError, export_native, export_native_with_navigation,
    native_bindings,
};
pub use report::{Capability, Diagnostic, DiagnosticCode, Report, SectionSummary, Severity};

pub const TARGET_SCHEMA: u32 = 623;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RelevanceTarget {
    Group,
    Question,
}

#[derive(Clone, Debug)]
pub struct RelevanceEdit {
    pub target: RelevanceTarget,
    pub id: SourceId,
    pub expression: String,
}

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

    /// Inventory source data not carried by the current executable definition.
    /// An empty inventory is not proof of execution or participation continuity.
    pub fn projection_gaps(&self) -> ProjectionGaps {
        execution::inspect(&self.xml, &self.report, self.limits.max_diagnostics)
    }

    /// Resolve the selected group options, using only supplied source context.
    pub fn group_settings(
        &self,
        context: Option<&SettingsContext>,
    ) -> Result<GroupSettingsReport, &'static str> {
        settings::inspect(self, context)
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

    /// Patch only the requested group/question relevance fields. All other bytes stay unchanged.
    pub fn with_relevances(&self, edits: &[RelevanceEdit]) -> Result<Self, Error> {
        if self.report.source_schema != TARGET_SCHEMA {
            return Err(Error::UnsupportedSchema(self.report.source_schema));
        }
        if self.report.has_errors() {
            return Err(Error::InvalidStructure);
        }
        let updated = structure::replace_relevances(&self.xml, edits, self.limits)?;
        let result = Self::parse(updated.as_bytes(), self.limits)?;
        if result.report.has_errors() {
            return Err(Error::InvalidStructure);
        }
        Ok(result)
    }
}
