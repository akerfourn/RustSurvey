use super::{
    fields::{
        issue, number, small_number, source_id, survey_reference, text, translation_language,
    },
    tables::rows,
};
use crate::{
    DiagnosticCode, Group, LocalizedTitle, Mandatory, Question, QuestionFamily, Report, Severity,
    SourceId, Subquestion, SurveySource, xml::Document,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn read_model(xml: &Document, report: &mut Report) -> SurveySource {
    let mut model = SurveySource::default();
    read_languages(xml, &mut model, report);
    read_survey(xml, &mut model, report);
    read_groups(xml, &mut model, report);
    read_questions(xml, &mut model, report);
    read_subquestions(xml, &mut model, report);
    model
}

fn read_languages(xml: &Document, model: &mut SurveySource, report: &mut Report) {
    let headers: Vec<usize> = xml.children_named(xml.root, "languages").collect();
    if headers.len() != 1 {
        issue(
            report,
            DiagnosticCode::InvalidField,
            "languages",
            "expected one language list",
        );
        return;
    }
    let mut seen = BTreeSet::new();
    for &id in &xml.elements[headers[0]].children {
        let language = &xml.elements[id];
        if !language.unqualified
            || language.name != "language"
            || !language.children.is_empty()
            || language.text.is_empty()
            || language.text.trim() != language.text
        {
            issue(
                report,
                DiagnosticCode::InvalidField,
                "languages",
                "invalid language entry",
            );
        } else if !seen.insert(language.text.clone()) {
            issue(
                report,
                DiagnosticCode::DuplicateLanguage,
                "languages",
                "duplicate language entry",
            );
        } else {
            model.languages.push(language.text.clone());
        }
    }
    if model.languages.is_empty() {
        issue(
            report,
            DiagnosticCode::InvalidField,
            "languages",
            "at least one language is required",
        );
    }
}

fn read_survey(xml: &Document, model: &mut SurveySource, report: &mut Report) {
    let surveys = rows(xml, "surveys");
    if surveys.len() != 1 {
        issue(
            report,
            DiagnosticCode::InvalidTable,
            "surveys",
            "expected exactly one survey row",
        );
        return;
    }
    let row = surveys[0];
    model.survey_id = source_id(xml, row, "sid", "surveys.rows[0]", report);
    model.base_language = text(xml, row, "language", "surveys.rows[0]", report).map(str::to_owned);
    model.show_welcome = match xml.field(row, "showwelcome") {
        Some("Y") => Some(true),
        Some("N") => Some(false),
        None | Some("") | Some("I") => {
            report.push(
                Severity::Warning,
                DiagnosticCode::UnresolvedSetting,
                "surveys.rows[0].showwelcome".to_owned(),
                "showwelcome requires source group settings; no local default is applied",
            );
            None
        }
        _ => {
            issue(
                report,
                DiagnosticCode::InvalidField,
                "surveys.rows[0].showwelcome",
                "showwelcome must be Y or N, or I for inheritance",
            );
            None
        }
    };
    if let Some(language) = &model.base_language
        && !model.languages.contains(language)
    {
        issue(
            report,
            DiagnosticCode::MissingReference,
            "surveys.rows[0].language",
            "base language is not declared",
        );
    }
    let languages: BTreeSet<&str> = model.languages.iter().map(String::as_str).collect();
    let mut seen = BTreeSet::new();
    for (index, row) in rows(xml, "surveys_languagesettings")
        .into_iter()
        .enumerate()
    {
        let path = format!("surveys_languagesettings.rows[{index}]");
        survey_reference(
            xml,
            row,
            "surveyls_survey_id",
            &path,
            model.survey_id,
            report,
        );
        let Some(language) =
            translation_language(xml, row, "surveyls_language", &path, &languages, report)
        else {
            continue;
        };
        if !seen.insert(language.to_owned()) {
            issue(
                report,
                DiagnosticCode::DuplicateLanguage,
                &path,
                "duplicate localized survey settings",
            );
        }
        model.titles.push(LocalizedTitle {
            language: language.to_owned(),
            value: xml.field(row, "surveyls_title").map(str::to_owned),
        });
    }
    if seen.len() != model.languages.len() {
        issue(
            report,
            DiagnosticCode::MissingReference,
            "surveys_languagesettings",
            "localized survey settings are incomplete",
        );
    }
}

fn read_groups(xml: &Document, model: &mut SurveySource, report: &mut Report) {
    let mut seen = BTreeSet::new();
    for (index, row) in rows(xml, "groups").into_iter().enumerate() {
        let path = format!("groups.rows[{index}]");
        survey_reference(xml, row, "sid", &path, model.survey_id, report);
        let id = source_id(xml, row, "gid", &path, report);
        let order = small_number(xml, row, "group_order", &path, report);
        if let (Some(id), Some(order)) = (id, order) {
            if !seen.insert(id) {
                issue(
                    report,
                    DiagnosticCode::DuplicateId,
                    &path,
                    "duplicate group identifier",
                );
            }
            model.groups.push(Group {
                id,
                order,
                relevance: xml.field(row, "grelevance").unwrap_or("1").to_owned(),
                randomization_group: xml
                    .field(row, "randomization_group")
                    .unwrap_or("")
                    .to_owned(),
                labels: Vec::new(),
            });
        }
    }
}

fn read_questions(xml: &Document, model: &mut SurveySource, report: &mut Report) {
    let groups: BTreeSet<SourceId> = model.groups.iter().map(|group| group.id).collect();
    let mut ids = BTreeSet::new();
    let mut codes = BTreeSet::new();
    for (index, row) in rows(xml, "questions").into_iter().enumerate() {
        let path = format!("questions.rows[{index}]");
        survey_reference(xml, row, "sid", &path, model.survey_id, report);
        if number(xml, row, "parent_qid", &path, report) != Some(0) {
            issue(
                report,
                DiagnosticCode::InvalidField,
                &path,
                "top-level question must have parent_qid zero",
            );
        }
        if let Some(question) = read_question(xml, row, &path, report) {
            if !groups.contains(&question.group_id) {
                issue(
                    report,
                    DiagnosticCode::MissingReference,
                    &path,
                    "question group does not exist",
                );
            }
            if !ids.insert(question.id) || !codes.insert(question.code.clone()) {
                issue(
                    report,
                    DiagnosticCode::DuplicateId,
                    &path,
                    "duplicate question identifier or code",
                );
            }
            model.questions.push(question);
        }
    }
}

fn read_question(xml: &Document, row: usize, path: &str, report: &mut Report) -> Option<Question> {
    let id = source_id(xml, row, "qid", path, report)?;
    let group_id = source_id(xml, row, "gid", path, report)?;
    let code = text(xml, row, "title", path, report)?.to_owned();
    let source_type = text(xml, row, "type", path, report)?.to_owned();
    let order = small_number(xml, row, "question_order", path, report)?;
    let mandatory = match xml.field(row, "mandatory") {
        Some("N") => Mandatory::No,
        Some("S") => Mandatory::Soft,
        Some("Y") => Mandatory::Strict,
        _ => {
            issue(
                report,
                DiagnosticCode::InvalidField,
                path,
                "mandatory must be N, S or Y",
            );
            return None;
        }
    };
    let family = QuestionFamily::from_code(&source_type);
    if family == QuestionFamily::Unknown {
        report.push(
            Severity::Warning,
            DiagnosticCode::UnsupportedQuestionType,
            path.to_owned(),
            "unknown question type is preserved and cannot be executed",
        );
    }
    Some(Question {
        id,
        group_id,
        code,
        source_type,
        theme: xml
            .field(row, "question_theme_name")
            .unwrap_or("core")
            .to_owned(),
        family,
        mandatory,
        order,
        relevance: xml.field(row, "relevance").unwrap_or("1").to_owned(),
        randomization_group: xml
            .field(row, "randomization_group")
            .unwrap_or("")
            .to_owned(),
        validation: xml.field(row, "preg").unwrap_or("").to_owned(),
        other: xml.field(row, "other") == Some("Y"),
        same_default: match xml.field(row, "same_default") {
            None | Some("") => None,
            Some("0") => Some(false),
            Some("1") => Some(true),
            _ => {
                issue(
                    report,
                    DiagnosticCode::InvalidField,
                    format!("{path}.same_default"),
                    "same_default must be 0 or 1 when present",
                );
                None
            }
        },
        labels: Vec::new(),
        attributes: Vec::new(),
    })
}

fn read_subquestions(xml: &Document, model: &mut SurveySource, report: &mut Report) {
    let parents: BTreeMap<SourceId, SourceId> =
        model.questions.iter().map(|q| (q.id, q.group_id)).collect();
    let mut ids: BTreeSet<SourceId> = parents.keys().copied().collect();
    for (index, row) in rows(xml, "subquestions").into_iter().enumerate() {
        let path = format!("subquestions.rows[{index}]");
        survey_reference(xml, row, "sid", &path, model.survey_id, report);
        let Some(question) = read_subquestion(xml, row, &path, report) else {
            continue;
        };
        if parents.get(&question.parent_id) != Some(&question.group_id) {
            issue(
                report,
                DiagnosticCode::MissingReference,
                &path,
                "subquestion parent is missing or belongs to another group",
            );
        }
        if !ids.insert(question.id) {
            issue(
                report,
                DiagnosticCode::DuplicateId,
                &path,
                "duplicate question identifier",
            );
        }
        model.subquestions.push(question);
    }
}

fn read_subquestion(
    xml: &Document,
    row: usize,
    path: &str,
    report: &mut Report,
) -> Option<Subquestion> {
    Some(Subquestion {
        id: source_id(xml, row, "qid", path, report)?,
        parent_id: source_id(xml, row, "parent_qid", path, report)?,
        group_id: source_id(xml, row, "gid", path, report)?,
        code: text(xml, row, "title", path, report)?.to_owned(),
        scale: small_number(xml, row, "scale_id", path, report)?,
        order: xml
            .field(row, "question_order")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        relevance: xml.field(row, "relevance").unwrap_or("1").to_owned(),
        labels: Vec::new(),
    })
}
