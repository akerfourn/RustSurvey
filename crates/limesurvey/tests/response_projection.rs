use rustsurvey_domain::{Definition, SourceReference};
use rustsurvey_limesurvey::{
    FieldBinding, Limits, LssDocument, ResponseProjector, ResponseRow, SurveySource, export_native,
    source_bindings,
};
use serde_json::Value;

fn fixture() -> (
    Definition,
    LssDocument,
    Vec<FieldBinding>,
    Vec<String>,
    ResponseRow,
) {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../examples/rules/collection-forms.json"
    ))
    .unwrap();
    let mut definition: Definition = serde_json::from_value(corpus["definition"].clone()).unwrap();
    definition.upgrade().unwrap();
    let exported = export_native(
        &definition,
        rustsurvey_limesurvey::SourceId(701),
        Limits::default(),
    )
    .unwrap();
    let source = LssDocument::parse(&exported.xml, Limits::default()).unwrap();
    attach_fixture_identities(&mut definition, source.source_model().unwrap());
    let fields = source_bindings(&definition, source.source_model().unwrap()).unwrap();
    assert_eq!(fields.issue_count, 0);
    let mut columns: Vec<_> = fields
        .fields
        .iter()
        .map(|field| field.column.clone())
        .collect();
    columns.extend(["id", "submitdate", "startlanguage", "lastpage", "token"].map(str::to_owned));
    let mut row: ResponseRow = columns
        .iter()
        .map(|column| (column.clone(), None))
        .collect();
    for field in &fields.fields {
        if let Some(value) = corpus["baseline"]["answers"][&field.runtime_key].as_str() {
            row.insert(field.column.clone(), Some(value.into()));
        }
    }
    row.insert("id".into(), Some("001".into()));
    row.insert("startlanguage".into(), Some("fr".into()));
    (definition, source, fields.fields, columns, row)
}

fn attach_fixture_identities(definition: &mut Definition, source: &SurveySource) {
    for group in &mut definition.groups {
        let original = source.groups.iter().find(|item| item.order == 0).unwrap();
        group.source = Some(SourceReference {
            id: original.id.0.to_string(),
            kind: None,
        });
        for question in &mut group.questions {
            let original = source
                .questions
                .iter()
                .find(|item| item.code == question.code)
                .unwrap();
            question.source = Some(SourceReference {
                id: original.id.0.to_string(),
                kind: Some(original.source_type.clone()),
            });
            for row in &mut question.subquestions {
                let original = source
                    .subquestions
                    .iter()
                    .find(|item| {
                        item.parent_id == original.id
                            && item.code == row.code
                            && item.scale == row.scale
                    })
                    .unwrap();
                row.source = Some(SourceReference {
                    id: original.id.0.to_string(),
                    kind: None,
                });
            }
            for choice in &mut question.choices {
                let original = source
                    .answers
                    .iter()
                    .find(|item| {
                        item.question_id == original.id
                            && item.code == choice.code
                            && item.scale == choice.scale
                    })
                    .unwrap();
                choice.source = Some(SourceReference {
                    id: original.id.0.to_string(),
                    kind: None,
                });
            }
        }
    }
}

fn column<'a>(fields: &'a [FieldBinding], key: &str) -> &'a str {
    &fields
        .iter()
        .find(|field| field.runtime_key == key)
        .unwrap()
        .column
}

#[test]
fn test_response_projection_should_restore_all_cells_without_recalculating_or_clearing_history() {
    let (definition, source, _, columns, row) = fixture();
    let projector =
        ResponseProjector::compile(&definition, source.source_model().unwrap(), &columns).unwrap();
    let projected = projector.project(&row).unwrap();
    assert_eq!(
        projected.answers["CALC"], "forged",
        "the historical calculation must not be recomputed"
    );
    assert_eq!(
        projected.answers["P_A2comment"], "inactive",
        "historical inactive cells must not be cleared"
    );
    assert_eq!(projected.answers["L"], "01");
    assert_eq!(projected.answers["DUAL_A1_1"], "10");
    assert_eq!(projected.answers["NUM_A2_B2"], "4");
    assert_eq!(projected.answers["TXT_A2_B2"], "texte");
    assert_eq!(projected.metadata["id"].as_deref(), Some("001"));
    assert!(projected.issues.is_empty());
    assert_eq!(projector.restore(&projected).unwrap(), row);
}

