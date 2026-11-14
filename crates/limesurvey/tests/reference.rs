use rustsurvey_domain::Definition;
use rustsurvey_limesurvey::{
    Limits, LssDocument, ResponseTable, SourceId, SurveySource, export_native,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs::File, io::Read, path::Path};

fn read_bounded(path: &Path) -> Vec<u8> {
    let maximum = 8 * 1024 * 1024;
    let mut bytes = Vec::new();
    File::open(path)
        .unwrap()
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(
        bytes.len() as u64 <= maximum,
        "reference artifact must fit the documented XML limit"
    );
    bytes
}

fn labels(source: &[rustsurvey_limesurvey::LocalizedQuestion]) -> BTreeMap<&str, Value> {
    source
        .iter()
        .map(|item| {
            (
                item.language.as_str(),
                json!({"text":item.text,"help":item.help}),
            )
        })
        .collect()
}

fn questions(model: &SurveySource) -> BTreeMap<&str, Value> {
    let mut result = BTreeMap::new();
    for question in &model.questions {
        let group = model
            .groups
            .iter()
            .find(|group| group.id == question.group_id)
            .unwrap();
        let attributes: BTreeMap<_, _> = question
            .attributes
            .iter()
            .map(|item| {
                (
                    format!("{}:{}", item.name, item.language.as_deref().unwrap_or("")),
                    item.value.as_str(),
                )
            })
            .collect();
        let rows: BTreeMap<_, _> = model.subquestions.iter().filter(|row| row.parent_id == question.id).map(|row| {
            (format!("{}:{}", row.scale, row.code), json!({"order":row.order,"relevance":row.relevance,"labels":labels(&row.labels)}))
        }).collect();
        let answers: BTreeMap<_, _> = model
            .answers
            .iter()
            .filter(|answer| answer.question_id == question.id)
            .map(|answer| {
                let labels: BTreeMap<_, _> = answer
                    .labels
                    .iter()
                    .map(|label| (&label.language, &label.text))
                    .collect();
                (
                    format!("{}:{}", answer.scale, answer.code),
                    json!({"order":answer.order,"labels":labels}),
                )
            })
            .collect();
        result.insert(
            question.code.as_str(),
            json!({
                "groupOrder":group.order,"order":question.order,"type":question.source_type,
                "theme":question.theme,"mandatory":question.mandatory,"other":question.other,
                "relevance":question.relevance,"randomizationGroup":question.randomization_group,
                "validation":question.validation,"labels":labels(&question.labels),
                "attributes":attributes,"rows":rows,"answers":answers,
            }),
        );
    }
    result
}

fn survey_texts(model: &SurveySource) -> BTreeMap<&str, Value> {
    let mut result = BTreeMap::new();
    for language in &model.languages {
        let title = model
            .titles
            .iter()
            .find(|item| &item.language == language)
            .unwrap();
        let messages = model
            .messages
            .iter()
            .find(|item| &item.language == language)
            .unwrap();
        result.insert(
            language.as_str(),
            json!({"title":title.value,"welcome":messages.welcome,"end":messages.end}),
        );
    }
    result
}

fn check_responses(
    directory: &Path,
    summary: &Value,
    bindings: &[rustsurvey_limesurvey::FieldBinding],
) {
    let original: ResponseTable =
        serde_json::from_slice(&read_bounded(&directory.join("../responses.json"))).unwrap();
    let exported = ResponseTable::parse(
        &read_bounded(&directory.join("roundtrip.lsr")),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(exported.rows.len(), original.rows.len());
    let stored = summary["responses"].as_array().unwrap();
    let short = bindings
        .iter()
        .find(|binding| binding.runtime_key == "S")
        .unwrap();
    let long = bindings
        .iter()
        .find(|binding| binding.runtime_key == "T")
        .unwrap();
    let map_column = |column: &String| {
        summary["import"]["FieldReMap"]
            .get(column)
            .and_then(Value::as_str)
            .unwrap_or(column)
            .to_owned()
    };
    let lsr = summary["lsrResponses"].as_array().unwrap();
    let lsr_draft = lsr
        .iter()
        .find(|row| row["id"].as_i64() == Some(2) || row["id"].as_str() == Some("2"))
        .unwrap();
    assert!(
        lsr_draft[map_column(&short.column)].is_null(),
        "the pinned upstream LSR importer converts empty text to NULL"
    );
    let altered = lsr_draft[map_column(&long.column)].as_str().unwrap();
    assert!(
        !altered.contains('\'') && !altered.contains('`'),
        "the pinned LSR importer removes apostrophes and backticks"
    );
    assert_eq!(stored.len(), original.rows.len());
    let campaign = summary["campaignResponses"].as_array().unwrap();
    assert_eq!(
        campaign.len(),
        original.rows.len(),
        "VV without source IDs must create both responses in another survey"
    );
    for row in &original.rows {
        let id = row["id"].as_ref().unwrap();
        let database = stored
            .iter()
            .find(|value| {
                value["id"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| value["id"].to_string())
                    == *id
            })
            .unwrap();
        let roundtrip = exported
            .rows
            .iter()
            .find(|value| value["id"].as_ref() == Some(id))
            .unwrap();
        for (column, value) in row {
            let actual_column = summary["import"]["FieldReMap"]
                .get(column)
                .and_then(Value::as_str)
                .unwrap_or(column);
            let actual = &database[actual_column];
            let expected = value.as_deref();
            let actual = actual
                .as_str()
                .map(str::to_owned)
                .or_else(|| actual.as_i64().map(|number| number.to_string()));
            let binding = bindings.iter().find(|binding| binding.column == *column);
            let key = binding
                .map(|binding| binding.runtime_key.as_str())
                .unwrap_or("");
            let normalized = match (key, expected) {
                ("N" | "K_A1" | "K_A2", Some(value)) => {
                    actual.as_deref() == Some(value)
                        || actual.as_deref() == Some(&format!("{value}.0000000000"))
                }
                ("D", Some(value)) => {
                    actual.as_deref() == Some(value)
                        || actual.as_deref() == Some(&format!("{value} 00:00:00"))
                }
                _ => actual.as_deref() == expected,
            };
            assert!(
                normalized,
                "reference database must preserve response {id}, column {column}"
            );
            if let Some(binding) = binding {
                let target = summary["campaignBindings"][&binding.runtime_key]
                    .as_str()
                    .unwrap();
                let target_row = campaign
                    .iter()
                    .find(|value| {
                        value["id"]
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| value["id"].to_string())
                            == *id
                    })
                    .unwrap();
                assert_eq!(
                    target_row[target], database[actual_column],
                    "campaign export must use codes independently of source IDs"
                );
            }
            // The upstream CDATA exporter changes ]]> and emits literal CR; preserve what its XML actually represents.
            let represented = actual.map(|value| {
                value
                    .replace("]]>", "]] >")
                    .replace("\r\n", "\n")
                    .replace('\r', "\n")
            });
            assert_eq!(
                roundtrip.get(actual_column).unwrap(),
                &represented,
                "read actual exported response {id}, column {column}"
            );
        }
    }
}

