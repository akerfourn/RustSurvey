use crate::{
    DiagnosticCode, Error, Report, SectionSummary, Severity, SurveySource, TARGET_SCHEMA,
    xml::Document,
};

mod defaults;
mod details;
mod edit;
mod fields;
mod project;
mod relevance;
mod tables;
mod translations;

pub(crate) use details::metadata;
pub(crate) use edit::replace_title;
pub(crate) use relevance::replace as replace_relevances;

const CHECKED: &[&str] = &[
    "surveys",
    "surveys_languagesettings",
    "groups",
    "questions",
    "subquestions",
    "answers",
    "group_l10ns",
    "question_l10ns",
    "answer_l10ns",
    "defaultvalues",
    "defaultvalue_l10ns",
];
const UNCHECKED: &[&str] = &[
    "assessments",
    "conditions",
    "question_attributes",
    "quota",
    "quota_members",
    "quota_languagesettings",
    "survey_url_parameters",
    "plugin_settings",
    "themes",
    "themes_inherited",
];

pub(crate) fn schema(xml: &Document) -> Result<u32, Error> {
    let root = &xml.elements[xml.root];
    if !root.unqualified
        || root.name != "document"
        || !root.text.trim().is_empty()
        || xml.field(xml.root, "LimeSurveyDocType") != Some("Survey")
    {
        return Err(Error::NotSurveyDocument);
    }
    xml.field(xml.root, "DBVersion")
        .and_then(|value| value.trim().parse::<u32>().ok())
        .filter(|&value| value > 0)
        .ok_or(Error::InvalidMetadata("DBVersion"))
}

pub(crate) fn inspect(xml: &Document, report: &mut Report) -> Option<SurveySource> {
    summarize(xml, report);
    if report.source_schema != TARGET_SCHEMA {
        report.push(
            Severity::Warning,
            DiagnosticCode::UnsupportedSchema,
            "DBVersion".to_owned(),
            "source is preserved; structural validation only supports schema 623",
        );
        return None;
    }
    for &section in CHECKED {
        tables::validate_table(xml, section, report);
        report.checked_sections.push(section);
    }
    let mut model = project::read_model(xml, report);
    translations::validate_translations(xml, &model, report);
    details::enrich(xml, &mut model);
    defaults::read(xml, &mut model, report);
    Some(model)
}

fn summarize(xml: &Document, report: &mut Report) {
    for (index, &id) in xml.elements[xml.root].children.iter().enumerate() {
        let element = &xml.elements[id];
        if element.unqualified
            && ["LimeSurveyDocType", "DBVersion", "languages"].contains(&element.name.as_str())
        {
            continue;
        }
        let count = xml
            .children_named(id, "rows")
            .map(|rows| xml.children_named(rows, "row").count())
            .sum();
        report.sections.push(SectionSummary {
            name: element.name.clone(),
            rows: count,
        });
        if element.unqualified && CHECKED.contains(&element.name.as_str()) {
            continue;
        }
        let code = if element.unqualified && UNCHECKED.contains(&element.name.as_str()) {
            DiagnosticCode::UncheckedSectionPreserved
        } else {
            DiagnosticCode::UnknownSectionPreserved
        };
        report.push(
            Severity::Warning,
            code,
            format!("document.children[{index}]"),
            "section is preserved verbatim but its semantics are not validated",
        );
    }
}
