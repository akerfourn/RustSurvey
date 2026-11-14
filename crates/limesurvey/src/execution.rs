//! Conservative inventory of source fields not projected into the native model.
use crate::{Report, xml::Document};
use rustsurvey_domain::ValidationIssue;
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionGaps {
    pub structural_errors: usize,
    pub unprojected_sections: usize,
    pub unprojected_columns: usize,
    pub missing_settings: usize,
    pub extended_elements: usize,
    pub issue_count: usize,
    pub issues: Vec<ValidationIssue>,
    #[serde(skip)]
    limit: usize,
}

impl ProjectionGaps {
    fn push(&mut self, path: String, message: &str) {
        self.issue_count += 1;
        if self.issues.len() < self.limit {
            self.issues.push(ValidationIssue {
                path,
                message: message.into(),
            });
        }
    }
}

pub(super) fn inspect(xml: &Document, report: &Report, limit: usize) -> ProjectionGaps {
    inspect_profile(
        xml,
        report,
        limit,
        |section| fields(section).map(<[_]>::to_vec),
        true,
    )
}

pub(crate) fn inspect_profile(
    xml: &Document,
    report: &Report,
    limit: usize,
    projected_fields: fn(&str) -> Option<Vec<&'static str>>,
    check_missing_settings: bool,
) -> ProjectionGaps {
    let mut gaps = ProjectionGaps {
        structural_errors: report.errors,
        unprojected_sections: 0,
        unprojected_columns: 0,
        missing_settings: 0,
        extended_elements: 0,
        issue_count: 0,
        issues: Vec::new(),
        limit,
    };
    if report.source_schema != crate::TARGET_SCHEMA || report.has_errors() {
        gaps.push(
            "source".into(),
            "La structure source doit être valide au schéma 623.",
        );
    }
    for (index, element) in xml.elements.iter().enumerate() {
        if element.has_attributes || !element.unqualified {
            gaps.extended_elements += 1;
            gaps.push(
                format!("source.elements[{index}]"),
                "Les attributs ou espaces de noms XML ne sont pas interprétés.",
            );
        }
    }
    for (index, &id) in xml.elements[xml.root].children.iter().enumerate() {
        let section = &xml.elements[id];
        if section.unqualified
            && ["LimeSurveyDocType", "DBVersion", "languages"].contains(&section.name.as_str())
        {
            continue;
        }
        let path = format!("source.sections[{index}]");
        let Some(projected) = projected_fields(&section.name).filter(|_| section.unqualified)
        else {
            let empty = empty_known_section(xml, id);
            if !empty {
                gaps.unprojected_sections += 1;
                gaps.push(
                    path,
                    &format!("Section source non projetée : {}. Ses données ne sont pas interprétées par le modèle exécutable.", technical_name(&section.name)),
                );
            }
            continue;
        };
        inspect_columns(xml, id, &projected, &path, &mut gaps);
    }
    if check_missing_settings {
        inspect_settings(xml, &mut gaps);
    }
    inspect_subquestion_attributes(xml, &mut gaps);
    gaps
}

fn inspect_settings(xml: &Document, gaps: &mut ProjectionGaps) {
    let row = xml
        .children_named(xml.root, "surveys")
        .next()
        .and_then(|table| xml.children_named(table, "rows").next())
        .and_then(|rows| xml.children_named(rows, "row").next());
    for name in [
        "format",
        "template",
        "anonymized",
        "usecookie",
        "allowsave",
        "tokenanswerspersistence",
        "alloweditaftercompletion",
        "datestamp",
        "ipaddr",
        "refurl",
        "savetimings",
        "allowprev",
        "shownoanswer",
        "questionindex",
        "navigationdelay",
        "assessments",
        "expires",
        "startdate",
        "allowregister",
        "autoredirect",
    ] {
        if row
            .and_then(|id| xml.field(id, name))
            .is_none_or(crate::settings::inherits_value)
        {
            gaps.missing_settings += 1;
            gaps.push(format!("source.surveys.{name}"), "Valeur effective absente ou héritée. Un réglage local par défaut ne doit pas remplacer un réglage source inconnu.");
        }
    }
}

fn inspect_subquestion_attributes(xml: &Document, gaps: &mut ProjectionGaps) {
    let mut parents = BTreeSet::new();
    for table in xml.children_named(xml.root, "questions") {
        for rows in xml.children_named(table, "rows") {
            for row in xml.children_named(rows, "row") {
                if let Some(qid) = xml.field(row, "qid") {
                    parents.insert(qid);
                }
            }
        }
    }
    for table in xml.children_named(xml.root, "question_attributes") {
        for rows in xml.children_named(table, "rows") {
            for (index, row) in xml.children_named(rows, "row").enumerate() {
                if xml
                    .field(row, "qid")
                    .is_none_or(|qid| !parents.contains(qid))
                {
                    gaps.push(format!("source.question_attributes.rows[{index}]"), "Attribut de sous-question ou référence inconnue non projeté dans le modèle exécutable.");
                }
            }
        }
    }
}

