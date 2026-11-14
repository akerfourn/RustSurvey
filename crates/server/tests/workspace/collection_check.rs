use super::*;

const LSS: &str = include_str!("../../../limesurvey/tests/fixtures/minimal-623.lss");

#[tokio::test]
async fn test_import_should_preserve_inherited_welcome_without_substituting_a_local_default() {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let source = LSS.replacen("<fieldname>sid</fieldname>", "<fieldname>sid</fieldname><fieldname>showwelcome</fieldname><fieldname>gsid</fieldname>", 1)
        .replacen("<sid>701</sid>", "<sid>701</sid><showwelcome>I</showwelcome><gsid>2</gsid>", 1);
    let response = import_request(&fixture, source.into_bytes(), Some(&cookie), Some(&csrf)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let saved = json(response).await;
    assert_eq!(saved["definition"]["showWelcome"], Value::Null);
    assert_eq!(saved["source"]["report"]["errors"], 0);
    let id = saved["id"].as_str().unwrap();
    let report = json(check(&fixture, id, 1, Some(&cookie)).await).await;
    let welcome = report["groupSettings"]["settings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|setting| setting["name"] == "showwelcome")
        .unwrap();
    assert_eq!(welcome["status"], "unresolved");
    assert_eq!(welcome["value"], Value::Null);
    assert_eq!(report["execution"], "blocked");
    let response = call(
        &fixture.app,
        "GET",
        &format!("/api/admin/studies/{id}/revisions/1/modified.lss"),
        None,
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let xml = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let document =
        rustsurvey_limesurvey::LssDocument::parse(&xml, rustsurvey_limesurvey::Limits::default())
            .unwrap();
    assert_eq!(document.source_model().unwrap().show_welcome, None);
    assert!(String::from_utf8_lossy(&xml).contains("<showwelcome>I</showwelcome>"));
}

async fn check(
    fixture: &Fixture,
    id: &str,
    revision: i64,
    cookie: Option<&str>,
) -> axum::response::Response {
    call(
        &fixture.app,
        "GET",
        &format!("/api/admin/studies/{id}/revisions/{revision}/collection-check"),
        None,
        cookie,
        None,
    )
    .await
}

#[tokio::test]
async fn test_collection_check_should_qualify_private_source_gaps_without_authorizing_collection() {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let response = import_request(
        &fixture,
        LSS.as_bytes().to_vec(),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let saved = json(response).await;
    let id = saved["id"].as_str().unwrap();
    assert_eq!(
        check(&fixture, id, 1, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let response = check(&fixture, id, 1, Some(&cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let report = json(response).await;
    assert_eq!(report["runtime"], "supported");
    assert_eq!(report["mapping"], "complete");
    assert_eq!(report["mappedCells"], 2);
    assert_eq!(report["execution"], "blocked");
    assert_eq!(report["continuity"], "unavailable");
    assert!(report["projection"]["unprojectedColumns"].as_u64().unwrap() > 0);
    assert!(report["projection"]["missingSettings"].as_u64().unwrap() > 0);
    let serialized = report.to_string();
    assert!(!serialized.contains("0007"));
    assert!(!serialized.contains("synthetic()"));
    let response = call(
        &fixture.app,
        "POST",
        &format!("/api/admin/studies/{id}/runs"),
        Some(json!({"expectedRevision":1})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error = json(response).await;
    assert_eq!(error["error"]["code"], "imported_collection_unavailable");
    assert_eq!(error["error"]["issues"], report["issues"]);
    let runs = json(
        call(
            &fixture.app,
            "GET",
            &format!("/api/admin/studies/{id}/runs"),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(runs["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_collection_check_should_evaluate_the_requested_revision_after_a_condition_is_fixed() {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let source = LSS
        .replace(
            "<fieldname>mandatory</fieldname>",
            "<fieldname>mandatory</fieldname><fieldname>relevance</fieldname>",
        )
        .replace(
            "<mandatory>S</mandatory>",
            "<mandatory>S</mandatory><relevance>legacyFunction()</relevance>",
        );
    let response = import_request(&fixture, source.into_bytes(), Some(&cookie), Some(&csrf)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let saved = json(response).await;
    let id = saved["id"].as_str().unwrap();
    let before = json(check(&fixture, id, 1, Some(&cookie)).await).await;
    assert_eq!(before["runtime"], "blocked");
    let mut definition = saved["definition"].clone();
    definition["groups"][0]["questions"][1]["relevance"] = json!("CHOICE == '01'");
    let response = call(
        &fixture.app,
        "PUT",
        &format!("/api/admin/studies/{id}"),
        Some(json!({"expectedRevision":1,"definition":definition})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    let status = response.status();
    let updated = json(response).await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["revision"], 2);
    let after = json(check(&fixture, id, 2, Some(&cookie)).await).await;
    assert_eq!(after["runtime"], "supported");
    assert_eq!(after["revision"], 2);
    assert_eq!(after["sourceSha256"], before["sourceSha256"]);
    assert_eq!(
        json(check(&fixture, id, 1, Some(&cookie)).await).await,
        before
    );
    assert_eq!(
        check(&fixture, id, 3, Some(&cookie)).await.status(),
        StatusCode::NOT_FOUND
    );
}
