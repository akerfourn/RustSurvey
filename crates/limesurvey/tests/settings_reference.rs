use rustsurvey_limesurvey::{Limits, LssDocument, SettingValue, SettingsContext};
use std::{collections::BTreeMap, fs::File, io::Read, path::Path};

fn read(path: &Path) -> Vec<u8> {
    let mut bytes = Vec::new();
    File::open(path)
        .unwrap()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 8 * 1024 * 1024);
    bytes
}

#[test]
#[ignore = "requires the isolated LimeSurvey 6.5.14 reference harness"]
fn test_settings_reference_should_match_group_inheritance_and_explicit_values_in_limesurvey() {
    let input = std::env::var_os("RUSTSURVEY_REFERENCE_LSS").expect("synthetic reference LSS path");
    let directory = Path::new(&input).parent().unwrap();
    for case in 0..4 {
        let document = LssDocument::parse(
            &read(&directory.join(format!("settings-{case}.lss"))),
            Limits::default(),
        )
        .unwrap();
        assert!(
            !document.report().has_errors(),
            "case {case}: {:?}",
            document.report()
        );
        let context: SettingsContext = serde_json::from_slice(&read(
            &directory.join(format!("settings-{case}-context.json")),
        ))
        .unwrap();
        let expected: BTreeMap<String, String> = serde_json::from_slice(&read(
            &directory.join(format!("settings-{case}-expected.json")),
        ))
        .unwrap();
        let report = document
            .group_settings(Some(&context))
            .unwrap_or_else(|error| {
                panic!(
                    "case {case}, survey group {}, supplied groups {:?}: {error}",
                    context.survey_group_id,
                    context
                        .groups
                        .iter()
                        .map(|group| &group.id)
                        .collect::<Vec<_>>()
                )
            });
        assert!(report.complete, "case {case}: {:?}", report.issues);
        assert_eq!(report.settings.len(), expected.len());
        let extracted: SettingsContext = serde_json::from_slice(&read(
            &directory.join(format!("settings-{case}-extracted.json")),
        ))
        .unwrap();
        let extracted_report = document.group_settings(Some(&extracted)).unwrap();
        assert_eq!(
            serde_json::to_value(&extracted_report).unwrap(),
            serde_json::to_value(&report).unwrap(),
            "the read-only source helper must reproduce the explicit test context"
        );
        for setting in report.settings {
            let actual = match setting.value.unwrap() {
                SettingValue::Flag(true) => "Y".into(),
                SettingValue::Flag(false) => "N".into(),
                SettingValue::Code(code) => code.into(),
                SettingValue::Integer(value) => value.to_string(),
            };
            assert_eq!(
                actual, expected[setting.name],
                "case {case}, option {}",
                setting.name
            );
        }
    }
}
