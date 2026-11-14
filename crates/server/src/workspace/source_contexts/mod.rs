mod handlers;
mod repository;

use super::{AppError, WorkspaceState};
use axum::{
    Router,
    http::StatusCode,
    routing::{get, post},
};
pub(super) use repository::{Metadata, StoredContext, selected};

pub(super) fn routes(state: WorkspaceState) -> Router<WorkspaceState> {
    let writes = Router::new()
        .route(
            "/api/admin/studies/{id}/revisions/{revision}/settings-contexts",
            post(handlers::upload),
        )
        .route_layer(axum::middleware::from_fn_with_state(state, super::protect));
    Router::new()
        .merge(writes)
        .route(
            "/api/admin/studies/{id}/revisions/{revision}/settings-contexts",
            get(handlers::list),
        )
        .route(
            "/api/admin/source-settings-contexts/{id}",
            get(handlers::get),
        )
        .route(
            "/api/admin/source-settings-contexts/{id}/original.json",
            get(handlers::original),
        )
}

fn invalid() -> AppError {
    AppError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_settings_context",
        "Contexte source invalide : vérifiez son format, ses identités et sa hiérarchie de groupes.",
    )
}

pub(super) fn valid_id(id: &str) -> Result<(), AppError> {
    if !uuid::Uuid::try_parse(id).is_ok_and(|parsed| parsed.to_string() == id) {
        return Err(AppError::not_found());
    }
    Ok(())
}
