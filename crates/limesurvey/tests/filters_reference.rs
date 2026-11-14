#[path = "../../domain/tests/support/filters.rs"]
mod fixture;
use rustsurvey_domain::collection::{Answers, Questionnaire};
use rustsurvey_limesurvey::{Limits, SourceId, export_native};
use serde_json::{Value, json};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
};

fn directory() -> PathBuf {
    let directory = PathBuf::from(
        std::env::var_os("RUSTSURVEY_FILTER_REFERENCE_DIR")
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
    directory
}

#[test]
#[ignore = "requires the isolated LimeSurvey 6.5.14 reference harness"]
fn test_filter_reference_should_prepare_synthetic_structures() {
    let directory = directory();
    let mut manifest = Vec::new();
    for (index, case) in fixture::cases().iter().enumerate() {
        let mut definition = fixture::definition(case);
        let checks_mandatory = !case["expectedChoices"].is_null();
        if checks_mandatory {
            definition.groups[0].questions.last_mut().unwrap().mandatory =
                rustsurvey_domain::Mandatory::Strict;
        }
        let input: Answers = serde_json::from_value(fixture::answers(case, &definition)).unwrap();
        let runtime = Questionnaire::compile_forms(&definition).unwrap();
        let snapshot = runtime.evaluate(&input, false).unwrap();
        let mandatory_violation = checks_mandatory.then(|| {
            runtime.evaluate(&input, true).err().is_some_and(|issues| {
                issues.iter().any(|issue| {
                    issue.path == "TARGET" && issue.message == "Cette question est obligatoire."
                })
            })
        });
        let expected: Answers = snapshot
            .answers
            .iter()
            .filter(|(key, _)| key.starts_with("TARGET"))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        assert_eq!(
            serde_json::to_value(expected).unwrap(),
            case["expected"],
            "{}",
            case["name"]
        );
        let source = snapshot.answers;
        let target = definition.groups[0].questions.last().unwrap();
        let mut rows: Vec<_> = target
            .subquestions
            .iter()
            .map(|row| format!("TARGET_{}", row.code))
            .collect();
        if target.other && case["expectedChoices"].is_null() {
            rows.push("TARGET_other".into());
        }
        let expected: std::collections::BTreeMap<_, _> = rows
            .iter()
            .map(|key| (key.clone(), case["expected"].get(key).is_some()))
            .collect();
        let file = format!("filters-{index}.lss");
        let xml = export_native(&definition, SourceId(800 + index as u64), Limits::default())
            .unwrap()
            .xml;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(&file))
            .unwrap()
            .write_all(&xml)
            .unwrap();
        manifest.push(json!({"name":case["name"],"file":file,"answers":source,"expected":expected,"choices":case["expectedChoices"],"mandatoryViolation":mandatory_violation}));
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("filter-cases.json"))
        .unwrap()
        .write_all(&serde_json::to_vec(&manifest).unwrap())
        .unwrap();
}

fn read(path: PathBuf) -> Value {
    let mut bytes = Vec::new();
    File::open(path)
        .unwrap()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 1024 * 1024);
    serde_json::from_slice(&bytes).unwrap()
}

#[test]
#[ignore = "requires the isolated LimeSurvey 6.5.14 reference harness"]
fn test_filter_reference_should_match_actual_limesurvey_row_relevance() {
    let directory = directory();
    let manifest = read(directory.join("filter-cases.json"));
    let actual = read(directory.join("output/filter-results.json"));
    let cases = manifest.as_array().unwrap();
    let results = actual.as_array().unwrap();
    assert_eq!(cases.len(), fixture::cases().len());
    assert_eq!(results.len(), cases.len());
    for (case, result) in cases.iter().zip(results) {
        assert_eq!(result["name"], case["name"]);
        assert_eq!(result["active"], case["expected"], "{}", case["name"]);
        assert_eq!(
            result["choices"], case["choices"],
            "{}: {result}",
            case["name"]
        );
        assert_eq!(
            result["mandatoryViolation"], case["mandatoryViolation"],
            "{}: {result}",
            case["name"]
        );
    }
}
