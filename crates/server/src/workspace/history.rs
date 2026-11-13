use super::{AppError, WorkspaceState, auth, forms, repository};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use rustsurvey_domain::Definition;
use sea_orm::{
    ConnectionTrait,
    sea_query::{Expr, ExprTrait, Order, Query as Sql},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
pub(super) struct Page {
    before: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Revision {
    study_id: String,
    revision: i64,
    created_at: i64,
    pub(super) definition: Definition,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Restore {
    expected_revision: i64,
    revision: i64,
}

fn valid_id(id: &str) -> Result<(), AppError> {
    if uuid::Uuid::try_parse(id).is_err() {
        return Err(AppError::not_found());
    }
    Ok(())
}

pub(super) async fn list(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Query(page): Query<Page>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::authorize_read(&state, &headers).await?;
    valid_id(&id)?;
    let before = page.before.unwrap_or(1_000_000_000);
    if !(1..=1_000_000_000).contains(&before) {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "invalid_page",
            "Pagination invalide.",
        ));
    }
    let mut rows = state
        .database
        .query_all(
            &Sql::select()
                .columns(["revision", "created_at"])
                .from("definition_revisions")
                .and_where(Expr::col("study_id").eq(&id))
                .and_where(Expr::col("revision").lt(before))
                .order_by("revision", Order::Desc)
                .limit(21)
                .to_owned(),
        )
        .await?;
    let has_more = rows.len() > 20;
    rows.truncate(20);
    let mut items = Vec::new();
    for row in rows {
        items.push(json!({"revision":row.try_get::<i64>("", "revision")?, "createdAt":row.try_get::<i64>("", "created_at")?}));
    }
    Ok(Json(json!({"items":items, "hasMore":has_more})))
}

pub(super) async fn load(
    state: &WorkspaceState,
    id: &str,
    revision: i64,
) -> Result<Revision, AppError> {
    valid_id(id)?;
    let row = state
        .database
        .query_one(
            &Sql::select()
                .columns(["document", "created_at"])
                .from("definition_revisions")
                .and_where(Expr::col("study_id").eq(id))
                .and_where(Expr::col("revision").eq(revision))
                .to_owned(),
        )
        .await?
        .ok_or_else(AppError::not_found)?;
    let document: String = row.try_get("", "document")?;
    let mut definition: Definition =
        serde_json::from_str(&document).map_err(|_| AppError::internal())?;
    definition.upgrade().map_err(|_| AppError::internal())?;
    Ok(Revision {
        study_id: id.to_owned(),
        revision,
        created_at: row.try_get("", "created_at")?,
        definition,
    })
}

pub(super) async fn get(
    State(state): State<WorkspaceState>,
    Path((id, revision)): Path<(String, i64)>,
    headers: HeaderMap,
) -> Result<Json<Revision>, AppError> {
    auth::authorize_read(&state, &headers).await?;
    Ok(Json(load(&state, &id, revision).await?))
}

pub(super) async fn export(
    State(state): State<WorkspaceState>,
    Path((id, revision)): Path<(String, i64)>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    auth::authorize_read(&state, &headers).await?;
    let saved = load(&state, &id, revision).await?;
    let document =
        serde_json::to_string_pretty(&saved.definition).map_err(|_| AppError::internal())?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"definition-{id}-r{revision}.json\""),
            ),
        ],
        document,
    )
        .into_response())
}

pub(super) async fn restore(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Result<Json<Restore>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<repository::StoredStudy>, AppError> {
    auth::authorize_write(&state, &headers).await?;
    let Json(body) = body.map_err(AppError::json)?;
    if !(1..1_000_000_000).contains(&body.expected_revision) {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "invalid_revision",
            "Révision invalide.",
        ));
    }
    let saved = load(&state, &id, body.revision).await?;
    let current = repository::get(&state.database, &id).await?;
    if current.source.is_some()
        && current.definition.without_conditions() != saved.definition.without_conditions()
    {
        return Err(AppError::new(
            StatusCode::CONFLICT,
            "incompatible_revision",
            "Cette révision ne correspond pas à la structure source actuelle.",
        ));
    }
    let document = forms::document(&saved.definition, rustsurvey_domain::MAX_DOCUMENT_BYTES)?;
    // Historical rules have already been accepted at import/save time. Restoring must preserve them.
    Ok(Json(
        repository::update(
            &state.database,
            &id,
            body.expected_revision,
            &saved.definition,
            document,
            state.runtime.now()?,
        )
        .await?,
    ))
}
