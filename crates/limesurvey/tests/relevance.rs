use rustsurvey_limesurvey::{Error, Limits, LssDocument, RelevanceEdit, RelevanceTarget, SourceId};

const SOURCE: &[u8] = include_bytes!("fixtures/minimal-623.lss");
fn edit(target: RelevanceTarget, id: u64, expression: &str) -> RelevanceEdit {
    RelevanceEdit {
        target,
        id: SourceId(id),
        expression: expression.to_owned(),
    }
}

#[test]
fn test_relevance_export_should_insert_declared_fields_preserve_extensions_and_roundtrip_rules() {
    let source = LssDocument::parse(SOURCE, Limits::default()).unwrap();
    let expression = "CHOICE == '01' && 1 < 2";
    let modified = source
        .with_relevances(&[
            edit(RelevanceTarget::Group, 20, "CHOICE != '02'"),
            edit(RelevanceTarget::Question, 102, expression),
        ])
        .unwrap();
    assert!(!modified.report().has_errors());
    let model = modified.source_model().unwrap();
    assert_eq!(model.groups[0].relevance, "CHOICE != '02'");
    assert_eq!(model.questions[1].relevance, expression);
    let text = std::str::from_utf8(modified.original_xml()).unwrap();
    let extension = std::str::from_utf8(SOURCE)
        .unwrap()
        .split("  <!-- Unknown data")
        .nth(1)
        .unwrap();
    assert!(
        text.ends_with(extension),
        "uninterpreted source bytes must survive edits"
    );
    assert!(text.contains("<fieldname>grelevance</fieldname>"));
    assert!(text.contains("&amp;&amp; 1 &lt; 2"));
    assert_eq!(
        source.original_xml(),
        SOURCE,
        "exporting must not mutate the archived document"
    );
    assert_eq!(
        modified
            .with_relevances(&[edit(RelevanceTarget::Group, 20, "CHOICE != '02'")])
            .unwrap()
            .original_xml(),
        modified.original_xml()
    );
}

#[test]
fn test_relevance_edit_should_preserve_every_byte_outside_explicit_field_content() {
    let original = include_str!("../../../examples/surveys/controls.lss");
    let original = format!(
        "\u{feff}{}",
        original.replacen(
            "<grelevance>1</grelevance>",
            "<grelevance data-flag='kept'><![CDATA[1]]></grelevance>",
            1
        )
    );
    let source = LssDocument::parse(original.as_bytes(), Limits::default()).unwrap();
    let id = source.source_model().unwrap().groups[0].id.0;
    let modified = source
        .with_relevances(&[edit(RelevanceTarget::Group, id, "GATE == 'Y'")])
        .unwrap();
    let expected = original.replacen("<![CDATA[1]]>", "GATE == &apos;Y&apos;", 1);
    assert_eq!(modified.original_xml(), expected.as_bytes());
}

#[test]
fn test_relevance_export_should_refuse_ambiguous_targets_legacy_conditions_and_output_overflows() {
    let source = LssDocument::parse(SOURCE, Limits::default()).unwrap();
    let duplicate = edit(RelevanceTarget::Question, 101, "0");
    assert!(matches!(
        source.with_relevances(&[duplicate.clone(), duplicate]),
        Err(Error::InvalidRelevanceEdit)
    ));
    assert!(matches!(
        source.with_relevances(&[edit(RelevanceTarget::Question, 99999, "0")]),
        Err(Error::InvalidRelevanceEdit)
    ));
    let legacy = std::str::from_utf8(SOURCE).unwrap().replace(
        "</document>",
        "<conditions/><conditions><rows><row><cid>1</cid></row></rows></conditions></document>",
    );
    let legacy = LssDocument::parse(legacy.as_bytes(), Limits::default()).unwrap();
    assert!(matches!(
        legacy.with_relevances(&[edit(RelevanceTarget::Question, 101, "0")]),
        Err(Error::LegacyConditionsPreventEdit)
    ));
    assert_eq!(
        legacy.with_relevances(&[]).unwrap().original_xml(),
        legacy.original_xml()
    );
    let limited = LssDocument::parse(
        SOURCE,
        Limits {
            max_bytes: SOURCE.len(),
            ..Limits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        limited.with_relevances(&[edit(RelevanceTarget::Question, 101, "0")]),
        Err(Error::LimitExceeded {
            resource: "output bytes",
            ..
        })
    ));
}
