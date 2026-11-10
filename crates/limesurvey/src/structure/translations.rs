use super::{
    fields::{issue, small_number, source_id, text, translation_language},
    tables::rows,
};
use crate::{DiagnosticCode, Report, SourceId, SurveySource, xml::Document};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate_translations(xml: &Document, model: &SurveySource, report: &mut Report) {
    let languages: BTreeSet<&str> = model.languages.iter().map(String::as_str).collect();
    let groups: BTreeSet<SourceId> = model.groups.iter().map(|group| group.id).collect();
    let questions: BTreeSet<SourceId> = model
        .questions
        .iter()
        .map(|q| q.id)
        .chain(model.subquestions.iter().map(|q| q.id))
        .collect();
    let main_questions: BTreeSet<SourceId> = model.questions.iter().map(|q| q.id).collect();
    let answers = answer_ids(xml, &main_questions, report);
    for (section, field, targets) in [
        ("group_l10ns", "gid", &groups),
        ("question_l10ns", "qid", &questions),
        ("answer_l10ns", "aid", &answers),
    ] {
        localized_rows(xml, section, field, targets, &languages, report);
    }
}

fn answer_ids(
    xml: &Document,
    questions: &BTreeSet<SourceId>,
    report: &mut Report,
) -> BTreeSet<SourceId> {
    let mut ids = BTreeSet::new();
    let mut codes = BTreeSet::new();
    for (index, row) in rows(xml, "answers").into_iter().enumerate() {
        let path = format!("answers.rows[{index}]");
        let id = source_id(xml, row, "aid", &path, report);
        let question = source_id(xml, row, "qid", &path, report);
        let scale = small_number(xml, row, "scale_id", &path, report);
        let code = text(xml, row, "code", &path, report);
        if let Some(id) = id
            && !ids.insert(id)
        {
            issue(
                report,
                DiagnosticCode::DuplicateId,
                &path,
                "duplicate answer identifier",
            );
        }
        if let Some(question) = question
            && !questions.contains(&question)
        {
            issue(
                report,
                DiagnosticCode::MissingReference,
                &path,
                "answer question does not exist",
            );
        }
        if let (Some(question), Some(scale), Some(code)) = (question, scale, code)
            && !codes.insert((question, scale, code))
        {
            issue(
                report,
                DiagnosticCode::DuplicateId,
                &path,
                "duplicate answer code within a question scale",
            );
        }
    }
    ids
}

fn localized_rows(
    xml: &Document,
    section: &str,
    field: &str,
    targets: &BTreeSet<SourceId>,
    languages: &BTreeSet<&str>,
    report: &mut Report,
) {
    let mut localized: BTreeMap<SourceId, BTreeSet<&str>> = BTreeMap::new();
    for (index, row) in rows(xml, section).into_iter().enumerate() {
        let path = format!("{section}.rows[{index}]");
        let id = source_id(xml, row, field, &path, report);
        let language = translation_language(xml, row, "language", &path, languages, report);
        if let (Some(id), Some(language)) = (id, language) {
            if !targets.contains(&id) {
                issue(
                    report,
                    DiagnosticCode::MissingReference,
                    &path,
                    "translation refers to a missing object",
                );
            }
            if !localized.entry(id).or_default().insert(language) {
                issue(
                    report,
                    DiagnosticCode::DuplicateLanguage,
                    &path,
                    "duplicate translation for this object and language",
                );
            }
        }
    }
    for id in targets {
        if localized.get(id).map_or(0, BTreeSet::len) != languages.len() {
            issue(
                report,
                DiagnosticCode::MissingReference,
                section,
                "object translations are incomplete",
            );
        }
    }
}
