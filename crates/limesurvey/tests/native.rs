use rustsurvey_domain::{Definition, QuestionMedia};
use rustsurvey_limesurvey::{
    Error, Limits, LssDocument, NativeExportError, SourceId, export_native,
};

fn definition() -> Definition {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../examples/rules/collection-forms.json"
    ))
    .unwrap();
    serde_json::from_value(corpus["definition"].clone()).unwrap()
}

#[test]
fn test_native_export_should_refuse_filter_targets_that_the_reference_would_ignore() {
    use rustsurvey_domain::QuestionKind::{ArrayColumn, Dropdown, ShortText, SingleChoiceComment};
    for kind in [ArrayColumn, Dropdown, SingleChoiceComment, ShortText] {
        for name in ["array_filter", "array_filter_exclude"] {
            let mut definition = definition();
            let question = definition.groups[0]
                .questions
                .iter_mut()
                .find(|question| question.kind == kind)
                .unwrap();
            question.attributes.push(rustsurvey_domain::Attribute {
                name: name.into(),
                value: "M".into(),
                language: None,
            });
            let NativeExportError::Unsupported(issues) =
                export_native(&definition, SourceId(701), Limits::default())
                    .err()
                    .expect("unsupported filter must refuse export")
            else {
                panic!("expected a compatibility diagnostic")
            };
            assert!(
                issues
                    .iter()
                    .any(|issue| issue.message.contains("n'applique pas ce filtre")),
                "{kind:?}: {issues:?}"
            );
        }
    }
}

#[test]
fn test_native_export_should_preserve_all_28_types_axes_choices_and_rules_in_schema_623() {
    let definition = definition();
    let exported = export_native(&definition, SourceId(701), Limits::default()).unwrap();
    assert_eq!(
        export_native(&definition, SourceId(701), Limits::default())
            .unwrap()
            .xml,
        exported.xml
    );
    let parsed = LssDocument::parse(&exported.xml, Limits::default()).unwrap();
    assert!(!parsed.report().has_errors());
    let model = parsed.source_model().unwrap();
    assert_eq!(model.survey_id, Some(SourceId(701)));
    assert_eq!(model.questions.len(), 28);
    assert_eq!(model.groups.len(), definition.groups.len());
    let questions: Vec<_> = definition
        .groups
        .iter()
        .flat_map(|group| &group.questions)
        .collect();
    for (source, question) in questions.iter().zip(&model.questions) {
        assert_eq!(question.code, source.code);
        assert_eq!(question.relevance, source.relevance);
        assert_eq!(
            question
                .labels
                .iter()
                .find(|label| label.language == definition.language)
                .unwrap()
                .text
                .as_ref(),
            Some(&source.title)
        );
        let rows: Vec<_> = model
            .subquestions
            .iter()
            .filter(|row| row.parent_id == question.id)
            .collect();
        assert_eq!(rows.len(), source.subquestions.len());
        for (row, original) in rows.iter().zip(&source.subquestions) {
            assert_eq!(
                (&row.code, row.scale, &row.relevance),
                (&original.code, original.scale, &original.relevance)
            );
        }
        let answers: Vec<_> = model
            .answers
            .iter()
            .filter(|answer| answer.question_id == question.id)
            .collect();
        assert_eq!(answers.len(), source.choices.len());
        for (answer, original) in answers.iter().zip(&source.choices) {
            assert_eq!(
                (&answer.code, answer.scale),
                (&original.code, original.scale)
            );
        }
    }
    let equation = model
        .questions
        .iter()
        .find(|question| question.code == "CALC")
        .unwrap();
    assert_eq!(
        equation
            .attributes
            .iter()
            .find(|attribute| attribute.name == "equation")
            .unwrap()
            .value,
        "{sum(K_A1, K_A2)}"
    );
}