#[test]
#[ignore = "requires the isolated LimeSurvey 6.5.14 reference harness"]
fn test_reference_should_preserve_native_structure_through_actual_limesurvey_import_and_export() {
    let definition_path =
        std::env::var_os("RUSTSURVEY_REFERENCE_DEFINITION").expect("synthetic definition path");
    let source_path =
        std::env::var_os("RUSTSURVEY_REFERENCE_LSS").expect("actual LimeSurvey export path");
    let definition: Definition =
        serde_json::from_slice(&read_bounded(Path::new(&definition_path))).unwrap();
    let expected = export_native(&definition, SourceId(701), Limits::default()).unwrap();
    let summary_path = Path::new(&source_path).with_file_name("summary.json");
    let summary: Value = serde_json::from_slice(&read_bounded(&summary_path)).unwrap();
    let actual_bindings = summary["bindings"].as_object().unwrap();
    check_responses(
        Path::new(&source_path).parent().unwrap(),
        &summary,
        &expected.bindings,
    );
    assert_eq!(
        actual_bindings.len(),
        expected.bindings.len(),
        "all typed cells must have an actual reference column"
    );
    for binding in &expected.bindings {
        let column = summary["import"]["FieldReMap"]
            .get(&binding.column)
            .and_then(Value::as_str)
            .unwrap_or(&binding.column);
        assert_eq!(
            actual_bindings
                .get(&binding.runtime_key)
                .and_then(Value::as_str),
            Some(column),
            "cell {} must match createFieldMap",
            binding.runtime_key
        );
    }
    let expected = LssDocument::parse(&expected.xml, Limits::default()).unwrap();
    let actual =
        LssDocument::parse(&read_bounded(Path::new(&source_path)), Limits::default()).unwrap();
    assert!(
        !actual.report().has_errors(),
        "actual LimeSurvey export should parse: {:?}",
        actual.report().diagnostics
    );
    assert_eq!(actual.report().source_schema, 623);
    let expected = expected.source_model().unwrap();
    let actual = actual.source_model().unwrap();
    assert_eq!(actual.base_language, expected.base_language);
    assert_eq!(actual.show_welcome, expected.show_welcome);
    assert_eq!(survey_texts(actual), survey_texts(expected));
    assert_eq!(actual.languages.len(), expected.languages.len());
    for language in &expected.languages {
        assert!(actual.languages.contains(language));
    }
    let actual_questions = questions(actual);
    let expected_questions = questions(expected);
    assert_eq!(actual_questions.len(), expected_questions.len());
    for (code, question) in expected_questions {
        assert_eq!(
            actual_questions.get(code),
            Some(&question),
            "question {code} must survive the actual reference import/export"
        );
    }
    assert_eq!(actual.groups.len(), expected.groups.len());
    for (left, right) in actual.groups.iter().zip(&expected.groups) {
        assert_eq!(left.order, right.order);
        assert_eq!(left.relevance, right.relevance);
        for expected_label in &right.labels {
            let label = left
                .labels
                .iter()
                .find(|label| label.language == expected_label.language)
                .unwrap();
            assert_eq!(label.name, expected_label.name);
            assert_eq!(label.description, expected_label.description);
        }
    }
}
