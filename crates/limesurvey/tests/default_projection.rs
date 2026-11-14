#[path = "../../domain/tests/support/defaults.rs"]
mod fixture;
use rustsurvey_domain::{Question, SourceReference, defaults::QuestionDefaults};
use rustsurvey_limesurvey::{
    Limits, LssDocument, SourceId, SurveySource, export_native, project_question_defaults,
};

fn target(model: &SurveySource, code: &str, mut question: Question) -> Question {
    let source = model
        .questions
        .iter()
        .find(|source| source.code == code)
        .unwrap();
    question.source = Some(SourceReference {
        id: source.id.0.to_string(),
        kind: Some(source.source_type.clone()),
    });
    for candidate in &mut question.subquestions {
        let row = model
            .subquestions
            .iter()
            .find(|row| {
                row.parent_id == source.id
                    && row.code == candidate.code
                    && row.scale == candidate.scale
            })
            .unwrap();
        candidate.source = Some(SourceReference {
            id: row.id.0.to_string(),
            kind: None,
        });
    }
    question.defaults = None;
    question
}

fn configuration(index: usize) -> (LssDocument, Question, QuestionDefaults) {
    let case = &fixture::cases()[index];
    let definition = fixture::definition(case);
    let bytes = export_native(&definition, SourceId(701), Limits::default())
        .unwrap()
        .xml;
    let document = LssDocument::parse(&bytes, Limits::default()).unwrap();
    let question = definition.groups[0]
        .questions
        .iter()
        .find(|question| question.code == case["question"].as_str().unwrap())
        .unwrap()
        .clone();
    let expected = question.defaults.clone().unwrap();
    let target = target(
        document.source_model().unwrap(),
        &question.code.clone(),
        question,
    );
    (document, target, expected)
}

#[test]
fn test_default_projection_should_roundtrip_supported_targets_languages_and_expressions_without_evaluating()
 {
    for (index, case) in fixture::cases().iter().enumerate() {
        if case["nativeSupported"] == false {
            continue;
        }
        let (document, target, mut expected) = configuration(index);
        let model = document.source_model().unwrap();
        let source = model
            .questions
            .iter()
            .find(|source| source.code == target.code)
            .unwrap();
        let mut actual = project_question_defaults(model, source, &target)
            .unwrap()
            .unwrap();
        actual
            .values
            .sort_by(|left, right| left.language.cmp(&right.language));
        expected
            .values
            .sort_by(|left, right| left.language.cmp(&right.language));
        assert_eq!(actual, expected, "{}", case["name"]);
    }
}

#[test]
fn test_default_projection_should_refuse_a_coincident_row_code_without_its_source_identity() {
    let index = fixture::cases()
        .iter()
        .position(|case| case["row"].is_string() && case["nativeSupported"] != false)
        .unwrap();
    let (document, mut target, _) = configuration(index);
    let model = document.source_model().unwrap();
    let source = model
        .questions
        .iter()
        .find(|source| source.code == target.code)
        .unwrap();
    for row in &mut target.subquestions {
        row.source = None;
    }
    let issues = project_question_defaults(model, source, &target).unwrap_err();
    assert!(issues[0].message.contains("identité source"));
    assert!(
        target.defaults.is_none(),
        "a failed projection must not install partial values"
    );
}

#[test]
fn test_default_projection_should_preserve_unknown_settings_and_unsupported_targets_as_diagnostics()
{
    let (document, target, _) = configuration(0);
    for (before, after, expected) in [
        ("<same_default>0</same_default>", "", "same_default"),
        (
            "<specialtype/>",
            "<specialtype>private-secret-value</specialtype>",
            "Cible",
        ),
        (
            "<scale_id>0</scale_id>",
            "<scale_id>1</scale_id>",
            "échelle",
        ),
    ] {
        let xml = String::from_utf8(document.original_xml().to_vec()).unwrap();
        assert!(
            xml.contains(before),
            "fixture must exercise the changed source field: {before}"
        );
        let changed =
            LssDocument::parse(xml.replace(before, after).as_bytes(), Limits::default()).unwrap();
        let model = changed.source_model().unwrap();
        let source = model
            .questions
            .iter()
            .find(|source| source.code == target.code)
            .unwrap();
        let issues = project_question_defaults(model, source, &target).unwrap_err();
        assert!(issues[0].message.contains(expected), "{issues:?}");
        assert!(
            !serde_json::to_string(&issues)
                .unwrap()
                .contains("private-secret-value")
        );
    }
}

#[test]
fn test_default_projection_should_refuse_oversized_text_without_discarding_its_source() {
    let (document, target, _) = configuration(0);
    let model = document.source_model().unwrap();
    let source = model
        .questions
        .iter()
        .find(|source| source.code == target.code)
        .unwrap();
    let original = &model.default_values[0].values[0].value;
    let replacement = "x".repeat(4097);
    let xml = String::from_utf8(document.original_xml().to_vec())
        .unwrap()
        .replace(
            &format!("<defaultvalue>{original}</defaultvalue>"),
            &format!("<defaultvalue>{replacement}</defaultvalue>"),
        );
    let changed = LssDocument::parse(xml.as_bytes(), Limits::default()).unwrap();
    let model = changed.source_model().unwrap();
    let changed_source = model
        .questions
        .iter()
        .find(|question| question.id == source.id)
        .unwrap();
    let issues = project_question_defaults(model, changed_source, &target).unwrap_err();
    assert!(issues[0].message.contains("4096"));
    assert_eq!(changed.original_xml(), xml.as_bytes());
}
