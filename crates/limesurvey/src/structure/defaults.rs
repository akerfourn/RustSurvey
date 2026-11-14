use super::{
    fields::{issue, number, small_number, source_id, text},
    tables::rows,
};
use crate::{
    DefaultValue, DiagnosticCode, LocalizedDefaultValue, Report, SurveySource, xml::Document,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn read(xml: &Document, model: &mut SurveySource, report: &mut Report) {
    let questions: BTreeSet<_> = model.questions.iter().map(|question| question.id).collect();
    let subquestions: BTreeMap<_, _> = model
        .subquestions
        .iter()
        .map(|row| (row.id, row.parent_id))
        .collect();
    let mut positions = BTreeMap::new();
    let mut targets = BTreeSet::new();
    for (index, row) in rows(xml, "defaultvalues").into_iter().enumerate() {
        let path = format!("defaultvalues.rows[{index}]");
        let Some(value) = read_value(xml, row, &path, report) else {
            continue;
        };
        if positions.contains_key(&value.id) {
            issue(
                report,
                DiagnosticCode::InvalidField,
                &path,
                "duplicate default value identifier",
            );
            continue;
        }
        if !questions.contains(&value.question_id)
            || value
                .subquestion_id
                .is_some_and(|id| subquestions.get(&id) != Some(&value.question_id))
        {
            issue(
                report,
                DiagnosticCode::InvalidField,
                &path,
                "default value refers to an unknown question or a subquestion of another question",
            );
        }
        if !targets.insert((
            value.question_id,
            value.subquestion_id,
            value.scale,
            value.special_type.clone(),
        )) {
            issue(
                report,
                DiagnosticCode::InvalidField,
                &path,
                "duplicate default value target",
            );
        }
        positions.insert(value.id, model.default_values.len());
        model.default_values.push(value);
    }
    read_translations(xml, model, &positions, report);
    if !model.default_values.is_empty() {
        report.push(crate::Severity::Warning, DiagnosticCode::UncheckedSectionPreserved, "defaultvalues".into(), "default coordinates and text are preserved; execution requires a qualified initialization profile");
    }
}

fn read_value(xml: &Document, row: usize, path: &str, report: &mut Report) -> Option<DefaultValue> {
    let id = source_id(xml, row, "dvid", path, report)?;
    let question_id = source_id(xml, row, "qid", path, report)?;
    let subquestion = number(xml, row, "sqid", path, report)?;
    let subquestion_id = (subquestion != 0).then_some(crate::SourceId(subquestion));
    let scale = small_number(xml, row, "scale_id", path, report)?;
    let special_type = scalar(xml, row, "specialtype", path, report)?.to_owned();
    Some(DefaultValue {
        id,
        question_id,
        subquestion_id,
        scale,
        special_type,
        values: Vec::new(),
    })
}

fn read_translations(
    xml: &Document,
    model: &mut SurveySource,
    positions: &BTreeMap<crate::SourceId, usize>,
    report: &mut Report,
) {
    let mut seen = BTreeSet::new();
    let languages: BTreeSet<_> = model.languages.iter().map(String::as_str).collect();
    for (index, row) in rows(xml, "defaultvalue_l10ns").into_iter().enumerate() {
        let path = format!("defaultvalue_l10ns.rows[{index}]");
        let Some(id) = source_id(xml, row, "dvid", &path, report) else {
            continue;
        };
        let Some(language) = text(xml, row, "language", &path, report) else {
            continue;
        };
        let Some(value) = scalar(xml, row, "defaultvalue", &path, report) else {
            continue;
        };
        if !languages.contains(language) || !seen.insert((id, language)) {
            issue(
                report,
                DiagnosticCode::InvalidField,
                &path,
                "unknown language or duplicate default value translation",
            );
            continue;
        }
        let Some(&position) = positions.get(&id) else {
            issue(
                report,
                DiagnosticCode::InvalidField,
                &path,
                "default value translation refers to an unknown identifier",
            );
            continue;
        };
        model.default_values[position]
            .values
            .push(LocalizedDefaultValue {
                language: language.to_owned(),
                value: value.to_owned(),
            });
    }
}

fn scalar<'a>(
    xml: &'a Document,
    row: usize,
    field: &str,
    path: &str,
    report: &mut Report,
) -> Option<&'a str> {
    let value = xml.field(row, field);
    if value.is_none() {
        issue(
            report,
            DiagnosticCode::InvalidField,
            format!("{path}.{field}"),
            "expected a scalar field; an empty value is allowed",
        );
    }
    value
}
