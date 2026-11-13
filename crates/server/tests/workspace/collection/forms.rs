use super::*;

#[tokio::test]
async fn test_forms_run_should_validate_and_store_all_28_native_types_with_the_editor_core_theme() {
    roundtrip(&fixture().await).await;
}

pub(super) async fn roundtrip(fixture: &Fixture) {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../../examples/rules/collection-forms.json"
    ))
    .unwrap();
    let (cookie, csrf) = login(fixture).await;
    let study = create(fixture, &cookie, &csrf, corpus["definition"].clone()).await;
    let created = call(
        &fixture.app,
        "POST",
        &format!("/api/admin/studies/{}/runs", study["id"].as_str().unwrap()),
        Some(json!({"expectedRevision":1})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    let status = created.status();
    let run = json(created).await;
    assert_eq!(status, StatusCode::CREATED, "{run}");
    assert_eq!(run["profile"], "native-forms-v1");
    let id = start(fixture, run["id"].as_str().unwrap(), 281).await;
    let path = format!("/api/responses/{id}");
    let command = json!({"expectedVersion":0,"operationId":identifier(2081),"answers":corpus["baseline"]["answers"],"checkpoint":{"language":"fr","groupId":"forms"},"submit":true});
    let mut invalid = command.clone();
    invalid["answers"]["DUAL_A1_1"] = json!("01");
    let rejected = participant(&fixture.app, "PUT", &path, Some(invalid), Some(TOKEN)).await;
    assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json(rejected).await["error"]["code"], "invalid_answers");
    let submitted = participant(
        &fixture.app,
        "PUT",
        &path,
        Some(command.clone()),
        Some(TOKEN),
    )
    .await;
    assert_eq!(submitted.status(), StatusCode::OK);
    let acknowledgement = json(submitted).await;
    assert_eq!(
        json(participant(&fixture.app, "PUT", &path, Some(command), Some(TOKEN)).await).await,
        acknowledgement
    );
    let stored = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    assert_eq!(stored["answers"], corpus["baseline"]["expected"]);
    assert_eq!(stored["version"], 1);
    assert_eq!(stored["status"], "submitted");
}

#[tokio::test]
async fn test_existing_simple_profile_should_remain_readable_and_writable_after_adding_the_forms_profile()
 {
    let fixture = fixture().await;
    let (_, run, _, _) = campaign(&fixture).await;
    fixture
        .database
        .execute(
            &Query::update()
                .table("runs")
                .value("profile", "native-simple-v1")
                .and_where(Expr::col("id").eq(&run))
                .to_owned(),
        )
        .await
        .unwrap();
    let id = start(&fixture, &run, 282).await;
    let path = format!("/api/responses/{id}");
    let saved = participant(
        &fixture.app,
        "PUT",
        &path,
        Some(update(0, 2082, json!({"GATE":"N"}), true)),
        Some(TOKEN),
    )
    .await;
    assert_eq!(saved.status(), StatusCode::OK);
    let stored = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    assert_eq!(stored["run"]["profile"], "native-simple-v1");
    assert_eq!(stored["answers"]["GATE"], "N");
}
