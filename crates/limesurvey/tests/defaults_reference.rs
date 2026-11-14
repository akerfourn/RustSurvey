#[path = "../../domain/tests/support/defaults.rs"]
mod fixture;
use rustsurvey_limesurvey::{Limits, LssDocument, NativeExportError, SourceId, export_native};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

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

fn verify_projection(model: &rustsurvey_limesurvey::SurveySource, case: &Value) {
    let definition = fixture::definition(case);
    let mut target = definition.groups[0]
        .questions
        .iter()
        .find(|question| question.code == case["question"].as_str().unwrap())
        .unwrap()
        .clone();
    let original = model
        .questions
        .iter()
        .find(|question| question.code == target.code)
        .unwrap();
    target.source = Some(rustsurvey_domain::SourceReference {
        id: original.id.0.to_string(),
        kind: Some(original.source_type.clone()),
    });
    for row in &mut target.subquestions {
        row.source = model
            .subquestions
            .iter()
            .find(|source| {
                source.parent_id == original.id
                    && source.code == row.code
                    && source.scale == row.scale
            })
            .map(|source| rustsurvey_domain::SourceReference {
                id: source.id.0.to_string(),
                kind: None,
            });
    }
    let expected = target.defaults.take().unwrap();
    let result = rustsurvey_limesurvey::project_question_defaults(model, original, &target);
    if case["nativeSupported"] == false {
        assert!(result.is_err(), "{}", case["name"]);
    } else {
        let mut actual = result.unwrap().unwrap();
        let mut expected = expected;
        actual
            .values
            .sort_by(|left, right| left.language.cmp(&right.language));
        expected
            .values
            .sort_by(|left, right| left.language.cmp(&right.language));
        assert_eq!(actual, expected, "{}", case["name"]);
    }
}

fn directory() -> PathBuf {
    let path = PathBuf::from(
        std::env::var_os("RUSTSURVEY_DEFAULT_REFERENCE_DIR")
            .expect("isolated synthetic reference directory"),
    );
    assert!(path.is_absolute() && path.is_dir());
    assert!(
        path.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("rustsurvey-limesurvey-reference-")
    );
    path
}

#[test]
#[ignore = "requires the isolated LimeSurvey 6.5.14 reference harness"]
fn test_native_default_reference_should_prepare_versioned_structures() {
    let directory = directory();
    let mut supported = Vec::new();
    for (index, case) in fixture::cases().iter().enumerate() {
        let definition = fixture::definition(case);
        let result = export_native(
            &definition,
            SourceId(1000 + index as u64),
            Limits::default(),
        );
        if case["nativeSupported"] == false {
            assert!(
                matches!(result, Err(NativeExportError::Unsupported(_))),
                "{}",
                case["name"]
            );
            continue;
        }
        let bytes = result
            .unwrap_or_else(|error| panic!("{}: {error:?}", case["name"]))
            .xml;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(format!("native-defaults-{index}.lss")))
            .unwrap()
            .write_all(&bytes)
            .unwrap();
        supported.push(index);
    }
    assert_eq!(supported.len(), 29);
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("native-default-cases.json"))
        .unwrap()
        .write_all(&serde_json::to_vec(&supported).unwrap())
        .unwrap();
}

