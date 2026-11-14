use rustsurvey_limesurvey::{Limits, LssDocument, SourceId};
use serde_json::json;

const BASE: &str = include_str!("fixtures/minimal-623.lss");

fn fixture(values: &str, translations: &str) -> String {
    let tables = format!(
        "<defaultvalues><fields><fieldname>dvid</fieldname><fieldname>qid</fieldname><fieldname>sqid</fieldname><fieldname>scale_id</fieldname><fieldname>specialtype</fieldname></fields><rows>{values}</rows></defaultvalues><defaultvalue_l10ns><fields><fieldname>dvid</fieldname><fieldname>language</fieldname><fieldname>defaultvalue</fieldname></fields><rows>{translations}</rows></defaultvalue_l10ns>"
    );
    BASE.replace("</document>", &format!("{tables}</document>"))
}

const VALUE: &str =
    "<row><dvid>1</dvid><qid>102</qid><sqid>103</sqid><scale_id>0</scale_id><specialtype/></row>";
const TRANSLATION: &str = "<row><dvid>1</dvid><language>fr</language><defaultvalue><![CDATA[  001 été {CHOICE}  ]]></defaultvalue></row>";

#[test]
fn test_defaults_should_preserve_source_targets_and_lexical_values_without_execution() {
    let xml = fixture(VALUE, TRANSLATION);
    let document = LssDocument::parse(xml.as_bytes(), Limits::default()).unwrap();
    assert!(!document.report().has_errors(), "{:?}", document.report());
    let model = document.source_model().unwrap();
    let value = &model.default_values[0];
    assert_eq!(value.id, SourceId(1));
    assert_eq!(value.question_id, SourceId(102));
    assert_eq!(value.subquestion_id, Some(SourceId(103)));
    assert_eq!(value.scale, 0);
    assert_eq!(value.special_type, "");
    assert_eq!(value.values[0].language, "fr");
    assert_eq!(value.values[0].value, "  001 été {CHOICE}  ");
    assert_eq!(
        value.values.len(),
        1,
        "an absent translation must not be invented"
    );
    assert_eq!(document.original_xml(), xml.as_bytes());
    assert!(
        document.projection_gaps().issue_count > 0,
        "source conservation does not authorize execution"
    );
}

#[test]
fn test_defaults_should_distinguish_empty_values_and_absent_language_entries() {
    let xml = fixture(
        VALUE,
        &TRANSLATION.replace("<![CDATA[  001 été {CHOICE}  ]]>", ""),
    );
    let document = LssDocument::parse(xml.as_bytes(), Limits::default()).unwrap();
    assert!(!document.report().has_errors());
    let value = &document.source_model().unwrap().default_values[0];
    assert_eq!(
        serde_json::to_value(&value.values).unwrap(),
        json!([{"language":"fr","value":""}])
    );
}

#[test]
fn test_defaults_should_diagnose_invalid_or_ambiguous_references_without_choosing_a_winner() {
    let cases = [
        (
            format!("{VALUE}{VALUE}"),
            TRANSLATION.to_owned(),
            "duplicate default value identifier",
        ),
        (
            format!("{VALUE}{}", VALUE.replace("<dvid>1", "<dvid>2")),
            TRANSLATION.to_owned(),
            "duplicate default value target",
        ),
        (
            VALUE.replace("<qid>102", "<qid>101"),
            TRANSLATION.to_owned(),
            "subquestion of another question",
        ),
        (
            VALUE.to_owned(),
            TRANSLATION.replace("<dvid>1", "<dvid>2"),
            "unknown identifier",
        ),
        (
            VALUE.to_owned(),
            format!("{TRANSLATION}{TRANSLATION}"),
            "duplicate default value translation",
        ),
        (
            VALUE.to_owned(),
            TRANSLATION.replace("<language>fr", "<language>de"),
            "unknown language",
        ),
    ];
    for (values, translations, message) in cases {
        let xml = fixture(&values, &translations);
        let document = LssDocument::parse(xml.as_bytes(), Limits::default()).unwrap();
        assert!(
            document
                .report()
                .diagnostics
                .iter()
                .any(|issue| issue.message.contains(message)),
            "{message}: {:?}",
            document.report()
        );
        assert_eq!(document.original_xml(), xml.as_bytes());
    }
}

#[test]
fn test_defaults_should_keep_unknown_special_targets_without_interpreting_them() {
    let xml = fixture(
        &VALUE.replace("<specialtype/>", "<specialtype>extension</specialtype>"),
        TRANSLATION,
    );
    let document = LssDocument::parse(xml.as_bytes(), Limits::default()).unwrap();
    assert!(!document.report().has_errors());
    assert_eq!(
        document.source_model().unwrap().default_values[0].special_type,
        "extension"
    );
    assert!(document.projection_gaps().unprojected_sections > 0);
}

#[test]
fn test_same_default_should_distinguish_explicit_flags_and_an_unavailable_setting() {
    for (flag, expected) in [
        (None, None),
        (Some("0"), Some(false)),
        (Some("1"), Some(true)),
    ] {
        let mut xml = BASE.to_owned();
        if let Some(flag) = flag {
            xml = xml.replacen(
                "<fieldname>mandatory</fieldname>",
                "<fieldname>same_default</fieldname><fieldname>mandatory</fieldname>",
                1,
            );
            xml = xml.replace(
                "<mandatory>",
                &format!("<same_default>{flag}</same_default><mandatory>"),
            );
        }
        let document = LssDocument::parse(xml.as_bytes(), Limits::default()).unwrap();
        assert!(!document.report().has_errors());
        assert!(
            document
                .source_model()
                .unwrap()
                .questions
                .iter()
                .all(|question| question.same_default == expected)
        );
    }
}

#[test]
fn test_defaults_should_reject_missing_scalar_fields_and_invalid_numeric_coordinates() {
    for value in [
        VALUE.replace("<scale_id>0</scale_id>", "<scale_id>-1</scale_id>"),
        VALUE.replace("<specialtype/>", ""),
    ] {
        let xml = fixture(&value, TRANSLATION);
        let document = LssDocument::parse(xml.as_bytes(), Limits::default()).unwrap();
        assert!(document.report().has_errors());
        assert_eq!(document.original_xml(), xml.as_bytes());
    }
    let xml = fixture(
        VALUE,
        &TRANSLATION.replace(
            "<defaultvalue><![CDATA[  001 été {CHOICE}  ]]></defaultvalue>",
            "",
        ),
    );
    let document = LssDocument::parse(xml.as_bytes(), Limits::default()).unwrap();
    assert!(
        document
            .report()
            .diagnostics
            .iter()
            .any(|issue| issue.message.contains("empty value is allowed"))
    );
}
