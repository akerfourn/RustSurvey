#[path = "../../domain/tests/support/defaults.rs"]
mod fixture;
use rustsurvey_limesurvey::{Limits, LssDocument, NativeExportError, SourceId, export_native};
use std::collections::BTreeMap;

#[test]
fn test_native_default_export_should_preserve_languages_targets_and_empty_values() {
    for case in fixture::cases() {
        let definition = fixture::definition(&case);
        let result = export_native(&definition, SourceId(701), Limits::default());
        if case["nativeSupported"] == false {
            assert!(matches!(result, Err(NativeExportError::Unsupported(_))));
            continue;
        }
        let exported = result.unwrap_or_else(|error| panic!("{}: {error:?}", case["name"]));
        let document = LssDocument::parse(&exported.xml, Limits::default()).unwrap();
        assert!(
            !document.report().has_errors(),
            "{}: {:?}",
            case["name"],
            document.report()
        );
        let model = document.source_model().unwrap();
        assert_eq!(model.default_values.len(), 1);
        let value = &model.default_values[0];
        let question = model
            .questions
            .iter()
            .find(|question| question.code == case["question"].as_str().unwrap())
            .unwrap();
        assert_eq!(value.question_id, question.id);
        assert_eq!(
            question.same_default,
            Some(case["sameDefault"].as_bool().unwrap_or(false))
        );
        let values: BTreeMap<_, _> = value
            .values
            .iter()
            .map(|value| (&value.language, &value.value))
            .collect();
        assert_eq!(serde_json::to_value(values).unwrap(), case["values"]);
    }
}

#[test]
fn test_default_coordinates_should_survive_question_and_row_code_changes() {
    let case = fixture::cases()
        .into_iter()
        .find(|case| case["question"] == "Q" && case["row"] == "A1")
        .unwrap();
    let mut definition = fixture::definition(&case);
    let question = definition.groups[0]
        .questions
        .iter_mut()
        .find(|question| question.code == "Q")
        .unwrap();
    let defaults = question.defaults.clone();
    question.code = "RENAMED".into();
    question.subquestions[0].code = "ROW".into();
    let bytes = export_native(&definition, SourceId(701), Limits::default())
        .unwrap()
        .xml;
    let document = LssDocument::parse(&bytes, Limits::default()).unwrap();
    let model = document.source_model().unwrap();
    let question = model
        .questions
        .iter()
        .find(|question| question.code == "RENAMED")
        .unwrap();
    let row = model
        .subquestions
        .iter()
        .find(|row| row.parent_id == question.id && row.code == "ROW")
        .unwrap();
    assert_eq!(model.default_values[0].subquestion_id, Some(row.id));
    assert_eq!(
        definition.groups[0]
            .questions
            .iter()
            .find(|question| question.code == "RENAMED")
            .unwrap()
            .defaults,
        defaults
    );
}
