//! SQL-backed questionnaire authoring and a single administrator account.
mod auth;
mod auth_repository;
mod error;
mod forms;
mod history;
mod imports;
mod lss_export;
mod lss_projection;
mod media;
mod media_body;
mod media_content;
mod media_repository;
mod password_policy;
mod repository;
mod runtime;

pub use error::AppError;
pub use password_policy::PasswordPolicy;
pub use runtime::{Runtime, SystemRuntime};

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post, put},
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct WorkspaceState {
    database: DatabaseConnection,
    runtime: Arc<dyn Runtime>,
    login_slots: Arc<Semaphore>,
    secure_cookie: bool,
    writes: Arc<Semaphore>,
    imports: Arc<Semaphore>,
    objects: Arc<dyn rustsurvey_object_store::ObjectStore>,
    object_reads: Arc<Semaphore>,
}

impl WorkspaceState {
    pub fn new(database: DatabaseConnection, secure_cookie: bool) -> Self {
        Self::with_runtime(database, secure_cookie, Arc::new(SystemRuntime))
    }
    pub fn with_runtime(
        database: DatabaseConnection,
        secure_cookie: bool,
        runtime: Arc<dyn Runtime>,
    ) -> Self {
        Self {
            objects: Arc::new(crate::object_store::SqlObjectStore::new(database.clone())),
            object_reads: Arc::new(Semaphore::new(4)),
            database,
            runtime,
            login_slots: Arc::new(Semaphore::new(1)),
            writes: Arc::new(Semaphore::new(2)),
            imports: Arc::new(Semaphore::new(2)),
            secure_cookie,
        }
    }
    pub fn with_object_store(
        mut self,
        objects: Arc<dyn rustsurvey_object_store::ObjectStore>,
    ) -> Self {
        self.objects = objects;
        self
    }
}

pub fn routes(state: WorkspaceState) -> Router {
    let authentication = Router::new()
        .route("/api/admin/session", get(auth::session))
        .route("/api/admin/login", post(auth::login))
        .route("/api/admin/logout", post(auth::logout))
        .layer(DefaultBodyLimit::max(4096));
    let protected = Router::new()
        .route(
            "/api/admin/studies",
            post(forms::create).layer(DefaultBodyLimit::max(
                rustsurvey_domain::MAX_DOCUMENT_BYTES + 4096,
            )),
        )
        .route("/api/admin/imports/lss", post(imports::import))
        .route("/api/admin/studies/{id}", put(forms::update))
        .route("/api/admin/studies/{id}/restore", post(history::restore))
        .route(
            "/api/admin/studies/{id}/media/{media_id}",
            put(media::upload),
        )
        .route_layer(axum::middleware::from_fn_with_state(state.clone(), protect));
    Router::new()
        .route("/api/studies", get(forms::list))
        .route("/api/studies/{id}", get(forms::get))
        .route(
            "/api/studies/{id}/media/{media_id}",
            get(media::get_public).head(media::head_public),
        )
        .route(
            "/api/admin/studies/{id}/media/{media_id}",
            get(media::get_private).head(media::head_private),
        )
        .route("/api/admin/studies/{id}/media", get(media::list))
        .route("/api/admin/studies/{id}/revisions", get(history::list))
        .route(
            "/api/admin/studies/{id}/revisions/{revision}",
            get(history::get),
        )
        .route(
            "/api/admin/studies/{id}/revisions/{revision}/definition.json",
            get(history::export),
        )
        .route(
            "/api/admin/studies/{id}/revisions/{revision}/modified.lss",
            get(lss_export::export),
        )
        .route(
            "/api/admin/studies/{id}/original.lss",
            get(imports::original),
        )
        .merge(protected)
        .layer(DefaultBodyLimit::max(
            imports::MAX_IMPORTED_DOCUMENT_BYTES + 4096,
        ))
        .merge(authentication)
        .layer(axum::middleware::from_fn(api_headers))
        .with_state(state)
}

pub async fn bootstrap_admin(
    state: &WorkspaceState,
    username: &str,
    password: &str,
    policy: PasswordPolicy,
) -> Result<bool, AppError> {
    auth::bootstrap(state, username, password, policy).await
}

async fn protect(
    axum::extract::State(state): axum::extract::State<WorkspaceState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if let Err(error) = auth::authorize_write(&state, request.headers()).await {
        return error.into_response();
    }
    let Ok(_permit) = state.writes.try_acquire() else {
        return AppError::new(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "write_busy",
            "Deux enregistrements sont déjà en cours. Réessayez.",
        )
        .into_response();
    };
    next.run(request).await
}
async fn api_headers(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::http::{HeaderValue, StatusCode, header};
    use axum::response::IntoResponse;
    let mut response =
        match tokio::time::timeout(std::time::Duration::from_secs(15), next.run(request)).await {
            Ok(response) => response,
            Err(_) => AppError::new(
                StatusCode::REQUEST_TIMEOUT,
                "request_timeout",
                "La requête a dépassé le délai autorisé.",
            )
            .into_response(),
        };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}
