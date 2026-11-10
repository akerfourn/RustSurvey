use rustsurvey_limesurvey::{
    Capability, DiagnosticCode, Error, Limits, LssDocument, Mandatory, QuestionFamily, SourceId,
};

const SOURCE: &str = include_str!("fixtures/minimal-623.lss");
const FRENCH_TITLE: &str = "<![CDATA[Étude été — 日本語]]>";

fn parse(source: &str) -> LssDocument {
    LssDocument::parse(source.as_bytes(), Limits::default()).expect("fixture should parse")
}

#[test]
fn test_parse_should_preserve_every_byte_when_source_contains_unknown_data() {
    let document = parse(SOURCE);

    assert_eq!(document.original_xml(), SOURCE.as_bytes());
    assert!(!document.report().has_errors(), "{:?}", document.report());
    let extension = document
        .report()
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagnosticCode::UnknownSectionPreserved)
        .expect("opaque extension should have a diagnostic");
    assert_eq!(extension.location, "document.children[12]");
    assert!(matches!(
        document.report().execution,
        Capability::NotImplemented
    ));
    assert!(matches!(
        document.report().continuation,
        Capability::NotImplemented
    ));
}

#[test]
fn test_parse_should_project_source_relationships_when_schema_is_623() {
    let document = parse(SOURCE);
    let model = document
        .source_model()
        .expect("target schema has a read model");

    assert_eq!(model.survey_id, Some(SourceId(701)));
    assert_eq!(model.languages, ["fr", "en"]);
    assert_eq!(model.groups.len(), 1);
    assert_eq!(model.questions.len(), 2);
    assert_eq!(model.questions[0].family, QuestionFamily::Choice);
    assert_eq!(model.questions[1].mandatory, Mandatory::Soft);
    assert_eq!(model.subquestions[0].parent_id, model.questions[1].id);
}

#[test]
fn test_parse_should_distinguish_null_and_empty_when_localized_title_is_optional() {
    let absent = SOURCE.replace(
        &format!("<surveyls_title>{FRENCH_TITLE}</surveyls_title>"),
        "",
    );
    let empty = SOURCE.replace(FRENCH_TITLE, "");

    let absent = parse(&absent);
    let empty = parse(&empty);

    assert_eq!(absent.source_model().unwrap().titles[0].value, None);
    assert_eq!(
        empty.source_model().unwrap().titles[0].value.as_deref(),
        Some("")
    );
    assert_eq!(
        absent.with_title("fr", "New").err(),
        Some(Error::TitleNotFound)
    );
}

#[test]
fn test_with_title_should_change_only_selected_content_when_new_text_contains_xml() {
    let document = parse(SOURCE);
    let title = "Été <b> & ]]>\r\n日本語";

    let updated = document
        .with_title("fr", title)
        .expect("title update should work");
    let expected = SOURCE.replace(FRENCH_TITLE, "Été &lt;b&gt; &amp; ]]&gt;&#13;\n日本語");

    assert_eq!(updated.original_xml(), expected.as_bytes());
    let model = updated.source_model().unwrap();
    assert_eq!(model.titles[0].value.as_deref(), Some(title));
    assert_eq!(model.titles[1].value.as_deref(), Some("Summer study"));
    assert_eq!(
        document.original_xml(),
        SOURCE.as_bytes(),
        "the source remains immutable"
    );
}

#[test]
fn test_with_title_should_preserve_bom_and_crlf_when_offsets_include_multibyte_text() {
    let source = format!("\u{feff}{}", SOURCE.replace('\n', "\r\n"));

    let updated = parse(&source)
        .with_title("fr", "New")
        .expect("update UTF-8 source");

    assert_eq!(
        updated.original_xml(),
        source.replace(FRENCH_TITLE, "New").as_bytes()
    );
}

#[test]
fn test_with_title_should_preserve_original_serialization_when_value_is_unchanged() {
    let updated = parse(SOURCE)
        .with_title("fr", "Étude été — 日本語")
        .expect("no-op title edit");

    assert_eq!(updated.original_xml(), SOURCE.as_bytes());
}

#[test]
fn test_with_title_should_preserve_attributes_when_old_element_is_self_closing() {
    let source = SOURCE.replace(
        &format!("<surveyls_title>{FRENCH_TITLE}</surveyls_title>"),
        "<surveyls_title xml:space='preserve' />",
    );
    let updated = parse(&source)
        .with_title("fr", "New")
        .expect("expand empty title");
    let expected = source.replace(
        "<surveyls_title xml:space='preserve' />",
        "<surveyls_title xml:space='preserve' >New</surveyls_title>",
    );

    assert_eq!(updated.original_xml(), expected.as_bytes());
}