#[test]
fn test_native_export_should_preserve_unicode_xml_metacharacters_and_carriage_returns() {
    let mut definition = definition();
    let text = "été 日本語 & <value> ]]> \r\nquoted \"text\"";
    definition.groups[0].questions[0].title = text.into();
    definition.languages = vec!["fr".into(), "en".into()];
    definition
        .translations
        .push(rustsurvey_domain::SurveyTranslation {
            language: "en".into(),
            title: Some("English study".into()),
            welcome: None,
            end: Some("The end".into()),
        });
    let exported = export_native(&definition, SourceId(1), Limits::default()).unwrap();
    let parsed = LssDocument::parse(&exported.xml, Limits::default()).unwrap();
    let model = parsed.source_model().unwrap();
    assert_eq!(model.languages, ["fr", "en"]);
    assert_eq!(model.questions[0].labels[0].text.as_deref(), Some(text));
    assert_eq!(
        model
            .messages
            .iter()
            .find(|item| item.language == "en")
            .unwrap()
            .end
            .as_deref(),
        Some("The end")
    );
}

#[test]
fn test_native_export_should_refuse_unrepresentable_data_instead_of_omitting_it() {
    let mut original = definition();
    original.groups[0].questions[0].media.push(QuestionMedia {
        id: "00000000-0000-4000-8000-000000000001".into(),
        alt: "synthetic".into(),
        language: None,
    });
    let error = export_native(&original, SourceId(1), Limits::default())
        .err()
        .unwrap();
    let NativeExportError::Unsupported(issues) = error else {
        panic!("expected unsupported media");
    };
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("ressources"))
    );
    let mut original = definition();
    original.groups[0].questions[0]
        .attributes
        .push(rustsurvey_domain::Attribute {
            name: "unknown_active_attribute".into(),
            value: "0".into(),
            language: None,
        });
    let error = export_native(&original, SourceId(1), Limits::default())
        .err()
        .unwrap();
    let NativeExportError::Unsupported(issues) = error else {
        panic!("expected unsupported attribute");
    };
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("attribut"))
    );
}

#[test]
fn test_native_export_should_refuse_codes_that_limesurvey_would_rename() {
    let mut definition = definition();
    definition.groups[0].questions[0].code = "LANG".into();
    let Err(NativeExportError::Unsupported(issues)) =
        export_native(&definition, SourceId(1), Limits::default())
    else {
        panic!("a reserved question code must block export");
    };
    assert!(issues.iter().any(|item| item.message.contains("renommé")));
    definition.groups[0].questions[0].code = "VALID".into();
    let question = definition.groups[0]
        .questions
        .iter_mut()
        .find(|question| !question.subquestions.is_empty())
        .unwrap();
    question.subquestions[0].code = "time".into();
    let Err(NativeExportError::Unsupported(issues)) =
        export_native(&definition, SourceId(1), Limits::default())
    else {
        panic!("a reserved subquestion code must block export");
    };
    assert!(
        issues
            .iter()
            .any(|item| item.message.contains("sous-question"))
    );
}

#[test]
fn test_native_export_should_refuse_answer_codes_that_the_reference_would_truncate() {
    let mut definition = definition();
    let question = definition.groups[0]
        .questions
        .iter_mut()
        .find(|question| !question.choices.is_empty())
        .unwrap();
    question.choices[0].code = "TOOLONG".into();
    let Err(NativeExportError::Unsupported(issues)) =
        export_native(&definition, SourceId(1), Limits::default())
    else {
        panic!("long answer codes must block export");
    };
    assert!(issues.iter().any(|issue| issue.message.contains("1 à 5")));
}

#[test]
fn test_native_export_should_enforce_xml_size_and_character_limits_without_silent_normalization() {
    let limits = Limits {
        max_bytes: 256,
        ..Limits::default()
    };
    assert!(matches!(
        export_native(&definition(), SourceId(1), limits),
        Err(NativeExportError::Format(Error::LimitExceeded { .. }))
    ));
    let mut definition = definition();
    definition.groups[0].questions[0].title = "invalid\u{0001}XML".into();
    assert!(matches!(
        export_native(&definition, SourceId(1), Limits::default()),
        Err(NativeExportError::Format(Error::InvalidXmlCharacter))
    ));
}
