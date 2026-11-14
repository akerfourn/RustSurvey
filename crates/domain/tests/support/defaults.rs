use rustsurvey_domain::{
    Definition,
    cells::{Coordinate, Role},
    defaults::{DefaultValue, QuestionDefaults},
};
use serde_json::Value;

pub fn cases() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!(
        "../../../../examples/rules/defaults-reference.json"
    ))
    .unwrap()["cases"]
        .as_array()
        .unwrap()
        .clone()
}

pub fn definition(case: &Value) -> Definition {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../examples/rules/collection-forms.json"
    ))
    .unwrap();
    let mut definition: Definition = serde_json::from_value(corpus["definition"].clone()).unwrap();
    definition.upgrade().unwrap();
    definition.languages = vec!["fr".into(), "en".into()];
    for question in definition
        .groups
        .iter_mut()
        .flat_map(|group| &mut group.questions)
    {
        question.relevance = "1".into();
        for row in &mut question.subquestions {
            row.relevance = "1".into()
        }
    }
    let question = definition.groups[0]
        .questions
        .iter_mut()
        .find(|question| question.code == case["question"].as_str().unwrap())
        .unwrap();
    if let Some(relevance) = case["relevance"].as_str() {
        question.relevance = relevance.into()
    }
    if let Some(code) = case["filter"].as_str() {
        question.attributes.push(rustsurvey_domain::Attribute {
            name: "array_filter".into(),
            value: code.into(),
            language: None,
        });
    }
    let coordinate = if let Some(code) = case["row"].as_str() {
        let row = question
            .subquestions
            .iter()
            .find(|row| row.code == code && row.scale == 0)
            .unwrap();
        Coordinate::Subquestion {
            id: row.id.clone(),
            scale: None,
        }
    } else {
        Coordinate::Primary
    };
    let role = if case["special"] == "other" {
        question.other = true;
        Role::Other
    } else {
        Role::Answer
    };
    let values = case["values"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(language, value)| DefaultValue {
            coordinate: coordinate.clone(),
            role,
            language: language.clone(),
            value: value.as_str().unwrap().into(),
        })
        .collect();
    question.defaults = Some(QuestionDefaults {
        same_for_all_languages: case["sameDefault"].as_bool().unwrap_or(false),
        values,
    });
    definition
}