#[test]
fn test_parse_should_block_edits_when_question_group_is_missing() {
    let source = SOURCE.replacen("<gid>20</gid>", "<gid>999</gid>", 1);
    let document = parse(&source);

    assert!(document.report().diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::MissingReference
            && diagnostic.location.starts_with("questions.rows[")
    }));
    assert_eq!(
        document.with_title("fr", "New").err(),
        Some(Error::InvalidStructure)
    );
    assert_eq!(document.original_xml(), source.as_bytes());
}

#[test]
fn test_parse_should_report_duplicate_ids_when_questions_share_an_identifier() {
    let source = SOURCE.replacen("<qid>102</qid>", "<qid>101</qid>", 1);
    let document = parse(&source);

    assert!(
        document
            .report()
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::DuplicateId)
    );
    assert!(document.report().has_errors());
}

#[test]
fn test_parse_should_report_missing_translations_when_question_language_is_absent() {
    let source = SOURCE.replace(
        "<row><qid>103</qid><language>en</language><question>Detail</question></row>",
        "",
    );
    let document = parse(&source);

    assert!(document.report().diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::MissingReference
            && diagnostic.location == "question_l10ns"
    }));
}

#[test]
fn test_parse_should_report_shadowed_fields_when_row_contains_duplicate_language() {
    let source = SOURCE.replacen(
        "<surveyls_language>fr</surveyls_language>",
        "<surveyls_language>fr</surveyls_language><surveyls_language>en</surveyls_language>",
        1,
    );
    let document = parse(&source);

    assert!(document.report().has_errors());
    assert!(
        document
            .report()
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::InvalidField)
    );
}

#[test]
fn test_parse_should_preserve_unknown_type_when_execution_is_not_supported() {
    let source = SOURCE.replacen("<type>L</type>", "<type>custom</type>", 1);
    let document = parse(&source);

    assert!(!document.report().has_errors());
    assert_eq!(
        document.source_model().unwrap().questions[0].family,
        QuestionFamily::Unknown
    );
    assert!(
        document
            .report()
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::UnsupportedQuestionType)
    );
    assert_eq!(document.original_xml(), source.as_bytes());
}

#[test]
fn test_parse_should_preserve_older_schema_when_structural_validation_is_unavailable() {
    let source = SOURCE.replace("<DBVersion>623</DBVersion>", "<DBVersion>617</DBVersion>");
    let document = parse(&source);

    assert!(document.source_model().is_none());
    assert!(document.report().checked_sections.is_empty());
    assert_eq!(document.original_xml(), source.as_bytes());
    assert_eq!(
        document.with_title("fr", "New").err(),
        Some(Error::UnsupportedSchema(617))
    );
}

#[test]
fn test_report_should_keep_error_state_when_diagnostic_output_is_truncated() {
    let source = SOURCE.replacen("<gid>20</gid>", "<gid>999</gid>", 1);
    let limits = Limits {
        max_diagnostics: 1,
        ..Limits::default()
    };

    let document = LssDocument::parse(source.as_bytes(), limits).expect("XML remains valid");

    assert_eq!(document.report().diagnostics.len(), 1);
    assert!(document.report().omitted_diagnostics > 0);
    assert!(document.report().errors > 0);
    assert!(document.report().has_errors());
}

#[test]
fn test_report_should_exclude_user_values_when_unknown_data_is_present() {
    let report = serde_json::to_string(parse(SOURCE).report()).expect("serialize report");

    assert!(!report.contains("synthetic()"));
    assert!(!report.contains("Summer study"));
}

#[test]
fn test_with_title_should_reject_output_when_escaping_exceeds_the_byte_limit() {
    let limits = Limits {
        max_bytes: SOURCE.len() + 10,
        ..Limits::default()
    };
    let document = LssDocument::parse(SOURCE.as_bytes(), limits).unwrap();

    let error = document
        .with_title("fr", &"&".repeat(100))
        .err()
        .expect("output must be bounded");

    assert_eq!(
        error,
        Error::LimitExceeded {
            resource: "output bytes",
            limit: limits.max_bytes
        }
    );
    assert_eq!(document.original_xml(), SOURCE.as_bytes());
}
