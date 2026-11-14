use super::*;

const LSS: &[u8] = include_bytes!("../../../limesurvey/tests/fixtures/minimal-623.lss");

fn context() -> Vec<u8> {
    serde_json::to_vec_pretty(
        &json!({"schemaVersion":1,"surveyId":"701","surveyGroupId":"2",
        "groups":[{"id":"2","parentId":null,"values":{"showwelcome":"N"}}],
        "global":{"format":"G","navigationdelay":"0","questionindex":"0"}}),
    )
    .unwrap()
}

async fn imported(fixture: &Fixture) -> (String, String, String) {
    let (cookie, csrf) = login(fixture).await;
    let response = import_request(fixture, LSS.to_vec(), Some(&cookie), Some(&csrf)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let study = json(response).await;
    (study["id"].as_str().unwrap().into(), cookie, csrf)
}

async fn upload(
    app: &Router,
    study: &str,
    revision: i64,
    bytes: Vec<u8>,
    cookie: Option<&str>,
    csrf: Option<&str>,
) -> axum::response::Response {
    let mut request = Request::post(format!(
        "/api/admin/studies/{study}/revisions/{revision}/settings-contexts"
    ))
    .header(header::CONTENT_TYPE, "application/json")
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

async fn selected(
    app: &Router,
    study: &str,
    revision: i64,
    id: Option<&str>,
    cookie: &str,
) -> axum::response::Response {
    let mut path = format!("/api/admin/studies/{study}/revisions/{revision}/collection-check");
    if let Some(id) = id {
        path.push_str(&format!("?contextId={id}"));
    }
    call(app, "GET", &path, None, Some(cookie), None).await
}

#[tokio::test]
async fn test_source_context_should_remain_private_immutable_and_resolvable_after_restart() {
    roundtrip(&fixture().await).await;
}

pub(super) async fn roundtrip(fixture: &Fixture) {
    let (study, cookie, csrf) = imported(fixture).await;
    assert_eq!(
        upload(&fixture.app, &study, 1, b"invalid".to_vec(), None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        upload(
            &fixture.app,
            &study,
            1,
            b"invalid".to_vec(),
            Some(&cookie),
            None
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let before = json(selected(&fixture.app, &study, 1, None, &cookie).await).await;
    assert_eq!(before["sourceContext"], Value::Null);
    let mut bytes = context();
    bytes.resize(65536, b' ');
    let response = upload(
        &fixture.app,
        &study,
        1,
        bytes.clone(),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let metadata = json(response).await;
    assert_eq!(metadata["studyId"], study);
    assert_eq!(metadata["revision"], 1);
    assert_eq!(metadata["size"], bytes.len());
    let id = metadata["id"].as_str().unwrap();
    let original_path = format!("/api/admin/source-settings-contexts/{id}/original.json");
    for path in [
        original_path.clone(),
        format!("/api/admin/source-settings-contexts/{id}"),
        format!("/api/admin/studies/{study}/revisions/1/settings-contexts"),
    ] {
        assert_eq!(
            call(&fixture.app, "GET", &path, None, None, None)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let retried = upload(
        &fixture.app,
        &study,
        1,
        bytes.clone(),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(retried.status(), StatusCode::OK);
    assert_eq!(json(retried).await, metadata);
    let restarted = routes(WorkspaceState::with_runtime(
        fixture.database.clone(),
        false,
        fixture.runtime.clone(),
    ));
    let response = call(&restarted, "GET", &original_path, None, Some(&cookie), None).await;
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert!(
        response.headers()[header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .starts_with("attachment;")
    );
    assert_eq!(to_bytes(response.into_body(), 65536).await.unwrap(), bytes);
    let report = json(selected(&restarted, &study, 1, Some(id), &cookie).await).await;
    assert_eq!(report["sourceContext"], metadata);
    let welcome = report["groupSettings"]["settings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|setting| setting["name"] == "showwelcome")
        .unwrap();
    assert_eq!(welcome["value"], false);
    assert_eq!(welcome["origin"], json!({"layer":"group","groupId":"2"}));
    assert_eq!(report["execution"], "blocked");
    assert_eq!(
        json(selected(&restarted, &study, 1, None, &cookie).await).await,
        before,
        "a new context must not change the default assessment"
    );
    let public = json(
        call(
            &restarted,
            "GET",
            &format!("/api/studies/{study}"),
            None,
            None,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(public["revision"], 1);
    assert_eq!(public["definition"]["showWelcome"], Value::Null);
    assert!(!public.to_string().contains(id));
    let mut definition = public["definition"].clone();
    definition["groups"][0]["questions"][1]["relevance"] = json!("CHOICE == '01'");
    let updated = call(
        &restarted,
        "PUT",
        &format!("/api/admin/studies/{study}"),
        Some(json!({"expectedRevision":1,"definition":definition})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    assert_eq!(
        selected(&restarted, &study, 2, Some(id), &cookie)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        json(selected(&restarted, &study, 1, Some(id), &cookie).await).await,
        report
    );
    let second = upload(&restarted, &study, 2, bytes, Some(&cookie), Some(&csrf)).await;
    assert_eq!(second.status(), StatusCode::CREATED);
    assert_ne!(json(second).await["id"], id);
}

#[tokio::test]
async fn test_source_context_should_refuse_ambiguous_foreign_or_oversized_input_without_saving_it()
{
    let fixture = fixture().await;
    let (study, cookie, csrf) = imported(&fixture).await;
    let duplicate = br#"{"schemaVersion":1,"surveyId":"701","surveyGroupId":"0","groups":[],"global":{"showwelcome":null,"showwelcome":"N"}}"#;
    let foreign = String::from_utf8(context()).unwrap().replace("701", "702");
    for bytes in [
        duplicate.to_vec(),
        foreign.into_bytes(),
        br#"{"private":"synthetic-private-invalid-context"}"#.to_vec(),
    ] {
        let response = upload(&fixture.app, &study, 1, bytes, Some(&cookie), Some(&csrf)).await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(
            !json(response)
                .await
                .to_string()
                .contains("synthetic-private-invalid-context")
        );
    }
    assert_eq!(
        upload(
            &fixture.app,
            &study,
            1,
            vec![b' '; 65537],
            Some(&cookie),
            Some(&csrf)
        )
        .await
        .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let list = json(
        call(
            &fixture.app,
            "GET",
            &format!("/api/admin/studies/{study}/revisions/1/settings-contexts"),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(list["items"], json!([]));
}

#[tokio::test]
async fn test_source_context_should_enforce_its_limit_and_deduplicate_competing_requests() {
    concurrent_limit(&fixture().await).await;
}

#[tokio::test]
async fn test_source_context_should_reject_altered_resolution_while_retaining_its_original() {
    let fixture = fixture().await;
    let (study, cookie, csrf) = imported(&fixture).await;
    let bytes = context();
    let metadata = json(
        upload(
            &fixture.app,
            &study,
            1,
            bytes.clone(),
            Some(&cookie),
            Some(&csrf),
        )
        .await,
    )
    .await;
    let id = metadata["id"].as_str().unwrap();
    fixture
        .database
        .execute(
            &Query::update()
                .table("source_settings_contexts")
                .value("report", "{}")
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await
        .unwrap();
    let response = selected(&fixture.app, &study, 1, Some(id), &cookie).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        json(response).await["error"]["code"],
        "settings_context_resolution_changed"
    );
    let path = format!("/api/admin/source-settings-contexts/{id}/original.json");
    let original = call(&fixture.app, "GET", &path, None, Some(&cookie), None).await;
    assert_eq!(to_bytes(original.into_body(), 65536).await.unwrap(), bytes);
    fixture
        .database
        .execute(
            &Query::update()
                .table("source_settings_contexts")
                .value("document", "altered private test bytes")
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await
        .unwrap();
    assert_eq!(
        call(&fixture.app, "GET", &path, None, Some(&cookie), None)
            .await
            .status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        selected(&fixture.app, &study, 1, Some(id), &cookie)
            .await
            .status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[tokio::test]
#[ignore = "requires actual artifacts generated by the isolated LimeSurvey reference harness"]
async fn test_source_context_reference_should_persist_actual_limesurvey_options() {
    let path = std::env::var_os("RUSTSURVEY_REFERENCE_LSS").expect("synthetic reference LSS path");
    let directory = std::path::Path::new(&path).parent().unwrap();
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    for case in 0..4 {
        let lss = std::fs::read(directory.join(format!("settings-{case}.lss"))).unwrap();
        let context =
            std::fs::read(directory.join(format!("settings-{case}-extracted.json"))).unwrap();
        assert!(lss.len() <= 8 * 1024 * 1024 && context.len() <= 65536);
        let response = import_request(&fixture, lss, Some(&cookie), Some(&csrf)).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let study = json(response).await;
        let study_id = study["id"].as_str().unwrap();
        let response = upload(
            &fixture.app,
            study_id,
            1,
            context,
            Some(&cookie),
            Some(&csrf),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let metadata = json(response).await;
        assert_eq!(metadata["complete"], true);
        let report =
            json(selected(&fixture.app, study_id, 1, metadata["id"].as_str(), &cookie).await).await;
        assert_eq!(report["sourceContext"], metadata);
        assert_eq!(report["groupSettings"]["complete"], true);
        assert_eq!(
            report["groupSettings"]["settings"]
                .as_array()
                .unwrap()
                .len(),
            30
        );
        assert_eq!(report["execution"], "blocked");
    }
}

pub(super) async fn concurrent_limit(fixture: &Fixture) {
    let (study, cookie, csrf) = imported(fixture).await;
    let bytes = context();
    let (first, second) = tokio::join!(
        upload(
            &fixture.app,
            &study,
            1,
            bytes.clone(),
            Some(&cookie),
            Some(&csrf)
        ),
        upload(
            &fixture.app,
            &study,
            1,
            bytes.clone(),
            Some(&cookie),
            Some(&csrf)
        )
    );
    assert!([StatusCode::CREATED, StatusCode::OK].contains(&first.status()));
    assert!([StatusCode::CREATED, StatusCode::OK].contains(&second.status()));
    assert_eq!(json(first).await["id"], json(second).await["id"]);
    for spaces in 1..31 {
        let mut input = bytes.clone();
        input.extend(vec![b' '; spaces]);
        assert_eq!(
            upload(&fixture.app, &study, 1, input, Some(&cookie), Some(&csrf))
                .await
                .status(),
            StatusCode::CREATED
        );
    }
    let mut a = bytes.clone();
    a.extend(vec![b' '; 31]);
    let mut b = bytes.clone();
    b.extend(vec![b' '; 32]);
    let (a, b) = tokio::join!(
        upload(&fixture.app, &study, 1, a, Some(&cookie), Some(&csrf)),
        upload(&fixture.app, &study, 1, b, Some(&cookie), Some(&csrf))
    );
    let mut statuses = [a.status().as_u16(), b.status().as_u16()];
    statuses.sort();
    assert_eq!(statuses, [201, 409]);
    let list = json(
        call(
            &fixture.app,
            "GET",
            &format!("/api/admin/studies/{study}/revisions/1/settings-contexts"),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(list["items"].as_array().unwrap().len(), 32);
    assert_eq!(
        upload(&fixture.app, &study, 1, bytes, Some(&cookie), Some(&csrf))
            .await
            .status(),
        StatusCode::OK
    );
}
