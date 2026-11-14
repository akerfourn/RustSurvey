#[path = "support/defaults.rs"]
mod fixture;
use rustsurvey_domain::{
    Definition,
    cells::{Coordinate, Role},
    collection::Questionnaire,
    defaults::DefaultValue,
};
use serde_json::json;

#[test]
fn test_default_definitions_should_validate_supported_cells_and_refuse_ignored_targets() {
    for case in fixture::cases() {
        let definition = fixture::definition(&case);
        let issues = definition.validate();
        if case["nativeSupported"] == false {
            assert!(
                issues
                    .iter()
                    .any(|issue| issue.message.contains("Cellule incompatible")),
                "{}: {issues:?}",
                case["name"]
            );
        } else {
            assert!(issues.is_empty(), "{}: {issues:?}", case["name"]);
            let issues = Questionnaire::compile_forms(&definition)
                .err()
                .expect("defaults must not silently execute in a legacy collection profile");
            assert!(issues.iter().any(|issue| {
                issue
                    .message
                    .contains("initialisation des valeurs par défaut")
            }));
        }
    }
}

#[test]
fn test_definition_upgrade_should_read_v3_without_rewriting_input_and_require_v4_for_defaults() {
    let definition = fixture::definition(&fixture::cases()[0]);
    let mut legacy = serde_json::to_value(&definition).unwrap();
    legacy["schemaVersion"] = json!(3);
    let mut invalid: Definition = serde_json::from_value(legacy.clone()).unwrap();
    assert!(invalid.upgrade().unwrap_err().contains("v4"));
    for question in legacy["groups"][0]["questions"].as_array_mut().unwrap() {
        question.as_object_mut().unwrap().remove("defaults");
    }
    let before = legacy.clone();
    let mut upgraded: Definition = serde_json::from_value(legacy.clone()).unwrap();
    upgraded.upgrade().unwrap();
    assert_eq!(upgraded.schema_version, 4);
    assert_eq!(legacy, before);
    assert!(
        upgraded
            .groups
            .iter()
            .flat_map(|group| &group.questions)
            .all(|question| question.defaults.is_none())
    );
}

#[test]
fn test_defaults_should_keep_empty_values_and_diagnose_duplicate_or_removed_cells() {
    let mut definition = fixture::definition(&fixture::cases()[0]);
    let question = definition.groups[0]
        .questions
        .iter_mut()
        .find(|question| question.code == "S")
        .unwrap();
    let defaults = question.defaults.as_mut().unwrap();
    defaults.values[0].value = "".into();
    let encoded = serde_json::to_value(&definition).unwrap();
    let decoded: Definition = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, definition);
    let defaults = definition.groups[0].questions[0].defaults.as_mut().unwrap();
    defaults.values.push(defaults.values[0].clone());
    assert!(
        definition
            .validate()
            .iter()
            .any(|issue| issue.message.contains("dupliquée"))
    );
    defaults_for(&mut definition).values = vec![DefaultValue {
        coordinate: Coordinate::Subquestion {
            id: "removed".into(),
            scale: None,
        },
        role: Role::Answer,
        language: "fr".into(),
        value: "x".into(),
    }];
    assert!(
        definition
            .validate()
            .iter()
            .any(|issue| issue.message.contains("Cellule incompatible"))
    );
}

fn defaults_for(definition: &mut Definition) -> &mut rustsurvey_domain::defaults::QuestionDefaults {
    definition.groups[0].questions[0].defaults.as_mut().unwrap()
}
