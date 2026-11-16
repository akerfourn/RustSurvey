use super::*;
#[path = "../../../domain/tests/support/defaults.rs"]
pub(super) mod fixture_data;

pub(super) async fn roundtrip(fixture: &Fixture) {
    let (cookie, csrf) = login(fixture).await;
    let definition =
        serde_json::to_value(fixture_data::definition(&fixture_data::cases()[0])).unwrap();
    let stored = create(fixture, &cookie, &csrf, definition.clone()).await;
    let id = stored["id"].as_str().unwrap();
    assert_eq!(stored["definition"]["schemaVersion"], 4);
    let original = &definition["groups"][0]["questions"][0]["defaults"];
    assert_eq!(
        &stored["definition"]["groups"][0]["questions"][0]["defaults"],
        original
    );
    let mut updated = definition;
    updated["groups"][0]["questions"][0]["code"] = json!("RENAMED");
    let response = call(
        &fixture.app,
        "PUT",
        &format!("/api/admin/studies/{id}"),
        Some(json!({"expectedRevision":1,"definition":updated})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let current = json(response).await;
    assert_eq!(current["revision"], 2);
    let response = call(
        &fixture.app,
        "GET",
        &format!("/api/admin/studies/{id}/revisions/2/definition.lss"),
        None,
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let parsed =
        rustsurvey_limesurvey::LssDocument::parse(&bytes, rustsurvey_limesurvey::Limits::default())
            .unwrap();
    assert!(!parsed.report().has_errors());
    let model = parsed.source_model().unwrap();
    let question = model
        .questions
        .iter()
        .find(|question| question.code == "RENAMED")
        .unwrap();
    assert_eq!(model.default_values[0].question_id, question.id);
    let response = call(
        &fixture.app,
        "POST",
        &format!("/api/admin/studies/{id}/runs"),
        Some(json!({"expectedRevision":2})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let run = json(response).await;
    assert_eq!(run["profile"], "native-defaults-v1");
    assert_eq!(run["navigation"]["format"], "group");
    let previous = json(
        call(
            &fixture.app,
            "GET",
            &format!("/api/admin/studies/{id}/revisions/1"),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(
        previous["definition"]["groups"][0]["questions"][0]["code"],
        "S"
    );
    imported_roundtrip(fixture).await;
}

async fn imported_roundtrip(fixture: &Fixture) {
    let (cookie, csrf) = login(fixture).await;
    let definition = fixture_data::definition(&fixture_data::cases()[0]);
    let xml = rustsurvey_limesurvey::export_native(
        &definition,
        rustsurvey_limesurvey::SourceId(701),
        rustsurvey_limesurvey::Limits::default(),
    )
    .unwrap()
    .xml;
    let response = import_request(fixture, xml.clone(), Some(&cookie), Some(&csrf)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let imported = json(response).await;
    assert_eq!(
        imported["source"]["report"]["defaultProjection"]["status"],
        "complete"
    );
    assert_eq!(
        imported["source"]["report"]["defaultProjection"]["projectedValues"],
        2
    );
    let id = imported["id"].as_str().unwrap();
    let mut native: rustsurvey_domain::Definition =
        serde_json::from_value(imported["definition"].clone()).unwrap();
    let imported_equation = native.groups[0]
        .questions
        .iter()
        .find(|question| question.code == "CALC")
        .unwrap();
    let native_equation = definition.groups[0]
        .questions
        .iter()
        .find(|question| question.code == "CALC")
        .unwrap();
    assert_eq!(
        imported_equation.attributes[0].value,
        native_equation.attributes[0].value
    );
    // Imported themes remain preserved and unqualified; isolate default initialization here.
    assert!(rustsurvey_domain::collection::Questionnaire::compile_defaults(&native).is_err());
    for question in native
        .groups
        .iter_mut()
        .flat_map(|group| &mut group.questions)
    {
        question.theme = "core".into();
    }
    let runtime = rustsurvey_domain::collection::Questionnaire::compile_defaults(&native).unwrap();
    let questions = native
        .groups
        .iter()
        .flat_map(|group| &group.questions)
        .map(|question| question.id.clone())
        .collect();
    let initialized = runtime
        .evaluate_initialized(&Default::default(), false, "fr", &questions)
        .unwrap();
    assert_eq!(initialized.answers.get("S").unwrap(), "bonjour");
    let explicit = [("S".to_string(), String::new())].into_iter().collect();
    let initialized = runtime
        .evaluate_initialized(&explicit, false, "en", &questions)
        .unwrap();
    assert_eq!(initialized.answers.get("S").unwrap(), "");
    let reloaded = json(
        call(
            &fixture.app,
            "GET",
            &format!("/api/admin/studies/{id}"),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(reloaded["definition"], imported["definition"]);
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
        to_bytes(original.into_body(), 8 * 1024 * 1024)
            .await
            .unwrap()
            .as_ref(),
        xml.as_slice()
    );
    let campaign = call(
        &fixture.app,
        "POST",
        &format!("/api/admin/studies/{id}/runs"),
        Some(json!({"expectedRevision":1})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(campaign.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        json(campaign).await["error"]["code"],
        "imported_collection_unavailable"
    );
}

#[tokio::test]
async fn test_imported_defaults_should_preserve_unsupported_source_without_installing_partial_defaults()
 {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let definition = fixture_data::definition(&fixture_data::cases()[0]);
    let exported = rustsurvey_limesurvey::export_native(
        &definition,
        rustsurvey_limesurvey::SourceId(701),
        rustsurvey_limesurvey::Limits::default(),
    )
    .unwrap();
    let parsed = rustsurvey_limesurvey::LssDocument::parse(
        &exported.xml,
        rustsurvey_limesurvey::Limits::default(),
    )
    .unwrap();
    let question = parsed
        .source_model()
        .unwrap()
        .questions
        .iter()
        .find(|question| question.code == "S")
        .unwrap();
    let mut xml = String::from_utf8(exported.xml).unwrap();
    let section = xml.find("<defaultvalues>").unwrap();
    let position = section + xml[section..].find("<rows>").unwrap() + "<rows>".len();
    // Keep the valid translations and add an unsupported target on the same question.
    let invalid = format!(
        "<row><dvid>9999</dvid><qid>{}</qid><sqid>0</sqid><scale_id>0</scale_id><specialtype>source-private-marker</specialtype></row>",
        question.id.0
    );
    xml.insert_str(position, &invalid);
    let response = import_request(&fixture, xml.into_bytes(), Some(&cookie), Some(&csrf)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let imported = json(response).await;
    let report = &imported["source"]["report"]["defaultProjection"];
    assert_eq!(report["status"], "partial");
    assert_eq!(report["projectedValues"], 0);
    assert!(!report.to_string().contains("source-private-marker"));
    let questions = imported["definition"]["groups"][0]["questions"]
        .as_array()
        .unwrap();
    assert_eq!(
        questions
            .iter()
            .find(|question| question["code"] == "S")
            .unwrap()["defaults"],
        Value::Null
    );
}

#[tokio::test]
async fn test_defaults_should_persist_export_and_select_the_versioned_collection_profile() {
    roundtrip(&fixture().await).await;
}
