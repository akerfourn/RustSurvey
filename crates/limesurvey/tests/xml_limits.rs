use rustsurvey_limesurvey::{Error, Limits, LssDocument};

const SOURCE: &[u8] = include_bytes!("fixtures/minimal-623.lss");

fn error(source: &[u8], limits: Limits) -> Error {
    LssDocument::parse(source, limits)
        .err()
        .expect("input should be rejected")
}

#[test]
fn test_parse_should_reject_dtd_when_document_mentions_external_resources() {
    let source = b"<!DOCTYPE document SYSTEM 'https://invalid.example/never-fetch'><document/>";

    assert_eq!(error(source, Limits::default()), Error::DtdForbidden);
    assert!(
        error(source, Limits::default())
            .to_string()
            .contains("forbidden")
    );
}

#[test]
fn test_parse_should_reject_dtd_when_entities_would_expand_recursively() {
    let source = b"<!DOCTYPE document [<!ENTITY a 'x'><!ENTITY b '&a;&a;'>]><document/>";

    assert_eq!(error(source, Limits::default()), Error::DtdForbidden);
}

#[test]
fn test_parse_should_reject_processing_instruction_when_stylesheet_is_requested() {
    assert_eq!(
        error(
            b"<?xml-stylesheet href='private'?><document/>",
            Limits::default()
        ),
        Error::ProcessingInstructionForbidden
    );
}

#[test]
fn test_parse_should_accept_dtd_text_when_it_is_only_cdata_content() {
    let source = std::str::from_utf8(SOURCE)
        .unwrap()
        .replace("synthetic()", "<!DOCTYPE example>");

    let document =
        LssDocument::parse(source.as_bytes(), Limits::default()).expect("CDATA is inert text");

    assert_eq!(document.original_xml(), source.as_bytes());
}

#[test]
fn test_parse_should_reject_syntax_errors_when_document_is_not_well_formed() {
    for source in [
        "<document><a></b></document>",
        "<document/><!--x--><document/>",
        "<document/ >",
        "<document>",
        "<document/>trailing",
    ] {
        assert!(
            matches!(
                error(source.as_bytes(), Limits::default()),
                Error::InvalidXml { .. }
            ),
            "{source}"
        );
    }
}

#[test]
fn test_parse_should_reject_duplicate_attributes_when_values_could_shadow_each_other() {
    assert!(matches!(
        error(b"<document a='1' a='2'/>", Limits::default()),
        Error::InvalidXml { .. }
    ));
}

#[test]
fn test_parse_should_reject_unbound_attribute_prefix_when_namespace_is_missing() {
    assert!(matches!(
        error(b"<document bad:attribute='value'/>", Limits::default()),
        Error::InvalidXml { .. }
    ));
}

#[test]
fn test_parse_should_reject_duplicate_expanded_attributes_when_prefixes_alias_same_namespace() {
    assert!(matches!(
        error(
            b"<document xmlns:a='urn:test' xmlns:b='urn:test' a:x='1' b:x='2'/>",
            Limits::default()
        ),
        Error::InvalidXml { .. }
    ));
}

#[test]
fn test_parse_should_reject_literal_markup_when_attribute_contains_an_unescaped_bracket() {
    assert!(matches!(
        error(b"<document attribute='<'/>", Limits::default()),
        Error::InvalidXml { .. }
    ));
}

#[test]
fn test_parse_should_reject_invalid_character_when_numeric_reference_is_illegal_in_xml10() {
    assert_eq!(
        error(b"<document>&#1;</document>", Limits::default()),
        Error::InvalidXmlCharacter
    );
}

#[test]
fn test_parse_should_reject_non_utf8_encoding_when_declaration_requests_latin1() {
    assert_eq!(
        error(
            b"<?xml version='1.0' encoding='ISO-8859-1'?><document/>",
            Limits::default()
        ),
        Error::UnsupportedEncoding
    );
}

#[test]
fn test_parse_should_reject_declaration_when_whitespace_precedes_it() {
    assert!(matches!(
        error(b"   <?xml version='1.0'?><document/>", Limits::default()),
        Error::InvalidXml { .. }
    ));
}

#[test]
fn test_parse_should_reject_other_document_types_when_file_is_not_a_survey() {
    let source = std::str::from_utf8(SOURCE).unwrap().replace(
        ">Survey</LimeSurveyDocType>",
        ">Responses</LimeSurveyDocType>",
    );

    assert_eq!(
        error(source.as_bytes(), Limits::default()),
        Error::NotSurveyDocument
    );
}

#[test]
fn test_parse_should_reject_input_when_size_exceeds_the_byte_limit() {
    let limits = Limits {
        max_bytes: SOURCE.len() - 1,
        ..Limits::default()
    };

    assert_eq!(
        error(SOURCE, limits),
        Error::LimitExceeded {
            resource: "input bytes",
            limit: limits.max_bytes
        }
    );
}

#[test]
fn test_parse_should_reject_nesting_when_depth_exceeds_the_limit() {
    let limits = Limits {
        max_depth: 2,
        ..Limits::default()
    };

    assert_eq!(
        error(SOURCE, limits),
        Error::LimitExceeded {
            resource: "element depth",
            limit: 2
        }
    );
}

#[test]
fn test_parse_should_reject_elements_when_count_exceeds_the_limit() {
    let limits = Limits {
        max_elements: 1,
        ..Limits::default()
    };

    assert_eq!(
        error(SOURCE, limits),
        Error::LimitExceeded {
            resource: "element count",
            limit: 1
        }
    );
}

#[test]
fn test_parse_should_reject_attributes_when_count_exceeds_the_limit() {
    let limits = Limits {
        max_attributes: 1,
        ..Limits::default()
    };

    assert_eq!(
        error(b"<document one='1' two='2'/>", limits),
        Error::LimitExceeded {
            resource: "attributes per element",
            limit: 1
        }
    );
}

#[test]
fn test_parse_should_preserve_bom_when_input_uses_utf8_bom() {
    let source = [b"\xef\xbb\xbf".as_slice(), SOURCE].concat();
    let document = LssDocument::parse(&source, Limits::default()).expect("UTF-8 BOM is supported");

    assert_eq!(document.original_xml(), source);
}
