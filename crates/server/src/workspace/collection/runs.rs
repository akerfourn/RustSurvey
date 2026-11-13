use super::super::{auth, history, repository as studies, runtime};
use super::{AppError, WorkspaceState, access, repository};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use rustsurvey_domain::collection::{PROFILE, Questionnaire};
use sea_orm::{
    ConnectionTrait,
    sea_query::{Expr, ExprTrait, Query},
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Create {
    expected_revision: i64,
}

pub(super) async fn create(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Result<Json<Create>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    auth::authorize_write(&state, &headers).await?;
    access::id(&id)?;
    let Json(body) = body.map_err(AppError::json)?;
    let study = studies::get(&state.database, &id).await?;
    if study.revision != body.expected_revision {
        return Err(super::conflict());
    }
    if study.source.is_some() {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "imported_collection_unavailable",
            "La collecte des imports LimeSurvey doit encore être vérifiée. Utilisez leur aperçu local.",
        ));
    }
    let permit = state.imports.clone().try_acquire_owned().map_err(|_| {
        AppError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "collection_busy",
            "Préparation occupée. Réessayez.",
        )
    })?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        Questionnaire::compile(&study.definition)
            .map(|_| ())
            .map_err(|issues| {
                AppError::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "unsupported_collection_profile",
                    "Le questionnaire dépasse le profil de collecte disponible.",
                )
                .with_issues(issues)
            })
    })
    .await
    .map_err(|_| AppError::internal())??;
    let run_id = runtime::identifier(state.runtime.as_ref())?;
    let now = state.runtime.now()?;
    state
        .database
        .execute(
            Query::insert()
                .into_table("runs")
                .columns([
                    "id",
                    "study_id",
                    "definition_revision",
                    "profile",
                    "is_open",
                    "write_version",
                    "created_at",
                ])
                .values([
                    run_id.clone().into(),
                    id.into(),
                    body.expected_revision.into(),
                    PROFILE.into(),
                    true.into(),
                    0_i64.into(),
                    now.into(),
                ])?,
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!(repository::run(&state.database, &run_id).await?)),
    ))
}

pub(super) async fn list(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    auth::authorize_read(&state, &headers).await?;
    access::id(&id)?;
    Ok(Json(json!({"items": repository::runs(&state, &id).await?})))
}

pub(super) async fn get(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let run = repository::run(&state.database, &id).await?;
    let revision = history::load(&state, &run.study_id, run.revision).await?;
    Ok(Json(json!({"run": run, "definition": revision.definition})))
}

pub(super) async fn close(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    auth::authorize_write(&state, &headers).await?;
    access::id(&id)?;
    repository::run(&state.database, &id).await?;
    state
        .database
        .execute(
            &Query::update()
                .table("runs")
                .value("is_open", false)
                .and_where(Expr::col("id").eq(&id))
                .to_owned(),
        )
        .await?;
    Ok(Json(json!(repository::run(&state.database, &id).await?)))
}
