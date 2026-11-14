use super::super::{AppError, WorkspaceState, auth, history, repository as studies, runtime};
use super::{
    prepare::{self, Plan},
    repository::{self, Batch},
};
use axum::{
    Json,
    body::to_bytes,
    extract::{Path, Query, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use rustsurvey_object_store::{MAX_OBJECT_BYTES, ObjectKey, ObjectMetadata, StoreError};
use serde::Deserialize;
use serde_json::Value;
use std::error::Error as _;
use std::sync::Arc;
use tokio::sync::OwnedSemaphorePermit;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Target {
    revision: i64,
}

pub(super) fn valid_id(id: &str) -> Result<(), AppError> {
    if !uuid::Uuid::try_parse(id).is_ok_and(|parsed| parsed.to_string() == id) {
        return Err(AppError::not_found());
    }
    Ok(())
}

fn slot(state: &WorkspaceState) -> Result<Arc<OwnedSemaphorePermit>, AppError> {
    state
        .imports
        .clone()
        .try_acquire_owned()
        .map(Arc::new)
        .map_err(|_| {
            AppError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "import_busy",
                "Deux traitements d'import/export sont déjà en cours.",
            )
        })
}

pub(super) fn unavailable(_: StoreError) -> AppError {
    AppError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "response_import_object_unavailable",
        "Le fichier source privé n'est pas disponible ou son intégrité n'est pas vérifiée.",
    )
}

fn changed() -> AppError {
    AppError::new(
        StatusCode::CONFLICT,
        "response_import_mapping_changed",
        "Les correspondances de cet import ont changé. Les lignes déjà conservées restent intactes.",
    )
}

async fn plan(
    state: &WorkspaceState,
    study: &str,
    revision: i64,
    bytes: Vec<u8>,
    permit: Arc<OwnedSemaphorePermit>,
) -> Result<(Plan, Vec<u8>, ObjectMetadata, String), AppError> {
    let info = studies::import_info(&state.database, study)
        .await?
        .ok_or_else(|| {
            AppError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "response_import_requires_lss",
                "Importez d'abord la structure LSS de l'étude.",
            )
        })?;
    let definition = history::load(state, study, revision).await?.definition;
    let original = studies::original(&state.database, study).await?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let metadata = ObjectMetadata::for_bytes(&bytes).map_err(unavailable)?;
        let actual = ObjectMetadata::for_bytes(original.as_bytes()).map_err(unavailable)?;
        if actual.sha256 != info.sha256 {
            return Err(AppError::internal());
        }
        let prepared = prepare::prepare(definition, original, &bytes)?;
        Ok((prepared, bytes, metadata, info.sha256))
    })
    .await
    .map_err(|_| AppError::internal())?
}

async fn save_original(
    state: &WorkspaceState,
    batch: &Batch,
    bytes: Vec<u8>,
    metadata: &ObjectMetadata,
) -> Result<(), AppError> {
    let key = ObjectKey::parse(&batch.object_key).map_err(unavailable)?;
    match state.objects.put(&key, bytes).await {
        Ok(stored) if stored == *metadata => Ok(()),
        Err(StoreError::Conflict) => {
            let stored = state.objects.head(&key).await.map_err(unavailable)?;
            if stored == *metadata {
                return Ok(());
            }
            Err(unavailable(StoreError::Conflict))
        }
        _ => Err(unavailable(StoreError::Unavailable)),
    }
}

