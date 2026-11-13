use super::*;

const INVITATION: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

async fn request(
    fixture: &Fixture,
    method: &str,
    path: &str,
    body: Option<Value>,
    token: &str,
    grant: Option<&str>,
    cookie: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("X-Requested-With", "RustSurvey")
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    if let Some(grant) = grant {
        builder = builder.header("X-Participation-Token", grant);
    }
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    let body = if let Some(body) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(body.to_string())
    } else {
        Body::empty()
    };
    fixture
        .app
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap()
}

async fn invite(fixture: &Fixture, run: &str, cookie: &str, csrf: &str, maximum: Value) -> Value {
    let response = call(
        &fixture.app,
        "POST",
        &format!("/api/admin/runs/{run}/invitations"),
        Some(json!({"id":identifier(500),"token":INVITATION,"maxUses":maximum})),
        Some(cookie),
        Some(csrf),
    )
    .await;
    let status = response.status();
    let result = json(response).await;
    assert_eq!(status, StatusCode::CREATED, "{result}");
    result
}

async fn invited_start(fixture: &Fixture, run: &str, serial: u64, persistent: bool) -> Value {
    let mut token = TOKEN;
    if persistent {
        token = INVITATION;
    }
    let response = request(
        fixture,
        "POST",
        &format!("/api/runs/{run}/responses"),
        Some(json!({"responseId":identifier(serial),"language":"fr"})),
        token,
        Some(INVITATION),
        None,
    )
    .await;
    let status = response.status();
    let result = json(response).await;
    assert_eq!(status, StatusCode::CREATED, "{result}");
    result
}

async fn uses(fixture: &Fixture, run: &str, cookie: &str) -> i64 {
    let response = call(
        &fixture.app,
        "GET",
        &format!("/api/admin/runs/{run}/invitations"),
        None,
        Some(cookie),
        None,
    )
    .await;
    let document = json(response).await;
    assert!(!document.to_string().contains(INVITATION));
    assert!(document["items"][0].get("secretHash").is_none());
    assert!(document["items"][0].get("responseId").is_none());
    document["items"][0]["uses"].as_i64().unwrap()
}

#[tokio::test]
async fn test_browser_cookie_should_block_new_entries_and_submissions_but_allow_private_reads_and_retries()
 {
    let fixture = fixture().await;
    let (_, run, _, _) = campaign_with_policy(&fixture, json!({"mode":"browserOnce"})).await;
    let first = start(&fixture, &run, 201).await;
    let second = start(&fixture, &run, 202).await;
    let path = format!("/api/responses/{first}");
    let draft = participant(
        &fixture.app,
        "PUT",
        &path,
        Some(update(0, 2001, json!({"GATE":"N"}), false)),
        Some(TOKEN),
    )
    .await;
    assert_eq!(draft.status(), StatusCode::OK);
    assert!(draft.headers().get(header::SET_COOKIE).is_none());
    let command = update(1, 2002, json!({"GATE":"N"}), true);
    let submitted = participant(
        &fixture.app,
        "PUT",
        &path,
        Some(command.clone()),
        Some(TOKEN),
    )
    .await;
    assert_eq!(submitted.status(), StatusCode::OK);
    let cookie = submitted.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .to_owned();
    for expected in ["HttpOnly", "SameSite=Lax", "Max-Age=31536000", "Path=/api"] {
        assert!(cookie.contains(expected), "{cookie}");
    }
    let cookie = cookie.split(';').next().unwrap();
    let ack = json(submitted).await;
    let denied = request(
        &fixture,
        "POST",
        &format!("/api/runs/{run}/responses"),
        Some(json!({"responseId":identifier(203),"language":"fr"})),
        TOKEN,
        None,
        Some(cookie),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::CONFLICT);
    assert_eq!(
        json(denied).await["error"]["code"],
        "browser_already_completed"
    );
    let denied = request(
        &fixture,
        "PUT",
        &format!("/api/responses/{second}"),
        Some(update(0, 2003, json!({"GATE":"N"}), true)),
        TOKEN,
        None,
        Some(cookie),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::CONFLICT);
    let replay = request(
        &fixture,
        "PUT",
        &path,
        Some(command),
        TOKEN,
        None,
        Some(cookie),
    )
    .await;
    assert_eq!(replay.status(), StatusCode::OK);
    assert!(replay.headers().contains_key(header::SET_COOKIE));
    assert_eq!(json(replay).await, ack);
    let read = request(&fixture, "GET", &path, None, TOKEN, None, Some(cookie)).await;
    assert_eq!(read.status(), StatusCode::OK);
    let entry = request(
        &fixture,
        "GET",
        &format!("/api/runs/{run}"),
        None,
        TOKEN,
        None,
        Some(cookie),
    )
    .await;
    assert_eq!(json(entry).await["browserCompleted"], true);
    // Clearing the cookie bypasses this advisory browser restriction.
    start(&fixture, &run, 204).await;
}

