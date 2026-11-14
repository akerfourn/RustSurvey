use rustsurvey_domain::{Attribute, Definition, QuestionKind, rules};
use serde_json::{Value, json};

fn definition() -> Definition {
    serde_json::from_value(json!({"schemaVersion":2,"title":"Conditions","language":"fr","welcome":"","end":"","groups":[
        {"id":"g1","title":"Controls","description":"","questions":[{"id":"q1","code":"AGE","title":"Age","help":"","kind":"number","mandatory":"no","choices":[]}]},
        {"id":"g2","title":"Follow-up","description":"","questions":[{"id":"q2","code":"DETAIL","title":"Details","help":"","kind":"short_text","mandatory":"strict","choices":[]}]}]})).unwrap()
}

#[test]
fn test_validation_should_allow_self_references_but_reject_unknown_variables() {
    let mut value = definition();
    value.groups[0].questions[0].attributes.push(Attribute {
        name: "em_validation_q".to_owned(),
        value: "AGE >= 18".to_owned(),
        language: None,
    });
    assert!(rules::validate(&value, None).is_empty());
    value.groups[0].questions[0].attributes[0].value = "MISSING > 0".to_owned();
    let issues = rules::validate(&value, None);
    assert!(
        issues
            .iter()
            .any(|issue| issue.path.ends_with("attributes[0].value")
                && issue.message.contains("MISSING"))
    );
}

#[test]
fn test_calculations_should_reject_cycles_and_accept_dependencies_on_prior_answers() {
    let mut value = definition();
    let calculated = &mut value.groups[1].questions[0];
    calculated.kind = QuestionKind::Equation;
    calculated.attributes.push(Attribute {
        name: "equation".to_owned(),
        value: "AGE * 2".to_owned(),
        language: None,
    });
    assert!(rules::validate(&value, None).is_empty());
    value.groups[0].questions[0].relevance = "DETAIL > 0".to_owned();
    assert!(
        rules::validate(&value, None)
            .iter()
            .any(|issue| issue.message.contains("cycle"))
    );
}

#[test]
fn test_settings_should_refuse_invalid_slider_ranges_and_preserve_unchanged_imports() {
    let mut value = definition();
    let question = &mut value.groups[0].questions[0];
    question.kind = QuestionKind::MultipleNumbers;
    for (name, setting) in [
        ("slider_layout", "1"),
        ("slider_min", "10"),
        ("slider_max", "5"),
    ] {
        question.attributes.push(Attribute {
            name: name.to_owned(),
            value: setting.to_owned(),
            language: None,
        });
    }
    assert!(
        rules::validate(&value, None)
            .iter()
            .any(|issue| issue.message.contains("Intervalle invalide"))
    );
    assert!(rules::validate(&value, Some(&value)).is_empty());
}

#[test]
fn test_free_numeric_matrix_inputs_should_only_compare_explicit_bounds_and_ignore_the_list_step() {
    let mut value = definition();
    let question = &mut value.groups[0].questions[0];
    question.kind = QuestionKind::NumberMatrix;
    for (name, setting) in [
        ("input_boxes", "1"),
        ("multiflexible_min", " 50 "),
        ("multiflexible_step", "0"),
    ] {
        question.attributes.push(Attribute {
            name: name.to_owned(),
            value: setting.to_owned(),
            language: None,
        });
    }
    assert!(
        rules::validate(&value, None).is_empty(),
        "an omitted upper bound must remain unbounded"
    );
    value.groups[0].questions[0].attributes.push(Attribute {
        name: "multiflexible_max".to_owned(),
        value: "40".to_owned(),
        language: None,
    });
    assert!(
        rules::validate(&value, None)
            .iter()
            .any(|issue| issue.message.contains("Intervalle invalide"))
    );
}
#[test]
fn test_rule_syntax_should_match_the_shared_client_server_corpus() {
    let corpus: Value =
        serde_json::from_str(include_str!("../../../examples/rules/conditions.json")).unwrap();
    for case in corpus["cases"].as_array().unwrap() {
        let source = case["source"].as_str().unwrap();
        let result = rules::references(source);
        if case["variables"].is_null() {
            assert!(result.is_err(), "unexpected accepted syntax: {source}");
        } else {
            let actual: Vec<_> = result
                .unwrap_or_else(|error| panic!("{source}: {error}"))
                .into_iter()
                .collect();
            let mut expected: Vec<_> = case["variables"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap())
                .collect();
            expected.sort();
            assert_eq!(actual, expected, "{source}");
        }
    }
}
#[test]
fn test_rule_validation_should_reject_missing_references_and_cycles_including_group_dependencies() {
    let mut value = definition();
    value.groups[1].relevance = "AGE >= 18".to_owned();
    assert!(rules::validate(&value, None).is_empty());
    value.groups[1].questions[0].relevance = "UNKNOWN == 'Y'".to_owned();
    assert!(
        rules::validate(&value, None)
            .iter()
            .any(|issue| issue.path.ends_with(".relevance") && issue.message.contains("UNKNOWN"))
    );
    value.groups[1].questions[0].relevance = "1".to_owned();
    value.groups[0].questions[0].relevance = "!is_empty(DETAIL)".to_owned();
    assert!(
        rules::validate(&value, None)
            .iter()
            .any(|issue| issue.message.contains("cycle"))
    );
}
#[test]
fn test_unchanged_imported_rules_should_remain_preserved_while_invalid_edits_are_refused() {
    let mut previous = definition();
    previous.groups[1].questions[0].relevance = "legacy_function(UNKNOWN)".to_owned();
    assert!(rules::validate(&previous, Some(&previous)).is_empty());
    let mut edited = previous.clone();
    edited.groups[1].questions[0].relevance = "legacy_function(AGE)".to_owned();
    assert!(!rules::validate(&edited, Some(&previous)).is_empty());
    edited.groups[1].questions[0].relevance = "AGE.NAOK >= 18".to_owned();
    assert!(rules::validate(&edited, Some(&previous)).is_empty());
}
#[test]
fn test_expression_budgets_should_bound_depth_size_and_token_count() {
    assert!(rules::references(&"1".repeat(4097)).is_err());
    assert!(rules::references(&format!("{}1{}", "(".repeat(40), ")".repeat(40))).is_err());
    assert!(rules::references(&vec!["1"; 300].join("+")).is_err());
}
#[test]
fn test_legacy_upgrade_should_preserve_ids_choice_codes_and_unconditional_behavior() {
    let mut old:Definition=serde_json::from_value(json!({"schemaVersion":1,"title":"Old","language":"fr","welcome":"","end":"","groups":[{"id":"g1","title":"Group","description":"","questions":[{"id":"q1","code":"CHOICES","title":"Choices","help":"","kind":"multiple_choice","mandatory":"no","choices":[{"id":"a1","code":"01","label":"First"},{"id":"a2","code":"02","label":"Second"}]}]}]})).unwrap();
    old.upgrade().unwrap();
    assert_eq!(old.schema_version, 4);
    assert_eq!(old.groups[0].relevance, "1");
    let question = &old.groups[0].questions[0];
    assert_eq!(question.id, "q1");
    assert!(question.choices.is_empty());
    assert_eq!(question.subquestions[0].code, "01");
    assert_eq!(question.subquestions[0].id, "a1");
    assert!(old.validate().is_empty());
    let before = serde_json::to_value(&old).unwrap();
    old.upgrade().unwrap();
    assert_eq!(serde_json::to_value(&old).unwrap(), before);
}
