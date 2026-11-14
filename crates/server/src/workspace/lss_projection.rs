use super::{AppError, Runtime, runtime};
use rustsurvey_domain as domain;
use rustsurvey_limesurvey as lss;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DefaultProjection {
    profile: &'static str,
    status: &'static str,
    projected_values: usize,
    source_values: usize,
    issue_count: usize,
    issues: Vec<domain::ValidationIssue>,
}

pub(super) struct Projection {
    pub definition: domain::Definition,
    pub defaults: DefaultProjection,
}

fn source(id: lss::SourceId, kind: Option<&str>) -> Option<domain::SourceReference> {
    Some(domain::SourceReference {
        id: id.0.to_string(),
        kind: kind.map(str::to_owned),
    })
}
fn translations(labels: &[lss::LocalizedQuestion]) -> Vec<domain::Translation> {
    labels
        .iter()
        .map(|label| domain::Translation {
            language: label.language.clone(),
            text: label.text.clone(),
            help: label.help.clone(),
        })
        .collect()
}
fn localized<'a>(
    labels: &'a [lss::LocalizedQuestion],
    language: &str,
) -> Option<&'a lss::LocalizedQuestion> {
    labels
        .iter()
        .find(|label| label.language == language)
        .or_else(|| labels.first())
}
pub(super) fn project(
    study: &lss::SurveySource,
    runtime: &dyn Runtime,
) -> Result<Projection, AppError> {
    let language = study
        .base_language
        .clone()
        .unwrap_or_else(|| "fr".to_owned());
    let messages = study
        .messages
        .iter()
        .find(|item| item.language == language)
        .or_else(|| study.messages.first());
    let title = study
        .titles
        .iter()
        .find(|item| item.language == language)
        .or_else(|| study.titles.first())
        .and_then(|item| item.value.clone())
        .unwrap_or_else(|| "Questionnaire importé".to_owned());
    let mut definition = domain::Definition {
        schema_version: domain::SCHEMA_VERSION,
        title,
        language: language.clone(),
        welcome: messages
            .and_then(|item| item.welcome.clone())
            .unwrap_or_default(),
        end: messages
            .and_then(|item| item.end.clone())
            .unwrap_or_default(),
        languages: study.languages.clone(),
        show_welcome: study.show_welcome,
        groups: Vec::new(),
        translations: study
            .languages
            .iter()
            .map(|language| {
                let message = study
                    .messages
                    .iter()
                    .find(|item| item.language == *language);
                domain::SurveyTranslation {
                    language: language.clone(),
                    title: study
                        .titles
                        .iter()
                        .find(|item| item.language == *language)
                        .and_then(|item| item.value.clone()),
                    welcome: message.and_then(|item| item.welcome.clone()),
                    end: message.and_then(|item| item.end.clone()),
                }
            })
            .collect(),
    };
    let mut groups: Vec<_> = study.groups.iter().collect();
    groups.sort_by_key(|item| item.order);
    for group in groups {
        let label = group
            .labels
            .iter()
            .find(|item| item.language == language)
            .or_else(|| group.labels.first());
        let mut mapped = domain::Group {
            id: runtime::identifier(runtime)?,
            title: label
                .and_then(|item| item.name.clone())
                .unwrap_or_else(|| "Groupe".to_owned()),
            description: label
                .and_then(|item| item.description.clone())
                .unwrap_or_default(),
            relevance: group.relevance.clone(),
            randomization_group: group.randomization_group.clone(),
            source: source(group.id, None),
            translations: group
                .labels
                .iter()
                .map(|item| domain::Translation {
                    language: item.language.clone(),
                    text: item.name.clone(),
                    help: item.description.clone(),
                })
                .collect(),
            questions: Vec::new(),
        };
        let mut questions: Vec<_> = study
            .questions
            .iter()
            .filter(|item| item.group_id == group.id)
            .collect();
        questions.sort_by_key(|item| item.order);
        for question in questions {
            mapped
                .questions
                .push(project_question(study, question, &language, runtime)?);
        }
        definition.groups.push(mapped);
    }
    let defaults = apply_defaults(study, &mut definition)?;
    Ok(Projection {
        definition,
        defaults,
    })
}

