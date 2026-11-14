use super::*;
#[path = "../../../../domain/tests/support/filters.rs"]
mod fixture_data;

pub(super) async fn roundtrip(fixture: &Fixture) {
    let (cookie, csrf) = login(fixture).await;
    for (index, case) in fixture_data::cases()
        .into_iter()
        .filter(|case| {
            [
                "ranking selects by answer code rather than position",
                "checkbox matrix zero is unselected",
                "cascade keeps an ancestor restriction through missing intermediate codes",
                "unselected other is hidden and its comment cleared",
                "radio filter clears an unavailable choice",
                "ranking filter removes a middle choice and compacts positions",
                "filtered ranking is normalized before serving as another filter source",
            ]
            .contains(&case["name"].as_str().unwrap())
        })
        .enumerate()
    {
        let definition = fixture_data::definition(&case);
        let answers = fixture_data::answers(&case, &definition);
        let study = create(
            fixture,
            &cookie,
            &csrf,
            serde_json::to_value(definition).unwrap(),
        )
        .await;
        let response = call(
            &fixture.app,
            "POST",
            &format!("/api/admin/studies/{}/runs", study["id"].as_str().unwrap()),
            Some(json!({"expectedRevision":1})),
            Some(&cookie),
            Some(&csrf),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let run = json(response).await;
        let id = start(fixture, run["id"].as_str().unwrap(), 3900 + index as u64).await;
        let path = format!("/api/responses/{id}");
        let command = json!({"expectedVersion":0,"operationId":identifier(3950 + index as u64),"answers":answers,"checkpoint":{"language":"fr","groupId":"forms"},"submit":true});
        let response = participant(
            &fixture.app,
            "PUT",
            &path,
            Some(command.clone()),
            Some(TOKEN),
        )
        .await;
        let status = response.status();
        let receipt = json(response).await;
        assert_eq!(status, StatusCode::OK, "{}: {receipt}", case["name"]);
        let stored = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
        let target: serde_json::Map<String, Value> = stored["answers"]
            .as_object()
            .unwrap()
            .iter()
            .filter(|(key, _)| key.starts_with("TARGET"))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        assert_eq!(Value::Object(target), case["expected"], "{}", case["name"]);
        assert_eq!(stored["status"], "submitted");
        assert_eq!(
            json(participant(&fixture.app, "PUT", &path, Some(command), Some(TOKEN)).await).await,
            receipt
        );
    }
}

#[tokio::test]
async fn test_filter_collection_should_persist_typed_selections_cascades_and_other_without_stale_values()
 {
    roundtrip(&fixture().await).await;
}

#[tokio::test]
async fn test_filter_authoring_should_refuse_an_incompatible_source_edit_without_changing_the_revision()
 {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let mut definition = fixture_data::definition(&json!({"kind":"multiple_text","answers":{}}));
    let study = create(
        &fixture,
        &cookie,
        &csrf,
        serde_json::to_value(&definition).unwrap(),
    )
    .await;
    definition.groups[0].questions[0] =
        fixture_data::definition(&json!({"kind":"single_choice","answers":{}})).groups[0].questions
            [0]
        .clone();
    let path = format!("/api/admin/studies/{}", study["id"].as_str().unwrap());
    let response = call(
        &fixture.app,
        "PUT",
        &path,
        Some(json!({"expectedRevision":1,"definition":definition})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error = json(response).await;
    assert!(
        error["error"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["message"].as_str().unwrap().contains("Le filtre SRC"))
    );
    let read_path = format!("/api/studies/{}", study["id"].as_str().unwrap());
    let current = json(call(&fixture.app, "GET", &read_path, None, None, None).await).await;
    assert_eq!(current["revision"], 1);
    assert_eq!(
        current["definition"]["groups"][0]["questions"][0]["kind"],
        "multiple_text"
    );
}