#[test]
fn test_response_projection_should_keep_null_empty_and_unknown_initial_language_distinct() {
    let (definition, source, fields, mut columns, mut row) = fixture();
    row.insert(column(&fields, "S").into(), Some("".into()));
    row.insert(column(&fields, "T").into(), None);
    row.insert("startlanguage".into(), Some("unknown-language".into()));
    let projector =
        ResponseProjector::compile(&definition, source.source_model().unwrap(), &columns).unwrap();
    let projected = projector.project(&row).unwrap();
    assert_eq!(projected.answers.get("S").map(String::as_str), Some(""));
    assert!(!projected.answers.contains_key("T"));
    assert_eq!(
        projected.metadata["startlanguage"].as_deref(),
        Some("unknown-language")
    );
    assert_eq!(projector.restore(&projected).unwrap(), row);
    columns.retain(|name| name != "startlanguage");
    row.remove("startlanguage");
    let projector =
        ResponseProjector::compile(&definition, source.source_model().unwrap(), &columns).unwrap();
    let projected = projector.project(&row).unwrap();
    assert!(!projected.metadata.contains_key("startlanguage"));
    assert_eq!(projector.restore(&projected).unwrap(), row);
}

#[test]
fn test_response_projection_should_retain_midnight_date_lexemes_until_the_answer_changes() {
    let (definition, source, fields, columns, mut row) = fixture();
    let date = column(&fields, "D");
    row.insert(date.into(), Some("2024-02-29 00:00:00".into()));
    let projector =
        ResponseProjector::compile(&definition, source.source_model().unwrap(), &columns).unwrap();
    let mut projected = projector.project(&row).unwrap();
    assert_eq!(projected.answers["D"], "2024-02-29");
    assert_eq!(
        projected.original_values[date].source_value,
        "2024-02-29 00:00:00"
    );
    assert_eq!(projector.restore(&projected).unwrap(), row);
    projected.answers.insert("D".into(), "2024-03-01".into());
    assert_eq!(
        projector.restore(&projected).unwrap()[date].as_deref(),
        Some("2024-03-01")
    );
    row.insert(date.into(), Some("2024-02-29 13:00:00".into()));
    let projected = projector.project(&row).unwrap();
    assert_eq!(projected.answers["D"], "2024-02-29 13:00:00");
    assert_eq!(projected.issue_count, 1);
    assert_eq!(projector.restore(&projected).unwrap(), row);
}

#[test]
fn test_response_projection_should_preserve_unknown_columns_and_invalid_historical_numbers_with_diagnostics()
 {
    let (definition, source, fields, mut columns, mut row) = fixture();
    columns.push("future_metadata".into());
    row.insert(
        "future_metadata".into(),
        Some("synthetic-private-value".into()),
    );
    row.insert(
        column(&fields, "N").into(),
        Some("historically-invalid".into()),
    );
    let projector =
        ResponseProjector::compile(&definition, source.source_model().unwrap(), &columns).unwrap();
    let projected = projector.project(&row).unwrap();
    assert_eq!(projected.answers["N"], "historically-invalid");
    assert_eq!(
        projected.metadata["future_metadata"].as_deref(),
        Some("synthetic-private-value")
    );
    assert_eq!(projected.issue_count, 1);
    assert!(
        projected
            .issues
            .iter()
            .all(|issue| !issue.message.contains("synthetic-private-value"))
    );
    assert_eq!(projector.restore(&projected).unwrap(), row);
}

#[test]
fn test_response_projection_should_refuse_missing_columns_mismatched_identities_and_undeclared_values()
 {
    let (mut definition, source, fields, mut columns, mut row) = fixture();
    columns.retain(|name| name != column(&fields, "S"));
    let issues = ResponseProjector::compile(&definition, source.source_model().unwrap(), &columns)
        .err()
        .unwrap();
    assert!(
        issues[0]
            .message
            .contains("Colonne source attendue absente")
    );
    columns.push(column(&fields, "S").into());
    let projector =
        ResponseProjector::compile(&definition, source.source_model().unwrap(), &columns).unwrap();
    row.insert("undeclared".into(), Some("not exposed".into()));
    let issues = projector.project(&row).unwrap_err();
    assert!(issues[0].message.contains("non déclarée"));
    definition.groups[0].questions[0]
        .source
        .as_mut()
        .unwrap()
        .id = "999999".into();
    assert!(
        ResponseProjector::compile(&definition, source.source_model().unwrap(), &columns).is_err()
    );
}

#[test]
fn test_response_projection_should_reject_oversized_cells_and_unknown_native_coordinates() {
    let (definition, source, fields, columns, mut row) = fixture();
    let projector =
        ResponseProjector::compile(&definition, source.source_model().unwrap(), &columns).unwrap();
    let mut projected = projector.project(&row).unwrap();
    projected
        .answers
        .insert("UNKNOWN".into(), "not exposed".into());
    assert!(projector.restore(&projected).is_err());
    row.insert(column(&fields, "S").into(), Some("🦀".repeat(32_769)));
    let issues = projector.project(&row).unwrap_err();
    assert!(issues[0].message.contains("limites du document"));
}
