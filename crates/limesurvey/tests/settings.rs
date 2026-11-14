use rustsurvey_limesurvey::{Limits, LssDocument, SettingsContext};
use serde_json::{Value, json};

const SOURCE: &str = include_str!("fixtures/minimal-623.lss");

#[test]
fn test_settings_context_should_reject_duplicate_names_including_null_values() {
    for values in [
        r#"{"showwelcome":null,"showwelcome":"Y"}"#,
        r#"{"showwelcome":"N","showwelcome":"Y"}"#,
    ] {
        let json = format!(
            r#"{{"schemaVersion":1,"surveyId":"701","surveyGroupId":"0","groups":[],"global":{values}}}"#
        );
        let error = serde_json::from_str::<SettingsContext>(&json).unwrap_err();
        assert!(error.to_string().contains("duplicate setting name"));
    }
}

fn document(settings: &str) -> LssDocument {
    let names = [
        "gsid",
        "showwelcome",
        "format",
        "questionindex",
        "navigationdelay",
    ];
    let fields = names
        .map(|name| format!("<fieldname>{name}</fieldname>"))
        .join("");
    let source = SOURCE
        .replacen(
            "<fieldname>sid</fieldname>",
            &format!("{fields}<fieldname>sid</fieldname>"),
            1,
        )
        .replacen("<sid>701</sid>", &format!("<sid>701</sid>{settings}"), 1);
    let document = LssDocument::parse(source.as_bytes(), Limits::default()).unwrap();
    assert!(!document.report().has_errors(), "{:?}", document.report());
    document
}

fn context() -> SettingsContext {
    serde_json::from_value(json!({
        "schemaVersion":1, "surveyId":"701", "surveyGroupId":"2",
        "groups":[
            {"id":"2","parentId":"1","values":{"showwelcome":"I","format":"A","navigationdelay":"0","questionindex":"0"}},
            {"id":"1","parentId":null,"values":{"showwelcome":"N","navigationdelay":"4","questionindex":"2"}}
        ],
        "global":{"showwelcome":"Y","format":"G","navigationdelay":"0","questionindex":"0"}
    })).unwrap()
}

fn setting(report: &impl serde::Serialize, name: &str) -> Value {
    let report = serde_json::to_value(report).unwrap();
    report["settings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["name"] == name)
        .unwrap()
        .clone()
}

#[test]
fn test_settings_should_follow_group_ancestors_and_php_zero_inheritance_with_provenance() {
    let source = document(
        "<gsid>2</gsid><showwelcome>I</showwelcome><format>I</format><navigationdelay>0</navigationdelay><questionindex>0</questionindex>",
    );
    let report = source.group_settings(Some(&context())).unwrap();
    assert_eq!(setting(&report, "showwelcome")["value"], false);
    assert_eq!(
        setting(&report, "showwelcome")["origin"],
        json!({"layer":"group","groupId":"1"})
    );
    assert_eq!(setting(&report, "format")["value"], "A");
    assert_eq!(setting(&report, "format")["origin"]["groupId"], "2");
    assert_eq!(setting(&report, "navigationdelay")["value"], 4);
    assert_eq!(setting(&report, "questionindex")["value"], "2");
    assert_eq!(source.source_model().unwrap().show_welcome, None);
}

#[test]
fn test_settings_should_keep_explicit_values_and_leave_unresolved_context_unknown() {
    let source = document(
        "<gsid>2</gsid><showwelcome>Y</showwelcome><format>S</format><navigationdelay>3</navigationdelay>",
    );
    let report = source.group_settings(Some(&context())).unwrap();
    assert_eq!(setting(&report, "showwelcome")["origin"]["layer"], "survey");
    assert_eq!(setting(&report, "format")["value"], "S");
    let standalone = source.group_settings(None).unwrap();
    assert_eq!(setting(&standalone, "showwelcome")["value"], true);
    assert_eq!(
        setting(&standalone, "questionindex")["status"],
        "unresolved"
    );
    assert!(!standalone.complete);
}

#[test]
fn test_settings_should_use_global_values_for_group_zero_and_require_its_context() {
    let source = document("<gsid>0</gsid><showwelcome>N</showwelcome><format>S</format>");
    let mut context = context();
    context.survey_group_id = "0".into();
    let report = source.group_settings(Some(&context)).unwrap();
    assert_eq!(setting(&report, "showwelcome")["value"], true);
    assert_eq!(setting(&report, "format")["value"], "G");
    assert_eq!(
        setting(&source.group_settings(None).unwrap(), "showwelcome")["status"],
        "unresolved"
    );
}

#[test]
fn test_settings_should_refuse_foreign_context_cycles_missing_ancestors_and_unknown_keys() {
    let source = document("<gsid>2</gsid><showwelcome>I</showwelcome>");
    let mut foreign = context();
    foreign.survey_id = "702".into();
    assert_eq!(
        source.group_settings(Some(&foreign)).unwrap_err(),
        "settings context belongs to another survey"
    );
    let mut cyclic = context();
    cyclic.groups[1].parent_id = Some("2".into());
    assert_eq!(
        cyclic.validate().unwrap_err(),
        "cyclic settings group hierarchy"
    );
    let mut missing = context();
    missing.groups[1].parent_id = Some("99".into());
    assert_eq!(
        missing.validate().unwrap_err(),
        "missing settings group ancestor"
    );
    let mut unknown = context();
    unknown
        .global
        .insert("private_unknown".into(), Some("not disclosed".into()));
    assert_eq!(
        unknown.validate().unwrap_err(),
        "unknown setting or setting value exceeds 64 bytes"
    );
}

#[test]
fn test_settings_should_not_expose_invalid_source_values_in_its_report() {
    let source = document("<gsid>2</gsid><navigationdelay>private-invalid-value</navigationdelay>");
    let report = source.group_settings(None).unwrap();
    assert_eq!(setting(&report, "navigationdelay")["status"], "invalid");
    assert!(
        !serde_json::to_string(&report)
            .unwrap()
            .contains("private-invalid-value")
    );
}
