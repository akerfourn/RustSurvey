use rustsurvey_limesurvey::{Limits, LssDocument, SourceId};

const DEMO: &str = include_str!("../../../examples/surveys/demo.lss");

#[test]
fn test_projection_should_expose_labels_choices_attributes_and_relevance_without_editing_source() {
    let document = LssDocument::parse(DEMO.as_bytes(), Limits::default()).unwrap();
    let model = document.source_model().unwrap();

    assert!(!document.report().has_errors(), "{:?}", document.report());
    assert_eq!(model.groups[1].relevance, "STACK == 'rust'");
    assert_eq!(model.questions[5].relevance, "GOALS_other == 'Y'");
    assert_eq!(
        model.questions[0].labels[0].text.as_deref(),
        Some("Comment s’appelle votre projet ?")
    );
    assert_eq!(model.questions[2].attributes[0].name, "num_value_int_only");
    assert_eq!(model.questions[2].attributes[0].value, "1");
    assert_eq!(model.subquestions[2].code, "other");
    assert_eq!(model.answers[0].question_id, SourceId(102));
    assert_eq!(model.answers[0].code, "rust");
    assert_eq!(model.answers[0].labels[1].text.as_deref(), Some("Rust"));
    assert_eq!(document.original_xml(), DEMO.as_bytes());
}

#[test]
fn test_metadata_should_expose_titles_when_structural_schema_is_unsupported() {
    let source = DEMO.replace("<DBVersion>623</DBVersion>", "<DBVersion>617</DBVersion>");
    let document = LssDocument::parse(source.as_bytes(), Limits::default()).unwrap();
    let metadata = document.metadata();

    assert!(document.source_model().is_none());
    assert_eq!(metadata.survey_id.as_deref(), Some("900001"));
    assert_eq!(metadata.languages, ["fr", "en"]);
    assert_eq!(
        metadata.titles[1].value.as_deref(),
        Some("Discover RustSurvey")
    );
}

#[test]
fn test_source_id_should_serialize_as_text_when_identifier_exceeds_javascript_integer_precision() {
    let serialized = serde_json::to_string(&SourceId(u64::MAX)).unwrap();

    assert_eq!(serialized, "\"18446744073709551615\"");
}