#[tokio::test]
async fn test_independent_invitation_should_consume_only_a_valid_submission_and_keep_no_response_link()
 {
    let fixture = fixture().await;
    let (_, run, cookie, csrf) = campaign_with_policy(
        &fixture,
        json!({"mode":"invitation","persistence":false,"allowEditAfterSubmit":false}),
    )
    .await;
    invite(&fixture, &run, &cookie, &csrf, json!(1)).await;
    let denied = participant(
        &fixture.app,
        "POST",
        &format!("/api/runs/{run}/responses"),
        Some(json!({"responseId":identifier(210),"language":"fr"})),
        Some(TOKEN),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        json(denied).await["error"]["code"],
        "invitation_unavailable"
    );
    let first = invited_start(&fixture, &run, 211, false).await;
    let second = invited_start(&fixture, &run, 212, false).await;
    let path = format!("/api/responses/{}", first["id"].as_str().unwrap());
    let invalid = request(
        &fixture,
        "PUT",
        &path,
        Some(update(0, 2010, json!({}), true)),
        TOKEN,
        Some(INVITATION),
        None,
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(uses(&fixture, &run, &cookie).await, 0);
    let missing_grant = request(
        &fixture,
        "PUT",
        &path,
        Some(update(0, 2011, json!({}), false)),
        TOKEN,
        None,
        None,
    )
    .await;
    assert_eq!(missing_grant.status(), StatusCode::FORBIDDEN);
    let draft = request(
        &fixture,
        "PUT",
        &path,
        Some(update(0, 2012, json!({"GATE":"N"}), false)),
        TOKEN,
        Some(INVITATION),
        None,
    )
    .await;
    assert_eq!(draft.status(), StatusCode::OK);
    assert_eq!(uses(&fixture, &run, &cookie).await, 0);
    let command = update(1, 2013, json!({"GATE":"N"}), true);
    let submitted = request(
        &fixture,
        "PUT",
        &path,
        Some(command.clone()),
        TOKEN,
        Some(INVITATION),
        None,
    )
    .await;
    assert_eq!(submitted.status(), StatusCode::OK);
    let ack = json(submitted).await;
    assert_eq!(uses(&fixture, &run, &cookie).await, 1);
    assert_eq!(
        json(
            request(
                &fixture,
                "PUT",
                &path,
                Some(command),
                TOKEN,
                Some(INVITATION),
                None
            )
            .await
        )
        .await,
        ack
    );
    let denied = request(
        &fixture,
        "PUT",
        &format!("/api/responses/{}", second["id"].as_str().unwrap()),
        Some(update(0, 2014, json!({"GATE":"N"}), true)),
        TOKEN,
        Some(INVITATION),
        None,
    )
    .await;
    assert_eq!(denied.status(), StatusCode::CONFLICT);
    assert_eq!(json(denied).await["error"]["code"], "invitation_exhausted");
    let row = fixture
        .database
        .query_one(
            &Query::select()
                .columns(["secret_hash", "response_id"])
                .from("invitations")
                .and_where(Expr::col("run_id").eq(&run))
                .to_owned(),
        )
        .await
        .unwrap()
        .unwrap();
    assert_ne!(
        row.try_get::<String>("", "secret_hash").unwrap(),
        INVITATION
    );
    assert!(
        row.try_get::<Option<String>>("", "response_id")
            .unwrap()
            .is_none()
    );
    assert_eq!(
        participant(&fixture.app, "GET", &path, None, Some(TOKEN))
            .await
            .status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn test_concurrent_submissions_should_have_one_winner_for_the_last_invitation_use() {
    let fixture = fixture().await;
    let (_, run, cookie, csrf) = campaign_with_policy(
        &fixture,
        json!({"mode":"invitation","persistence":false,"allowEditAfterSubmit":false}),
    )
    .await;
    invite(&fixture, &run, &cookie, &csrf, json!(1)).await;
    let first = invited_start(&fixture, &run, 221, false).await;
    let second = invited_start(&fixture, &run, 222, false).await;
    let paths = [
        format!("/api/responses/{}", first["id"].as_str().unwrap()),
        format!("/api/responses/{}", second["id"].as_str().unwrap()),
    ];
    let (left, right) = tokio::join!(
        request(
            &fixture,
            "PUT",
            &paths[0],
            Some(update(0, 2021, json!({"GATE":"N"}), true)),
            TOKEN,
            Some(INVITATION),
            None
        ),
        request(
            &fixture,
            "PUT",
            &paths[1],
            Some(update(0, 2022, json!({"GATE":"N"}), true)),
            TOKEN,
            Some(INVITATION),
            None
        )
    );
    let statuses = [left.status(), right.status()];
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::OK)
            .count(),
        1
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::CONFLICT)
            .count(),
        1
    );
    assert_eq!(uses(&fixture, &run, &cookie).await, 1);
}

