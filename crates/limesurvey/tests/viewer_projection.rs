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

#[test]
fn test_projection_should_expose_welcome_and_end_per_language_without_changing_html() {
    let document = LssDocument::parse(DEMO.as_bytes(), Limits::default()).unwrap();
    let model = document.source_model().unwrap();

    assert!(model.show_welcome);
    assert_eq!(model.messages.len(), 2);
    assert_eq!(
        model.messages[0].welcome.as_deref(),
        Some(
            "<p>Essayez les choix, les conditions et les tableaux. Vos réponses restent locales.</p>"
        )
    );
    assert_eq!(
        model.messages[1].end.as_deref(),
        Some("<p>Thank you for trying this, {PROJECT}!</p>")
    );
    assert_eq!(document.original_xml(), DEMO.as_bytes());
}

#[test]
fn test_projection_should_keep_message_content_when_welcome_display_is_disabled() {
    let source = DEMO.replace(
        "<showwelcome>Y</showwelcome>",
        "<showwelcome>N</showwelcome>",
    );
    let document = LssDocument::parse(source.as_bytes(), Limits::default()).unwrap();
    let model = document.source_model().unwrap();

    assert!(!model.show_welcome);
    assert!(model.messages[0].welcome.is_some());
}

#[test]
fn test_projection_should_distinguish_missing_and_empty_messages_when_source_is_optional() {
    let source = include_str!("fixtures/minimal-623.lss");
    let absent = LssDocument::parse(source.as_bytes(), Limits::default()).unwrap();
    let source = DEMO.replace(
        "&lt;p&gt;Thank you for trying this, {PROJECT}!&lt;/p&gt;",
        "",
    );
    let empty = LssDocument::parse(source.as_bytes(), Limits::default()).unwrap();

    assert!(absent.source_model().unwrap().messages[0].welcome.is_none());
    assert_eq!(
        empty.source_model().unwrap().messages[1].end.as_deref(),
        Some("")
    );
}

#[test]
fn test_projection_should_report_invalid_welcome_flag_when_source_setting_is_unknown() {
    let source = DEMO.replace(
        "<showwelcome>Y</showwelcome>",
        "<showwelcome>invalid</showwelcome>",
    );
    let document = LssDocument::parse(source.as_bytes(), Limits::default()).unwrap();

    assert!(
        document
            .report()
            .diagnostics
            .iter()
            .any(
                |diagnostic| diagnostic.location == "surveys.rows[0].showwelcome"
                    && diagnostic.message.contains("Y or N")
            )
    );
}