fn apply_defaults(
    study: &lss::SurveySource,
    definition: &mut domain::Definition,
) -> Result<DefaultProjection, AppError> {
    let sources: std::collections::BTreeMap<_, _> = study
        .questions
        .iter()
        .map(|source| (source.id.0.to_string(), source))
        .collect();
    let mut issues = Vec::new();
    let mut issue_count = 0;
    let mut projected_values = 0;
    for question in definition
        .groups
        .iter_mut()
        .flat_map(|group| &mut group.questions)
    {
        let source = question
            .source
            .as_ref()
            .and_then(|identity| sources.get(&identity.id))
            .ok_or_else(AppError::internal)?;
        match lss::project_question_defaults(study, source, question) {
            Ok(defaults) => {
                projected_values += defaults
                    .as_ref()
                    .map_or(0, |defaults| defaults.values.len());
                question.defaults = defaults;
            }
            Err(errors) => {
                issue_count += errors.len();
                let remaining = 128_usize.saturating_sub(issues.len());
                issues.extend(errors.into_iter().take(remaining));
            }
        }
    }
    let status = if issue_count == 0 {
        "complete"
    } else {
        "partial"
    };
    Ok(DefaultProjection {
        profile: "source-defaults-v1",
        status,
        projected_values,
        issue_count,
        source_values: study
            .default_values
            .iter()
            .map(|value| value.values.len())
            .sum(),
        issues,
    })
}
fn project_question(
    study: &lss::SurveySource,
    question: &lss::Question,
    language: &str,
    runtime: &dyn Runtime,
) -> Result<domain::Question, AppError> {
    let label = localized(&question.labels, language);
    let mut mapped = domain::Question {
        id: runtime::identifier(runtime)?,
        code: question.code.clone(),
        title: label.and_then(|item| item.text.clone()).unwrap_or_default(),
        help: label.and_then(|item| item.help.clone()).unwrap_or_default(),
        kind: kind(&question.source_type),
        mandatory: match question.mandatory {
            lss::Mandatory::No => domain::Mandatory::No,
            lss::Mandatory::Soft => domain::Mandatory::Soft,
            lss::Mandatory::Strict => domain::Mandatory::Strict,
        },
        choices: Vec::new(),
        subquestions: Vec::new(),
        relevance: question.relevance.clone(),
        translations: translations(&question.labels),
        attributes: question
            .attributes
            .iter()
            .map(|item| domain::Attribute {
                name: item.name.clone(),
                value: attribute_value(question, item),
                language: item.language.clone(),
            })
            .collect(),
        other: question.other,
        validation: question.validation.clone(),
        randomization_group: question.randomization_group.clone(),
        theme: question.theme.clone(),
        source: source(question.id, Some(&question.source_type)),
        media: Vec::new(),
        defaults: None,
    };
    let mut choices: Vec<_> = study
        .answers
        .iter()
        .filter(|item| item.question_id == question.id)
        .collect();
    choices.sort_by_key(|item| (item.scale, item.order));
    for choice in choices {
        let label = choice
            .labels
            .iter()
            .find(|item| item.language == language)
            .or_else(|| choice.labels.first());
        mapped.choices.push(domain::Choice {
            id: runtime::identifier(runtime)?,
            code: choice.code.clone(),
            label: label.and_then(|item| item.text.clone()).unwrap_or_default(),
            help: String::new(),
            scale: choice.scale,
            relevance: domain::always(),
            source: source(choice.id, None),
            translations: choice
                .labels
                .iter()
                .map(|item| domain::Translation {
                    language: item.language.clone(),
                    text: item.text.clone(),
                    help: None,
                })
                .collect(),
        });
    }
    let mut rows: Vec<_> = study
        .subquestions
        .iter()
        .filter(|item| item.parent_id == question.id)
        .collect();
    rows.sort_by_key(|item| (item.scale, item.order));
    for row in rows {
        let label = localized(&row.labels, language);
        mapped.subquestions.push(domain::Choice {
            id: runtime::identifier(runtime)?,
            code: row.code.clone(),
            label: label.and_then(|item| item.text.clone()).unwrap_or_default(),
            help: label.and_then(|item| item.help.clone()).unwrap_or_default(),
            scale: row.scale,
            relevance: row.relevance.clone(),
            source: source(row.id, None),
            translations: translations(&row.labels),
        });
    }
    Ok(mapped)
}
fn attribute_value(question: &lss::Question, attribute: &lss::QuestionAttribute) -> String {
    if question.source_type == "*" && attribute.name == "equation" {
        let expression = attribute
            .value
            .trim()
            .strip_prefix('{')
            .and_then(|value| value.strip_suffix('}'));
        if let Some(expression) = expression
            && domain::rules::Expression::compile(expression).is_ok()
        {
            // LimeSurvey stores a braced template; native equation attributes store its expression.
            return expression.to_owned();
        }
    }
    attribute.value.clone()
}
fn kind(code: &str) -> domain::QuestionKind {
    use domain::QuestionKind::*;
    match code {
        "S" => ShortText,
        "T" => LongText,
        "U" => HugeText,
        "N" => Number,
        "D" => Date,
        "L" => SingleChoice,
        "!" => Dropdown,
        "M" => MultipleChoice,
        "Y" => YesNo,
        "X" => Information,
        "5" => RatingFive,
        "G" => Gender,
        "O" => SingleChoiceComment,
        "P" => MultipleChoiceComment,
        "Q" => MultipleText,
        "K" => MultipleNumbers,
        "A" => ArrayFive,
        "B" => ArrayTen,
        "C" => ArrayYesNo,
        "E" => ArrayIncrease,
        "F" => ArrayChoice,
        "H" => ArrayColumn,
        "1" => DualScale,
        ":" => NumberMatrix,
        ";" => TextMatrix,
        "R" => Ranking,
        "I" => Language,
        "*" => Equation,
        "|" => FileUpload,
        _ => Unsupported,
    }
}
