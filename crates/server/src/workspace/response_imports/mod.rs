mod handlers;
mod prepare;
mod projection;
mod reads;
mod repository;

use super::WorkspaceState;
use axum::{
    Router,
    routing::{get, post},
};

pub(super) fn routes(state: WorkspaceState) -> Router<WorkspaceState> {
    let writes = Router::new()
        .route(
            "/api/admin/studies/{id}/response-imports",
            post(handlers::upload),
        )
        .route(
            "/api/admin/response-imports/{id}/continue",
            post(handlers::resume),
        )
        .route_layer(axum::middleware::from_fn_with_state(state, super::protect));
    Router::new()
        .merge(writes)
        .route("/api/admin/studies/{id}/response-imports", get(reads::list))
        .route("/api/admin/response-imports/{id}", get(reads::get))
        .route(
            "/api/admin/response-imports/{id}/original.lsr",
            get(reads::original),
        )
        .route(
            "/api/admin/response-imports/{id}/mapping.json",
            get(reads::schema),
        )
        .route(
            "/api/admin/response-imports/{id}/records",
            get(reads::records),
        )
        .route(
            "/api/admin/response-imports/{id}/records/{ordinal}/record.json",
            get(reads::record),
        )
        .route(
            "/api/admin/response-imports/{id}/records/{ordinal}/native-cells.json",
            get(projection::get),
        )
}
