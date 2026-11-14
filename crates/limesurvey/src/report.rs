use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    UnsupportedSchema,
    InvalidTable,
    InvalidField,
    UnresolvedSetting,
    DuplicateId,
    MissingReference,
    SurveyMismatch,
    DuplicateLanguage,
    UnsupportedQuestionType,
    UnknownSectionPreserved,
    UncheckedSectionPreserved,
}

#[derive(Debug, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: DiagnosticCode,
    pub location: String,
    pub message: &'static str,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Available,
    Blocked,
    NotImplemented,
}

#[derive(Debug, Serialize)]
pub struct SectionSummary {
    pub name: String,
    pub rows: usize,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub source_schema: u32,
    pub target_schema: u32,
    pub original_export: Capability,
    pub localized_title_edit: Capability,
    pub execution: Capability,
    pub continuation: Capability,
    pub checked_sections: Vec<&'static str>,
    pub sections: Vec<SectionSummary>,
    pub errors: usize,
    pub warnings: usize,
    pub omitted_diagnostics: usize,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip)]
    pub(crate) diagnostic_limit: usize,
}

impl Report {
    pub fn has_errors(&self) -> bool {
        self.errors > 0
    }

    pub(crate) fn new(source_schema: u32, diagnostic_limit: usize) -> Self {
        Self {
            source_schema,
            target_schema: crate::TARGET_SCHEMA,
            original_export: Capability::Available,
            localized_title_edit: Capability::Blocked,
            execution: Capability::NotImplemented,
            continuation: Capability::NotImplemented,
            checked_sections: Vec::new(),
            sections: Vec::new(),
            errors: 0,
            warnings: 0,
            omitted_diagnostics: 0,
            diagnostics: Vec::new(),
            diagnostic_limit,
        }
    }

    pub(crate) fn push(
        &mut self,
        severity: Severity,
        code: DiagnosticCode,
        location: String,
        message: &'static str,
    ) {
        match severity {
            Severity::Error => self.errors += 1,
            Severity::Warning => self.warnings += 1,
        }
        if self.diagnostics.len() >= self.diagnostic_limit {
            self.omitted_diagnostics += 1;
            return;
        }
        self.diagnostics.push(Diagnostic {
            severity,
            code,
            location,
            message,
        });
    }
}
