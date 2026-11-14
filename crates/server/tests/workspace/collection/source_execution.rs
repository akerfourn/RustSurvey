use super::*;
#[path = "../../../../limesurvey/tests/support/import_execution.rs"]
mod source_fixture;

struct Prepared {
    study: Value,
    request: Value,
    cookie: String,
    csrf: String,
    source_survey_id: String,
}

async fn prepare(fixture: &Fixture, xml: Vec<u8>, context: Value, facts: Value) -> Prepared {
    let (cookie, csrf) = login(fixture).await;
    let response = import_request(fixture, xml, Some(&cookie), Some(&csrf)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let study = json(response).await;
    let id = study["id"].as_str().unwrap();
    let response = call(
        &fixture.app,
        "POST",
        &format!("/api/admin/studies/{id}/revisions/1/settings-contexts"),
        Some(context),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let context = json(response).await;
    let source_survey_id = facts["surveyId"].as_str().unwrap().into();
    Prepared {
        study,
        cookie,
        csrf,
        source_survey_id,
        request: json!({"expectedRevision":1,"sourceExecution":{"contextId":context["id"],"facts":facts}}),
    }
}

async fn fixture_source(fixture: &Fixture) -> Prepared {
    let (definition, xml, context, facts) = source_fixture::fixture();
    let question = definition.groups[0]
        .questions
        .iter()
        .find(|question| question.code == "S")
        .unwrap();
    let id = &question.source.as_ref().unwrap().id;
    let defaults = format!(
        "<defaultvalues><fields><fieldname>dvid</fieldname><fieldname>qid</fieldname><fieldname>sqid</fieldname><fieldname>scale_id</fieldname><fieldname>specialtype</fieldname></fields><rows><row><dvid>1</dvid><qid>{id}</qid><sqid>0</sqid><scale_id>0</scale_id><specialtype/></row></rows></defaultvalues><defaultvalue_l10ns><fields><fieldname>dvid</fieldname><fieldname>language</fieldname><fieldname>defaultvalue</fieldname></fields><rows><row><dvid>1</dvid><language>fr</language><defaultvalue>seed</defaultvalue></row></rows></defaultvalue_l10ns>"
    );
    let xml = xml.replace("</document>", &format!("{defaults}</document>"));
    prepare(
        fixture,
        xml.into_bytes(),
        serde_json::to_value(context).unwrap(),
        serde_json::to_value(facts).unwrap(),
    )
    .await
}

pub(super) async fn roundtrip(fixture: &Fixture) {
    let prepared = fixture_source(fixture).await;
    exercise(fixture, prepared, true).await;
}

async fn exercise(fixture: &Fixture, prepared: Prepared, defaults: bool) {
    let study = prepared.study["id"].as_str().unwrap();
    let response = call(
        &fixture.app,
        "POST",
        &format!("/api/admin/studies/{study}/runs"),
        Some(prepared.request.clone()),
        Some(&prepared.cookie),
        Some(&prepared.csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let run = json(response).await;
    let run_id = run["id"].as_str().unwrap();
    let mut profile = "native-pages-v1";
    if defaults {
        profile = "native-defaults-v1";
    }
    assert_eq!(run["profile"], profile);
    let id = start(fixture, run_id, 6600).await;
    let path = format!("/api/responses/{id}");
    let before = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    if defaults {
        assert_eq!(before["answers"]["S"], "seed");
    }
    assert!(
        !before.to_string().contains(
            prepared.request["sourceExecution"]["contextId"]
                .as_str()
                .unwrap()
        )
    );
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../../examples/rules/collection-forms.json"
    ))
    .unwrap();
    let command = json!({"expectedVersion":0,"operationId":"00000000-0000-4000-8000-000000006601",
        "answers":corpus["baseline"]["answers"],"checkpoint":before["checkpoint"],"submit":false});
    let response = participant(
        &fixture.app,
        "PUT",
        &path,
        Some(command.clone()),
        Some(TOKEN),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let acknowledgement = json(response).await;
    let after = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    assert_eq!(after["answers"], corpus["baseline"]["expected"]);
    let mut edited = prepared.study["definition"].clone();
    edited["groups"][0]["questions"][0]["relevance"] = json!("0");
    let updated = call(
        &fixture.app,
        "PUT",
        &format!("/api/admin/studies/{study}"),
        Some(json!({"expectedRevision":1,"definition":edited})),
        Some(&prepared.cookie),
        Some(&prepared.csrf),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let restarted = routes(WorkspaceState::with_runtime(
        fixture.database.clone(),
        false,
        fixture.runtime.clone(),
    ));
    let resumed = json(participant(&restarted, "GET", &path, None, Some(TOKEN)).await).await;
    assert_eq!(
        resumed, after,
        "the campaign must retain its executable revision after edits and restart"
    );
    assert_eq!(
        json(participant(&restarted, "PUT", &path, Some(command), Some(TOKEN)).await).await,
        acknowledgement
    );
    let mut final_command = json!({"expectedVersion":1,"operationId":"00000000-0000-4000-8000-000000006602",
        "answers":corpus["baseline"]["answers"],"checkpoint":resumed["checkpoint"],"submit":true});
    let response = participant(
        &restarted,
        "PUT",
        &path,
        Some(final_command.take()),
        Some(TOKEN),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = call(
        &restarted,
        "POST",
        &format!("/api/admin/runs/{run_id}/close"),
        Some(json!({})),
        Some(&prepared.cookie),
        Some(&prepared.csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let vv = call(
        &restarted,
        "GET",
        &format!("/api/admin/runs/{run_id}/responses.vv"),
        None,
        Some(&prepared.cookie),
        None,
    )
    .await;
    assert_eq!(vv.status(), StatusCode::OK);
    let bytes = to_bytes(vv.into_body(), 8 * 1024 * 1024).await.unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.contains("CALC") && !text.contains("forged"));
    let lss = call(
        &restarted,
        "GET",
        &format!("/api/admin/runs/{run_id}/definition.lss"),
        None,
        Some(&prepared.cookie),
        None,
    )
    .await;
    assert_eq!(lss.status(), StatusCode::OK);
    let bytes = to_bytes(lss.into_body(), 8 * 1024 * 1024).await.unwrap();
    let parsed =
        rustsurvey_limesurvey::LssDocument::parse(&bytes, rustsurvey_limesurvey::Limits::default())
            .unwrap();
    assert_eq!(
        parsed
            .source_model()
            .unwrap()
            .survey_id
            .unwrap()
            .0
            .to_string(),
        prepared.source_survey_id
    );
}

#[tokio::test]
async fn test_imported_campaign_should_collect_resume_submit_and_export_its_frozen_source_revision()
{
    roundtrip(&fixture().await).await;
}

#[tokio::test]
async fn test_imported_campaign_should_refuse_unknown_facts_and_policy_overrides_before_creating_a_run()
 {
    let fixture = fixture().await;
    let prepared = fixture_source(&fixture).await;
    let path = format!(
        "/api/admin/studies/{}/runs",
        prepared.study["id"].as_str().unwrap()
    );
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            &path,
            Some(prepared.request.clone()),
            None,
            None
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    for change in ["facts", "policy", "navigation"] {
        let mut request = prepared.request.clone();
        match change {
            "facts" => request["sourceExecution"]["facts"]["deleteNonValues"] = json!(false),
            "policy" => request["participationPolicy"] = json!({"mode":"browserOnce"}),
            _ => request["navigation"] = json!({"format":"all","showProgress":true}),
        }
        let response = call(
            &fixture.app,
            "POST",
            &path,
            Some(request),
            Some(&prepared.cookie),
            Some(&prepared.csrf),
        )
        .await;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{change}"
        );
    }
    let runs = json(
        call(
            &fixture.app,
            "GET",
            &path,
            None,
            Some(&prepared.cookie),
            None,
        )
        .await,
    )
    .await;
    assert!(runs["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
#[ignore = "requires artifacts from the isolated LimeSurvey reference harness"]
async fn test_import_execution_reference_should_collect_actual_source_after_http_import() {
    use std::{fs::File, io::Read, path::PathBuf};
    let directory = PathBuf::from(
        std::env::var_os("RUSTSURVEY_EXECUTION_REFERENCE_DIR")
            .expect("isolated reference directory"),
    );
    assert!(
        directory.is_absolute()
            && directory
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("rustsurvey-limesurvey-reference-")
    );
    fn read(path: PathBuf) -> Vec<u8> {
        let mut bytes = Vec::new();
        File::open(path)
            .unwrap()
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .unwrap();
        assert!(bytes.len() <= 8 * 1024 * 1024);
        bytes
    }
    let fixture = fixture().await;
    let prepared = prepare(
        &fixture,
        read(directory.join("output/execution.lss")),
        serde_json::from_slice(&read(directory.join("output/execution-context.json"))).unwrap(),
        serde_json::from_slice(&read(directory.join("output/execution-facts.json"))).unwrap(),
    )
    .await;
    exercise(&fixture, prepared, false).await;
}

#[tokio::test]
async fn test_imported_campaign_should_keep_metadata_available_when_its_private_execution_snapshot_is_unreadable()
 {
    let fixture = fixture().await;
    let prepared = fixture_source(&fixture).await;
    let path = format!(
        "/api/admin/studies/{}/runs",
        prepared.study["id"].as_str().unwrap()
    );
    let run = json(
        call(
            &fixture.app,
            "POST",
            &path,
            Some(prepared.request),
            Some(&prepared.cookie),
            Some(&prepared.csrf),
        )
        .await,
    )
    .await;
    let id = run["id"].as_str().unwrap();
    fixture
        .database
        .execute(
            &Query::update()
                .table("runs")
                .value("execution_document", "private-invalid-snapshot")
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await
        .unwrap();
    let unavailable = call(
        &fixture.app,
        "GET",
        &format!("/api/runs/{id}"),
        None,
        None,
        None,
    )
    .await;
    assert_eq!(unavailable.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        !json(unavailable)
            .await
            .to_string()
            .contains("private-invalid-snapshot")
    );
    let listed = json(
        call(
            &fixture.app,
            "GET",
            &path,
            None,
            Some(&prepared.cookie),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(listed["items"].as_array().unwrap().len(), 1);
    assert!(listed["items"][0].get("executionDocument").is_none());
    let close = call(
        &fixture.app,
        "POST",
        &format!("/api/admin/runs/{id}/close"),
        Some(json!({})),
        Some(&prepared.cookie),
        Some(&prepared.csrf),
    )
    .await;
    assert_eq!(close.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_imported_campaign_should_keep_its_frozen_execution_when_old_facts_lack_clock_metadata()
 {
    let fixture = fixture().await;
    let prepared = fixture_source(&fixture).await;
    let path = format!(
        "/api/admin/studies/{}/runs",
        prepared.study["id"].as_str().unwrap()
    );
    let run = json(
        call(
            &fixture.app,
            "POST",
            &path,
            Some(prepared.request),
            Some(&prepared.cookie),
            Some(&prepared.csrf),
        )
        .await,
    )
    .await;
    let id = run["id"].as_str().unwrap();
    let before = json(
        call(
            &fixture.app,
            "GET",
            &format!("/api/runs/{id}"),
            None,
            None,
            None,
        )
        .await,
    )
    .await;
    let row = fixture
        .database
        .query_one(
            &Query::select()
                .column("execution_document")
                .from("runs")
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await
        .unwrap()
        .unwrap();
    let mut document: Value =
        serde_json::from_str(&row.try_get::<String>("", "execution_document").unwrap()).unwrap();
    document["facts"]
        .as_object_mut()
        .unwrap()
        .remove("timeZone");
    document["facts"]
        .as_object_mut()
        .unwrap()
        .remove("timeAdjust");
    fixture
        .database
        .execute(
            &Query::update()
                .table("runs")
                .value("execution_document", document.to_string())
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await
        .unwrap();
    let after = json(
        call(
            &fixture.app,
            "GET",
            &format!("/api/runs/{id}"),
            None,
            None,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(
        after, before,
        "reading an existing run must not requalify its historical source environment"
    );
}
