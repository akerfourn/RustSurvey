use std::{error::Error as _, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rustsurvey_limesurvey::{
    Diagnostic, Limits, LssDocument, Report, SectionSummary, SourceMetadata, SurveySource,
};
use serde::Serialize;
use tokio::sync::Semaphore;

pub const MAX_UPLOAD_BYTES: usize = 8 * 1024 * 1024;
const PARALLEL_INSPECTIONS: usize = 2;
const BODY_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone)]
struct InspectionState {
    slots: Arc<Semaphore>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Inspection<'a> {
    metadata: SourceMetadata,
    study: Option<&'a SurveySource>,
    report: InspectionReport<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectionReport<'a> {
    source_schema: u32,
    target_schema: u32,
    errors: usize,
    warnings: usize,
    omitted_diagnostics: usize,
    checked_sections: &'a [&'static str],
    sections: &'a [SectionSummary],
    diagnostics: &'a [Diagnostic],
}

impl<'a> From<&'a Report> for InspectionReport<'a> {
    fn from(report: &'a Report) -> Self {
        Self {
            source_schema: report.source_schema,
            target_schema: report.target_schema,
            errors: report.errors,
            warnings: report.warnings,
            omitted_diagnostics: report.omitted_diagnostics,
            checked_sections: &report.checked_sections,
            sections: &report.sections,
            diagnostics: &report.diagnostics,
        }
    }
}

#[derive(Serialize)]
struct ApiErrorBody {
    error: ApiErrorDetails,
}

#[derive(Serialize)]
struct ApiErrorDetails {
    code: &'static str,
    message: String,
}

struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = (
            self.status,
            Json(ApiErrorBody {
                error: ApiErrorDetails {
                    code: self.code,
                    message: self.message,
                },
            }),
        )
            .into_response();
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        if self.status == StatusCode::SERVICE_UNAVAILABLE {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
        }
        response
    }
}

pub fn routes() -> Router {
    Router::new()
        .route("/api/surveys/inspect", post(inspect))
        .route("/api/surveys/example.lss", get(example))
        .with_state(InspectionState {
            slots: Arc::new(Semaphore::new(PARALLEL_INSPECTIONS)),
        })
}

async fn inspect(
    State(state): State<InspectionState>,
    request: Request,
) -> Result<Response, ApiError> {
    let content_type = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim();
    if !["application/xml", "text/xml", "application/octet-stream"].contains(&content_type) {
        return Err(ApiError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_media_type",
            "send the LSS file as an XML request body",
        ));
    }
    let permit = state.slots.try_acquire_owned().map_err(|_| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "inspection_busy",
            "all inspection slots are busy; retry later",
        )
    })?;
    let body = tokio::time::timeout(
        BODY_TIMEOUT,
        to_bytes(request.into_body(), MAX_UPLOAD_BYTES),
    )
    .await
    .map_err(|_| {
        ApiError::new(
            StatusCode::REQUEST_TIMEOUT,
            "upload_timeout",
            "upload timed out",
        )
    })?
    .map_err(|error| {
        if error
            .source()
            .is_some_and(|source| source.is::<http_body_util::LengthLimitError>())
        {
            ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "upload_too_large",
                "the LSS file exceeds 8 MiB",
            )
        } else {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "invalid_body",
                "the upload could not be read",
            )
        }
    })?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let document = LssDocument::parse(&body, Limits::default()).map_err(|error| {
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_lss",
                error.to_string(),
            )
        })?;
        let result = Inspection {
            metadata: document.metadata(),
            study: document.source_model(),
            report: document.report().into(),
        };
        let json = serde_json::to_vec(&result).map_err(|_| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "serialization_failed",
                "inspection response could not be generated",
            )
        })?;
        Ok((
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            Body::from(json),
        )
            .into_response())
    })
    .await
    .map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "inspection_failed",
            "inspection did not complete",
        )
    })?
}

async fn example() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "application/xml; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=rustsurvey-demo.lss",
            ),
        ],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/surveys/demo.lss"
        ))
        .as_slice(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_inspect_should_reject_immediately_when_all_slots_are_occupied() {
        let slots = Arc::new(Semaphore::new(1));
        let _permit = slots.try_acquire().unwrap();
        let router = Router::new()
            .route("/api/surveys/inspect", post(inspect))
            .with_state(InspectionState {
                slots: slots.clone(),
            });

        let response = router
            .oneshot(
                Request::post("/api/surveys/inspect")
                    .header(header::CONTENT_TYPE, "application/xml")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.headers()[header::RETRY_AFTER], "1");
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"]["code"], "inspection_busy");
    }
}
