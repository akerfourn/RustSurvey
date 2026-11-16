use super::*;

#[path = "collection/defaults.rs"]
mod defaults;
#[path = "collection/exclusive.rs"]
mod exclusive;
#[path = "collection/exports.rs"]
mod exports;
#[path = "collection/filters.rs"]
mod filters;
#[path = "collection/forms.rs"]
mod forms;
#[path = "collection/navigation.rs"]
mod navigation;
#[path = "collection/participation.rs"]
mod participation;
#[path = "collection/source_execution.rs"]
mod source_execution;

const TOKEN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[tokio::test]
async fn test_invalid_access_should_be_rejected_before_reading_the_private_answer_document() {
    let fixture = fixture().await;
    let (_, run, _, _) = campaign(&fixture).await;
    let id = start(&fixture, &run, 109).await;
    fixture
        .database
        .execute(
            &Query::update()
                .table("responses")
                .value("document", "deliberately invalid test JSON")
                .and_where(Expr::col("id").eq(&id))
                .to_owned(),
        )
        .await
        .unwrap();
    let wrong = "b".repeat(64);
    let response = participant(
        &fixture.app,
        "GET",
        &format!("/api/responses/{id}"),
        None,
        Some(&wrong),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(json(response).await["error"]["code"], "response_not_found");
    let duplicate = participant(
        &fixture.app,
        "POST",
        &format!("/api/runs/{run}/responses"),
        Some(json!({"responseId":id,"language":"fr"})),
        Some(&wrong),
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::NOT_FOUND);
}

pub(super) async fn roundtrip(fixture: &Fixture) {
    let (_, run, cookie, csrf) = campaign(fixture).await;
    let id = start(fixture, &run, 106).await;
    let path = format!("/api/responses/{id}");
    let command = update(0, 1008, json!({"GATE":"Y","AGE":"40"}), true);
    let response = participant(
        &fixture.app,
        "PUT",
        &path,
        Some(command.clone()),
        Some(TOKEN),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let acknowledgement = json(response).await;
    assert_eq!(
        json(participant(&fixture.app, "PUT", &path, Some(command), Some(TOKEN)).await).await,
        acknowledgement
    );
    let saved = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    assert_eq!(saved["answers"]["SCORE"], "41");
    assert_eq!(saved["status"], "submitted");
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            &format!("/api/admin/runs/{run}/close"),
            Some(json!({})),
            Some(&cookie),
            Some(&csrf)
        )
        .await
        .status(),
        StatusCode::OK
    );
    participation::roundtrip(fixture).await;
    forms::roundtrip(fixture).await;
    exports::roundtrip(fixture).await;
    navigation::roundtrip(fixture).await;
    exclusive::roundtrip(fixture).await;
    filters::roundtrip(fixture).await;
    defaults::roundtrip(fixture).await;
    source_execution::roundtrip(fixture).await;
}