async fn load_rows(
    state: &WorkspaceState,
    batch: Batch,
    prepared: Plan,
) -> Result<Batch, AppError> {
    if batch.schema != prepared.schema || batch.row_count != prepared.rows.len() as i64 {
        return Err(changed());
    }
    if batch.state == "prepared" {
        return Ok(batch);
    }
    let token = runtime::identifier(state.runtime.as_ref())?;
    let Some(loaded) = repository::claim(state, &batch.id, &token, state.runtime.now()?).await?
    else {
        let latest = repository::get(state, &batch.id).await?;
        if latest.state == "prepared" {
            return Ok(latest);
        }
        return Err(AppError::new(
            StatusCode::CONFLICT,
            "response_import_retry_limit",
            "La limite de tentatives de cet import est atteinte.",
        ));
    };
    let from = usize::try_from(loaded).map_err(|_| AppError::internal())?;
    if from > prepared.rows.len() {
        return Err(AppError::internal());
    }
    let until = from.saturating_add(500).min(prepared.rows.len());
    for chunk in prepared.rows[from..until].chunks(50) {
        repository::append(state, &batch.id, &token, chunk, state.runtime.now()?).await?;
    }
    if until == prepared.rows.len() {
        repository::finish(state, &batch.id, &token, state.runtime.now()?).await?;
    }
    repository::get(state, &batch.id).await
}

pub(super) async fn upload(
    State(state): State<WorkspaceState>,
    Path(study): Path<String>,
    Query(target): Query<Target>,
    request: Request,
) -> Result<Response, AppError> {
    auth::authorize_write(&state, request.headers()).await?;
    valid_id(&study)?;
    if target.revision < 1 {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "invalid_revision",
            "Révision enregistrée invalide.",
        ));
    }
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
        return Err(AppError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "invalid_media_type",
            "Envoyez le fichier LSR comme corps XML.",
        ));
    }
    let permit = slot(&state)?;
    let bytes = to_bytes(request.into_body(), MAX_OBJECT_BYTES)
        .await
        .map_err(|error| {
            if error
                .source()
                .is_some_and(|source| source.is::<http_body_util::LengthLimitError>())
            {
                return AppError::new(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "response_import_too_large",
                    "Import LSR limité à 8 Mio.",
                );
            }
            AppError::new(
                StatusCode::BAD_REQUEST,
                "invalid_body",
                "Le fichier LSR n'a pas pu être reçu.",
            )
        })?
        .to_vec();
    let (prepared, bytes, metadata, source_sha256) =
        plan(&state, &study, target.revision, bytes, permit.clone()).await?;
    let mut created = false;
    let batch = if let Some(batch) =
        repository::find(&state, &study, target.revision, &metadata.sha256).await?
    {
        batch
    } else {
        created = true;
        let now = state.runtime.now()?;
        repository::create(
            &state,
            Batch {
                id: runtime::identifier(state.runtime.as_ref())?,
                study_id: study,
                revision: target.revision,
                source_sha256: source_sha256.clone(),
                sha256: metadata.sha256.clone(),
                object_key: runtime::hex(&state.runtime.random()?),
                size: metadata.size as i64,
                state: "pending".into(),
                row_count: prepared.rows.len() as i64,
                loaded_rows: 0,
                schema: prepared.schema.clone(),
                report: prepared.report.clone(),
                created_at: now,
                updated_at: now,
            },
        )
        .await?
    };
    if batch.source_sha256 != source_sha256 || batch.size != metadata.size as i64 {
        return Err(changed());
    }
    save_original(&state, &batch, bytes, &metadata).await?;
    let batch = load_rows(&state, batch, prepared).await?;
    let status = if batch.state != "prepared" {
        StatusCode::ACCEPTED
    } else if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(batch.summary()?)).into_response())
}

pub(super) async fn resume(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    auth::authorize_write(&state, &headers).await?;
    valid_id(&id)?;
    let permit = slot(&state)?;
    let batch = repository::get(&state, &id).await?;
    if batch.state == "prepared" {
        return Ok(Json(batch.summary()?));
    }
    let key = ObjectKey::parse(&batch.object_key).map_err(unavailable)?;
    let original = state.objects.get(&key, None).await.map_err(unavailable)?;
    let (prepared, _, metadata, source_hash) = plan(
        &state,
        &batch.study_id,
        batch.revision,
        original.bytes,
        permit.clone(),
    )
    .await?;
    if metadata.sha256 != batch.sha256
        || metadata.size as i64 != batch.size
        || source_hash != batch.source_sha256
    {
        return Err(changed());
    }
    let batch = load_rows(&state, batch, prepared).await?;
    Ok(Json(batch.summary()?))
}
