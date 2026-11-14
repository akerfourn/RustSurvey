use super::*;

#[tokio::test]
async fn test_vv_export_should_use_closed_pinned_campaign_and_preserve_private_response_values() {
    roundtrip(&fixture().await).await;
}

pub(super) async fn roundtrip(fixture: &Fixture) {
    let (cookie, csrf) = login(fixture).await;
    let mut corpus: Value = serde_json::from_str(include_str!(
        "../../../../../examples/rules/collection-forms.json"
    ))
    .unwrap();
    corpus["definition"]["languages"] = json!(["fr", "en"]);
    let study = create(fixture, &cookie, &csrf, corpus["definition"].clone()).await;
    let run = json(
        call(
            &fixture.app,
            "POST",
            &format!("/api/admin/studies/{}/runs", study["id"].as_str().unwrap()),
            Some(json!({"expectedRevision":1})),
            Some(&cookie),
            Some(&csrf),
        )
        .await,
    )
    .await;
    let run_id = run["id"].as_str().unwrap();
    let path = format!("/api/admin/runs/{run_id}/responses.vv");
    assert_eq!(
        call(&fixture.app, "GET", &path, None, None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&fixture.app, "GET", &path, None, Some(&cookie), None)
            .await
            .status(),
        StatusCode::CONFLICT
    );
    let first = start(fixture, run_id, 801).await;
    let submitted = participant(&fixture.app, "PUT", &format!("/api/responses/{first}"), Some(json!({"expectedVersion":0,"operationId":identifier(8001),"answers":corpus["baseline"]["answers"],"checkpoint":{"language":"fr","groupId":"forms"},"submit":true})), Some(TOKEN)).await;
    assert_eq!(submitted.status(), StatusCode::OK);
    let second = start(fixture, run_id, 802).await;
    let draft_command = json!({
        "expectedVersion": 0, "operationId": identifier(8002),
        "answers": {"S":"", "T":"  O'Neil `code`\r\n\t\"text\" {question_not_shown}  "},
        "checkpoint": {"language":"en", "groupId":"forms"}, "submit":false
    });
    let saved = participant(
        &fixture.app,
        "PUT",
        &format!("/api/responses/{second}"),
        Some(draft_command),
        Some(TOKEN),
    )
    .await;
    assert_eq!(saved.status(), StatusCode::OK);
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            &format!("/api/admin/runs/{run_id}/close"),
            Some(json!({})),
            Some(&cookie),
            Some(&csrf)
        )
        .await
        .status(),
        StatusCode::OK
    );
    let mut edited = corpus["definition"].clone();
    edited["groups"][0]["questions"][0]["code"] = json!("RENAMED");
    assert_eq!(
        call(
            &fixture.app,
            "PUT",
            &format!("/api/admin/studies/{}", study["id"].as_str().unwrap()),
            Some(json!({"expectedRevision":1,"definition":edited})),
            Some(&cookie),
            Some(&csrf)
        )
        .await
        .status(),
        StatusCode::OK
    );
    let response = call(&fixture.app, "GET", &path, None, Some(&cookie), None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let output = String::from_utf8(
        to_bytes(response.into_body(), 8 * 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    let lines: Vec<_> = output.lines().collect();
    assert_eq!(lines.len(), 4, "two headers, one submission and one draft");
    assert_eq!(lines[0], lines[1]);
    let header: Vec<_> = lines[1].split('\t').collect();
    let row = |line: &str| {
        header
            .iter()
            .copied()
            .zip(line.split('\t'))
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let submission = row(lines[2]);
    let draft = row(lines[3]);
    assert_eq!(submission["L"], "01");
    assert_eq!(submission["DUAL_A1_1"], "10");
    assert_eq!(submission["CALC"], "10");
    assert_eq!(submission["submitdate"], "2027-01-15 08:00:00");
    assert_eq!(draft["submitdate"], "{question_not_shown}");
    assert_eq!(
        draft["startlanguage"], "fr",
        "changing checkpoint language must not replace the initial language"
    );
    assert_eq!(draft["S"], "");
    assert_eq!(draft["N"], "{question_not_shown}");
    assert_eq!(
        draft["T"],
        "  O'Neil `code`{cr}{newline}{tab}{quote}text{quote} {lbrace}question_not_shown}  "
    );
    assert!(!header.contains(&"RENAMED"));
    assert!(!header.contains(&"token"));
    assert!(!output.contains(TOKEN));
    assert!(!output.contains(&first));
    assert!(!output.contains(&second));
}

#[tokio::test]
async fn test_vv_export_should_fail_the_stream_when_a_private_document_cannot_be_read() {
    let fixture = fixture().await;
    let (_, run, cookie, csrf) = campaign(&fixture).await;
    let id = start(&fixture, &run, 803).await;
    call(
        &fixture.app,
        "POST",
        &format!("/api/admin/runs/{run}/close"),
        Some(json!({})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    fixture
        .database
        .execute(
            &Query::update()
                .table("responses")
                .value("document", "invalid synthetic JSON")
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await
        .unwrap();
    let response = call(
        &fixture.app,
        "GET",
        &format!("/api/admin/runs/{run}/responses.vv"),
        None,
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        to_bytes(response.into_body(), 1024 * 1024).await.is_err(),
        "an interrupted export must not look like a complete file"
    );
}

#[tokio::test]
async fn test_vv_export_should_cross_sql_pages_without_skipping_or_duplicating_responses() {
    let fixture = fixture().await;
    let (_, run, cookie, csrf) = campaign(&fixture).await;
    for serial in 850..905 {
        start(&fixture, &run, serial).await;
    }
    call(
        &fixture.app,
        "POST",
        &format!("/api/admin/runs/{run}/close"),
        Some(json!({})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    let response = call(
        &fixture.app,
        "GET",
        &format!("/api/admin/runs/{run}/responses.vv"),
        None,
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let output = String::from_utf8(
        to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert_eq!(
        output.lines().count(),
        57,
        "all 55 responses must be exported after two headers"
    );
}

#[tokio::test]
async fn test_vv_export_should_refuse_to_guess_initial_language_for_legacy_multilingual_responses()
{
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let mut definition = collection_definition();
    definition["languages"] = json!(["fr", "en"]);
    let study = create(&fixture, &cookie, &csrf, definition).await;
    let run = json(
        call(
            &fixture.app,
            "POST",
            &format!("/api/admin/studies/{}/runs", study["id"].as_str().unwrap()),
            Some(json!({"expectedRevision":1})),
            Some(&cookie),
            Some(&csrf),
        )
        .await,
    )
    .await;
    let run = run["id"].as_str().unwrap();
    let id = start(&fixture, run, 906).await;
    call(
        &fixture.app,
        "POST",
        &format!("/api/admin/runs/{run}/close"),
        Some(json!({})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    rollback_after(&fixture.database, 5).await;
    Migrator::up(&fixture.database, None).await.unwrap();
    let response = call(
        &fixture.app,
        "GET",
        &format!("/api/admin/runs/{run}/responses.vv"),
        None,
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        json(response).await["error"]["code"],
        "initial_language_unavailable"
    );
    let native = call(
        &fixture.app,
        "GET",
        &format!("/api/admin/runs/{run}/responses/{id}/response.json"),
        None,
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(native.status(), StatusCode::OK);
    assert_eq!(json(native).await["initialLanguage"], Value::Null);
}
