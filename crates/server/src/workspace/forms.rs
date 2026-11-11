use super::{AppError, WorkspaceState, auth, repository, runtime};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use rustsurvey_domain::{Definition, MAX_DOCUMENT_BYTES};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub(super) struct Page {
    #[serde(default)]
    offset: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Update {
    expected_revision: i64,
    definition: Definition,
}
fn response(value: impl serde::Serialize) -> Response {
    ([(header::CACHE_CONTROL, "no-store")], Json(value)).into_response()
}
fn document(definition: &Definition, maximum: usize) -> Result<String, AppError> {
    let document = serde_json::to_string(definition).map_err(|_| AppError::internal())?;
    if document.len() > maximum {
        return Err(AppError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "definition_too_large",
            "La définition dépasse la taille maximale autorisée.",
        ));
    }
    let issues = definition.validate();
    if !issues.is_empty() {
        return Err(AppError::invalid_definition(issues));
    }
    Ok(document)
}
pub(super) async fn list(
    State(state): State<WorkspaceState>,
    Query(page): Query<Page>,
) -> Result<Response, AppError> {
    if page.offset > 100000 {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "invalid_page",
            "Pagination invalide.",
        ));
    }
    let mut items = repository::list(&state.database, page.offset).await?;
    let has_more = items.len() > 50;
    items.truncate(50);
    Ok(response(json!({"items":items, "hasMore":has_more})))
}
pub(super) async fn get(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    if uuid::Uuid::try_parse(&id).is_err() {
        return Err(AppError::not_found());
    }
    Ok(response(repository::get(&state.database, &id).await?))
}
pub(super) async fn create(
    State(state): State<WorkspaceState>,
    headers: HeaderMap,
    Json(mut definition): Json<Definition>,
) -> Result<Response, AppError> {
    auth::authorize_write(&state, &headers).await?;
    definition.upgrade().map_err(|message| {
        AppError::invalid_definition(vec![rustsurvey_domain::ValidationIssue {
            path: "schemaVersion".to_owned(),
            message: message.to_owned(),
        }])
    })?;
    let document = document(&definition, MAX_DOCUMENT_BYTES)?;
    let issues = rustsurvey_domain::rules::validate(&definition, None);
    if !issues.is_empty() {
        return Err(AppError::invalid_definition(issues));
    }
    let id = runtime::identifier(state.runtime.as_ref())?;
    let now = state.runtime.now()?;
    let study = repository::create(&state.database, &id, &definition, document, now, None).await?;
    Ok((StatusCode::CREATED, response(study)).into_response())
}
pub(super) async fn update(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(mut update): Json<Update>,
) -> Result<Response, AppError> {
    auth::authorize_write(&state, &headers).await?;
    if uuid::Uuid::try_parse(&id).is_err() {
        return Err(AppError::not_found());
    }
    if !(1..1_000_000_000).contains(&update.expected_revision) {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "invalid_revision",
            "Révision invalide.",
        ));
    }
    update.definition.upgrade().map_err(|message| {
        AppError::invalid_definition(vec![rustsurvey_domain::ValidationIssue {
            path: "schemaVersion".to_owned(),
            message: message.to_owned(),
        }])
    })?;
    let previous = repository::get(&state.database, &id).await?;
    if previous.revision != update.expected_revision {
        return Err(AppError::new(
            StatusCode::CONFLICT,
            "revision_conflict",
            "Le questionnaire a été modifié. Rechargez sa version avant de sauvegarder.",
        ));
    }
    let imported = previous.source.is_some();
    let document = document(&update.definition, MAX_DOCUMENT_BYTES)?;
    if imported
        && previous.definition.without_conditions() != update.definition.without_conditions()
    {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "imported_content_read_only",
            "Ce lot permet seulement de modifier les conditions des études importées. Les autres réglages sont conservés.",
        ));
    }
    let issues = rustsurvey_domain::rules::validate(
        &update.definition,
        if imported {
            Some(&previous.definition)
        } else {
            None
        },
    );
    if !issues.is_empty() {
        return Err(AppError::invalid_definition(issues));
    }
    let now = state.runtime.now()?;
    let study = repository::update(
        &state.database,
        &id,
        update.expected_revision,
        &update.definition,
        document,
        now,
    )
    .await?;
    Ok(response(study))
}
