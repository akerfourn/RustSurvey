use rustsurvey_limesurvey::{Limits, LssDocument};

const SOURCE: &str = include_str!("fixtures/minimal-623.lss");

fn inspect(source: &str) -> rustsurvey_limesurvey::ProjectionGaps {
    LssDocument::parse(source.as_bytes(), Limits::default())
        .unwrap()
        .projection_gaps()
}

#[test]
fn test_projection_gaps_should_find_unknown_settings_without_disclosing_their_values() {
    let gaps = inspect(SOURCE);
    assert_eq!(gaps.structural_errors, 0);
    assert_eq!(gaps.unprojected_columns, 2);
    assert_eq!(gaps.unprojected_sections, 1);
    assert!(gaps.extended_elements >= 2);
    assert!(gaps.missing_settings > 0);
    let report = serde_json::to_string(&gaps).unwrap();
    assert!(report.contains("custom_setting"));
    assert!(
        report.contains("optional_value"),
        "empty values still need qualification"
    );
    for private in ["0007", "synthetic()", "Summer study"] {
        assert!(
            !report.contains(private),
            "source values must stay in the original"
        );
    }
}

#[test]
fn test_projection_gaps_should_distinguish_empty_known_sections_from_unknown_or_active_content() {
    let empty = SOURCE.replace("</document>", "<quota><fields><fieldname>qlimit</fieldname></fields><rows/></quota><defaultvalues/></document>");
    assert_eq!(
        inspect(&empty).unprojected_sections,
        inspect(SOURCE).unprojected_sections
    );
    let active = empty
        .replace(
            "<rows/></quota>",
            "<rows><row><qlimit>0</qlimit></row></rows></quota>",
        )
        .replace(
            "<defaultvalues/>",
            "<defaultvalues><fields/><rows><row/></rows></defaultvalues><unknown_empty/>",
        );
    assert_eq!(
        inspect(&active).unprojected_sections,
        inspect(SOURCE).unprojected_sections + 3
    );
}

#[test]
fn test_projection_gaps_should_detect_subquestion_attributes_and_xml_extensions() {
    let source = SOURCE.replace("</document>", "<question_attributes><fields><fieldname>qid</fieldname><fieldname>attribute</fieldname><fieldname>value</fieldname></fields><rows><row><qid>103</qid><attribute>hidden</attribute><value>0</value></row></rows></question_attributes></document>")
        .replace("<surveys>", "<surveys><extension/>");
    let document = LssDocument::parse(source.as_bytes(), Limits::default()).unwrap();
    let gaps = document.projection_gaps();
    assert!(
        gaps.issues
            .iter()
            .any(|issue| issue.path.starts_with("source.question_attributes.rows"))
    );
    assert!(
        gaps.issues
            .iter()
            .any(|issue| issue.message.contains("Élément de table"))
    );
    assert_eq!(document.original_xml(), source.as_bytes());
}

#[test]
fn test_projection_gaps_should_bound_diagnostics_while_counting_omitted_findings() {
    let limits = Limits {
        max_diagnostics: 3,
        ..Limits::default()
    };
    let gaps = LssDocument::parse(SOURCE.as_bytes(), limits)
        .unwrap()
        .projection_gaps();
    assert_eq!(gaps.issues.len(), 3);
    assert!(gaps.issue_count > gaps.issues.len());
    assert!(gaps.missing_settings > 3);
}
