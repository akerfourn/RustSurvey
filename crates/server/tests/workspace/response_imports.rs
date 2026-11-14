use super::*;
use rustsurvey_limesurvey::{Limits, ResponseTable};
use std::collections::BTreeMap;

const LSS: &[u8] = include_bytes!("../../../limesurvey/tests/fixtures/minimal-623.lss");
const PRIVATE: &str = "  preserve private O'Neil `text` \r\n\t日本語 ]]>  ";

fn table(count: usize) -> ResponseTable {
    let columns = [
        "id",
        "submitdate",
        "startlanguage",
        "701X20X101",
        "701X20X102SQ001",
        "unknown_metadata",
        "token",
    ];
    let rows = (0..count)
        .map(|ordinal| {
            BTreeMap::from([
                ("id".into(), Some(format!("{:03}", ordinal + 1))),
                (
                    "submitdate".into(),
                    if ordinal == 0 {
                        Some("1980-01-01 00:00:00".into())
                    } else {
                        None
                    },
                ),
                ("startlanguage".into(), Some("fr".into())),
                ("701X20X101".into(), Some("01".into())),
                (
                    "701X20X102SQ001".into(),
                    if ordinal == 0 { Some("".into()) } else { None },
                ),
                ("unknown_metadata".into(), Some(PRIVATE.into())),
                ("token".into(), Some("synthetic-source-token".into())),
            ])
        })
        .collect();
    ResponseTable {
        languages: vec!["fr".into(), "en".into()],
        columns: columns.into_iter().map(str::to_owned).collect(),
        rows,
    }
}

async fn upload(
    app: &Router,
    study: &str,
    bytes: Vec<u8>,
    cookie: Option<&str>,
    csrf: Option<&str>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/admin/studies/{study}/response-imports?revision=1"
        ))
        .header(header::CONTENT_TYPE, "application/xml")
        .header("X-Requested-With", "RustSurvey");
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    if let Some(csrf) = csrf {
        request = request.header("X-CSRF-Token", csrf);
    }
    app.clone()
        .oneshot(request.body(Body::from(bytes)).unwrap())
        .await
        .unwrap()
}

