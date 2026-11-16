use super::{AppError, WorkspaceState, auth, forms, repository};
use axum::{
    Json,
    extract::{Path, Query as Parameters, State},
    http::{HeaderMap, StatusCode},
};
use sea_orm::{
    ConnectionTrait,
    sea_query::{Expr, ExprTrait, OnConflict, Order, Query, SelectStatement},
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Publish {
    expected_revision: i64,
}

pub(super) fn latest(study: Expr) -> SelectStatement {
    Query::select()
        .expr(Expr::col(("candidate", "revision")).max())
        .from_as("study_publications", "candidate")
        .and_where(Expr::col(("candidate", "study_id")).eq(study))
        .to_owned()
}

pub(super) async fn current(
    connection: &impl ConnectionTrait,
    id: &str,
) -> Result<Option<i64>, AppError> {
    let row = connection
        .query_one(&latest(Expr::val(id)))
        .await?
        .ok_or_else(AppError::internal)?;
    Ok(row.try_get_by_index(0)?)
}

fn published_query() -> SelectStatement {
    Query::select()
        .column(("studies", "id"))
        .column(("study_publications", "title"))
        .column(("study_publications", "revision"))
        .column(("studies", "created_at"))
        .expr_as(
            Expr::col(("study_publications", "published_at")),
            "updated_at",
        )
        .expr_as(
            Expr::col(("study_publications", "revision")),
            "published_revision",
        )
        .from("study_publications")
        .inner_join(
            "studies",
            Expr::col(("studies", "id")).equals(("study_publications", "study_id")),
        )
        .to_owned()
}

pub(super) async fn list(
    State(state): State<WorkspaceState>,
    Parameters(page): Parameters<forms::Page>,
) -> Result<Json<Value>, AppError> {
    forms::validate_page(page.offset)?;
    let rows = state
        .database
        .query_all(
            published_query()
                .and_where(
                    Expr::col(("study_publications", "revision"))
                        .eq(latest(Expr::col(("studies", "id")))),
                )
                .order_by(("studies", "created_at"), Order::Desc)
                .order_by(("studies", "id"), Order::Asc)
                .limit(51)
                .offset(page.offset),
        )
        .await?;
    let has_more = rows.len() > 50;
    let items = rows
        .iter()
        .take(50)
        .map(repository::summary)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(json!({ "items": items, "hasMore": has_more })))
}

async fn load(
    state: &WorkspaceState,
    id: &str,
    revision: Option<i64>,
) -> Result<repository::StoredStudy, AppError> {
    if uuid::Uuid::try_parse(id).is_err() {
        return Err(AppError::not_found());
    }
    let mut query = published_query();
    query.and_where(Expr::col(("study_publications", "study_id")).eq(id));
    if let Some(revision) = revision {
        query.and_where(Expr::col(("study_publications", "revision")).eq(revision));
    } else {
        query.and_where(Expr::col(("study_publications", "revision")).eq(latest(Expr::val(id))));
    }
    query
        .column(("definition_revisions", "document"))
        .inner_join(
            "definition_revisions",
            Expr::col(("definition_revisions", "study_id"))
                .equals(("study_publications", "study_id"))
                .and(
                    Expr::col(("definition_revisions", "revision"))
                        .equals(("study_publications", "revision")),
                ),
        );
    let row = state
        .database
        .query_one(&query)
        .await?
        .ok_or_else(AppError::not_found)?;
    let metadata = repository::summary(&row)?;
    let document: String = row.try_get("", "document")?;
    Ok(repository::StoredStudy {
        id: metadata.id,
        revision: metadata.revision,
        published_revision: metadata.published_revision,
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
        definition: repository::decode_definition(&document)?,
        source: repository::import_info(&state.database, id).await?,
    })
}

pub(super) async fn get(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
) -> Result<Json<repository::StoredStudy>, AppError> {
    Ok(Json(load(&state, &id, None).await?))
}

pub(super) async fn get_revision(
    State(state): State<WorkspaceState>,
    Path((id, revision)): Path<(String, i64)>,
) -> Result<Json<repository::StoredStudy>, AppError> {
    Ok(Json(load(&state, &id, Some(revision)).await?))
}

pub(super) async fn publish(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Result<Json<Publish>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>, AppError> {
    auth::authorize_write(&state, &headers).await?;
    let Json(body) = body.map_err(AppError::json)?;
    if uuid::Uuid::try_parse(&id).is_err() {
        return Err(AppError::not_found());
    }
    if !(1..1_000_000_000).contains(&body.expected_revision) {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "invalid_revision",
            "Révision invalide.",
        ));
    }
    // One statement selects the reviewed draft and records its publication. MAX(revision)
    // keeps concurrent/repeated publications from moving the public entry backwards.
    state
        .database
        .execute(
            Query::insert()
                .into_table("study_publications")
                .columns(["study_id", "revision", "title", "published_at"])
                .select_from(
                    Query::select()
                        .columns(["id", "revision", "title"])
                        .expr(Expr::val(state.runtime.now()?))
                        .from("studies")
                        .and_where(Expr::col("id").eq(&id))
                        .and_where(Expr::col("revision").eq(body.expected_revision))
                        .to_owned(),
                )?
                // The MySQL no-op must avoid `revision`, which also exists in the SELECT source.
                .on_conflict(
                    OnConflict::columns(["study_id", "revision"])
                        .do_nothing_on(["study_id"])
                        .to_owned(),
                ),
        )
        .await?;
    let exists = state
        .database
        .query_one(
            Query::select()
                .column("revision")
                .from("study_publications")
                .and_where(Expr::col("study_id").eq(&id))
                .and_where(Expr::col("revision").eq(body.expected_revision)),
        )
        .await?
        .is_some();
    if !exists {
        return Err(AppError::new(
            StatusCode::CONFLICT,
            "revision_conflict",
            "Le brouillon a changé. Rechargez-le avant de publier.",
        ));
    }
    Ok(Json(
        json!({"publishedRevision": current(&state.database, &id).await?}),
    ))
}
