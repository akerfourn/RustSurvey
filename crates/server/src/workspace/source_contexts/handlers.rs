use super::super::{
    AppError, WorkspaceState, auth, history, media_body::ObjectBody, repository as studies, runtime,
};
use super::{
    repository::{self, Metadata, StoredContext},
    valid_id,
};
use axum::{
    Json,
    body::{Body, to_bytes},
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use rustsurvey_limesurvey::{
    GROUP_SETTINGS_PROFILE, Limits, LssDocument, MAX_SETTINGS_CONTEXT_BYTES, SettingsContext,
};
use sha2::{Digest, Sha256};
use std::error::Error as _;

pub(super) async fn upload(
    State(state): State<WorkspaceState>,
    Path((study, revision)): Path<(String, i64)>,
    request: Request,
) -> Result<Response, AppError> {
    auth::authorize_write(&state, request.headers()).await?;
    valid_id(&study)?;
    let content_type = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim();
    if content_type != "application/json" {
        return Err(AppError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "json_required",
            "Envoyez le contexte source avec le type application/json.",
        ));
    }
    let permit = state
        .imports
        .clone()
        .try_acquire_owned()
        .map_err(|_| busy())?;
    let bytes = to_bytes(request.into_body(), MAX_SETTINGS_CONTEXT_BYTES)
        .await
        .map_err(|error| {
            if error
                .source()
                .is_some_and(|source| source.is::<http_body_util::LengthLimitError>())
            {
                AppError::new(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "settings_context_too_large",
                    "Contexte source limité à 64 Kio.",
                )
            } else {
                super::invalid()
            }
        })?;
    history::load(&state, &study, revision).await?;
    let source = studies::import_info(&state.database, &study)
        .await?
        .ok_or_else(AppError::not_found)?;
    let original = studies::original(&state.database, &study).await?;
    let id = runtime::identifier(state.runtime.as_ref())?;
    let now = state.runtime.now()?;
    let context = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        prepare(
            study,
            revision,
            id,
            now,
            bytes.to_vec(),
            source.sha256,
            original,
        )
    })
    .await
    .map_err(|_| AppError::internal())??;
    let (metadata, created) = repository::insert(&state, context).await?;
    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(metadata)).into_response())
}

fn prepare(
    study: String,
    revision: i64,
    id: String,
    now: i64,
    bytes: Vec<u8>,
    source_sha256: String,
    original: String,
) -> Result<StoredContext, AppError> {
    let context: SettingsContext = serde_json::from_slice(&bytes).map_err(|_| super::invalid())?;
    if runtime::hex(&Sha256::digest(original.as_bytes())) != source_sha256 {
        return Err(AppError::internal());
    }
    let source = LssDocument::parse(original.as_bytes(), Limits::default())
        .map_err(|_| AppError::internal())?;
    let report = source
        .group_settings(Some(&context))
        .map_err(|_| super::invalid())?;
    Ok(StoredContext {
        metadata: Metadata {
            id,
            study_id: study,
            revision,
            source_sha256,
            sha256: runtime::hex(&Sha256::digest(&bytes)),
            size: bytes.len() as i64,
            profile: GROUP_SETTINGS_PROFILE.into(),
            complete: report.complete,
            created_at: now,
        },
        document: String::from_utf8(bytes).map_err(|_| super::invalid())?,
        report: serde_json::to_string(&report).map_err(|_| AppError::internal())?,
    })
}

pub(super) async fn list(
    State(state): State<WorkspaceState>,
    Path((study, revision)): Path<(String, i64)>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::authorize_read(&state, &headers).await?;
    valid_id(&study)?;
    history::load(&state, &study, revision).await?;
    studies::import_info(&state.database, &study)
        .await?
        .ok_or_else(AppError::not_found)?;
    Ok(Json(
        serde_json::json!({"items":repository::list(&state, &study, revision).await?}),
    ))
}

pub(super) async fn get(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Metadata>, AppError> {
    auth::authorize_read(&state, &headers).await?;
    valid_id(&id)?;
    Ok(Json(repository::get(&state, &id).await?))
}

pub(super) async fn original(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    auth::authorize_read(&state, &headers).await?;
    valid_id(&id)?;
    let permit = state
        .object_reads
        .clone()
        .try_acquire_owned()
        .map_err(|_| busy())?;
    let metadata = repository::get(&state, &id).await?;
    let stored = repository::selected(&state, &metadata.study_id, metadata.revision, &id).await?;
    stored.verify_bytes()?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"source-context-{id}.json\"").as_str(),
            ),
        ],
        Body::new(ObjectBody::new(stored.document.into_bytes(), permit)),
    )
        .into_response())
}

fn busy() -> AppError {
    AppError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "settings_context_busy",
        "Les traitements de contexte source sont occupés. Réessayez.",
    )
}
