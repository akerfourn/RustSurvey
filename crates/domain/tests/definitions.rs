use rustsurvey_domain::Definition;
use serde_json::json;

fn fixture() -> serde_json::Value {
    json!({"schemaVersion":1,"title":"Étude native","language":"fr","welcome":"","end":"Merci","groups":[{"id":"g1","title":"Premier groupe","description":"","questions":[{"id":"q1","code":"CHOICE","title":"Votre choix ?","help":"","kind":"single_choice","mandatory":"strict","choices":[{"id":"a1","code":"01","label":"Premier"},{"id":"a2","code":"02","label":"Second"}]}]}]})
}
#[test]
fn test_definition_should_preserve_unicode_and_codes_when_structure_is_valid() {
    let definition: Definition = serde_json::from_value(fixture()).unwrap();
    assert!(definition.validate().is_empty());
    assert_eq!(definition.title, "Étude native");
    assert_eq!(definition.groups[0].questions[0].choices[0].code, "01");
}
#[test]
fn test_definition_should_reject_unknown_fields_and_types_instead_of_dropping_them() {
    let mut value = fixture();
    value["unknownRule"] = json!(true);
    assert!(
        serde_json::from_value::<Definition>(value)
            .unwrap_err()
            .to_string()
            .contains("unknown field")
    );
    let mut value = fixture();
    value["groups"][0]["questions"][0]["kind"] = json!("future_type");
    assert!(
        serde_json::from_value::<Definition>(value)
            .unwrap_err()
            .to_string()
            .contains("unknown variant")
    );
}
#[test]
fn test_definition_should_diagnose_duplicate_ids_and_choice_codes() {
    let mut value = fixture();
    value["groups"][0]["questions"][0]["id"] = json!("g1");
    value["groups"][0]["questions"][0]["choices"][1]["code"] = json!("01");
    let issues = serde_json::from_value::<Definition>(value)
        .unwrap()
        .validate();
    assert!(
        issues
            .iter()
            .any(|issue| issue.path.ends_with(".id") && issue.message.contains("dupliqué"))
    );
    assert!(
        issues
            .iter()
            .any(|issue| issue.path.ends_with(".code") && issue.message.contains("dupliqué"))
    );
}
#[test]
fn test_definition_should_reject_ambiguous_answer_names_and_required_information() {
    let mut value = fixture();
    let mut second = value["groups"][0]["questions"][0].clone();
    second["id"] = json!("q2");
    second["code"] = json!("CHOICE_01");
    second["kind"] = json!("information");
    second["choices"] = json!([]);
    value["groups"][0]["questions"][0]["kind"] = json!("multiple_choice");
    value["groups"][0]["questions"]
        .as_array_mut()
        .unwrap()
        .push(second.clone());
    assert!(
        serde_json::from_value::<Definition>(value.clone())
            .unwrap()
            .validate()
            .iter()
            .any(|issue| issue.path.ends_with(".mandatory"))
    );
    second["kind"] = json!("short_text");
    value["groups"][0]["questions"][1] = second;
    assert!(
        serde_json::from_value::<Definition>(value)
            .unwrap()
            .validate()
            .iter()
            .any(|issue| issue.message.contains("ambigu"))
    );
}
#[test]
fn test_definition_should_bound_structure_text_and_schema_versions() {
    let mut value = fixture();
    value["schemaVersion"] = json!(5);
    value["title"] = json!("a".repeat(201));
    let issues = serde_json::from_value::<Definition>(value)
        .unwrap()
        .validate();
    assert!(issues.iter().any(|issue| issue.path == "schemaVersion"));
    assert!(issues.iter().any(|issue| issue.path == "title"));
    let mut value = fixture();
    value["groups"] = json!(vec![value["groups"][0].clone(); 51]);
    assert!(
        serde_json::from_value::<Definition>(value)
            .unwrap()
            .validate()
            .iter()
            .any(|issue| issue.message.contains("50 groupes"))
    );
}

#[test]
fn test_definition_should_refuse_more_answer_cells_than_the_viewer_can_execute() {
    let mut value = fixture();
    let mut question = value["groups"][0]["questions"][0].clone();
    question["kind"] = json!("multiple_choice");
    question["choices"] = json!(vec![question["choices"][0].clone(); 200]);
    value["groups"][0]["questions"] = json!(vec![question; 101]);
    let issues = serde_json::from_value::<Definition>(value)
        .unwrap()
        .validate();
    assert!(
        issues
            .iter()
            .any(|issue| issue.path == "groups" && issue.message.contains("20000 cellules"))
    );
}

#[test]
fn test_v2_upgrade_should_add_empty_media_without_changing_ids_and_reject_v3_references_in_an_old_format()
 {
    let mut value = fixture();
    value["schemaVersion"] = json!(2);
    let mut upgraded: Definition = serde_json::from_value(value.clone()).unwrap();
    upgraded.upgrade().unwrap();
    assert_eq!(upgraded.schema_version, 4);
    assert_eq!(upgraded.groups[0].questions[0].id, "q1");
    assert_eq!(upgraded.groups[0].questions[0].choices[0].code, "01");
    assert!(upgraded.groups[0].questions[0].media.is_empty());
    value["groups"][0]["questions"][0]["media"] = json!([{"id":"00000000-0000-4000-8000-000000000010","alt":"Synthetic pixel","language":null}]);
    let mut invalid: Definition = serde_json::from_value(value).unwrap();
    assert_eq!(
        invalid.upgrade().unwrap_err(),
        "Les références de médias nécessitent le format natif v3."
    );
    assert_eq!(
        invalid.schema_version, 2,
        "a rejected upgrade must not partially mutate the document"
    );
}