#[tokio::test]
async fn test_invalid_payloads_should_not_modify_answers_or_checkpoints() {
    let fixture = fixture().await;
    let (_, run, _, _) = campaign(&fixture).await;
    let id = start(&fixture, &run, 107).await;
    let path = format!("/api/responses/{id}");
    let unknown_field = update(0, 1010, json!({"UNKNOWN":"synthetic-only"}), false);
    let mut unknown_language = update(0, 1011, json!({}), false);
    unknown_language["checkpoint"]["language"] = json!("unknown");
    let mut unknown_group = update(0, 1012, json!({}), false);
    unknown_group["checkpoint"]["groupId"] = json!("unknown");
    let nonstring = update(0, 1013, json!({"GATE":1}), false);
    for command in [unknown_field, unknown_language, unknown_group, nonstring] {
        assert_eq!(
            participant(&fixture.app, "PUT", &path, Some(command), Some(TOKEN))
                .await
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let request = Request::builder()
        .method("PUT")
        .uri(&path)
        .header("X-Requested-With", "RustSurvey")
        .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(" ".repeat(2 * 1024 * 1024 + 1)))
        .unwrap();
    assert_eq!(
        fixture.app.clone().oneshot(request).await.unwrap().status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let saved = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    assert_eq!(saved["version"], 0);
    assert_eq!(saved["answers"], json!({}));
    assert_eq!(
        saved["checkpoint"],
        json!({"language":"fr","groupId":"main"})
    );
}

#[tokio::test]
async fn test_receipts_should_bound_daily_operations_and_expire_without_losing_the_response() {
    let fixture = fixture().await;
    let (_, run, _, _) = campaign(&fixture).await;
    let id = start(&fixture, &run, 108).await;
    let path = format!("/api/responses/{id}");
    let mut insert = Query::insert();
    insert.into_table("operation_receipts").columns([
        "response_id",
        "operation_id",
        "request_hash",
        "result",
        "expires_at",
    ]);
    let expires = fixture.runtime.now().unwrap() + 24 * 60 * 60;
    for serial in 5000..6024 {
        insert
            .values([
                id.clone().into(),
                identifier(serial).into(),
                "synthetic-hash".into(),
                "{}".into(),
                expires.into(),
            ])
            .unwrap();
    }
    fixture.database.execute(&insert).await.unwrap();
    let command = update(0, 1014, json!({"GATE":"N"}), false);
    let response = participant(
        &fixture.app,
        "PUT",
        &path,
        Some(command.clone()),
        Some(TOKEN),
    )
    .await;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        json(response).await["error"]["code"],
        "response_operation_limit"
    );
    fixture
        .runtime
        .now
        .fetch_add(24 * 60 * 60 + 1, Ordering::SeqCst);
    assert_eq!(
        participant(&fixture.app, "PUT", &path, Some(command), Some(TOKEN))
            .await
            .status(),
        StatusCode::OK
    );
    let receipts = fixture
        .database
        .query_all(
            &Query::select()
                .column("operation_id")
                .from("operation_receipts")
                .and_where(Expr::col("response_id").eq(&id))
                .to_owned(),
        )
        .await
        .unwrap();
    assert_eq!(receipts.len(), 1);
    let saved = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    assert_eq!(saved["answers"], json!({"GATE":"N"}));
    assert_eq!(saved["version"], 1);
}

#[tokio::test]
async fn test_collection_migration_should_preserve_preexisting_definitions_and_sessions() {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let stored = create(&fixture, &cookie, &csrf, definition()).await;
    rollback_after(&fixture.database, 3).await;
    assert!(
        !SchemaManager::new(&fixture.database)
            .has_table("responses")
            .await
            .unwrap()
    );
    Migrator::up(&fixture.database, None).await.unwrap();
    let path = format!("/api/studies/{}", stored["id"].as_str().unwrap());
    // The restored legacy layout made every saved study public before publications existed.
    let mut expected = stored;
    expected["publishedRevision"] = json!(1);
    assert_eq!(
        json(call(&fixture.app, "GET", &path, None, None, None).await).await,
        expected
    );
    let session = json(
        call(
            &fixture.app,
            "GET",
            "/api/admin/session",
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(session["authenticated"], true);
    assert!(
        SchemaManager::new(&fixture.database)
            .has_table("responses")
            .await
            .unwrap()
    );
}

fn identifier(value: u64) -> String {
    format!("00000000-0000-4000-8000-{value:012}")
}
fn collection_definition() -> Value {
    serde_json::from_str::<Value>(include_str!("../../../../examples/rules/collection.json"))
        .unwrap()["definition"]
        .clone()
}

async fn participant(
    app: &Router,
    method: &str,
    path: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("X-Requested-With", "RustSurvey");
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = if let Some(body) = body {
        request = request.header(header::CONTENT_TYPE, "application/json");
        Body::from(body.to_string())
    } else {
        Body::empty()
    };
    app.clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap()
}

async fn campaign(fixture: &Fixture) -> (String, String, String, String) {
    campaign_with_policy(fixture, json!({"mode":"open"})).await
}

async fn campaign_with_policy(
    fixture: &Fixture,
    policy: Value,
) -> (String, String, String, String) {
    let (cookie, csrf) = login(fixture).await;
    let study = create(fixture, &cookie, &csrf, collection_definition()).await;
    let study_id = study["id"].as_str().unwrap().to_owned();
    let response = call(
        &fixture.app,
        "POST",
        &format!("/api/admin/studies/{study_id}/runs"),
        Some(json!({"expectedRevision":1,"participationPolicy":policy})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    let status = response.status();
    let created = json(response).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    (
        study_id,
        created["id"].as_str().unwrap().to_owned(),
        cookie,
        csrf,
    )
}

async fn start(fixture: &Fixture, run: &str, serial: u64) -> String {
    let id = identifier(serial);
    let response = participant(
        &fixture.app,
        "POST",
        &format!("/api/runs/{run}/responses"),
        Some(json!({"responseId":id,"language":"fr"})),
        Some(TOKEN),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    id
}

fn update(version: i64, operation: u64, answers: Value, submit: bool) -> Value {
    json!({"expectedVersion":version,"operationId":identifier(operation),"answers":answers,"checkpoint":{"language":"fr","groupId":"details"},"submit":submit})
}

#[tokio::test]
async fn test_runs_should_require_admin_csrf_and_preserve_the_selected_revision() {
    let fixture = fixture().await;
    let (study, run, cookie, csrf) = campaign(&fixture).await;
    publish(&fixture, &cookie, &csrf, &study, 1).await;
    let participant_id = start(&fixture, &run, 3301).await;
    let path = format!("/api/admin/studies/{study}/runs");
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            &path,
            Some(json!({"expectedRevision":1})),
            None,
            None
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            &path,
            Some(json!({"expectedRevision":1})),
            Some(&cookie),
            None
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let mut definition = collection_definition();
    definition["title"] = json!("Future revision");
    let updated = call(
        &fixture.app,
        "PUT",
        &format!("/api/admin/studies/{study}"),
        Some(json!({"expectedRevision":1,"definition":definition})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    publish(&fixture, &cookie, &csrf, &study, 2).await;
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
    assert_eq!(public["revision"], 2);
    let resumed = json(
        participant(
            &fixture.app,
            "GET",
            &format!("/api/responses/{participant_id}"),
            None,
            Some(TOKEN),
        )
        .await,
    )
    .await;
    assert_eq!(resumed["run"]["revision"], 1);
    assert_eq!(resumed["definition"]["title"], "Collecte synthétique");
    let pinned =
        json(participant(&fixture.app, "GET", &format!("/api/runs/{run}"), None, None).await).await;
    assert_eq!(pinned["run"]["revision"], 1);
    assert_eq!(pinned["definition"]["title"], "Collecte synthétique");
    assert!(pinned.get("answers").is_none());
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            &path,
            Some(json!({"expectedRevision":1})),
            Some(&cookie),
            Some(&csrf)
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn test_opening_should_block_unsupported_native_features_and_limesurvey_imports() {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let mut definition = collection_definition();
    definition["groups"][0]["questions"][0]["attributes"] =
        json!([{"name":"time_limit","value":"5"}]);
    let study = create(&fixture, &cookie, &csrf, definition).await;
    let path = format!("/api/admin/studies/{}/runs", study["id"].as_str().unwrap());
    let response = call(
        &fixture.app,
        "POST",
        &path,
        Some(json!({"expectedRevision":1})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        json(response).await["error"]["code"],
        "unsupported_collection_profile"
    );
    let request = Request::builder()
        .method("POST")
        .uri("/api/admin/imports/lss")
        .header(header::CONTENT_TYPE, "application/xml")
        .header(header::COOKIE, &cookie)
        .header("X-CSRF-Token", &csrf)
        .header("X-Requested-With", "RustSurvey")
        .body(Body::from(include_str!(
            "../../../../examples/surveys/demo.lss"
        )))
        .unwrap();
    let imported = json(fixture.app.clone().oneshot(request).await.unwrap()).await;
    let path = format!(
        "/api/admin/studies/{}/runs",
        imported["id"].as_str().unwrap()
    );
    let response = call(
        &fixture.app,
        "POST",
        &path,
        Some(json!({"expectedRevision":1})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        json(response).await["error"]["code"],
        "imported_collection_unavailable"
    );
}

#[tokio::test]
async fn test_start_should_be_repeatable_and_keep_response_secrets_out_of_urls_and_storage() {
    let fixture = fixture().await;
    let (_, run, _, _) = campaign(&fixture).await;
    let id = start(&fixture, &run, 100).await;
    let initial = json(
        participant(
            &fixture.app,
            "GET",
            &format!("/api/responses/{id}"),
            None,
            Some(TOKEN),
        )
        .await,
    )
    .await;
    let repeated = participant(
        &fixture.app,
        "POST",
        &format!("/api/runs/{run}/responses"),
        Some(json!({"responseId":id,"language":"fr"})),
        Some(TOKEN),
    )
    .await;
    assert_eq!(repeated.status(), StatusCode::CREATED);
    assert_eq!(json(repeated).await, initial);
    assert_eq!(
        participant(
            &fixture.app,
            "GET",
            &format!("/api/responses/{id}?token={TOKEN}"),
            None,
            None
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        participant(
            &fixture.app,
            "GET",
            &format!("/api/responses/{id}"),
            None,
            Some(&"b".repeat(64))
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    assert!(!initial.to_string().contains(TOKEN));
    let row = fixture
        .database
        .query_one(
            &Query::select()
                .column("access_hash")
                .from("responses")
                .and_where(Expr::col("id").eq(&id))
                .to_owned(),
        )
        .await
        .unwrap()
        .unwrap();
    let hash: String = row.try_get("", "access_hash").unwrap();
    assert_ne!(hash, TOKEN);
    assert_eq!(hash.len(), 64);
}

#[tokio::test]
async fn test_saves_should_replay_lost_acknowledgements_reject_reused_ids_and_allow_one_concurrent_winner()
 {
    let fixture = fixture().await;
    let (_, run, _, _) = campaign(&fixture).await;
    let id = start(&fixture, &run, 101).await;
    let path = format!("/api/responses/{id}");
    let first = update(
        0,
        1000,
        json!({"GATE":"Y","AGE":"18","SCORE":"forged"}),
        false,
    );
    let response = participant(&fixture.app, "PUT", &path, Some(first.clone()), Some(TOKEN)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let acknowledgement = json(response).await;
    let replay = participant(&fixture.app, "PUT", &path, Some(first.clone()), Some(TOKEN)).await;
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(json(replay).await, acknowledgement);
    let mut changed = first;
    changed["answers"]["AGE"] = json!("19");
    assert_eq!(
        participant(&fixture.app, "PUT", &path, Some(changed), Some(TOKEN))
            .await
            .status(),
        StatusCode::CONFLICT
    );
    let left = update(1, 1001, json!({"GATE":"N"}), false);
    let right = update(1, 1002, json!({"GATE":"Y","AGE":"20"}), false);
    let (left, right) = tokio::join!(
        participant(&fixture.app, "PUT", &path, Some(left), Some(TOKEN)),
        participant(&fixture.app, "PUT", &path, Some(right), Some(TOKEN))
    );
    let mut statuses = [left.status().as_u16(), right.status().as_u16()];
    statuses.sort();
    assert_eq!(statuses, [200, 409]);
    let saved = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    assert_eq!(saved["version"], 2);
    assert_ne!(saved["answers"]["SCORE"], "forged");
}

#[tokio::test]
async fn test_submission_should_validate_on_the_server_clear_hidden_answers_and_be_immutable() {
    let fixture = fixture().await;
    let (_, run, _, _) = campaign(&fixture).await;
    let id = start(&fixture, &run, 102).await;
    let path = format!("/api/responses/{id}");
    let invalid = update(0, 1003, json!({}), true);
    let response = participant(&fixture.app, "PUT", &path, Some(invalid), Some(TOKEN)).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json(response).await["error"]["code"], "invalid_answers");
    let final_answers = update(
        0,
        1004,
        json!({"GATE":"N","AGE":"not-a-number","NOTE":"synthetic-private-text","SCORE":"forged"}),
        true,
    );
    let response = participant(
        &fixture.app,
        "PUT",
        &path,
        Some(final_answers.clone()),
        Some(TOKEN),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let acknowledgement = json(response).await;
    let saved = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    assert_eq!(saved["answers"], json!({"GATE":"N"}));
    assert_eq!(saved["status"], "submitted");
    assert_eq!(saved["checkpoint"]["groupId"], "main");
    assert_eq!(
        participant(
            &fixture.app,
            "PUT",
            &path,
            Some(update(1, 1005, json!({"GATE":"Y","AGE":"20"}), false)),
            Some(TOKEN)
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        json(participant(&fixture.app, "PUT", &path, Some(final_answers), Some(TOKEN)).await).await,
        acknowledgement
    );
}

#[tokio::test]
async fn test_closure_should_stop_new_writes_keep_resume_readable_and_protect_admin_exports() {
    let fixture = fixture().await;
    let (_, run, cookie, csrf) = campaign(&fixture).await;
    let id = start(&fixture, &run, 103).await;
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            &format!("/api/admin/runs/{run}/close"),
            Some(json!({})),
            Some(&cookie),
            Some(&csrf)
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        participant(
            &fixture.app,
            "POST",
            &format!("/api/runs/{run}/responses"),
            Some(json!({"responseId":identifier(104),"language":"fr"})),
            Some(TOKEN)
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    let path = format!("/api/responses/{id}");
    assert_eq!(
        participant(
            &fixture.app,
            "PUT",
            &path,
            Some(update(0, 1006, json!({"GATE":"N"}), false)),
            Some(TOKEN)
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        participant(&fixture.app, "GET", &path, None, Some(TOKEN))
            .await
            .status(),
        StatusCode::OK
    );
    let list = format!("/api/admin/runs/{run}/responses");
    assert_eq!(
        call(&fixture.app, "GET", &list, None, None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let items = json(call(&fixture.app, "GET", &list, None, Some(&cookie), None).await).await;
    assert_eq!(items["items"].as_array().unwrap().len(), 1);
    assert!(items["items"][0].get("answers").is_none());
    let export = format!("/api/admin/runs/{run}/responses/{id}/response.json");
    assert_eq!(
        call(&fixture.app, "GET", &export, None, None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = call(&fixture.app, "GET", &export, None, Some(&cookie), None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers()[header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .contains("attachment")
    );
    assert!(json(response).await.get("accessHash").is_none());
}

#[tokio::test]
async fn test_file_sqlite_should_resume_answers_checkpoint_and_receipts_after_reopening_the_database()
 {
    let directory = test_directory::Directory::create();
    let file = directory.path().join("collection.sqlite");
    let url = format!(
        "sqlite://{}?mode=rwc",
        file.display().to_string().replace('\\', "/")
    );
    let database = Database::connect(rustsurvey_server::database::options(url.clone()))
        .await
        .unwrap();
    let fixture = initialize(database).await;
    let (_, run, _, _) = campaign(&fixture).await;
    let id = start(&fixture, &run, 105).await;
    let path = format!("/api/responses/{id}");
    let mut command = update(0, 1007, json!({"GATE":"Y","AGE":"23","NOTE":"été"}), false);
    command["checkpoint"]["language"] = json!("en");
    let response = participant(
        &fixture.app,
        "PUT",
        &path,
        Some(command.clone()),
        Some(TOKEN),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let acknowledgement = json(response).await;
    let before = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    drop(fixture.app);
    fixture.database.close().await.unwrap();
    let database = Database::connect(rustsurvey_server::database::options(url))
        .await
        .unwrap();
    Migrator::up(&database, None).await.unwrap();
    let state =
        WorkspaceState::with_runtime(database.clone(), false, Arc::new(FixedRuntime::new()));
    let app = routes(state);
    let after = json(participant(&app, "GET", &path, None, Some(TOKEN)).await).await;
    assert_eq!(after, before);
    assert_eq!(
        after["checkpoint"],
        json!({"language":"en","groupId":"details"})
    );
    assert_eq!(after["answers"]["SCORE"], "24");
    assert_eq!(
        json(participant(&app, "PUT", &path, Some(command), Some(TOKEN)).await).await,
        acknowledgement
    );
    drop(app);
    database.close().await.unwrap();
}
