mod labels;

use super::layout::{Layout, base_theme, question_type};
use crate::{Error, xml_write::Xml};
use rustsurvey_domain::navigation::PageFormat;
use rustsurvey_domain::{Mandatory, QuestionKind};

pub(super) fn write(xml: &mut Xml, layout: &Layout<'_>) -> Result<(), Error> {
    survey(xml, layout)?;
    groups(xml, layout)?;
    questions(xml, layout)?;
    subquestions(xml, layout)?;
    answers(xml, layout)?;
    attributes(xml, layout)?;
    labels::write(xml, layout)?;
    super::defaults::write(xml, layout)
}

fn flag(value: bool) -> &'static str {
    if value { "Y" } else { "N" }
}

fn survey(xml: &mut Xml, layout: &Layout<'_>) -> Result<(), Error> {
    xml.table(
        "surveys",
        &[
            "sid",
            "language",
            "additional_languages",
            "format",
            "template",
            "anonymized",
            "showwelcome",
            "showprogress",
            "allowprev",
            "datestamp",
            "ipaddr",
            "refurl",
            "savetimings",
            "usecookie",
            "allowsave",
            "tokenanswerspersistence",
            "alloweditaftercompletion",
        ],
    )?;
    let additional = layout
        .languages
        .iter()
        .copied()
        .filter(|language| *language != layout.definition.language)
        .collect::<Vec<_>>()
        .join(" ");
    xml.row(&[
        &layout.survey_id.to_string(),
        &layout.definition.language,
        &additional,
        match layout.navigation.format {
            PageFormat::Group => "G",
            PageFormat::Question => "S",
            PageFormat::All => "A",
        },
        "vanilla",
        "Y",
        flag(layout.definition.show_welcome.unwrap_or(true)),
        flag(layout.navigation.show_progress),
        "Y",
        "N",
        "N",
        "N",
        "N",
        "N",
        "Y",
        "N",
        "N",
    ])?;
    xml.end_table("surveys")
}

fn groups(xml: &mut Xml, layout: &Layout<'_>) -> Result<(), Error> {
    xml.table(
        "groups",
        &[
            "gid",
            "sid",
            "group_order",
            "randomization_group",
            "grelevance",
        ],
    )?;
    for group in &layout.groups {
        xml.row(&[
            &group.id.to_string(),
            &layout.survey_id.to_string(),
            &group.order.to_string(),
            &group.source.randomization_group,
            &group.source.relevance,
        ])?;
    }
    xml.end_table("groups")
}

fn questions(xml: &mut Xml, layout: &Layout<'_>) -> Result<(), Error> {
    xml.table(
        "questions",
        &[
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
            "same_default",
        ],
    )?;
    for question in &layout.questions {
        let source = question.source;
        let mandatory = match source.mandatory {
            Mandatory::Strict => "Y",
            Mandatory::Soft => "S",
            Mandatory::No => "N",
        };
        xml.row(&[
            &question.id.to_string(),
            "0",
            &layout.survey_id.to_string(),
            &question.group_id.to_string(),
            question_type(source.kind).ok_or(Error::InvalidStructure)?,
            &source.code,
            &question.order.to_string(),
            "0",
            flag(source.other),
            mandatory,
            &source.relevance,
            &source.randomization_group,
            &source.validation,
            base_theme(source.kind),
            if source
                .defaults
                .as_ref()
                .is_some_and(|defaults| defaults.same_for_all_languages)
            {
                "1"
            } else {
                "0"
            },
        ])?;
    }
    xml.end_table("questions")
}

fn subquestions(xml: &mut Xml, layout: &Layout<'_>) -> Result<(), Error> {
    xml.table(
        "subquestions",
        &[
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
            "same_default",
        ],
    )?;
    for question in &layout.questions {
        for row in &question.subquestions {
            xml.row(&[
                &row.id.to_string(),
                &question.id.to_string(),
                &layout.survey_id.to_string(),
                &question.group_id.to_string(),
                question_type(question.source.kind).ok_or(Error::InvalidStructure)?,
                &row.source.code,
                &row.order.to_string(),
                &row.source.scale.to_string(),
                "N",
                "N",
                &row.source.relevance,
                "0",
            ])?;
        }
    }
    xml.end_table("subquestions")
}

fn answers(xml: &mut Xml, layout: &Layout<'_>) -> Result<(), Error> {
    xml.table(
        "answers",
        &[
            "aid",
            "qid",
            "code",
            "sortorder",
            "assessment_value",
            "scale_id",
        ],
    )?;
    for question in &layout.questions {
        for answer in &question.answers {
            xml.row(&[
                &answer.id.to_string(),
                &question.id.to_string(),
                &answer.source.code,
                &answer.order.to_string(),
                "0",
                &answer.source.scale.to_string(),
            ])?;
        }
    }
    xml.end_table("answers")
}

fn attributes(xml: &mut Xml, layout: &Layout<'_>) -> Result<(), Error> {
    xml.table(
        "question_attributes",
        &["qid", "attribute", "value", "language"],
    )?;
    for question in &layout.questions {
        for attribute in &question.source.attributes {
            let mut value = attribute.value.clone();
            if question.source.kind == QuestionKind::Equation
                && attribute.name == "equation"
                && !value.is_empty()
            {
                value = format!("{{{}}}", value.trim());
            }
            xml.row(&[
                &question.id.to_string(),
                &attribute.name,
                &value,
                attribute.language.as_deref().unwrap_or(""),
            ])?;
        }
    }
    xml.end_table("question_attributes")
}
