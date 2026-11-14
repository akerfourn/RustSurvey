use super::super::{AppError, WorkspaceState, auth, media_body::ObjectBody};
use super::handlers::{unavailable, valid_id};
use super::repository;
use axum::{
    Json,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use rustsurvey_object_store::{ObjectKey, StoreError};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::OwnedSemaphorePermit;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Page {
    #[serde(default)]
    from: i64,
}

pub(super) async fn list(
    State(state): State<WorkspaceState>,
    Path(study): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    auth::authorize_read(&state, &headers).await?;
    valid_id(&study)?;
    Ok(Json(
        json!({"items":repository::list(&state, &study).await?}),
    ))
}

pub(super) async fn get(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    auth::authorize_read(&state, &headers).await?;
    valid_id(&id)?;
    Ok(Json(repository::metadata(&state, &id).await?.summary()?))
}

pub(super) async fn records(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    Query(page): Query<Page>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    auth::authorize_read(&state, &headers).await?;
    valid_id(&id)?;
    repository::metadata(&state, &id).await?;
    if page.from < 0 {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "invalid_page",
            "Position de ligne invalide.",
        ));
    }
    let mut items = repository::records(&state, &id, page.from).await?;
    let has_more = items.len() > 20;
    items.truncate(20);
    Ok(Json(json!({"items":items,"hasMore":has_more})))
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
        .map_err(|_| unavailable(StoreError::Capacity))?;
    let batch = repository::metadata(&state, &id).await?;
    let key = ObjectKey::parse(&batch.object_key).map_err(unavailable)?;
    let stored = state.objects.get(&key, None).await.map_err(unavailable)?;
    if stored.metadata.sha256 != batch.sha256 || stored.metadata.size as i64 != batch.size {
        return Err(unavailable(StoreError::Conflict));
    }
    attachment(
        format!("import-{id}.lsr"),
        "application/xml; charset=utf-8",
        stored.bytes,
        permit,
    )
}

pub(super) fn attachment(
    name: String,
    content_type: &'static str,
    bytes: Vec<u8>,
    permit: OwnedSemaphorePermit,
) -> Result<Response, AppError> {
    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\"").as_str(),
            ),
        ],
        Body::new(ObjectBody::new(bytes, permit)),
    )
        .into_response())
}

pub(super) async fn schema(
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
        .map_err(|_| unavailable(StoreError::Capacity))?;
    let batch = repository::get(&state, &id).await?;
    attachment(
        format!("import-{id}-mapping.json"),
        "application/json; charset=utf-8",
        batch.schema.into_bytes(),
        permit,
    )
}

pub(super) async fn record(
    State(state): State<WorkspaceState>,
    Path((id, ordinal)): Path<(String, i64)>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    auth::authorize_read(&state, &headers).await?;
    valid_id(&id)?;
    let permit = state
        .object_reads
        .clone()
        .try_acquire_owned()
        .map_err(|_| unavailable(StoreError::Capacity))?;
    let batch = repository::metadata(&state, &id).await?;
    if ordinal < 0 || ordinal >= batch.loaded_rows {
        return Err(AppError::not_found());
    }
    let record = repository::record(&state, &id, ordinal).await?;
    attachment(
        format!("import-{id}-row-{ordinal}.json"),
        "application/json; charset=utf-8",
        record.document.into_bytes(),
        permit,
    )
}