#[test]
#[ignore = "requires the isolated LimeSurvey 6.5.14 reference harness"]
fn test_default_reference_should_preserve_native_configuration_through_limesurvey() {
    let directory = directory();
    let actual: Value =
        serde_json::from_slice(&read(&directory.join("output/native-default-results.json")))
            .unwrap();
    let cases = fixture::cases();
    let supported: Vec<_> = cases
        .iter()
        .enumerate()
        .filter(|(_, case)| case["nativeSupported"] != false)
        .collect();
    assert_eq!(actual.as_array().unwrap().len(), supported.len());
    for ((index, case), result) in supported.into_iter().zip(actual.as_array().unwrap()) {
        assert_eq!(result["name"], case["name"]);
        let expected: Vec<_> = case["steps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|step| step["expected"].clone())
            .collect();
        assert_eq!(
            result["values"],
            serde_json::to_value(expected).unwrap(),
            "{}",
            case["name"]
        );
        for (step_index, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            if let Some(write) = step.get("expectedWrite") {
                assert_eq!(
                    &result["writes"][step_index], write,
                    "{} step {step_index}",
                    case["name"]
                );
            }
        }
        let bytes = read(&directory.join(format!("output/native-defaults-{index}.lss")));
        let document = LssDocument::parse(&bytes, Limits::default()).unwrap();
        assert!(
            !document.report().has_errors(),
            "{}: {:?}",
            case["name"],
            document.report()
        );
        let model = document.source_model().unwrap();
        let question = model
            .questions
            .iter()
            .find(|question| question.code == case["question"].as_str().unwrap())
            .unwrap();
        assert_eq!(
            question.same_default,
            Some(case["sameDefault"].as_bool().unwrap_or(false))
        );
        assert_eq!(model.default_values.len(), 1);
        let default = &model.default_values[0];
        assert_eq!(default.question_id, question.id);
        assert_eq!(default.scale, 0);
        assert_eq!(default.special_type, case["special"].as_str().unwrap_or(""));
        let subquestion = model.subquestions.iter().find(|row| {
            row.parent_id == question.id && row.code == case["row"].as_str().unwrap_or("")
        });
        assert_eq!(default.subquestion_id, subquestion.map(|row| row.id));
        let values: BTreeMap<_, _> = default
            .values
            .iter()
            .map(|value| (&value.language, &value.value))
            .collect();
        assert_eq!(serde_json::to_value(values).unwrap(), case["values"]);
        verify_projection(model, case);
    }
}

#[test]
#[ignore = "requires the isolated LimeSurvey 6.5.14 reference harness"]
fn test_default_reference_should_preserve_real_exports_and_match_initialization_states() {
    let directory = PathBuf::from(
        std::env::var_os("RUSTSURVEY_DEFAULT_REFERENCE_DIR")
            .expect("isolated synthetic reference directory"),
    );
    assert!(directory.is_absolute() && directory.is_dir());
    assert!(
        directory
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("rustsurvey-limesurvey-reference-")
    );
    let manifest: Value = serde_json::from_str(include_str!(
        "../../../examples/rules/defaults-reference.json"
    ))
    .unwrap();
    let results: Value =
        serde_json::from_slice(&read(&directory.join("output/default-results.json"))).unwrap();
    let cases = manifest["cases"].as_array().unwrap();
    assert_eq!(results.as_array().unwrap().len(), cases.len());
    for (index, case) in cases.iter().enumerate() {
        assert_eq!(results[index]["name"], case["name"]);
        let expected: Vec<_> = case["steps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|step| step["expected"].clone())
            .collect();
        let bytes = read(&directory.join(format!("output/defaults-{index}.lss")));
        let document = LssDocument::parse(&bytes, Limits::default()).unwrap();
        assert!(
            !document.report().has_errors(),
            "{}: {:?}",
            case["name"],
            document.report()
        );
        assert_eq!(document.original_xml(), bytes);
        let model = document.source_model().unwrap();
        assert_eq!(model.default_values.len(), 1, "{}", case["name"]);
        let question = model
            .questions
            .iter()
            .find(|question| question.code == case["question"].as_str().unwrap())
            .unwrap();
        assert_eq!(
            question.same_default,
            Some(case["sameDefault"].as_bool().unwrap_or(false))
        );
        let default = &model.default_values[0];
        assert_eq!(default.question_id, question.id);
        assert_eq!(default.scale, 0);
        assert_eq!(default.special_type, case["special"].as_str().unwrap_or(""));
        let values: BTreeMap<_, _> = default
            .values
            .iter()
            .map(|value| (&value.language, &value.value))
            .collect();
        assert_eq!(serde_json::to_value(values).unwrap(), case["values"]);
        verify_projection(model, case);
        let subquestion = model.subquestions.iter().find(|row| {
            row.parent_id == question.id && row.code == case["row"].as_str().unwrap_or("")
        });
        assert_eq!(default.subquestion_id, subquestion.map(|row| row.id));
        assert_eq!(
            results[index]["values"],
            serde_json::to_value(expected).unwrap(),
            "{}",
            case["name"]
        );
        for (step_index, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            if let Some(write) = step.get("expectedWrite") {
                assert_eq!(
                    &results[index]["writes"][step_index], write,
                    "{} step {step_index}",
                    case["name"]
                );
            }
        }
    }
}
