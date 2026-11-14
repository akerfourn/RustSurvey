use super::{NativeExportError, layout::question_type};
use crate::SourceId;
use rustsurvey_domain::{Definition, Question, QuestionKind, ValidationIssue};
use std::collections::BTreeSet;

fn issue(issues: &mut Vec<ValidationIssue>, path: &str, message: &str) {
    if issues.len() < 100 {
        issues.push(ValidationIssue {
            path: path.into(),
            message: message.into(),
        });
    }
}

pub(super) fn definition(definition: &Definition, id: SourceId) -> Result<(), NativeExportError> {
    let mut issues = definition.validate();
    issues.extend(rustsurvey_domain::rules::validate(definition, None));
    if !(1..=999_999_999).contains(&id.0) {
        issue(
            &mut issues,
            "surveyId",
            "L'identifiant d'échange doit être compris entre 1 et 999999999.",
        );
    }
    let languages: BTreeSet<_> = std::iter::once(definition.language.as_str())
        .chain(definition.languages.iter().map(String::as_str))
        .collect();
    for translation in &definition.translations {
        if !languages.contains(translation.language.as_str()) {
            issue(
                &mut issues,
                "translations",
                "Les traductions inactives nécessitent une archive native pour éviter leur perte.",
            );
        }
    }
    for group in &definition.groups {
        if group.source.is_some() {
            issue(
                &mut issues,
                &group.id,
                "Un import doit conserver son original et utiliser le réexport LSS ciblé.",
            );
        }
        for translation in &group.translations {
            if !languages.contains(translation.language.as_str()) {
                issue(
                    &mut issues,
                    &group.id,
                    "Une traduction appartient à une langue inactive.",
                );
            }
        }
        for question in &group.questions {
            validate_question(question, &languages, &mut issues);
        }
    }
    issues.truncate(100);
    if issues.is_empty() {
        Ok(())
    } else {
        Err(NativeExportError::Unsupported(issues))
    }
}

fn validate_question(
    question: &Question,
    languages: &BTreeSet<&str>,
    issues: &mut Vec<ValidationIssue>,
) {
    validate_codes(question, issues);
    let filters_supported = matches!(
        question.kind,
        QuestionKind::SingleChoice
            | QuestionKind::MultipleChoice
            | QuestionKind::MultipleChoiceComment
            | QuestionKind::MultipleText
            | QuestionKind::MultipleNumbers
            | QuestionKind::ArrayFive
            | QuestionKind::ArrayTen
            | QuestionKind::ArrayYesNo
            | QuestionKind::ArrayIncrease
            | QuestionKind::ArrayChoice
            | QuestionKind::DualScale
            | QuestionKind::NumberMatrix
            | QuestionKind::TextMatrix
            | QuestionKind::Ranking
    );
    if !filters_supported
        && question.attributes.iter().any(|attribute| {
            ["array_filter", "array_filter_exclude"].contains(&attribute.name.as_str())
                && !attribute.value.trim().is_empty()
        })
    {
        issue(
            issues,
            &question.id,
            "LimeSurvey 6.5.14 n'applique pas ce filtre à ce type de question ; il ne peut pas être exporté fidèlement.",
        );
    }
    if question.source.is_some() {
        issue(
            issues,
            &question.id,
            "Un import doit utiliser le réexport conservant ses données source.",
        );
    }
    if question_type(question.kind).is_none() {
        issue(
            issues,
            &question.id,
            "Type non représentable dans LimeSurvey 6.5.14.",
        );
    }
    if !question.media.is_empty() {
        issue(
            issues,
            &question.id,
            "Les images nécessitent un paquet de ressources ; un LSS seul ne les transporte pas.",
        );
    }
    if !["", "core"].contains(&question.theme.as_str()) {
        issue(
            issues,
            &question.id,
            "Le transfert de thèmes personnalisés n'est pas encore pris en charge.",
        );
    }
    for translation in &question.translations {
        if !languages.contains(translation.language.as_str()) {
            issue(
                issues,
                &question.id,
                "Une traduction appartient à une langue inactive.",
            );
        }
    }
    for answer in &question.choices {
        if answer.code.is_empty()
            || answer.code.len() > 5
            || !answer.code.bytes().all(|byte| byte.is_ascii_alphanumeric())
        {
            issue(
                issues,
                &answer.id,
                "Un code de réponse LimeSurvey doit contenir 1 à 5 caractères alphanumériques.",
            );
        }
        if !answer.help.is_empty()
            || answer
                .translations
                .iter()
                .any(|item| item.help.as_deref().is_some_and(|help| !help.is_empty()))
        {
            issue(
                issues,
                &answer.id,
                "Une aide d'option de réponse n'a pas de champ LSS équivalent.",
            );
        }
        if !["", "1"].contains(&answer.relevance.trim()) {
            issue(
                issues,
                &answer.id,
                "Une condition propre à une option de réponse n'est pas représentable par ce profil.",
            );
        }
    }
    for choice in question.choices.iter().chain(&question.subquestions) {
        if choice.source.is_some() {
            issue(
                issues,
                &choice.id,
                "Un alias d'import doit conserver sa structure source.",
            );
        }
        if choice
            .translations
            .iter()
            .any(|item| !languages.contains(item.language.as_str()))
        {
            issue(
                issues,
                &choice.id,
                "Une traduction appartient à une langue inactive.",
            );
        }
    }
    let mut seen = BTreeSet::new();
    for attribute in &question.attributes {
        let language = attribute.language.as_deref().unwrap_or("");
        if !seen.insert((&attribute.name, language)) {
            issue(
                issues,
                &question.id,
                "Attribut déclaré plusieurs fois pour la même langue.",
            );
        }
        if !language.is_empty() && !languages.contains(language) {
            issue(
                issues,
                &question.id,
                "Un attribut appartient à une langue inactive.",
            );
        }
        if !known_attribute(&attribute.name) {
            issue(
                issues,
                &question.id,
                "Un attribut n'appartient pas au profil d'export LSS vérifié.",
            );
        }
    }
    if question.kind == QuestionKind::Equation
        && question
            .attributes
            .iter()
            .all(|item| item.name != "equation" || item.value.is_empty())
    {
        // A question-text equation already uses LimeSurvey's braced-text representation.
        if !(question.title.trim().starts_with('{') && question.title.trim().ends_with('}')) {
            issue(
                issues,
                &question.id,
                "Une équation sans attribut explicite doit être entourée d'accolades.",
            );
        }
    }
}

