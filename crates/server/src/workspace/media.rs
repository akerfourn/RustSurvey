use super::{
    AppError, WorkspaceState, auth, media_content, media_repository as repository, runtime,
};
use axum::{
    Json,
    body::{Body, to_bytes},
    extract::{Path, Query, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use rustsurvey_object_store::{MAX_OBJECT_BYTES, ObjectKey, ObjectMetadata, StoreError};
use sea_orm::{
    ConnectionTrait,
    sea_query::{Expr, ExprTrait, Query as Sql},
};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeSet;

#[derive(Deserialize)]
pub(super) struct Upload {
    name: String,
}
#[derive(Deserialize)]
pub(super) struct Page {
    #[serde(default)]
    offset: u64,
}

fn valid_id(id: &str) -> Result<(), AppError> {
    if !uuid::Uuid::try_parse(id).is_ok_and(|uuid| uuid.to_string() == id) {
        return Err(not_found());
    }
    Ok(())
}
fn not_found() -> AppError {
    AppError::new(
        StatusCode::NOT_FOUND,
        "media_not_found",
        "Image introuvable.",
    )
}
fn unavailable(_: StoreError) -> AppError {
    AppError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "object_store_unavailable",
        "Le stockage objet est indisponible. Réessayez le même envoi.",
    )
}

pub(super) async fn upload(
    State(state): State<WorkspaceState>,
    Path((study, id)): Path<(String, String)>,
    Query(body): Query<Upload>,
    request: Request,
) -> Result<Json<repository::Media>, AppError> {
    auth::authorize_write(&state, request.headers()).await?;
    valid_id(&study)?;
    valid_id(&id)?;
    if body.name.is_empty()
        || body.name.len() > 1024
        || body.name.chars().count() > 255
        || body.name.chars().any(char::is_control)
    {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "invalid_filename",
            "Nom de fichier invalide ou supérieur à 255 caractères.",
        ));
    }
    // Ensure the owner exists without loading the questionnaire document.
    if state
        .database
        .query_one(
            &Sql::select()
                .column("id")
                .from("studies")
                .and_where(Expr::col("id").eq(&study))
                .to_owned(),
        )
        .await?
        .is_none()
    {
        return Err(AppError::not_found());
    }
    let bytes = to_bytes(request.into_body(), MAX_OBJECT_BYTES)
        .await
        .map_err(|_| {
            AppError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "media_too_large",
                "Image limitée à 8 Mio.",
            )
        })?
        .to_vec();
    let content_type = media_content::content_type(&bytes)?;
    let metadata = ObjectMetadata::for_bytes(&bytes).map_err(unavailable)?;
    let media = if let Some(media) = repository::get(&state.database, &study, &id).await? {
        if !repository::matches(&media, &body.name, content_type, &metadata) {
            return Err(repository::conflict());
        }
        if media.state == "ready" {
            let key = ObjectKey::parse(&media.object_key).map_err(unavailable)?;
            match state.objects.head(&key).await {
                Ok(stored) if stored == metadata => return Ok(Json(media)),
                Err(StoreError::NotFound) => {}
                _ => return Err(unavailable(StoreError::Unavailable)),
            }
        }
        media
    } else {
        let key = runtime::hex(&state.runtime.random()?);
        let media = repository::Media {
            id: id.clone(),
            study_id: study.clone(),
            filename: body.name,
            content_type: content_type.to_owned(),
            size: metadata.size as i64,
            sha256: metadata.sha256.clone(),
            state: "pending".to_owned(),
            created_at: state.runtime.now()?,
            object_key: key,
        };
        repository::begin(&state.database, &media).await?;
        media
    };
    let key = ObjectKey::parse(&media.object_key).map_err(unavailable)?;
    match state.objects.put(&key, bytes).await {
        Ok(stored) if stored == metadata => {}
        Err(StoreError::Conflict) => {
            // Recover a PUT that succeeded before its SQL acknowledgement was lost.
            if state.objects.head(&key).await.map_err(unavailable)? != metadata {
                return Err(repository::conflict());
            }
        }
        _ => return Err(unavailable(StoreError::Unavailable)),
    }
    repository::ready(&state.database, &id).await?;
    Ok(Json(repository::Media {
        state: "ready".to_owned(),
        ..media
    }))
}

pub(super) async fn list(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Query(page): Query<Page>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::authorize_read(&state, &headers).await?;
    valid_id(&id)?;
    if page.offset > 100_000 {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "invalid_page",
            "Pagination invalide.",
        ));
    }
    let mut items = repository::list(&state.database, &id, page.offset).await?;
    let has_more = items.len() > 50;
    items.truncate(50);
    Ok(Json(json!({"items":items, "hasMore":has_more})))
}

