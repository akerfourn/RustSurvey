use super::{Error, Layout, Xml};
use rustsurvey_domain::Translation;

fn text<'a>(
    translations: &'a [Translation],
    language: &str,
    base: &str,
    fallback: &'a str,
    help: bool,
) -> &'a str {
    if language == base {
        return fallback;
    }
    let translation = translations.iter().find(|item| item.language == language);
    let value = if help {
        translation.and_then(|item| item.help.as_deref())
    } else {
        translation.and_then(|item| item.text.as_deref())
    };
    value.unwrap_or(fallback)
}

pub(super) fn write(xml: &mut Xml, layout: &Layout<'_>) -> Result<(), Error> {
    survey(xml, layout)?;
    groups(xml, layout)?;
    questions(xml, layout)?;
    answers(xml, layout)
}

fn survey(xml: &mut Xml, layout: &Layout<'_>) -> Result<(), Error> {
    xml.table(
        "surveys_languagesettings",
        &[
            "surveyls_survey_id",
            "surveyls_language",
            "surveyls_title",
            "surveyls_description",
            "surveyls_welcometext",
            "surveyls_endtext",
            "surveyls_url",
            "surveyls_urldescription",
        ],
    )?;
    let definition = layout.definition;
    for language in &layout.languages {
        let translation = definition
            .translations
            .iter()
            .find(|item| item.language == *language && item.language != definition.language);
        let title = translation
            .and_then(|item| item.title.as_deref())
            .unwrap_or(&definition.title);
        let welcome = translation
            .and_then(|item| item.welcome.as_deref())
            .unwrap_or(&definition.welcome);
        let end = translation
            .and_then(|item| item.end.as_deref())
            .unwrap_or(&definition.end);
        xml.row(&[
            &layout.survey_id.to_string(),
            language,
            title,
            "",
            welcome,
            end,
            "",
            "",
        ])?;
    }
    xml.end_table("surveys_languagesettings")
}

fn groups(xml: &mut Xml, layout: &Layout<'_>) -> Result<(), Error> {
    xml.table(
        "group_l10ns",
        &["gid", "language", "group_name", "description"],
    )?;
    for group in &layout.groups {
        for language in &layout.languages {
            let title = text(
                &group.source.translations,
                language,
                &layout.definition.language,
                &group.source.title,
                false,
            );
            let description = text(
                &group.source.translations,
                language,
                &layout.definition.language,
                &group.source.description,
                true,
            );
            xml.row(&[&group.id.to_string(), language, title, description])?;
        }
    }
    xml.end_table("group_l10ns")
}

fn questions(xml: &mut Xml, layout: &Layout<'_>) -> Result<(), Error> {
    xml.table("question_l10ns", &["qid", "language", "question", "help"])?;
    for question in &layout.questions {
        for language in &layout.languages {
            let title = text(
                &question.source.translations,
                language,
                &layout.definition.language,
                &question.source.title,
                false,
            );
            let help = text(
                &question.source.translations,
                language,
                &layout.definition.language,
                &question.source.help,
                true,
            );
            xml.row(&[&question.id.to_string(), language, title, help])?;
        }
        for row in &question.subquestions {
            for language in &layout.languages {
                let title = text(
                    &row.source.translations,
                    language,
                    &layout.definition.language,
                    &row.source.label,
                    false,
                );
                let help = text(
                    &row.source.translations,
                    language,
                    &layout.definition.language,
                    &row.source.help,
                    true,
                );
                xml.row(&[&row.id.to_string(), language, title, help])?;
            }
        }
    }
    xml.end_table("question_l10ns")
}

fn answers(xml: &mut Xml, layout: &Layout<'_>) -> Result<(), Error> {
    xml.table("answer_l10ns", &["aid", "language", "answer"])?;
    for question in &layout.questions {
        for answer in &question.answers {
            for language in &layout.languages {
                let label = text(
                    &answer.source.translations,
                    language,
                    &layout.definition.language,
                    &answer.source.label,
                    false,
                );
                xml.row(&[&answer.id.to_string(), language, label])?;
            }
        }
    }
    xml.end_table("answer_l10ns")
}