fn validate_codes(question: &Question, issues: &mut Vec<ValidationIssue>) {
    if question.other
        && !matches!(
            question.kind,
            QuestionKind::SingleChoice
                | QuestionKind::Dropdown
                | QuestionKind::MultipleChoice
                | QuestionKind::MultipleChoiceComment
        )
    {
        issue(
            issues,
            &question.id,
            "L'option autre n'est pas représentable pour ce type LimeSurvey.",
        );
    }
    let code = question.code.as_bytes();
    let reserved = [
        "LANG", "SID", "SAVEDID", "TOKEN", "QID", "GID", "SGQ", "self", "that", "this",
    ];
    if code.len() > 20
        || !code.first().is_some_and(u8::is_ascii_alphabetic)
        || !code.iter().all(u8::is_ascii_alphanumeric)
        || reserved.contains(&question.code.as_str())
    {
        issue(
            issues,
            &question.id,
            "Le code serait renommé par LimeSurvey : utiliser 1 à 20 caractères alphanumériques, une lettre initiale et aucun mot réservé.",
        );
    }
    let mut codes = BTreeSet::new();
    for row in &question.subquestions {
        if row.code.is_empty()
            || row.code.len() > 20
            || !row.code.bytes().all(|byte| byte.is_ascii_alphanumeric())
            || row.code == "time"
            || (question.other && row.code.eq_ignore_ascii_case("other"))
            || (question.kind == QuestionKind::MultipleChoiceComment
                && row.code.ends_with("comment"))
            || !codes.insert((row.scale, row.code.to_ascii_lowercase()))
        {
            issue(
                issues,
                &row.id,
                "Le code de sous-question serait modifié ou ambigu dans LimeSurvey.",
            );
        }
    }
}

fn known_attribute(name: &str) -> bool {
    matches!(
        name,
        "hidden"
            | "equation"
            | "em_validation_q"
            | "em_validation_q_tip"
            | "min_num_value_n"
            | "max_num_value_n"
            | "num_value_int_only"
            | "min_num_value"
            | "max_num_value"
            | "equals_num_value"
            | "min_answers"
            | "max_answers"
            | "maximum_chars"
            | "numbers_only"
            | "prefix"
            | "suffix"
            | "placeholder"
            | "display_rows"
            | "display_columns"
            | "input_size"
            | "array_filter"
            | "array_filter_exclude"
            | "array_filter_style"
            | "exclude_all_others"
            | "exclude_all_others_auto"
            | "other_replace_text"
            | "other_numbers_only"
            | "other_comment_mandatory"
            | "slider_layout"
            | "slider_min"
            | "slider_max"
            | "slider_accuracy"
            | "slider_showminmax"
            | "slider_reset"
            | "multiflexible_checkbox"
            | "input_boxes"
            | "multiflexible_min"
            | "multiflexible_max"
            | "multiflexible_step"
            | "dualscale_headerA"
            | "dualscale_headerB"
            | "use_dropdown"
            | "commented_checkbox"
            | "commented_checkbox_auto"
            | "date_format"
            | "max_subquestions"
    )
}
