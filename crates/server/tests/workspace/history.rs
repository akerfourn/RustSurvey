use super::*;

pub(super) async fn roundtrip(fixture: &Fixture) {
    let (cookie, csrf) = login(fixture).await;
    let stored = create(fixture, &cookie, &csrf, definition()).await;
    let id = stored["id"].as_str().unwrap();
    let mut updated = definition();
    updated["title"] = json!("Second version");
    let update = call(
        &fixture.app,
        "PUT",
        &format!("/api/admin/studies/{id}"),
        Some(json!({"expectedRevision":1, "definition":updated})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(update.status(), StatusCode::OK);
    let revisions = format!("/api/admin/studies/{id}/revisions");
    assert_eq!(
        call(&fixture.app, "GET", &revisions, None, None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let list = json(call(&fixture.app, "GET", &revisions, None, Some(&cookie), None).await).await;
    assert_eq!(list["items"][0]["revision"], 2);
    assert_eq!(list["items"][1]["revision"], 1);
    let first = json(
        call(
            &fixture.app,
            "GET",
            &format!("{revisions}/1"),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(first["definition"], definition());
    let export = call(
        &fixture.app,
        "GET",
        &format!("{revisions}/1/definition.json"),
        None,
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(export.status(), StatusCode::OK);
    assert!(
        export.headers()[header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .contains("definition-")
    );
    let exported = json(export).await;
    assert_eq!(exported, definition());
    let imported = create(fixture, &cookie, &csrf, exported).await;
    assert_ne!(imported["id"], stored["id"]);
    let restore = format!("/api/admin/studies/{id}/restore");
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            &restore,
            Some(json!({"expectedRevision":2,"revision":1})),
            Some(&cookie),
            None
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let restored = json(
        call(
            &fixture.app,
            "POST",
            &restore,
            Some(json!({"expectedRevision":2,"revision":1})),
            Some(&cookie),
            Some(&csrf),
        )
        .await,
    )
    .await;
    assert_eq!(restored["revision"], 3);
    assert_eq!(restored["definition"], stored["definition"]);
    let stale = call(
        &fixture.app,
        "POST",
        &restore,
        Some(json!({"expectedRevision":2,"revision":2})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    let second = json(
        call(
            &fixture.app,
            "GET",
            &format!("{revisions}/2"),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(
        second["definition"], updated,
        "restoration must not overwrite history"
    );
    let page = json(
        call(
            &fixture.app,
            "GET",
            &format!("{revisions}?before=2"),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert_eq!(page["items"][0]["revision"], 1);
}

#[tokio::test]
async fn test_history_should_export_native_definitions_and_restore_without_overwriting_revisions() {
    roundtrip(&fixture().await).await;
}

#[tokio::test]
async fn test_native_import_should_reject_unknown_fields_with_a_structured_error_without_echoing_their_value()
 {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let mut invalid = definition();
    invalid["futureSetting"] = json!("SYNTHETIC_VALUE_NOT_TO_ECHO");
    let response = call(
        &fixture.app,
        "POST",
        "/api/admin/studies",
        Some(invalid),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = json(response).await;
    assert_eq!(body["error"]["code"], "invalid_definition_json");
    assert!(!body.to_string().contains("SYNTHETIC_VALUE_NOT_TO_ECHO"));
    assert!(
        json(call(&fixture.app, "GET", "/api/studies", None, None, None).await).await["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
