use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use rustsurvey_server::inspection::{MAX_UPLOAD_BYTES, routes};
use serde_json::Value;
use tower::ServiceExt;

const DEMO: &str = include_str!("../../../examples/surveys/demo.lss");

async fn inspect(source: impl Into<Body>, content_type: &str) -> axum::response::Response {
    routes()
        .oneshot(
            Request::post("/api/surveys/inspect")
                .header(header::CONTENT_TYPE, content_type)
                .body(source.into())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), 16 * 1024 * 1024)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn test_inspect_should_return_localized_questions_and_rules_when_upload_is_valid() {
    let response = inspect(DEMO, "application/xml; charset=utf-8").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body = json(response).await;
    assert_eq!(body["metadata"]["surveyId"], "900001");
    assert_eq!(
        body["metadata"]["titles"][0]["value"],
        "Découvrir RustSurvey"
    );
    assert_eq!(body["report"]["sourceSchema"], 623);
    assert_eq!(body["report"]["errors"], 0, "{body}");
    assert_eq!(body["study"]["groups"][1]["relevance"], "STACK == 'rust'");
    assert_eq!(body["study"]["questions"].as_array().unwrap().len(), 12);
    assert_eq!(body["study"]["questions"][0]["id"], "101");
    assert_eq!(body["study"]["questions"][0]["mandatory"], "strict");
    assert_eq!(body["study"]["subquestions"].as_array().unwrap().len(), 11);
    assert_eq!(body["study"]["answers"].as_array().unwrap().len(), 9);
    assert_eq!(body["study"]["showWelcome"], true);
    assert!(
        body["study"]["messages"][0]["welcome"]
            .as_str()
            .unwrap()
            .contains("tableaux")
    );
    assert!(
        body["study"]["messages"][1]["end"]
            .as_str()
            .unwrap()
            .contains("{PROJECT}")
    );
    assert_eq!(body["study"]["answers"][0]["labels"][0]["text"], "Rust");
}

#[tokio::test]
async fn test_inspect_should_reject_malformed_xml_when_upload_cannot_be_parsed() {
    let response = inspect("<document>", "application/xml").await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body = json(response).await;
    assert_eq!(body["error"]["code"], "invalid_lss");
    assert!(body["error"]["message"].as_str().unwrap().contains("XML"));
}

#[tokio::test]
async fn test_inspect_should_reject_upload_when_body_exceeds_the_limit() {
    let response = inspect(vec![b' '; MAX_UPLOAD_BYTES + 1], "application/xml").await;

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(json(response).await["error"]["code"], "upload_too_large");
}

#[tokio::test]
async fn test_inspect_should_reject_media_type_when_upload_is_not_xml() {
    let response = inspect(DEMO, "multipart/form-data").await;

    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(
        json(response).await["error"]["code"],
        "unsupported_media_type"
    );
}

#[tokio::test]
async fn test_inspect_should_keep_metadata_when_schema_has_no_viewer_projection() {
    let source = DEMO.replace("<DBVersion>623</DBVersion>", "<DBVersion>617</DBVersion>");
    let response = inspect(source, "application/xml").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert!(body["study"].is_null());
    assert_eq!(body["metadata"]["languages"][1], "en");
    assert_eq!(body["metadata"]["surveyId"], "900001");
    assert_eq!(body["report"]["sourceSchema"], 617);
    assert!(
        body["report"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["code"] == "unsupported_schema")
    );
}

#[tokio::test]
async fn test_inspect_should_report_structural_errors_when_question_reference_is_invalid() {
    let source = DEMO.replacen("<gid>10</gid>", "<gid>999</gid>", 1);
    let response = inspect(source, "application/xml").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert!(body["report"]["errors"].as_u64().unwrap() > 0);
    assert!(body["study"].is_object());
}

#[tokio::test]
async fn test_example_should_download_the_authored_fixture_when_requested() {
    let response = routes()
        .oneshot(
            Request::get("/api/surveys/example.lss")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "application/xml; charset=utf-8"
    );
    assert_eq!(
        to_bytes(response.into_body(), MAX_UPLOAD_BYTES)
            .await
            .unwrap()
            .as_ref(),
        DEMO.as_bytes()
    );
}

#[tokio::test]
async fn test_controls_should_download_and_inspect_all_non_file_question_types() {
    let expected = include_bytes!("../../../examples/surveys/controls.lss");
    let response = routes()
        .oneshot(
            Request::get("/api/surveys/controls.lss")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), MAX_UPLOAD_BYTES)
        .await
        .unwrap();
    assert_eq!(body.as_ref(), expected);

    let response = inspect(expected.as_slice(), "application/xml").await;
    let result = json(response).await;
    assert_eq!(result["report"]["errors"], 0, "{}", result["report"]);
    assert_eq!(result["study"]["questions"].as_array().unwrap().len(), 28);
    assert_eq!(result["study"]["questions"][0]["theme"], "core");
}
