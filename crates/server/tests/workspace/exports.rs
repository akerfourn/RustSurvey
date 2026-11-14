use super::*;

#[tokio::test]
async fn test_native_lss_should_export_a_private_pinned_revision_and_reimport_all_28_types() {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../examples/rules/collection-forms.json"
    ))
    .unwrap();
    let stored = create(&fixture, &cookie, &csrf, corpus["definition"].clone()).await;
    let id = stored["id"].as_str().unwrap();
    let path = format!("/api/admin/studies/{id}/revisions/1/definition.lss?surveyId=701");
    assert_eq!(
        call(&fixture.app, "GET", &path, None, None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = call(&fixture.app, "GET", &path, None, Some(&cookie), None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert!(
        response.headers()[header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .contains("attachment;")
    );
    let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let document =
        rustsurvey_limesurvey::LssDocument::parse(&bytes, rustsurvey_limesurvey::Limits::default())
            .unwrap();
    assert_eq!(document.source_model().unwrap().questions.len(), 28);
    let mut edited = stored["definition"].clone();
    edited["title"] = json!("A later version");
    assert_eq!(
        call(
            &fixture.app,
            "PUT",
            &format!("/api/admin/studies/{id}"),
            Some(json!({"expectedRevision":1,"definition":edited})),
            Some(&cookie),
            Some(&csrf)
        )
        .await
        .status(),
        StatusCode::OK
    );
    let historical = call(&fixture.app, "GET", &path, None, Some(&cookie), None).await;
    assert_eq!(
        to_bytes(historical.into_body(), 8 * 1024 * 1024)
            .await
            .unwrap(),
        bytes
    );
    let imported = import_request(&fixture, bytes.to_vec(), Some(&cookie), Some(&csrf)).await;
    let status = imported.status();
    let imported = json(imported).await;
    assert_eq!(status, StatusCode::CREATED, "{imported}");
    assert_eq!(
        imported["definition"]["groups"][0]["questions"]
            .as_array()
            .unwrap()
            .len(),
        28
    );
    assert!(imported["source"].is_object());
    let path = format!(
        "/api/admin/studies/{}/revisions/1/definition.lss",
        imported["id"].as_str().unwrap()
    );
    assert_eq!(
        call(&fixture.app, "GET", &path, None, Some(&cookie), None)
            .await
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_native_lss_should_explain_unknown_attributes_and_invalid_exchange_ids() {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let mut value = definition();
    value["groups"][0]["questions"][0]["attributes"] =
        json!([{"name":"unknown_active_attribute","value":"0"}]);
    let stored = create(&fixture, &cookie, &csrf, value).await;
    let path = format!(
        "/api/admin/studies/{}/revisions/1/definition.lss",
        stored["id"].as_str().unwrap()
    );
    let response = call(&fixture.app, "GET", &path, None, Some(&cookie), None).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error = json(response).await;
    assert_eq!(error["error"]["code"], "native_lss_export_blocked");
    assert!(!error["error"]["issues"].as_array().unwrap().is_empty());
    let stored = create(&fixture, &cookie, &csrf, definition()).await;
    let path = format!(
        "/api/admin/studies/{}/revisions/1/definition.lss?surveyId=0",
        stored["id"].as_str().unwrap()
    );
    let response = call(&fixture.app, "GET", &path, None, Some(&cookie), None).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        json(response).await["error"]["issues"][0]["path"],
        "surveyId"
    );
}

#[tokio::test]
async fn test_modified_export_should_block_legacy_conditions_and_leave_the_original_unchanged() {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let xml = std::str::from_utf8(&reference_lss()).unwrap().replace(
        "</document>",
        "<conditions><rows><row><cid>1</cid></row></rows></conditions></document>",
    );
    let stored = json(
        import_request(
            &fixture,
            xml.as_bytes().to_vec(),
            Some(&cookie),
            Some(&csrf),
        )
        .await,
    )
    .await;
    let id = stored["id"].as_str().unwrap();
    let mut edited = stored["definition"].clone();
    edited["groups"][2]["relevance"] = json!("0");
    let update = call(
        &fixture.app,
        "PUT",
        &format!("/api/admin/studies/{id}"),
        Some(json!({"expectedRevision":1,"definition":edited})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(update.status(), StatusCode::OK);
    let exported = call(
        &fixture.app,
        "GET",
        &format!("/api/admin/studies/{id}/revisions/2/modified.lss"),
        None,
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(exported.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        json(exported).await["error"]["code"],
        "legacy_conditions_block_export"
    );
    let original = call(
        &fixture.app,
        "GET",
        &format!("/api/admin/studies/{id}/original.lss"),
        None,
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(original.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(original.into_body(), 2 * 1024 * 1024)
            .await
            .unwrap()
            .as_ref(),
        xml.as_bytes()
    );
}