#[tokio::test]
async fn test_invitation_should_allow_multiple_independent_responses_up_to_its_limit() {
    let fixture = fixture().await;
    let (_, run, cookie, csrf) = campaign_with_policy(
        &fixture,
        json!({"mode":"invitation","persistence":false,"allowEditAfterSubmit":false}),
    )
    .await;
    invite(&fixture, &run, &cookie, &csrf, json!(2)).await;
    for serial in [231, 232] {
        let started = invited_start(&fixture, &run, serial, false).await;
        let path = format!("/api/responses/{}", started["id"].as_str().unwrap());
        assert_eq!(
            request(
                &fixture,
                "PUT",
                &path,
                Some(update(0, serial + 2000, json!({"GATE":"N"}), true)),
                TOKEN,
                Some(INVITATION),
                None
            )
            .await
            .status(),
            StatusCode::OK
        );
    }
    assert_eq!(uses(&fixture, &run, &cookie).await, 2);
    let denied = request(
        &fixture,
        "POST",
        &format!("/api/runs/{run}/responses"),
        Some(json!({"responseId":identifier(233),"language":"fr"})),
        TOKEN,
        Some(INVITATION),
        None,
    )
    .await;
    assert_eq!(denied.status(), StatusCode::CONFLICT);
    assert_eq!(json(denied).await["error"]["code"], "invitation_exhausted");
}

#[tokio::test]
async fn test_persistent_invitation_should_resume_one_response_and_keep_final_answers_immutable_by_default()
 {
    let fixture = fixture().await;
    roundtrip(&fixture).await;
}