async fn imported(fixture: &Fixture) -> (String, String, String) {
    let (cookie, csrf) = login(fixture).await;
    let response = import_request(fixture, LSS.to_vec(), Some(&cookie), Some(&csrf)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let study = json(response).await;
    (study["id"].as_str().unwrap().into(), cookie, csrf)
}

#[tokio::test]
async fn test_response_import_should_preserve_source_rows_privately_and_retry_without_duplicates() {
    roundtrip(&fixture().await).await;
}

pub(super) async fn roundtrip(fixture: &Fixture) {
    let (study, cookie, csrf) = imported(fixture).await;
    let mut source = table(3);
    source.rows[0].insert(
        "unknown_metadata".into(),
        Some(format!("{PRIVATE}{}", "x".repeat(70_000))),
    );
    let bytes = source.to_xml(Limits::default()).unwrap();
    assert_eq!(
        upload(&fixture.app, &study, b"invalid".to_vec(), None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        upload(
            &fixture.app,
            &study,
            b"invalid".to_vec(),
            Some(&cookie),
            None
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let response = upload(
        &fixture.app,
        &study,
        bytes.clone(),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    let status = response.status();
    let batch = json(response).await;
    assert_eq!(status, StatusCode::CREATED, "{batch}");
    assert_eq!(batch["state"], "prepared");
    assert_eq!(batch["loadedRows"], 3);
    assert_eq!(batch["report"]["submitted"], 1);
    assert_eq!(batch["report"]["drafts"], 2);
    assert_eq!(batch["report"]["mappedCells"], 2);
    assert_eq!(batch["report"]["unknownColumnCount"], 1);
    assert_eq!(batch["report"]["execution"], "unverified");
    assert!(!batch.to_string().contains("synthetic-source-token"));
    assert!(!batch.to_string().contains("O'Neil"));
    let id = batch["id"].as_str().unwrap();
    let base = format!("/api/admin/response-imports/{id}");
    for suffix in [
        "",
        "/original.lsr",
        "/mapping.json",
        "/records",
        "/records/0/record.json",
        "/records/0/native-cells.json",
    ] {
        assert_eq!(
            call(
                &fixture.app,
                "GET",
                &format!("{base}{suffix}"),
                None,
                None,
                None
            )
            .await
            .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let original = call(
        &fixture.app,
        "GET",
        &format!("{base}/original.lsr"),
        None,
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(original.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(
        to_bytes(original.into_body(), 8 * 1024 * 1024)
            .await
            .unwrap(),
        bytes
    );
    let mapping = json(
        call(
            &fixture.app,
            "GET",
            &format!("{base}/mapping.json"),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(mapping["bindings"].as_array().unwrap().len(), 2);
    assert_eq!(mapping["sourceSurveyId"], "701");
    for (ordinal, expected) in source.rows.iter().enumerate() {
        let values = json(
            call(
                &fixture.app,
                "GET",
                &format!("{base}/records/{ordinal}/record.json"),
                None,
                Some(&cookie),
                None,
            )
            .await,
        )
        .await;
        let actual: BTreeMap<String, Option<String>> = source
            .columns
            .iter()
            .map(|column| {
                (
                    column.clone(),
                    values
                        .get(column)
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                )
            })
            .collect();
        assert_eq!(&actual, expected);
        let response = call(
            &fixture.app,
            "GET",
            &format!("{base}/records/{ordinal}/native-cells.json"),
            None,
            Some(&cookie),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let converted = json(response).await;
        let mut expected_answers = serde_json::Map::new();
        for binding in mapping["bindings"].as_array().unwrap() {
            if let Some(Some(value)) = expected.get(binding["column"].as_str().unwrap()) {
                expected_answers
                    .insert(binding["runtimeKey"].as_str().unwrap().into(), json!(value));
            }
        }
        assert_eq!(converted["schemaVersion"], 1);
        assert_eq!(converted["revision"], 1);
        assert_eq!(
            converted["projection"]["answers"],
            Value::Object(expected_answers)
        );
        assert_eq!(
            converted["projection"]["metadata"]["id"],
            format!("{:03}", ordinal + 1)
        );
        assert_eq!(
            converted["projection"]["metadata"]["unknown_metadata"],
            expected["unknown_metadata"].as_deref().unwrap()
        );
        assert_eq!(converted["projection"]["issueCount"], 1);
        assert_eq!(converted["execution"], "blocked");
        assert_eq!(converted["continuity"], "unavailable");
    }
    let again = json(upload(&fixture.app, &study, bytes, Some(&cookie), Some(&csrf)).await).await;
    assert_eq!(again["id"], batch["id"]);
    let records = json(
        call(
            &fixture.app,
            "GET",
            &format!("{base}/records"),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(records["items"].as_array().unwrap().len(), 3);
    assert_eq!(records["items"][0]["sourceId"], "1");
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            &format!("/api/admin/studies/{study}/runs"),
            Some(json!({"expectedRevision":1})),
            Some(&cookie),
            Some(&csrf)
        )
        .await
        .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let public = json(
        call(
            &fixture.app,
            "GET",
            &format!("/api/studies/{study}"),
            None,
            None,
            None,
        )
        .await,
    )
    .await;
    assert!(!public.to_string().contains("synthetic-source-token"));
}

#[tokio::test]
async fn test_response_projection_should_reject_tampered_rows_after_authorization_without_creating_a_campaign()
 {
    let fixture = fixture().await;
    let (study, cookie, csrf) = imported(&fixture).await;
    let batch = json(
        upload(
            &fixture.app,
            &study,
            table(1).to_xml(Limits::default()).unwrap(),
            Some(&cookie),
            Some(&csrf),
        )
        .await,
    )
    .await;
    let id = batch["id"].as_str().unwrap();
    let path = format!("/api/admin/response-imports/{id}/records/0/native-cells.json");
    fixture
        .database
        .execute(
            &Query::update()
                .table("response_import_records")
                .value("document", "deliberately-invalid-synthetic-JSON")
                .and_where(Expr::col("batch_id").eq(id))
                .to_owned(),
        )
        .await
        .unwrap();
    assert_eq!(
        call(&fixture.app, "GET", &path, None, None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = call(&fixture.app, "GET", &path, None, Some(&cookie), None).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        !json(response)
            .await
            .to_string()
            .contains("deliberately-invalid")
    );
    let runs = json(
        call(
            &fixture.app,
            "GET",
            &format!("/api/admin/studies/{study}/runs"),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert!(runs["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_response_import_should_resume_from_committed_rows_after_router_restart() {
    resume_roundtrip(&fixture().await).await;
}

pub(super) async fn resume_roundtrip(fixture: &Fixture) {
    let (study, cookie, csrf) = imported(fixture).await;
    let bytes = table(520).to_xml(Limits::default()).unwrap();
    let response = upload(
        &fixture.app,
        &study,
        bytes.clone(),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let initial = json(response).await;
    assert_eq!(initial["loadedRows"], 500);
    let id = initial["id"].as_str().unwrap();
    let restarted = routes(WorkspaceState::with_runtime(
        fixture.database.clone(),
        false,
        fixture.runtime.clone(),
    ));
    let resumed = json(
        call(
            &restarted,
            "POST",
            &format!("/api/admin/response-imports/{id}/continue"),
            Some(json!({})),
            Some(&cookie),
            Some(&csrf),
        )
        .await,
    )
    .await;
    assert_eq!(resumed["state"], "prepared");
    assert_eq!(resumed["loadedRows"], 520);
    let repeated = json(upload(&restarted, &study, bytes, Some(&cookie), Some(&csrf)).await).await;
    assert_eq!(repeated["id"], initial["id"]);
    assert_eq!(repeated["loadedRows"], 520);
    let last = json(
        call(
            &restarted,
            "GET",
            &format!("/api/admin/response-imports/{id}/records?from=519"),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(last["items"].as_array().unwrap().len(), 1);
    assert_eq!(last["items"][0]["sourceId"], "520");
}

#[tokio::test]
async fn test_response_import_should_report_duplicate_ids_missing_columns_and_unknown_submission_state()
 {
    let fixture = fixture().await;
    let (study, cookie, csrf) = imported(&fixture).await;
    let mut source = table(2);
    source.languages = vec!["de".into()];
    source
        .columns
        .retain(|column| !["submitdate", "701X20X102SQ001"].contains(&column.as_str()));
    for row in &mut source.rows {
        row.remove("submitdate");
        row.remove("701X20X102SQ001");
        row.insert("id".into(), Some("001".into()));
    }
    let result = json(
        upload(
            &fixture.app,
            &study,
            source.to_xml(Limits::default()).unwrap(),
            Some(&cookie),
            Some(&csrf),
        )
        .await,
    )
    .await;
    assert_eq!(result["rowCount"], 2);
    assert_eq!(result["report"]["duplicateIds"], 1);
    assert_eq!(result["report"]["unknownStatus"], 2);
    assert_eq!(result["report"]["missingColumnCount"], 1);
    assert_eq!(result["report"]["mapping"], "partial");
    assert!(
        result["report"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["path"] == "languages")
    );
}

#[tokio::test]
async fn test_response_import_should_prepare_empty_xml_without_inventing_missing_values() {
    let fixture = fixture().await;
    let (study, cookie, csrf) = imported(&fixture).await;
    let mut source = table(0);
    source.columns.clear();
    let response = upload(
        &fixture.app,
        &study,
        source.to_xml(Limits::default()).unwrap(),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let result = json(response).await;
    assert_eq!(result["state"], "prepared");
    assert_eq!(result["rowCount"], 0);
    assert_eq!(result["loadedRows"], 0);
    assert_eq!(result["report"]["missingColumnCount"], 0);
    assert_eq!(result["report"]["mapping"], "complete");
}

#[tokio::test]
async fn test_response_import_should_refuse_foreign_survey_columns_before_creating_a_batch() {
    let fixture = fixture().await;
    let (study, cookie, csrf) = imported(&fixture).await;
    let bytes = String::from_utf8(table(1).to_xml(Limits::default()).unwrap())
        .unwrap()
        .replace("701X20", "702X20")
        .into_bytes();
    let response = upload(&fixture.app, &study, bytes, Some(&cookie), Some(&csrf)).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let batches = json(
        call(
            &fixture.app,
            "GET",
            &format!("/api/admin/studies/{study}/response-imports"),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert!(batches["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_response_import_should_keep_one_record_per_ordinal_when_resumes_compete() {
    let fixture = fixture().await;
    let (study, cookie, csrf) = imported(&fixture).await;
    let bytes = table(1200).to_xml(Limits::default()).unwrap();
    let initial = json(upload(&fixture.app, &study, bytes, Some(&cookie), Some(&csrf)).await).await;
    let id = initial["id"].as_str().unwrap();
    let path = format!("/api/admin/response-imports/{id}/continue");
    let (first, second) = tokio::join!(
        call(
            &fixture.app,
            "POST",
            &path,
            Some(json!({})),
            Some(&cookie),
            Some(&csrf)
        ),
        call(
            &fixture.app,
            "POST",
            &path,
            Some(json!({})),
            Some(&cookie),
            Some(&csrf)
        )
    );
    for response in [first, second] {
        assert!([StatusCode::OK, StatusCode::CONFLICT].contains(&response.status()));
    }
    let final_state = json(
        call(
            &fixture.app,
            "POST",
            &path,
            Some(json!({})),
            Some(&cookie),
            Some(&csrf),
        )
        .await,
    )
    .await;
    assert_eq!(final_state["state"], "prepared");
    assert_eq!(final_state["loadedRows"], 1200);
    let row = fixture
        .database
        .query_one(
            &Query::select()
                .expr(Expr::col("ordinal").count())
                .from("response_import_records")
                .and_where(Expr::col("batch_id").eq(id))
                .to_owned(),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get_by_index::<i64>(0).unwrap(), 1200);
}

#[tokio::test]
async fn test_source_bindings_should_match_all_native_cell_types_by_archived_identities() {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../examples/rules/collection-forms.json"
    ))
    .unwrap();
    let definition: rustsurvey_domain::Definition =
        serde_json::from_value(corpus["definition"].clone()).unwrap();
    let expected = rustsurvey_limesurvey::export_native(
        &definition,
        rustsurvey_limesurvey::SourceId(701),
        Limits::default(),
    )
    .unwrap();
    let imported =
        json(import_request(&fixture, expected.xml.clone(), Some(&cookie), Some(&csrf)).await)
            .await;
    let mut definition: rustsurvey_domain::Definition =
        serde_json::from_value(imported["definition"].clone()).unwrap();
    let lss = rustsurvey_limesurvey::LssDocument::parse(&expected.xml, Limits::default()).unwrap();
    let mappings =
        rustsurvey_limesurvey::source_bindings(&definition, lss.source_model().unwrap()).unwrap();
    assert!(mappings.issues.is_empty());
    assert_eq!(mappings.fields.len(), expected.bindings.len());
    for field in &expected.bindings {
        let actual = mappings
            .fields
            .iter()
            .find(|item| item.column == field.column)
            .unwrap();
        assert_eq!(actual.runtime_key, field.runtime_key);
        assert_ne!(actual.address.question_id, field.address.question_id);
    }
    let question_id = definition.groups[0].questions[0].id.clone();
    definition.groups[0].questions[0].source = None;
    let rejected =
        rustsurvey_limesurvey::source_bindings(&definition, lss.source_model().unwrap()).unwrap();
    assert!(rejected.issue_count > 0);
    assert!(
        !rejected
            .fields
            .iter()
            .any(|item| item.address.question_id == question_id),
        "a coincident code must not replace a missing source identity"
    );
}

#[tokio::test]
async fn test_response_projection_should_refuse_missing_columns_and_changed_stored_bindings() {
    let fixture = fixture().await;
    let (study, cookie, csrf) = imported(&fixture).await;
    let mut missing = table(1);
    missing.columns.retain(|column| column != "701X20X101");
    missing.rows[0].remove("701X20X101");
    let batch = json(
        upload(
            &fixture.app,
            &study,
            missing.to_xml(Limits::default()).unwrap(),
            Some(&cookie),
            Some(&csrf),
        )
        .await,
    )
    .await;
    let response = call(
        &fixture.app,
        "GET",
        &format!(
            "/api/admin/response-imports/{}/records/0/native-cells.json",
            batch["id"].as_str().unwrap()
        ),
        None,
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        json(response).await["error"]["code"],
        "response_projection_unavailable"
    );
    let complete = json(
        upload(
            &fixture.app,
            &study,
            table(1).to_xml(Limits::default()).unwrap(),
            Some(&cookie),
            Some(&csrf),
        )
        .await,
    )
    .await;
    let id = complete["id"].as_str().unwrap();
    let mut schema = json(
        call(
            &fixture.app,
            "GET",
            &format!("/api/admin/response-imports/{id}/mapping.json"),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    schema["bindings"][0]["runtimeKey"] = json!("SHADOW");
    fixture
        .database
        .execute(
            &Query::update()
                .table("response_imports")
                .value("schema_document", schema.to_string())
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await
        .unwrap();
    let response = call(
        &fixture.app,
        "GET",
        &format!("/api/admin/response-imports/{id}/records/0/native-cells.json"),
        None,
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error = json(response).await;
    assert_eq!(error["error"]["issues"][0]["path"], "mapping");
    assert!(!error.to_string().contains(PRIVATE));
}

#[tokio::test]
#[ignore = "requires artifacts generated by the isolated LimeSurvey reference harness"]
async fn test_reference_import_should_preserve_actual_limesurvey_response_xml_and_cell_values() {
    let lss_path =
        std::env::var_os("RUSTSURVEY_REFERENCE_LSS").expect("synthetic reference LSS path");
    let lsr_path = std::path::Path::new(&lss_path).with_file_name("roundtrip.lsr");
    let lss = std::fs::read(lss_path).unwrap();
    let lsr = std::fs::read(lsr_path).unwrap();
    assert!(lss.len() <= 8 * 1024 * 1024 && lsr.len() <= 8 * 1024 * 1024);
    let expected = ResponseTable::parse(&lsr, Limits::default()).unwrap();
    let source_document =
        rustsurvey_limesurvey::LssDocument::parse(&lss, Limits::default()).unwrap();
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let study = json(import_request(&fixture, lss, Some(&cookie), Some(&csrf)).await).await;
    let definition: rustsurvey_domain::Definition =
        serde_json::from_value(study["definition"].clone()).unwrap();
    let projector = rustsurvey_limesurvey::ResponseProjector::compile(
        &definition,
        source_document.source_model().unwrap(),
        &expected.columns,
    )
    .unwrap();
    let check = json(
        call(
            &fixture.app,
            "GET",
            &format!(
                "/api/admin/studies/{}/revisions/1/collection-check",
                study["id"].as_str().unwrap()
            ),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(check["mapping"], "complete");
    assert_eq!(check["mappedCells"], 52);
    assert_eq!(check["execution"], "blocked");
    assert!(check["projection"]["unprojectedColumns"].as_u64().unwrap() > 0);
    let response = upload(
        &fixture.app,
        study["id"].as_str().unwrap(),
        lsr.clone(),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    let status = response.status();
    let batch = json(response).await;
    assert_eq!(status, StatusCode::CREATED, "{batch}");
    assert_eq!(batch["report"]["mapping"], "complete");
    assert_eq!(batch["report"]["mappedCells"], 52);
    assert_eq!(batch["rowCount"], expected.rows.len());
    let id = batch["id"].as_str().unwrap();
    for (ordinal, row) in expected.rows.iter().enumerate() {
        let projected = projector.project(row).unwrap();
        assert_eq!(projector.restore(&projected).unwrap(), *row);
        assert_eq!(projected.issue_count, 0);
        let converted = call(
            &fixture.app,
            "GET",
            &format!("/api/admin/response-imports/{id}/records/{ordinal}/native-cells.json"),
            None,
            Some(&cookie),
            None,
        )
        .await;
        assert_eq!(converted.status(), StatusCode::OK);
        let converted = json(converted).await;
        assert_eq!(
            converted["projection"]["answers"],
            serde_json::to_value(&projected.answers).unwrap()
        );
        assert_eq!(
            converted["projection"]["originalValues"],
            serde_json::to_value(&projected.original_values).unwrap()
        );
        let actual = json(
            call(
                &fixture.app,
                "GET",
                &format!("/api/admin/response-imports/{id}/records/{ordinal}/record.json"),
                None,
                Some(&cookie),
                None,
            )
            .await,
        )
        .await;
        let decoded: BTreeMap<String, Option<String>> = expected
            .columns
            .iter()
            .map(|column| {
                (
                    column.clone(),
                    actual
                        .get(column)
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                )
            })
            .collect();
        assert!(
            decoded == *row,
            "values represented in the actual reference file must remain unchanged"
        );
    }
    let original = call(
        &fixture.app,
        "GET",
        &format!("/api/admin/response-imports/{id}/original.lsr"),
        None,
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(
        to_bytes(original.into_body(), 8 * 1024 * 1024)
            .await
            .unwrap(),
        lsr
    );
}
