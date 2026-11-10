use rustsurvey_limesurvey::{Capability, Error, Limits, LssDocument};

const SOURCE: &[u8] = include_bytes!("fixtures/minimal-623.lss");

#[test]
fn test_original_export_should_remain_available_when_structural_errors_block_title_editing() {
    let source =
        std::str::from_utf8(SOURCE)
            .unwrap()
            .replacen("<gid>20</gid>", "<gid>999</gid>", 1);

    let document = LssDocument::parse(source.as_bytes(), Limits::default())
        .expect("well-formed source can be archived");

    assert!(document.report().has_errors());
    assert!(matches!(
        document.report().original_export,
        Capability::Available
    ));
    assert!(matches!(
        document.report().localized_title_edit,
        Capability::Blocked
    ));
    assert_eq!(document.original_xml(), source.as_bytes());
    assert_eq!(
        document.with_title("fr", "New").err(),
        Some(Error::InvalidStructure)
    );
}
