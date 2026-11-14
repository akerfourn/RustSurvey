use rustsurvey_limesurvey::{Error, Limits, ResponseTable};
use std::collections::BTreeMap;

fn table() -> ResponseTable {
    ResponseTable {
        languages: vec!["fr".into(), "en".into()],
        columns: vec![
            "id".into(),
            "701X1X1".into(),
            "701X1X2A#0".into(),
            "unknown".into(),
        ],
        rows: vec![BTreeMap::from([
            ("id".into(), Some("01".into())),
            ("701X1X1".into(), Some("".into())),
            ("701X1X2A#0".into(), None),
            (
                "unknown".into(),
                Some("  été 日本語 & <x> ]]>\r\n\t0  ".into()),
            ),
        ])],
    }
}

#[test]
fn test_responses_should_preserve_null_empty_absent_columns_and_lexical_values() {
    let expected = table();
    let xml = expected.to_xml(Limits::default()).unwrap();
    assert_eq!(
        ResponseTable::parse(&xml, Limits::default()).unwrap(),
        expected
    );
    let text = std::str::from_utf8(&xml).unwrap();
    assert!(text.contains("<_701X1X1/>"));
    assert!(!text.contains("<_701X1X2A-0>"));
    assert!(!expected.columns.iter().any(|column| column == "token"));
}

#[test]
fn test_responses_should_refuse_ambiguous_fields_unknown_elements_and_unsupported_schema() {
    let xml = String::from_utf8(table().to_xml(Limits::default()).unwrap()).unwrap();
    for changed in [
        xml.replace("<id>01</id>", "<id>01</id><id>02</id>"),
        xml.replace("<id>01</id>", "<undeclared>01</undeclared>"),
        xml.replace("<id>01</id>", "<id><nested>01</nested></id>"),
        xml.replace("</document>", "<unknown/></document>"),
        xml.replace("<id>01</id>", "<id custom=\"semantic-extension\">01</id>"),
    ] {
        assert!(matches!(
            ResponseTable::parse(changed.as_bytes(), Limits::default()),
            Err(Error::InvalidStructure)
        ));
    }
    assert!(matches!(
        ResponseTable::parse(
            xml.replace("<DBVersion>623", "<DBVersion>622").as_bytes(),
            Limits::default()
        ),
        Err(Error::UnsupportedSchema(622))
    ));
}

#[test]
fn test_responses_should_bound_null_expansion_and_output_bytes() {
    let mut source = table();
    source.columns = (0..100).map(|index| format!("field{index}")).collect();
    source.rows = vec![BTreeMap::new(); 20];
    let xml = source.to_xml(Limits::default()).unwrap();
    let limits = Limits {
        max_elements: 500,
        ..Limits::default()
    };
    assert!(matches!(
        ResponseTable::parse(&xml, limits),
        Err(Error::LimitExceeded {
            resource: "response cells",
            ..
        })
    ));
    assert!(matches!(
        table().to_xml(Limits {
            max_bytes: 80,
            ..Limits::default()
        }),
        Err(Error::LimitExceeded { .. })
    ));
}

#[test]
fn test_responses_should_accept_an_empty_reference_export_without_inventing_columns() {
    let source = ResponseTable {
        languages: vec!["fr".into()],
        columns: Vec::new(),
        rows: Vec::new(),
    };
    let xml = source.to_xml(Limits::default()).unwrap();
    assert_eq!(
        ResponseTable::parse(&xml, Limits::default()).unwrap(),
        source
    );
}

#[test]
fn test_vv_should_distinguish_literal_markers_null_and_empty_without_trimming() {
    let source = ResponseTable {
        languages: vec!["fr".into()],
        columns: vec!["A".into(), "B".into(), "C".into()],
        rows: vec![BTreeMap::from([
            (
                "A".into(),
                Some("  {question_not_shown}\r\n\t\"O'Neil`\"  ".into()),
            ),
            ("B".into(), Some("".into())),
            ("C".into(), None),
        ])],
    };
    let aliases = BTreeMap::from([("A".into(), "Q1".into())]);
    let bytes = source.to_vv(&aliases, Limits::default()).unwrap();
    assert_eq!(
        String::from_utf8(bytes.clone()).unwrap(),
        "Q1\tB\tC\nQ1\tB\tC\n  {lbrace}question_not_shown}{cr}{newline}{tab}{quote}O'Neil`{quote}  \t\t{question_not_shown}\n"
    );
    let encoder =
        rustsurvey_limesurvey::VvEncoder::new(source.columns.clone(), &aliases, Limits::default())
            .unwrap();
    let mut streamed = encoder.header().to_vec();
    streamed.extend(encoder.row(&source.rows[0]).unwrap());
    assert_eq!(streamed, bytes);
    assert!(
        source
            .to_vv(
                &BTreeMap::from([("A".into(), "B".into())]),
                Limits::default()
            )
            .is_err()
    );
    assert!(matches!(
        source.to_vv(
            &aliases,
            Limits {
                max_bytes: 20,
                ..Limits::default()
            }
        ),
        Err(Error::LimitExceeded { .. })
    ));
}