pub(super) async fn reference(
    connection: &impl ConnectionTrait,
    study: &str,
    revision: i64,
    definition: &rustsurvey_domain::Definition,
) -> Result<(), AppError> {
    let ids: BTreeSet<_> = definition
        .groups
        .iter()
        .flat_map(|group| &group.questions)
        .flat_map(|question| &question.media)
        .map(|image| image.id.as_str())
        .collect();
    let ids: Vec<_> = ids.into_iter().collect();
    for batch in ids.chunks(256) {
        let rows = connection
            .query_all(
                &Sql::select()
                    .column("id")
                    .from("media")
                    .and_where(Expr::col("study_id").eq(study))
                    .and_where(Expr::col("state").eq("ready"))
                    .and_where(Expr::col("id").is_in(batch.iter().copied()))
                    .to_owned(),
            )
            .await?;
        if rows.len() != batch.len() {
            return Err(AppError::invalid_definition(vec![
                rustsurvey_domain::ValidationIssue {
                    path: "groups".to_owned(),
                    message: "Une image est absente, incomplète ou associée à une autre étude."
                        .to_owned(),
                },
            ]));
        }
        let mut insert = Sql::insert();
        insert
            .into_table("media_references")
            .columns(["study_id", "revision", "media_id"]);
        for id in batch {
            insert.values([study.into(), revision.into(), (*id).into()])?;
        }
        connection.execute(&insert).await?;
    }
    Ok(())
}

async fn read(
    state: WorkspaceState,
    study: String,
    id: String,
    headers: HeaderMap,
    private: bool,
    head_only: bool,
) -> Result<Response, AppError> {
    if private {
        auth::authorize_read(&state, &headers).await?;
    }
    valid_id(&study)?;
    valid_id(&id)?;
    if !private && !repository::is_public(&state.database, &study, &id).await? {
        return Err(not_found());
    }
    let media = repository::get(&state.database, &study, &id)
        .await?
        .filter(|media| media.state == "ready")
        .ok_or_else(not_found)?;
    let key = ObjectKey::parse(&media.object_key).map_err(unavailable)?;
    let metadata = state.objects.head(&key).await.map_err(unavailable)?;
    if metadata.size as i64 != media.size || metadata.sha256 != media.sha256 {
        return Err(unavailable(StoreError::Unavailable));
    }
    let mut range = None;
    if !head_only && let Some(value) = headers.get(header::RANGE) {
        let value = value.to_str().map_err(|_| {
            AppError::new(
                StatusCode::RANGE_NOT_SATISFIABLE,
                "invalid_range",
                "Plage d’octets invalide.",
            )
        })?;
        match media_content::range(value, metadata.size) {
            Ok(value) => range = Some(value),
            Err(error) => {
                let mut response = error.into_response();
                response.headers_mut().insert(
                    header::CONTENT_RANGE,
                    format!("bytes */{}", metadata.size)
                        .parse()
                        .map_err(|_| AppError::internal())?,
                );
                return Ok(response);
            }
        }
    }
    let mut response = if head_only {
        Body::empty().into_response()
    } else {
        let permit = state
            .object_reads
            .clone()
            .try_acquire_owned()
            .map_err(|_| {
                AppError::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "media_read_busy",
                    "Quatre lectures d’images sont déjà en cours. Réessayez.",
                )
            })?;
        let bytes = state
            .objects
            .get(&key, range)
            .await
            .map_err(unavailable)?
            .bytes;
        Body::new(super::media_body::ObjectBody::new(bytes, permit)).into_response()
    };
    let response_headers = response.headers_mut();
    response_headers.insert(
        header::CONTENT_TYPE,
        media
            .content_type
            .parse()
            .map_err(|_| AppError::internal())?,
    );
    response_headers.insert(
        header::ACCEPT_RANGES,
        "bytes".parse().map_err(|_| AppError::internal())?,
    );
    response_headers.insert(
        header::ETAG,
        format!("\"{}\"", metadata.sha256)
            .parse()
            .map_err(|_| AppError::internal())?,
    );
    let length = range
        .map(|range| range.end - range.start)
        .unwrap_or(metadata.size);
    response_headers.insert(header::CONTENT_LENGTH, length.into());
    if let Some(range) = range {
        response_headers.insert(
            header::CONTENT_RANGE,
            format!("bytes {}-{}/{}", range.start, range.end - 1, metadata.size)
                .parse()
                .map_err(|_| AppError::internal())?,
        );
        *response.status_mut() = StatusCode::PARTIAL_CONTENT;
    }
    Ok(response)
}

pub(super) async fn get_private(
    State(state): State<WorkspaceState>,
    Path((study, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    read(state, study, id, headers, true, false).await
}
pub(super) async fn head_private(
    State(state): State<WorkspaceState>,
    Path((study, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    read(state, study, id, headers, true, true).await
}
pub(super) async fn get_public(
    State(state): State<WorkspaceState>,
    Path((study, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    read(state, study, id, headers, false, false).await
}
pub(super) async fn head_public(
    State(state): State<WorkspaceState>,
    Path((study, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    read(state, study, id, headers, false, true).await
}