fn inspect_columns(
    xml: &Document,
    section: usize,
    projected: &[&str],
    path: &str,
    gaps: &mut ProjectionGaps,
) {
    if !xml.elements[section].text.trim().is_empty() {
        gaps.extended_elements += 1;
        gaps.push(path.into(), "Contenu de table XML non interprété.");
    }
    for &child in &xml.elements[section].children {
        let element = &xml.elements[child];
        if !element.unqualified || !["fields", "rows"].contains(&element.name.as_str()) {
            gaps.extended_elements += 1;
            gaps.push(
                format!("{path}.elements[{child}]"),
                "Élément de table XML non interprété.",
            );
        }
    }
    let mut names = BTreeSet::new();
    for fields in xml.children_named(section, "fields") {
        for (index, id) in xml.children_named(fields, "fieldname").enumerate() {
            let name = xml.elements[id].text.as_str();
            if names.insert(name) && !projected.contains(&name) {
                unprojected_column(gaps, name, format!("{path}.fields[{index}]"));
            }
        }
    }
    // Include actual fields as well: unchecked/undeclared content must not disappear.
    for rows in xml.children_named(section, "rows") {
        for (index, row) in xml.children_named(rows, "row").enumerate() {
            for &id in &xml.elements[row].children {
                let name = xml.elements[id].name.as_str();
                if names.insert(name) && !projected.contains(&name) {
                    unprojected_column(gaps, name, format!("{path}.rows[{index}].elements[{id}]"));
                }
            }
        }
    }
}

fn unprojected_column(gaps: &mut ProjectionGaps, name: &str, location: String) {
    gaps.unprojected_columns += 1;
    let label = technical_name(name);
    gaps.push(location, &format!("Champ source non projeté : {label}. Sa valeur, même vide ou nulle, ne prouve pas l'absence d'effet."));
}

fn technical_name(name: &str) -> &str {
    if name.len() <= 80
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        name
    } else {
        "nom à consulter dans l'original"
    }
}

fn empty_known_section(xml: &Document, id: usize) -> bool {
    let section = &xml.elements[id];
    let known = [
        "conditions",
        "defaultvalues",
        "defaultvalue_l10ns",
        "assessments",
        "quota",
        "quota_members",
        "quota_languagesettings",
        "survey_url_parameters",
        "plugin_settings",
        "themes",
        "themes_inherited",
    ];
    if !section.unqualified
        || !known.contains(&section.name.as_str())
        || !section.text.trim().is_empty()
    {
        return false;
    }
    section.children.iter().all(|&child| {
        let element = &xml.elements[child];
        element.unqualified
            && element.text.trim().is_empty()
            && match element.name.as_str() {
                "fields" => element.children.iter().all(|&field| {
                    let item = &xml.elements[field];
                    item.unqualified && item.name == "fieldname" && item.children.is_empty()
                }),
                "rows" => element.children.is_empty(),
                _ => false,
            }
    })
}

pub(crate) fn fields(section: &str) -> Option<&'static [&'static str]> {
    Some(match section {
        "surveys" => &["sid", "language", "additional_languages", "showwelcome"],
        "surveys_languagesettings" => &[
            "surveyls_survey_id",
            "surveyls_language",
            "surveyls_title",
            "surveyls_welcometext",
            "surveyls_endtext",
        ],
        "groups" => &[
            "gid",
            "sid",
            "group_order",
            "randomization_group",
            "grelevance",
        ],
        "group_l10ns" => &["id", "gid", "language", "group_name", "description"],
        "questions" => &[
            "qid",
            "parent_qid",
            "sid",
            "gid",
            "type",
            "title",
            "question_order",
            "scale_id",
            "other",
            "mandatory",
            "relevance",
            "randomization_group",
            "preg",
            "question_theme_name",
        ],
        "subquestions" => &[
            "qid",
            "parent_qid",
            "sid",
            "gid",
            "title",
            "question_order",
            "scale_id",
            "relevance",
        ],
        "question_l10ns" => &["id", "qid", "language", "question", "help"],
        "answers" => &["aid", "qid", "code", "sortorder", "scale_id"],
        "answer_l10ns" => &["id", "aid", "language", "answer"],
        "question_attributes" => &["qid", "attribute", "value", "language"],
        _ => return None,
    })
}
