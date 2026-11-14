use super::*;

async fn page_campaign(fixture: &Fixture, format: &str) -> (String, String, String, String) {
    let (cookie, csrf) = login(fixture).await;
    let study = create(fixture, &cookie, &csrf, collection_definition()).await;
    let study_id = study["id"].as_str().unwrap().to_owned();
    let response = call(
        &fixture.app,
        "POST",
        &format!("/api/admin/studies/{study_id}/runs"),
        Some(json!({"expectedRevision":1,"navigation":{"format":format,"showProgress":false}})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let run = json(response).await;
    assert_eq!(run["profile"], "native-pages-v1");
    assert_eq!(
        run["navigation"],
        json!({"format":format,"showProgress":false})
    );
    (study_id, run["id"].as_str().unwrap().into(), cookie, csrf)
}

pub(super) async fn roundtrip(fixture: &Fixture) {
    for (index, format, first, target, target_group, exchange) in [
        (0, "group", "g:main", "g:details", Some("details"), "G"),
        (1, "question", "q:gate", "q:choice", Some("details"), "S"),
        (2, "all", "all", "all", None, "A"),
    ] {
        let (_, run, cookie, csrf) = page_campaign(fixture, format).await;
        let id = start(fixture, &run, 3000 + index).await;
        let path = format!("/api/responses/{id}");
        let initial = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
        assert_eq!(initial["checkpoint"]["pageId"], first);
        let mut command = update(0, 3100 + index, json!({"GATE":"Y","AGE":"40"}), false);
        command["checkpoint"] = json!({"language":"en","groupId":target_group,"pageId":target});
        let result = participant(
            &fixture.app,
            "PUT",
            &path,
            Some(command.clone()),
            Some(TOKEN),
        )
        .await;
        assert_eq!(result.status(), StatusCode::OK);
        let receipt = json(result).await;
        assert_eq!(receipt["checkpoint"], command["checkpoint"]);
        assert_eq!(
            json(participant(&fixture.app, "PUT", &path, Some(command), Some(TOKEN)).await).await,
            receipt
        );
        let saved = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
        assert_eq!(saved["checkpoint"], receipt["checkpoint"]);
        assert_eq!(saved["answers"]["SCORE"], "41");

        // Hiding the current page cannot leave a cursor on an inactive question/group.
        let mut changed = update(1, 3200 + index, json!({"GATE":"N","AGE":"40"}), false);
        changed["checkpoint"] = receipt["checkpoint"].clone();
        let result = participant(&fixture.app, "PUT", &path, Some(changed), Some(TOKEN)).await;
        assert_eq!(result.status(), StatusCode::OK);
        let normalized = json(result).await;
        assert_eq!(normalized["checkpoint"]["pageId"], first);

        let mut invalid = update(2, 3300 + index, json!({"GATE":"N"}), false);
        invalid["checkpoint"] = json!({"language":"fr","groupId":null,"pageId":"unknown"});
        let result = participant(&fixture.app, "PUT", &path, Some(invalid), Some(TOKEN)).await;
        assert_eq!(result.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(json(result).await["error"]["code"], "invalid_checkpoint");

        // A cursor on the first page does not bypass mandatory answers elsewhere.
        let mut submit = update(2, 3400 + index, json!({"GATE":"Y"}), true);
        submit["checkpoint"] = normalized["checkpoint"].clone();
        assert_eq!(
            participant(
                &fixture.app,
                "PUT",
                &path,
                Some(submit.clone()),
                Some(TOKEN)
            )
            .await
            .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        submit["answers"] = json!({"GATE":"N"});
        assert_eq!(
            participant(&fixture.app, "PUT", &path, Some(submit), Some(TOKEN))
                .await
                .status(),
            StatusCode::OK
        );

        let export = format!("/api/admin/runs/{run}/definition.lss");
        assert_eq!(
            call(&fixture.app, "GET", &export, None, None, None)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let response = call(&fixture.app, "GET", &export, None, Some(&cookie), None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
            .await
            .unwrap();
        let document = rustsurvey_limesurvey::LssDocument::parse(
            &bytes,
            rustsurvey_limesurvey::Limits::default(),
        )
        .unwrap();
        let context = serde_json::from_value(json!({
            "schemaVersion":1,"surveyId":"1","surveyGroupId":"1",
            "groups":[{"id":"1","parentId":null,"values":{}}],"global":{}
        }))
        .unwrap();
        let settings =
            serde_json::to_value(document.group_settings(Some(&context)).unwrap()).unwrap();
        for (name, expected) in [
            ("format", json!(exchange)),
            ("showprogress", json!(false)),
            ("allowprev", json!(true)),
        ] {
            let option = settings["settings"]
                .as_array()
                .unwrap()
                .iter()
                .find(|option| option["name"] == name)
                .unwrap();
            assert_eq!(option["value"], expected);
        }
        let pinned = json(
            call(
                &fixture.app,
                "POST",
                &format!("/api/admin/runs/{run}/close"),
                Some(json!({})),
                Some(&cookie),
                Some(&csrf),
            )
            .await,
        )
        .await;
        assert_eq!(pinned["navigation"], initial["run"]["navigation"]);
    }
}

#[tokio::test]
async fn test_native_navigation_should_persist_cursors_validate_the_whole_submission_and_export_settings()
 {
    roundtrip(&fixture().await).await;
}

#[tokio::test]
async fn test_navigation_migration_should_leave_legacy_profiles_documents_and_receipts_unchanged() {
    let fixture = fixture().await;
    let (_, run, _, _) = campaign(&fixture).await;
    let id = start(&fixture, &run, 3500).await;
    let path = format!("/api/responses/{id}");
    let command = update(0, 3501, json!({"GATE":"Y","AGE":"40"}), false);
    let receipt = json(
        participant(
            &fixture.app,
            "PUT",
            &path,
            Some(command.clone()),
            Some(TOKEN),
        )
        .await,
    )
    .await;
    let before = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    rollback_after(&fixture.database, 8).await;
    Migrator::up(&fixture.database, None).await.unwrap();
    let after = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    assert_eq!(after, before);
    assert_eq!(after["run"]["profile"], "native-forms-v1");
    assert_eq!(after["run"]["navigation"], Value::Null);
    assert!(after["checkpoint"].get("pageId").is_none());
    assert_eq!(
        json(participant(&fixture.app, "PUT", &path, Some(command), Some(TOKEN)).await).await,
        receipt
    );
    let mut new_cursor = update(1, 3502, json!({"GATE":"N"}), false);
    new_cursor["checkpoint"]["pageId"] = json!("g:main");
    let rejected = participant(&fixture.app, "PUT", &path, Some(new_cursor), Some(TOKEN)).await;
    assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json(rejected).await["error"]["code"], "invalid_checkpoint");
}

#[tokio::test]
async fn test_navigation_creation_should_reject_unknown_or_incomplete_settings() {
    let fixture = fixture().await;
    let (study, _, cookie, csrf) = campaign(&fixture).await;
    for navigation in [
        json!({"format":"unknown","showProgress":true}),
        json!({"format":"question"}),
        json!({"format":"all","showProgress":true,"allowPrevious":false}),
    ] {
        let response = call(
            &fixture.app,
            "POST",
            &format!("/api/admin/studies/{study}/runs"),
            Some(json!({"expectedRevision":1,"navigation":navigation})),
            Some(&cookie),
            Some(&csrf),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[tokio::test]
async fn test_navigation_start_should_skip_inactive_groups_and_allow_an_empty_relevant_submission()
{
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    for (index, format, page) in [
        (0, "group", "empty"),
        (1, "question", "empty"),
        (2, "all", "all"),
    ] {
        let mut definition = collection_definition();
        for group in definition["groups"].as_array_mut().unwrap() {
            group["relevance"] = json!("0");
        }
        let study = create(&fixture, &cookie, &csrf, definition).await;
        let response = call(
            &fixture.app,
            "POST",
            &format!("/api/admin/studies/{}/runs", study["id"].as_str().unwrap()),
            Some(json!({"expectedRevision":1,"navigation":{"format":format,"showProgress":true}})),
            Some(&cookie),
            Some(&csrf),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let run = json(response).await;
        let id = start(&fixture, run["id"].as_str().unwrap(), 3600 + index).await;
        let path = format!("/api/responses/{id}");
        let initial = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
        assert_eq!(
            initial["checkpoint"],
            json!({"language":"fr","groupId":null,"pageId":page})
        );
        let mut submit = update(0, 3700 + index, json!({}), true);
        submit["checkpoint"] = initial["checkpoint"].clone();
        assert_eq!(
            participant(&fixture.app, "PUT", &path, Some(submit), Some(TOKEN))
                .await
                .status(),
            StatusCode::OK
        );
    }
}
