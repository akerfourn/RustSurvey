use crate::{DiagnosticCode, Report, Severity, SourceId, xml::Document};
use std::collections::BTreeSet;

pub(super) fn issue(
    report: &mut Report,
    code: DiagnosticCode,
    location: impl Into<String>,
    message: &'static str,
) {
    report.push(Severity::Error, code, location.into(), message);
}

pub(super) fn number(
    xml: &Document,
    row: usize,
    field: &str,
    path: &str,
    report: &mut Report,
) -> Option<u64> {
    let value = xml
        .field(row, field)
        .and_then(|value| value.parse::<u64>().ok());
    if value.is_none() {
        issue(
            report,
            DiagnosticCode::InvalidField,
            format!("{path}.{field}"),
            "expected an unsigned integer field",
        );
    }
    value
}

pub(super) fn source_id(
    xml: &Document,
    row: usize,
    field: &str,
    path: &str,
    report: &mut Report,
) -> Option<SourceId> {
    let value = number(xml, row, field, path, report)?;
    if value == 0 {
        issue(
            report,
            DiagnosticCode::InvalidField,
            format!("{path}.{field}"),
            "source identifiers must be positive",
        );
        return None;
    }
    Some(SourceId(value))
}

pub(super) fn small_number(
    xml: &Document,
    row: usize,
    field: &str,
    path: &str,
    report: &mut Report,
) -> Option<u32> {
    let value = number(xml, row, field, path, report)?;
    match u32::try_from(value) {
        Ok(value) => Some(value),
        Err(_) => {
            issue(
                report,
                DiagnosticCode::InvalidField,
                format!("{path}.{field}"),
                "integer exceeds the supported range",
            );
            None
        }
    }
}

pub(super) fn text<'a>(
    xml: &'a Document,
    row: usize,
    field: &str,
    path: &str,
    report: &mut Report,
) -> Option<&'a str> {
    let value = xml.field(row, field).filter(|value| !value.is_empty());
    if value.is_none() {
        issue(
            report,
            DiagnosticCode::InvalidField,
            format!("{path}.{field}"),
            "expected a non-empty scalar field",
        );
    }
    value
}

pub(super) fn survey_reference(
    xml: &Document,
    row: usize,
    field: &str,
    path: &str,
    expected: Option<SourceId>,
    report: &mut Report,
) {
    if let Some(actual) = source_id(xml, row, field, path, report)
        && Some(actual) != expected
    {
        issue(
            report,
            DiagnosticCode::SurveyMismatch,
            format!("{path}.{field}"),
            "row belongs to another survey",
        );
    }
}

pub(super) fn translation_language<'a>(
    xml: &'a Document,
    row: usize,
    field: &str,
    path: &str,
    languages: &BTreeSet<&str>,
    report: &mut Report,
) -> Option<&'a str> {
    let language = text(xml, row, field, path, report)?;
    if !languages.contains(language) {
        issue(
            report,
            DiagnosticCode::MissingReference,
            format!("{path}.{field}"),
            "translation language is not declared",
        );
        return None;
    }
    Some(language)
}
