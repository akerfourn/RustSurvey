mod access;
mod export_body;
mod export_time;
mod exports;
mod handlers;
mod invitations;
mod navigation;
mod participation;
mod repository;
mod runs;
mod source_execution;
mod writes;

use super::{AppError, WorkspaceState};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::StatusCode,
    routing::{get, post, put},
};
use rustsurvey_domain::{
    Definition,
    collection::{Answers, Questionnaire, Snapshot},
    navigation::NavigationSettings,
    participation::ParticipationPolicy,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Checkpoint {
    pub language: String,
    pub group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_id: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Document {
    pub answers: Answers,
    pub checkpoint: Checkpoint,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Run {
    pub id: String,
    pub study_id: String,
    pub revision: i64,
    pub profile: String,
    pub is_open: bool,
    pub created_at: i64,
    pub participation_policy: ParticipationPolicy,
    pub navigation: Option<NavigationSettings>,
    #[serde(skip)]
    has_execution: bool,
}

pub(super) fn routes(state: WorkspaceState) -> Router<WorkspaceState> {
    let writes = Router::new()
        .route(
            "/api/admin/studies/{id}/runs",
            post(runs::create).layer(DefaultBodyLimit::max(64 * 1024)),
        )
        .route("/api/admin/runs/{id}/close", post(runs::close))
        .route(
            "/api/admin/runs/{id}/invitations",
            post(invitations::create).layer(DefaultBodyLimit::max(4096)),
        )
        .route(
            "/api/admin/runs/{run_id}/invitations/{id}/revoke",
            post(invitations::revoke),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            super::protect,
        ));
    let participant_writes = Router::new()
        .route(
            "/api/runs/{id}/responses",
            post(handlers::start).layer(DefaultBodyLimit::max(4096)),
        )
        .route("/api/responses/{id}", put(handlers::save))
        .route_layer(axum::middleware::from_fn_with_state(
            state,
            participant_write,
        ));
    Router::new()
        .merge(writes)
        .merge(participant_writes)
        .route("/api/admin/studies/{id}/runs", get(runs::list))
        .route("/api/admin/runs/{id}/responses", get(handlers::list))
        .route("/api/admin/runs/{id}/responses.vv", get(exports::vv))
        .route("/api/admin/runs/{id}/definition.lss", get(runs::export))
        .route("/api/admin/runs/{id}/invitations", get(invitations::list))
        .route(
            "/api/admin/runs/{run_id}/responses/{id}/response.json",
            get(handlers::export),
        )
        .route("/api/runs/{id}", get(runs::get))
        .route("/api/responses/{id}", get(handlers::get))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
}

async fn participant_write(
    axum::extract::State(state): axum::extract::State<WorkspaceState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, AppError> {
    access::client(request.headers())?;
    access::token_hash(request.headers())?;
    let _permit = access::write_slot(&state).await?;
    Ok(next.run(request).await)
}

fn conflict() -> AppError {
    AppError::new(
        StatusCode::CONFLICT,
        "response_conflict",
        "La réponse a changé. Rechargez-la avant de poursuivre.",
    )
}

fn missing() -> AppError {
    AppError::new(
        StatusCode::NOT_FOUND,
        "response_not_found",
        "Participation introuvable ou accès invalide.",
    )
}

fn closed() -> AppError {
    AppError::new(
        StatusCode::CONFLICT,
        "run_closed",
        "Cette campagne de collecte est fermée.",
    )
}

fn invalid(issues: Vec<rustsurvey_domain::ValidationIssue>) -> AppError {
    AppError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_answers",
        "Corrigez les réponses signalées.",
    )
    .with_issues(issues)
}

async fn evaluate(
    state: &WorkspaceState,
    definition: Definition,
    answers: Answers,
    submit: bool,
    run: &Run,
    previous: Option<&Checkpoint>,
    requested: &Checkpoint,
) -> Result<(Definition, Snapshot, Checkpoint), AppError> {
    let profile = run.profile.clone();
    let settings = run.navigation;
    let previous = previous.map(navigation::cursor);
    let requested = requested.clone();
    let permit = state.imports.clone().try_acquire_owned().map_err(|_| {
        AppError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "collection_busy",
            "Deux évaluations ou préparations sont déjà en cours. Réessayez.",
        )
    })?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let runtime = Questionnaire::compile_profile(&definition, &profile).map_err(invalid)?;
        if profile == rustsurvey_domain::collection::DEFAULTS_PROFILE {
            let settings = settings.ok_or_else(AppError::internal)?;
            let (snapshot, page) = runtime
                .enter_page(
                    &answers,
                    settings,
                    &requested.language,
                    previous.as_ref(),
                    Some(&navigation::cursor(&requested)),
                    submit,
                )
                .map_err(navigation::initialization_error)?;
            let checkpoint = Checkpoint {
                language: requested.language,
                page_id: Some(page.id),
                group_id: page.group_id,
            };
            return Ok((definition, snapshot, checkpoint));
        }
        let snapshot = runtime.evaluate(&answers, submit).map_err(invalid)?;
        let checkpoint = navigation::normalize(settings, &definition, &snapshot, &requested)?;
        Ok((definition, snapshot, checkpoint))
    })
    .await
    .map_err(|_| AppError::internal())?
}

fn checkpoint(definition: &Definition, checkpoint: &Checkpoint) -> Result<(), AppError> {
    if checkpoint.language != definition.language
        && !definition.languages.contains(&checkpoint.language)
    {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_checkpoint",
            "Langue du questionnaire inconnue.",
        ));
    }
    if checkpoint
        .group_id
        .as_ref()
        .is_some_and(|id| !definition.groups.iter().any(|group| &group.id == id))
    {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_checkpoint",
            "Groupe du questionnaire inconnu.",
        ));
    }
    Ok(())
}
