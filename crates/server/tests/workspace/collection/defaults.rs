use super::*;
use crate::default_tests::fixture_data;

pub(super) async fn roundtrip(fixture: &Fixture) {
    let (cookie, csrf) = login(fixture).await;
    let mut definition = fixture_data::definition(&fixture_data::cases()[0]);
    definition.groups[0]
        .questions
        .retain(|question| ["S", "T"].contains(&question.code.as_str()));
    let target = &mut definition.groups[0].questions[1];
    target.defaults = definition_default("depuis {S}");
    let group = definition.groups[0].id.clone();
    let first = format!("q:{}", definition.groups[0].questions[0].id);
    let second = format!("q:{}", definition.groups[0].questions[1].id);
    let study = create(
        fixture,
        &cookie,
        &csrf,
        serde_json::to_value(&definition).unwrap(),
    )
    .await;
    let study_id = study["id"].as_str().unwrap();
    let response = call(
        &fixture.app,
        "POST",
        &format!("/api/admin/studies/{study_id}/runs"),
        Some(json!({"expectedRevision":1,"navigation":{"format":"question","showProgress":true}})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let run = json(response).await;
    assert_eq!(run["profile"], "native-defaults-v1");
    let id = start(fixture, run["id"].as_str().unwrap(), 9001).await;
    let path = format!("/api/responses/{id}");
    let initial = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    assert_eq!(initial["answers"], json!({"S":"bonjour"}));
    assert_eq!(initial["checkpoint"]["pageId"], first);
    let command = json!({"expectedVersion":0,"operationId":identifier(9002),"answers":{"S":"modifié"},"checkpoint":{"language":"fr","groupId":group,"pageId":second},"submit":false});
    let response = participant(
        &fixture.app,
        "PUT",
        &path,
        Some(command.clone()),
        Some(TOKEN),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let receipt = json(response).await;
    assert_eq!(
        json(participant(&fixture.app, "PUT", &path, Some(command), Some(TOKEN)).await).await,
        receipt
    );
    let saved = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    assert_eq!(
        saved["answers"],
        json!({"S":"modifié","T":"depuis modifié"})
    );
    let clear = json!({"expectedVersion":1,"operationId":identifier(9003),"answers":{"S":"modifié","T":""},"checkpoint":{"language":"en","groupId":group,"pageId":second},"submit":false});
    assert_eq!(
        participant(&fixture.app, "PUT", &path, Some(clear), Some(TOKEN))
            .await
            .status(),
        StatusCode::OK
    );
    let reopened = routes(WorkspaceState::with_runtime(
        fixture.database.clone(),
        false,
        fixture.runtime.clone(),
    ));
    let resumed = json(participant(&reopened, "GET", &path, None, Some(TOKEN)).await).await;
    assert_eq!(resumed["answers"], json!({"S":"modifié","T":""}));
    assert_eq!(resumed["checkpoint"]["language"], "en");
    let malformed = json!({"expectedVersion":2,"operationId":identifier(9004),"answers":{"S":"modifié","T":""},"checkpoint":{"language":"en","groupId":group,"pageId":"unknown"},"submit":false});
    let response = participant(&reopened, "PUT", &path, Some(malformed), Some(TOKEN)).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json(response).await["error"]["code"], "invalid_checkpoint");
    assert_eq!(
        json(participant(&reopened, "GET", &path, None, Some(TOKEN)).await).await["version"],
        2
    );
    dynamic_default_should_be_atomic(fixture, &cookie, &csrf).await;
}

async fn dynamic_default_should_be_atomic(fixture: &Fixture, cookie: &str, csrf: &str) {
    let mut definition = fixture_data::definition(&fixture_data::cases()[0]);
    definition.groups[0]
        .questions
        .retain(|question| ["S", "N"].contains(&question.code.as_str()));
    definition.groups[0].questions[0].defaults = None;
    definition.groups[0].questions[1].defaults = definition_default("{1 / S}");
    let group = definition.groups[0].id.clone();
    let target = format!("q:{}", definition.groups[0].questions[1].id);
    let study = create(
        fixture,
        cookie,
        csrf,
        serde_json::to_value(definition).unwrap(),
    )
    .await;
    let run = json(call(&fixture.app, "POST", &format!("/api/admin/studies/{}/runs", study["id"].as_str().unwrap()), Some(json!({"expectedRevision":1,"navigation":{"format":"question","showProgress":true}})), Some(cookie), Some(csrf)).await).await;
    let id = start(fixture, run["id"].as_str().unwrap(), 9010).await;
    let path = format!("/api/responses/{id}");
    let before = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    let mut command = json!({"expectedVersion":0,"operationId":identifier(9011),"answers":{"S":"0"},"checkpoint":{"language":"fr","groupId":group,"pageId":target},"submit":false});
    let result = participant(
        &fixture.app,
        "PUT",
        &path,
        Some(command.clone()),
        Some(TOKEN),
    )
    .await;
    assert_eq!(result.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json(result).await["error"]["code"], "invalid_answers");
    assert_eq!(
        json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await,
        before
    );
    command["answers"] = json!({"S":"2"});
    command["operationId"] = json!(identifier(9012));
    assert_eq!(
        participant(&fixture.app, "PUT", &path, Some(command), Some(TOKEN))
            .await
            .status(),
        StatusCode::OK
    );
    let saved = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    assert_eq!(saved["answers"], json!({"S":"2","N":"0.5"}));
    assert_eq!(saved["version"], 1);
}

fn definition_default(value: &str) -> Option<rustsurvey_domain::defaults::QuestionDefaults> {
    use rustsurvey_domain::{
        cells::{Coordinate, Role},
        defaults::{DefaultValue, QuestionDefaults},
    };
    Some(QuestionDefaults {
        same_for_all_languages: true,
        values: vec![DefaultValue {
            coordinate: Coordinate::Primary,
            role: Role::Answer,
            language: "fr".into(),
            value: value.into(),
        }],
    })
}

#[tokio::test]
async fn test_default_collection_should_initialize_pages_once_and_keep_clearing_after_restart() {
    roundtrip(&fixture().await).await;
}
