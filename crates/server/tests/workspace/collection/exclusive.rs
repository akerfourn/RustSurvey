use super::*;

pub(super) async fn roundtrip(fixture: &Fixture) {
    let (cookie, csrf) = login(fixture).await;
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../../examples/rules/collection-forms.json"
    ))
    .unwrap();
    let mut definition = corpus["definition"].clone();
    let mut questions = Vec::new();
    for code in ["M", "P"] {
        let mut question = definition["groups"][0]["questions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|question| question["code"] == code)
            .unwrap()
            .clone();
        question["relevance"] = json!("1");
        question["other"] = json!(false);
        question["mandatory"] = json!("strict");
        for row in question["subquestions"].as_array_mut().unwrap() {
            row["relevance"] = json!("1");
        }
        if code == "M" {
            let mut third = question["subquestions"][1].clone();
            third["id"] = json!("exclusive-third-row");
            third["code"] = json!("A3");
            question["subquestions"].as_array_mut().unwrap().push(third);
            question["attributes"] = json!([{"name":"exclude_all_others","value":"A2;A3"}]);
        } else {
            question["attributes"] = json!([{"name":"exclude_all_others","value":"A2"},{"name":"exclude_all_others_auto","value":"1"}]);
        }
        questions.push(question);
    }
    definition["groups"][0]["questions"] = json!(questions);
    let study = create(fixture, &cookie, &csrf, definition).await;
    let run = call(
        &fixture.app,
        "POST",
        &format!("/api/admin/studies/{}/runs", study["id"].as_str().unwrap()),
        Some(json!({"expectedRevision":1,"navigation":{"format":"group","showProgress":true}})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(run.status(), StatusCode::CREATED);
    let run = json(run).await;
    let id = start(fixture, run["id"].as_str().unwrap(), 3800).await;
    let path = format!("/api/responses/{id}");
    let mut command = json!({"expectedVersion":0,"operationId":identifier(3801),"answers":{"M_A1":"Y","M_A2":"Y","M_A3":"Y","P_A1":"Y","P_A1comment":"old"},"checkpoint":{"language":"fr","groupId":"forms","pageId":"g:forms"},"submit":true});
    let invalid = participant(
        &fixture.app,
        "PUT",
        &path,
        Some(command.clone()),
        Some(TOKEN),
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        json(invalid).await["error"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["message"]
                .as_str()
                .unwrap()
                .contains("Plusieurs lignes exclusives"))
    );
    command["answers"].as_object_mut().unwrap().remove("M_A3");
    let accepted = participant(
        &fixture.app,
        "PUT",
        &path,
        Some(command.clone()),
        Some(TOKEN),
    )
    .await;
    assert_eq!(accepted.status(), StatusCode::OK);
    let receipt = json(accepted).await;
    let stored = json(participant(&fixture.app, "GET", &path, None, Some(TOKEN)).await).await;
    assert_eq!(stored["version"], 1);
    assert_eq!(stored["answers"], json!({"M_A2":"Y","P_A2":"Y"}));
    assert_eq!(
        json(participant(&fixture.app, "PUT", &path, Some(command), Some(TOKEN)).await).await,
        receipt
    );
}

#[tokio::test]
async fn test_exclusive_collection_should_normalize_answers_and_reject_ambiguous_input_atomically()
{
    roundtrip(&fixture().await).await;
}
