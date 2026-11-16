use super::*;

async fn save_title(
    fixture: &Fixture,
    cookie: &str,
    csrf: &str,
    id: &str,
    revision: i64,
    title: &str,
) {
    let mut document = definition();
    document["title"] = json!(title);
    let response = call(
        &fixture.app,
        "PUT",
        &format!("/api/admin/studies/{id}"),
        Some(json!({"expectedRevision":revision,"definition":document})),
        Some(cookie),
        Some(csrf),
    )
    .await;
    let status = response.status();
    let body = json(response).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

pub(super) async fn roundtrip(fixture: &Fixture) {
    let (cookie, csrf) = login(fixture).await;
    let draft = create(fixture, &cookie, &csrf, definition()).await;
    let id = draft["id"].as_str().unwrap();
    let entry = format!("/api/studies/{id}");
    let pinned = format!("{entry}/revisions/1");
    assert_eq!(draft["publishedRevision"], Value::Null);
    for path in [&entry, &pinned] {
        assert_eq!(
            call(&fixture.app, "GET", path, None, None, None)
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    let catalogue = json(call(&fixture.app, "GET", "/api/studies", None, None, None).await).await;
    assert!(
        !catalogue["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == id)
    );
    let private = json(
        call(
            &fixture.app,
            "GET",
            "/api/admin/studies",
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert!(
        private["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == id)
    );
    let first = publish(fixture, &cookie, &csrf, id, 1).await;
    assert_eq!(
        publish(fixture, &cookie, &csrf, id, 1).await,
        first,
        "publishing twice must be idempotent on every SQL backend"
    );
    let initial = json(call(&fixture.app, "GET", &pinned, None, None, None).await).await;

    save_title(fixture, &cookie, &csrf, id, 1, "Correction à publier").await;
    assert_eq!(
        json(call(&fixture.app, "GET", &entry, None, None, None).await).await,
        initial
    );
    let catalogue = json(call(&fixture.app, "GET", "/api/studies", None, None, None).await).await;
    let item = catalogue["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == id)
        .unwrap();
    assert_eq!(
        item["title"], draft["definition"]["title"],
        "draft titles must stay private too"
    );
    assert_eq!(item["revision"], 1);
    assert_eq!(
        call(
            &fixture.app,
            "GET",
            &format!("{entry}/revisions/2"),
            None,
            None,
            None
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );

    publish(fixture, &cookie, &csrf, id, 2).await;
    let newcomer = json(call(&fixture.app, "GET", &entry, None, None, None).await).await;
    assert_eq!(newcomer["revision"], 2);
    assert_eq!(
        publish(fixture, &cookie, &csrf, id, 1).await["publishedRevision"],
        2,
        "a late retry must not move new arrivals back to the older revision"
    );
    assert_eq!(newcomer["definition"]["title"], "Correction à publier");
    assert_eq!(
        json(call(&fixture.app, "GET", &pinned, None, None, None).await).await,
        initial,
        "existing visits keep the complete immutable definition after publication"
    );
    let restarted = routes(WorkspaceState::with_runtime(
        fixture.database.clone(),
        false,
        fixture.runtime.clone(),
    ));
    assert_eq!(
        json(call(&restarted, "GET", &pinned, None, None, None).await).await,
        initial,
        "pinning must survive an application restart"
    );
    assert_eq!(
        json(call(&restarted, "GET", &entry, None, None, None).await).await,
        newcomer
    );
}

#[tokio::test]
async fn test_publication_should_keep_drafts_private_and_old_visits_on_their_revision() {
    roundtrip(&fixture().await).await;
}

#[tokio::test]
async fn test_publication_should_require_authorization_and_never_publish_an_unreviewed_draft() {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let draft = create(&fixture, &cookie, &csrf, definition()).await;
    let id = draft["id"].as_str().unwrap();
    let endpoint = format!("/api/admin/studies/{id}/publish");
    let body = Some(json!({"expectedRevision":1}));
    assert_eq!(
        call(&fixture.app, "POST", &endpoint, body.clone(), None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            &endpoint,
            body.clone(),
            Some(&cookie),
            None
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    for path in [
        "/api/admin/studies".to_owned(),
        format!("/api/admin/studies/{id}"),
    ] {
        assert_eq!(
            call(&fixture.app, "GET", &path, None, None, None)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    save_title(&fixture, &cookie, &csrf, id, 1, "Another editor's draft").await;
    let stale = call(
        &fixture.app,
        "POST",
        &endpoint,
        body,
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    assert_eq!(json(stale).await["error"]["code"], "revision_conflict");
    assert_eq!(
        call(
            &fixture.app,
            "GET",
            &format!("/api/studies/{id}"),
            None,
            None,
            None
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    let (first, repeated) = tokio::join!(
        publish(&fixture, &cookie, &csrf, id, 2),
        publish(&fixture, &cookie, &csrf, id, 2)
    );
    assert_eq!(first, repeated);
    assert_eq!(first["publishedRevision"], 2);
    let rows = fixture
        .database
        .query_all(
            Query::select()
                .column("revision")
                .from("study_publications")
                .and_where(Expr::col("study_id").eq(id)),
        )
        .await
        .unwrap();
    assert_eq!(
        rows.len(),
        1,
        "retries must not create duplicate publications"
    );
    assert_eq!(rows[0].try_get::<i64>("", "revision").unwrap(), 2);
}
