use super::super::{auth, history, repository as studies, runtime};
use super::{AppError, WorkspaceState, access, repository};
use axum::{
    Json,
    extract::{Path, Query as HttpQuery, State},
    http::{HeaderMap, StatusCode},
    response::Response,
};
use rustsurvey_domain::collection::{
    DEFAULTS_PROFILE, FORMS_PROFILE, PAGES_PROFILE, Questionnaire,
};
use rustsurvey_domain::navigation::NavigationSettings;
use rustsurvey_domain::participation::ParticipationPolicy;
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
    participation_policy: Option<ParticipationPolicy>,
    navigation: Option<NavigationSettings>,
    source_execution: Option<super::source_execution::Request>,
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
    let requested_policy = body.participation_policy.clone().unwrap_or_default();
    requested_policy
        .validate()
        .map_err(super::participation::invalid_policy)?;
    let policy = serde_json::to_string(&requested_policy).map_err(|_| AppError::internal())?;
    let study = studies::get(&state.database, &id).await?;
    if study.revision != body.expected_revision {
        return Err(super::conflict());
    }
    if let Some(source) = study.source {
        if let Some(request) = body.source_execution {
            let prepared = super::source_execution::prepare(
                &state,
                &id,
                body.expected_revision,
                study.definition,
                source.sha256,
                request,
            )
            .await?;
            if body
                .participation_policy
                .is_some_and(|policy| policy != prepared.policy)
                || body
                    .navigation
                    .is_some_and(|settings| settings != prepared.navigation)
            {
                return Err(AppError::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "source_settings_override",
                    "La présentation et les accès doivent correspondre aux réglages source qualifiés.",
                ));
            }
            return insert(
                &state,
                id,
                body.expected_revision,
                &prepared.profile,
                serde_json::to_string(&prepared.policy).map_err(|_| AppError::internal())?,
                Some(
                    serde_json::to_string(&prepared.navigation)
                        .map_err(|_| AppError::internal())?,
                ),
                Some(prepared.document),
            )
            .await;
        }
        let check =
            super::super::collection_check::inspect(&state, &id, body.expected_revision).await?;
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "imported_collection_unavailable",
            "La collecte des imports LimeSurvey doit encore être vérifiée. Utilisez leur aperçu local.",
        ).with_issues(check.issues));
    }
    if body.source_execution.is_some() {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_source_execution",
            "Les faits source ne s'appliquent qu'aux études importées.",
        ));
    }
    let has_defaults = study
        .definition
        .groups
        .iter()
        .flat_map(|group| &group.questions)
        .any(|question| {
            question
                .defaults
                .as_ref()
                .is_some_and(|defaults| !defaults.values.is_empty())
        });
    let settings = match (body.navigation, has_defaults) {
        (None, true) => Some(NavigationSettings::default()),
        (settings, _) => settings,
    };
    let profile = if has_defaults {
        DEFAULTS_PROFILE
    } else if settings.is_some() {
        PAGES_PROFILE
    } else {
        FORMS_PROFILE
    };
    let navigation = settings
        .map(|settings| serde_json::to_string(&settings))
        .transpose()
        .map_err(|_| AppError::internal())?;
    let permit = state.imports.clone().try_acquire_owned().map_err(|_| {
        AppError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "collection_busy",
            "Préparation occupée. Réessayez.",
        )
    })?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        Questionnaire::compile_profile(&study.definition, profile)
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
    insert(
        &state,
        id,
        body.expected_revision,
        profile,
        policy,
        navigation,
        None,
    )
    .await
}

pub(super) async fn insert(
    state: &WorkspaceState,
    id: String,
    revision: i64,
    profile: &str,
    policy: String,
    navigation: Option<String>,
    execution_document: Option<String>,
) -> Result<(StatusCode, Json<Value>), AppError> {
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
                    "participation_policy",
                    "navigation",
                    "execution_document",
                ])
                .values([
                    run_id.clone().into(),
                    id.into(),
                    revision.into(),
                    profile.into(),
                    true.into(),
                    0_i64.into(),
                    now.into(),
                    policy.into(),
                    navigation.into(),
                    execution_document.into(),
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
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let run = repository::run(&state.database, &id).await?;
    let definition = super::source_execution::definition(&state, &run).await?;
    let browser_completed = super::participation::browser_completed(&run, &headers);
    Ok(Json(
        json!({"run": run, "definition": definition, "browserCompleted": browser_completed}),
    ))
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

pub(super) async fn export(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    HttpQuery(options): HttpQuery<super::super::lss_export::NativeOptions>,
) -> Result<Response, AppError> {
    auth::authorize_read(&state, &headers).await?;
    let run = repository::run(&state.database, &id).await?;
    if studies::import_info(&state.database, &run.study_id)
        .await?
        .is_some()
    {
        if !run.has_execution {
            return Err(AppError::internal());
        }
        if options.survey_id != 1 {
            return Err(AppError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "source_identifiers_frozen",
                "Le réexport d'un import conserve ses identifiants source.",
            ));
        }
        return super::super::lss_export::export(
            State(state),
            Path((run.study_id, run.revision)),
            headers,
        )
        .await;
    }
    let revision = history::load(&state, &run.study_id, run.revision).await?;
    super::super::lss_export::native_response(
        state,
        revision.definition,
        options,
        run.navigation.unwrap_or_default(),
        format!("campaign-{id}.lss"),
    )
    .await
}
