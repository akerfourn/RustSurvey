use std::collections::BTreeMap;

use crate::{
    AnswerOption, LocalizedGroup, LocalizedQuestion, LocalizedSurveyMessages, LocalizedText,
    LocalizedTitle, QuestionAttribute, SourceId, SourceMetadata, SurveySource, xml::Document,
};

use super::tables::rows;

fn id(xml: &Document, row: usize, field: &str) -> Option<SourceId> {
    xml.field(row, field)?.parse().ok().map(SourceId)
}

pub(super) fn enrich(xml: &Document, model: &mut SurveySource) {
    for row in rows(xml, "surveys_languagesettings") {
        if let Some(language) = xml.field(row, "surveyls_language") {
            model.messages.push(LocalizedSurveyMessages {
                language: language.to_owned(),
                welcome: xml.field(row, "surveyls_welcometext").map(str::to_owned),
                end: xml.field(row, "surveyls_endtext").map(str::to_owned),
            });
        }
    }
    let mut groups: BTreeMap<SourceId, Vec<LocalizedGroup>> = BTreeMap::new();
    for row in rows(xml, "group_l10ns") {
        if let (Some(id), Some(language)) = (id(xml, row, "gid"), xml.field(row, "language")) {
            groups.entry(id).or_default().push(LocalizedGroup {
                language: language.to_owned(),
                name: xml.field(row, "group_name").map(str::to_owned),
                description: xml.field(row, "description").map(str::to_owned),
            });
        }
    }
    for group in &mut model.groups {
        group.labels = groups.remove(&group.id).unwrap_or_default();
    }
    enrich_questions(xml, model);
    enrich_answers(xml, model);
}

fn enrich_questions(xml: &Document, model: &mut SurveySource) {
    let mut labels: BTreeMap<SourceId, Vec<LocalizedQuestion>> = BTreeMap::new();
    for row in rows(xml, "question_l10ns") {
        if let (Some(id), Some(language)) = (id(xml, row, "qid"), xml.field(row, "language")) {
            labels.entry(id).or_default().push(LocalizedQuestion {
                language: language.to_owned(),
                text: xml.field(row, "question").map(str::to_owned),
                help: xml.field(row, "help").map(str::to_owned),
            });
        }
    }
    let mut attributes: BTreeMap<SourceId, Vec<QuestionAttribute>> = BTreeMap::new();
    for row in rows(xml, "question_attributes") {
        if let (Some(id), Some(name)) = (id(xml, row, "qid"), xml.field(row, "attribute")) {
            attributes.entry(id).or_default().push(QuestionAttribute {
                name: name.to_owned(),
                value: xml.field(row, "value").unwrap_or("").to_owned(),
                language: xml.field(row, "language").map(str::to_owned),
            });
        }
    }
    for question in &mut model.questions {
        question.labels = labels.remove(&question.id).unwrap_or_default();
        question.attributes = attributes.remove(&question.id).unwrap_or_default();
    }
    for question in &mut model.subquestions {
        question.labels = labels.remove(&question.id).unwrap_or_default();
    }
}

fn enrich_answers(xml: &Document, model: &mut SurveySource) {
    let mut labels: BTreeMap<SourceId, Vec<LocalizedText>> = BTreeMap::new();
    for row in rows(xml, "answer_l10ns") {
        if let (Some(id), Some(language)) = (id(xml, row, "aid"), xml.field(row, "language")) {
            labels.entry(id).or_default().push(LocalizedText {
                language: language.to_owned(),
                text: xml.field(row, "answer").map(str::to_owned),
            });
        }
    }
    for row in rows(xml, "answers") {
        if let (Some(id), Some(question_id), Some(code)) = (
            id(xml, row, "aid"),
            id(xml, row, "qid"),
            xml.field(row, "code"),
        ) {
            model.answers.push(AnswerOption {
                id,
                question_id,
                code: code.to_owned(),
                order: xml
                    .field(row, "sortorder")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0),
                scale: xml
                    .field(row, "scale_id")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0),
                labels: labels.remove(&id).unwrap_or_default(),
            });
        }
    }
}

pub(crate) fn metadata(xml: &Document) -> SourceMetadata {
    let survey = rows(xml, "surveys").first().copied();
    let mut metadata = SourceMetadata {
        survey_id: survey
            .and_then(|row| xml.field(row, "sid"))
            .map(str::to_owned),
        base_language: survey
            .and_then(|row| xml.field(row, "language"))
            .map(str::to_owned),
        ..SourceMetadata::default()
    };
    if let Some(languages) = xml.children_named(xml.root, "languages").next() {
        for id in xml.children_named(languages, "language") {
            let entry = &xml.elements[id];
            if entry.children.is_empty() {
                metadata.languages.push(entry.text.clone());
            }
        }
    }
    for row in rows(xml, "surveys_languagesettings") {
        if let Some(language) = xml.field(row, "surveyls_language") {
            metadata.titles.push(LocalizedTitle {
                language: language.to_owned(),
                value: xml.field(row, "surveyls_title").map(str::to_owned),
            });
        }
    }
    metadata
}