#[tokio::test]
async fn test_unlimited_invitation_should_keep_accepting_independent_responses_after_its_first_submission()
 {
    let fixture = fixture().await;
    let (_, run, cookie, csrf) = campaign_with_policy(
        &fixture,
        json!({"mode":"invitation","persistence":false,"allowEditAfterSubmit":false}),
    )
    .await;
    invite(&fixture, &run, &cookie, &csrf, Value::Null).await;
    for serial in 291..294 {
        let started = invited_start(&fixture, &run, serial, false).await;
        let path = format!("/api/responses/{}", started["id"].as_str().unwrap());
        let response = request(
            &fixture,
            "PUT",
            &path,
            Some(update(0, serial + 2000, json!({"GATE":"N"}), true)),
            TOKEN,
            Some(INVITATION),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert_eq!(uses(&fixture, &run, &cookie).await, 3);
    invited_start(&fixture, &run, 294, false).await;
}

pub(super) async fn roundtrip(fixture: &Fixture) {
    let (_, run, cookie, csrf) = campaign_with_policy(
        fixture,
        json!({"mode":"invitation","persistence":true,"allowEditAfterSubmit":false}),
    )
    .await;
    invite(fixture, &run, &cookie, &csrf, json!(1)).await;
    let first = invited_start(fixture, &run, 241, true).await;
    let path = format!("/api/responses/{}", first["id"].as_str().unwrap());
    let draft = request(
        fixture,
        "PUT",
        &path,
        Some(update(0, 2041, json!({"GATE":"Y","AGE":"24"}), false)),
        INVITATION,
        None,
        None,
    )
    .await;
    assert_eq!(draft.status(), StatusCode::OK);
    let resumed = invited_start(fixture, &run, 242, true).await;
    assert_eq!(resumed["id"], first["id"]);
    assert_eq!(resumed["answers"]["AGE"], "24");
    assert_eq!(resumed["version"], 1);
    assert_eq!(
        request(
            fixture,
            "PUT",
            &path,
            Some(update(1, 2042, json!({"GATE":"N"}), true)),
            INVITATION,
            None,
            None
        )
        .await
        .status(),
        StatusCode::OK
    );
    let completed = invited_start(fixture, &run, 243, true).await;
    assert_eq!(completed["id"], first["id"]);
    assert_eq!(completed["status"], "submitted");
    assert_eq!(
        request(
            fixture,
            "PUT",
            &path,
            Some(update(2, 2043, json!({"GATE":"Y","AGE":"99"}), true)),
            INVITATION,
            None,
            None
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(uses(fixture, &run, &cookie).await, 1);
}

#[tokio::test]
async fn test_editable_invitation_should_correct_the_same_response_without_consuming_another_use() {
    let fixture = fixture().await;
    let (_, run, cookie, csrf) = campaign_with_policy(
        &fixture,
        json!({"mode":"invitation","persistence":true,"allowEditAfterSubmit":true}),
    )
    .await;
    invite(&fixture, &run, &cookie, &csrf, json!(1)).await;
    let first = invited_start(&fixture, &run, 251, true).await;
    let path = format!("/api/responses/{}", first["id"].as_str().unwrap());
    let commands = [
        update(0, 2051, json!({"GATE":"N"}), true),
        update(1, 2052, json!({"GATE":"Y"}), false),
        update(2, 2053, json!({"GATE":"Y","AGE":"42"}), true),
    ];
    for command in commands {
        let response = request(
            &fixture,
            "PUT",
            &path,
            Some(command.clone()),
            INVITATION,
            None,
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let ack = json(response).await;
        assert_eq!(
            json(
                request(
                    &fixture,
                    "PUT",
                    &path,
                    Some(command),
                    INVITATION,
                    None,
                    None
                )
                .await
            )
            .await,
            ack
        );
    }
    let resumed = invited_start(&fixture, &run, 252, true).await;
    assert_eq!(resumed["id"], first["id"]);
    assert_eq!(resumed["answers"]["AGE"], "42");
    assert_eq!(resumed["answers"]["SCORE"], "43");
    assert_eq!(resumed["version"], 3);
    assert_eq!(uses(&fixture, &run, &cookie).await, 1);
}

#[tokio::test]
async fn test_invitation_should_apply_validity_and_revocation_to_existing_writes() {
    let fixture = fixture().await;
    let (_, run, cookie, csrf) = campaign_with_policy(
        &fixture,
        json!({"mode":"invitation","persistence":true,"allowEditAfterSubmit":true}),
    )
    .await;
    let now = fixture.runtime.now().unwrap();
    let created = call(
        &fixture.app,
        "POST",
        &format!("/api/admin/runs/{run}/invitations"),
        Some(
            json!({"id":identifier(500),"token":INVITATION,"validFrom":now+10,"validUntil":now+20}),
        ),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let start_command = json!({"responseId":identifier(261),"language":"fr"});
    let path = format!("/api/runs/{run}/responses");
    assert_eq!(
        request(
            &fixture,
            "POST",
            &path,
            Some(start_command.clone()),
            INVITATION,
            Some(INVITATION),
            None
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    fixture.runtime.now.store(now + 10, Ordering::SeqCst);
    let started = invited_start(&fixture, &run, 261, true).await;
    let path = format!("/api/responses/{}", started["id"].as_str().unwrap());
    fixture.runtime.now.store(now + 20, Ordering::SeqCst);
    assert_eq!(
        request(
            &fixture,
            "PUT",
            &path,
            Some(update(0, 2061, json!({"GATE":"N"}), true)),
            INVITATION,
            None,
            None
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    fixture.runtime.now.store(now + 15, Ordering::SeqCst);
    assert_eq!(
        request(&fixture, "GET", &path, None, INVITATION, None, None)
            .await
            .status(),
        StatusCode::OK
    );
    let revoke_path = format!(
        "/api/admin/runs/{run}/invitations/{}/revoke",
        identifier(500)
    );
    for _ in 0..2 {
        assert_eq!(
            call(
                &fixture.app,
                "POST",
                &revoke_path,
                Some(json!({})),
                Some(&cookie),
                Some(&csrf)
            )
            .await
            .status(),
            StatusCode::OK
        );
    }
    assert_eq!(
        request(&fixture, "GET", &path, None, INVITATION, None, None)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            &fixture,
            "PUT",
            &path,
            Some(update(0, 2062, json!({"GATE":"N"}), true)),
            INVITATION,
            None,
            None
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(uses(&fixture, &run, &cookie).await, 0);
}

#[tokio::test]
async fn test_invitation_administration_should_require_csrf_and_make_creation_retries_safe() {
    let fixture = fixture().await;
    let (_, run, cookie, csrf) = campaign_with_policy(
        &fixture,
        json!({"mode":"invitation","persistence":false,"allowEditAfterSubmit":false}),
    )
    .await;
    let path = format!("/api/admin/runs/{run}/invitations");
    assert_eq!(
        call(&fixture.app, "GET", &path, None, None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let command = json!({"id":identifier(500),"token":INVITATION,"maxUses":1});
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            &path,
            Some(command.clone()),
            Some(&cookie),
            None
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let first = invite(&fixture, &run, &cookie, &csrf, json!(1)).await;
    let replay = invite(&fixture, &run, &cookie, &csrf, json!(1)).await;
    assert_eq!(first, replay);
    let changed = json!({"id":identifier(500),"token":INVITATION,"maxUses":2});
    let denied = call(
        &fixture.app,
        "POST",
        &path,
        Some(changed),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::CONFLICT);
    assert_eq!(json(denied).await["error"]["code"], "invitation_conflict");
    for maximum in [json!(0), json!(100_001)] {
        let command = json!({"id":identifier(501),"token":"c".repeat(64),"maxUses":maximum});
        let denied = call(
            &fixture.app,
            "POST",
            &path,
            Some(command),
            Some(&cookie),
            Some(&csrf),
        )
        .await;
        assert_eq!(denied.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            json(denied).await["error"]["code"],
            "invalid_participation_policy"
        );
    }
}

#[tokio::test]
async fn test_participation_migration_should_preserve_existing_runs_responses_and_receipts_with_open_access()
 {
    let fixture = fixture().await;
    let (_, run, _, _) = campaign(&fixture).await;
    let id = start(&fixture, &run, 271).await;
    let path = format!("/api/responses/{id}");
    let command = update(0, 2071, json!({"GATE":"N"}), true);
    let ack = json(
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
    Migrator::down(&fixture.database, Some(1)).await.unwrap();
    assert!(
        !SchemaManager::new(&fixture.database)
            .has_column("runs", "participation_policy")
            .await
            .unwrap()
    );
    Migrator::up(&fixture.database, None).await.unwrap();
    let stored = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    assert_eq!(stored["run"]["participationPolicy"], json!({"mode":"open"}));
    assert_eq!(stored["answers"]["GATE"], "N");
    assert_eq!(
        json(participant(&fixture.app, "PUT", &path, Some(command), Some(TOKEN)).await).await,
        ack
    );
}
