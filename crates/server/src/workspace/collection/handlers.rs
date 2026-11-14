use super::super::auth;
use super::{
    AppError, Checkpoint, Document, Run, WorkspaceState, access, navigation, participation,
    repository, writes,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use rustsurvey_domain::collection::Answers;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Start {
    pub response_id: String,
    pub language: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Update {
    pub expected_version: i64,
    pub operation_id: String,
    pub answers: Answers,
    pub checkpoint: Checkpoint,
    pub submit: bool,
}

impl Update {
    pub(super) fn hash(&self) -> Result<String, AppError> {
        Ok(access::digest(
            &serde_json::to_vec(self).map_err(|_| AppError::internal())?,
        ))
    }
}

#[derive(Deserialize)]
pub(super) struct Page {
    #[serde(default)]
    offset: u64,
}

async fn state_document(state: &WorkspaceState, id: &str, hash: &str) -> Result<Value, AppError> {
    let stored = repository::authorized(state, id, hash).await?;
    let run = repository::run(&state.database, &stored.run_id).await?;
    let definition = super::source_execution::definition(state, &run).await?;
    Ok(
        json!({"id": stored.id, "run": run, "definition": definition, "version": stored.version, "status": stored.status, "answers": stored.document.answers, "checkpoint": stored.document.checkpoint, "createdAt": stored.created_at, "updatedAt": stored.updated_at}),
    )
}

pub(super) async fn start(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Result<Json<Start>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    access::client(&headers)?;
    let hash = access::token_hash(&headers)?;
    let invitation_hash = access::invitation_hash(&headers)?;
    let Json(input) = body.map_err(AppError::json)?;
    access::id(&input.response_id)?;
    let run = repository::run(&state.database, &id).await?;
    let definition = super::source_execution::definition(&state, &run).await?;
    let document = navigation::initial(&state, &run, definition, input.language.clone()).await?;
    let response_id = writes::start(
        &state,
        &run,
        &input,
        &hash,
        document,
        invitation_hash.as_deref(),
        participation::browser_completed(&run, &headers),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(state_document(&state, &response_id, &hash).await?),
    ))
}

pub(super) async fn get(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let hash = access::token_hash(&headers)?;
    Ok(Json(state_document(&state, &id, &hash).await?))
}

pub(super) async fn save(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Result<Json<Update>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    access::client(&headers)?;
    let hash = access::token_hash(&headers)?;
    let invitation_hash = access::invitation_hash(&headers)?;
    let Json(input) = body.map_err(AppError::json)?;
    access::id(&input.operation_id)?;
    if !(0..1_000_000_000).contains(&input.expected_version) {
        return Err(super::conflict());
    }
    let stored = repository::authorized(&state, &id, &hash).await?;
    let request_hash = input.hash()?;
    if let Some(result) = writes::receipt(
        &state.database,
        &id,
        &input.operation_id,
        &request_hash,
        state.runtime.now()?,
    )
    .await?
    {
        let run = repository::run(&state.database, &stored.run_id).await?;
        return acknowledgement(result, &run, state.secure_cookie);
    }
    if stored.version != input.expected_version {
        return Err(super::conflict());
    }
    let run = repository::run(&state.database, &stored.run_id).await?;
    if stored.status != "draft" && !run.participation_policy.allows_edit() {
        return Err(super::conflict());
    }
    if !run.is_open {
        return Err(super::closed());
    }
    let definition = super::source_execution::definition(&state, &run).await?;
    super::checkpoint(&definition, &input.checkpoint)?;
    let (_, snapshot, checkpoint) = super::evaluate(
        &state,
        definition,
        input.answers.clone(),
        input.submit,
        &run,
        Some(&stored.document.checkpoint),
        &input.checkpoint,
    )
    .await?;
    let document = Document {
        answers: snapshot.answers,
        checkpoint,
    };
    let result = writes::commit(
        &state,
        &stored,
        &input,
        document,
        snapshot.warnings,
        invitation_hash.as_deref(),
        participation::browser_completed(&run, &headers),
    )
    .await?;
    acknowledgement(result, &run, state.secure_cookie)
}

fn acknowledgement(result: Value, run: &Run, secure: bool) -> Result<Response, AppError> {
    let submitted = result["status"] == "submitted";
    let mut response = Json(result).into_response();
    if submitted && let Some(cookie) = participation::completion_cookie(run, secure)? {
        response.headers_mut().append(header::SET_COOKIE, cookie);
    }
    Ok(response)
}

pub(super) async fn list(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Query(page): Query<Page>,
) -> Result<Json<Value>, AppError> {
    auth::authorize_read(&state, &headers).await?;
    repository::run(&state.database, &id).await?;
    if page.offset > 100_000 {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "invalid_page",
            "Pagination invalide.",
        ));
    }
    let mut items = repository::summaries(&state, &id, page.offset).await?;
    let has_more = items.len() > 20;
    items.truncate(20);
    Ok(Json(json!({"items": items, "hasMore": has_more})))
}

pub(super) async fn export(
    State(state): State<WorkspaceState>,
    Path((run_id, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    auth::authorize_read(&state, &headers).await?;
    access::id(&id)?;
    let stored = repository::find(&state.database, &id)
        .await?
        .ok_or_else(super::missing)?;
    if stored.run_id != run_id {
        return Err(super::missing());
    }
    let run = repository::run(&state.database, &run_id).await?;
    let document = json!({"id": id, "run": run, "version": stored.version, "status": stored.status, "answers": stored.document.answers, "checkpoint": stored.document.checkpoint, "initialLanguage":stored.initial_language, "createdAt": stored.created_at, "updatedAt": stored.updated_at});
    Ok((
        [(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"response-{id}.json\""),
        )],
        Json(document),
    )
        .into_response())
}
