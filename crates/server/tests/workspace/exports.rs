use super::*;

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
