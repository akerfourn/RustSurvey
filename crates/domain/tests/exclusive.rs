use rustsurvey_domain::{
    Attribute, Definition,
    collection::{Answers, Questionnaire},
};
use serde_json::{Value, json};

#[test]
fn test_exclusive_parser_should_reject_duplicate_attributes_without_choosing_a_value() {
    let mut definition = definition(&json!({"type":"M","exclusive":"A2"}));
    let question = &mut definition.groups[0].questions[0];
    let mut duplicate = question.attributes[0].clone();
    duplicate.value = "A1".into();
    question.attributes.push(duplicate);
    let error = rustsurvey_domain::exclusive::ExclusiveOptions::parse(question).unwrap_err();
    assert!(error.contains("déclarée plusieurs fois"));
}

fn definition(case: &Value) -> Definition {
    let mut source: Value = serde_json::from_str(include_str!(
        "../../../examples/rules/collection-forms.json"
    ))
    .unwrap();
    let questions = source["definition"]["groups"][0]["questions"]
        .as_array()
        .unwrap();
    let mut question = questions
        .iter()
        .find(|question| question["code"] == case["type"])
        .unwrap()
        .clone();
    question["mandatory"] = json!("no");
    question["relevance"] = json!("1");
    question["other"] = json!(case["other"] == true);
    question["attributes"] = json!([
        {"name":"exclude_all_others","value":case["exclusive"]},
        {"name":"exclude_all_others_auto","value":if case["auto"] == true {"1"} else {"0"}}
    ]);
    for row in question["subquestions"].as_array_mut().unwrap() {
        row["relevance"] = json!("1");
        if (case["hideExclusive"] == true && row["code"] == "A2")
            || (case["hideOthers"] == true && row["code"] == "A1")
        {
            row["relevance"] = json!("0");
        }
    }
    let mut selected = vec![question];
    if case["withFilter"] == true {
        let mut filter = questions
            .iter()
            .find(|question| question["code"] == "F")
            .unwrap()
            .clone();
        filter["mandatory"] = json!("no");
        filter["relevance"] = json!("1");
        filter["attributes"] = json!([{"name":"array_filter","value":"M"}]);
        for row in filter["subquestions"].as_array_mut().unwrap() {
            row["relevance"] = json!("1");
        }
        selected.push(filter);
    }
    source["definition"]["groups"][0]["questions"] = json!(selected);
    serde_json::from_value(source["definition"].clone()).unwrap()
}

#[test]
fn test_exclusive_options_should_match_the_shared_manual_and_automatic_corpus() {
    let corpus: Value =
        serde_json::from_str(include_str!("../../../examples/rules/exclusive.json")).unwrap();
    let mut cases = corpus["cases"].as_array().unwrap().clone();
    for fixture in corpus["manual"].as_array().unwrap() {
        let code = fixture["type"].as_str().unwrap();
        let value = fixture["value"].as_str().unwrap();
        cases.push(json!({"name":format!("manual {code}"),"type":code,"exclusive":"A2","answers":{format!("{code}_A1"):value,format!("{code}_A2"):value},"expected":{format!("{code}_A2"):value}}));
    }
    for case in cases {
        let definition = definition(&case);
        let runtime = Questionnaire::compile_forms(&definition).unwrap();
        let answers: Answers = serde_json::from_value(case["answers"].clone()).unwrap();
        let result = runtime.evaluate(&answers, false);
        if let Some(error) = case["error"].as_str() {
            assert!(
                result
                    .err()
                    .unwrap()
                    .iter()
                    .any(|issue| issue.message.contains(error)),
                "{}",
                case["name"]
            );
        } else {
            let snapshot = result.unwrap();
            assert_eq!(
                serde_json::to_value(snapshot.answers).unwrap(),
                case["expected"],
                "{}",
                case["name"]
            );
        }
    }
}

#[test]
fn test_exclusive_settings_should_validate_codes_types_and_edited_rows_without_rejecting_unchanged_imports()
 {
    let mut definition = definition(&json!({"type":"M","exclusive":"A2"}));
    let original = definition.clone();
    definition.groups[0].questions[0].subquestions[1].code = "NONE".into();
    assert!(
        rustsurvey_domain::rules::validate(&definition, Some(&original))
            .iter()
            .any(|issue| issue
                .message
                .contains("code de ligne exclusive est inconnu"))
    );
    let question = &mut definition.groups[0].questions[0];
    question.attributes[0].value = "NONE".into();
    assert!(rustsurvey_domain::rules::validate(&definition, None).is_empty());
    definition.groups[0].questions[0]
        .attributes
        .push(Attribute {
            name: "exclude_all_others".into(),
            value: "UNKNOWN".into(),
            language: Some("en".into()),
        });
    assert!(Questionnaire::compile_forms(&definition).is_err());
    assert!(rustsurvey_domain::rules::validate(&definition, Some(&definition)).is_empty());
}
