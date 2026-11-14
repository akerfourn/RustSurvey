use super::super::{history, repository as studies, runtime, source_contexts};
use super::{AppError, Run, WorkspaceState};
use axum::http::StatusCode;
use rustsurvey_domain::{
    Definition,
    collection::{DEFAULTS_PROFILE, PAGES_PROFILE},
    navigation::NavigationSettings,
    participation::ParticipationPolicy,
};
use rustsurvey_limesurvey::{
    IMPORT_EXECUTION_PROFILE, Limits, LssDocument, SettingsContext, SourceRuntimeFacts,
};
use sea_orm::{
    ConnectionTrait,
    sea_query::{Expr, ExprTrait, Query},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::OwnedSemaphorePermit;

const MAX_EXECUTION_BYTES: usize = 8 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Request {
    pub context_id: String,
    pub facts: SourceRuntimeFacts,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Snapshot {
    schema_version: u32,
    source_profile: String,
    study_id: String,
    revision: i64,
    source_sha256: String,
    context_id: String,
    context_sha256: String,
    facts: SourceRuntimeFacts,
    native_profile: String,
    navigation: NavigationSettings,
    participation_policy: ParticipationPolicy,
    pub definition: Definition,
}

pub(super) struct Prepared {
    pub document: String,
    pub profile: String,
    pub navigation: NavigationSettings,
    pub policy: ParticipationPolicy,
}

fn blocked() -> AppError {
    AppError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "unsupported_import_execution",
        "Les réglages et fonctions de cette révision ne correspondent pas au profil d'exécution importé disponible.",
    )
}

pub(super) async fn prepare(
    state: &WorkspaceState,
    id: &str,
    revision: i64,
    definition: Definition,
    sha256: String,
    request: Request,
) -> Result<Prepared, AppError> {
    source_contexts::valid_id(&request.context_id)?;
    let permit = state.imports.clone().try_acquire_owned().map_err(|_| {
        AppError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "collection_busy",
            "Préparation occupée. Réessayez.",
        )
    })?;
    let context = source_contexts::selected(state, id, revision, &request.context_id).await?;
    let original = studies::original(&state.database, id).await?;
    let study_id = id.to_owned();
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        if runtime::hex(&Sha256::digest(original.as_bytes())) != sha256 {
            return Err(AppError::internal());
        }
        let source = LssDocument::parse(original.as_bytes(), Limits::default())
            .map_err(|_| AppError::internal())?;
        context.resolve(&source, &sha256)?;
        let selected: SettingsContext =
            serde_json::from_str(&context.document).map_err(|_| AppError::internal())?;
        let plan = source
            .compile_import(&definition, &selected, &request.facts)
            .map_err(|issues| blocked().with_issues(issues))?;
        let snapshot = Snapshot {
            schema_version: 1,
            source_profile: IMPORT_EXECUTION_PROFILE.into(),
            study_id,
            revision,
            source_sha256: sha256,
            context_id: request.context_id,
            context_sha256: context.metadata.sha256,
            facts: request.facts,
            native_profile: plan.native_profile.into(),
            navigation: plan.navigation,
            participation_policy: plan.participation_policy,
            definition: plan.definition,
        };
        let document = serde_json::to_string(&snapshot).map_err(|_| AppError::internal())?;
        if document.len() > MAX_EXECUTION_BYTES {
            return Err(blocked());
        }
        Ok(Prepared {
            document,
            profile: snapshot.native_profile,
            navigation: snapshot.navigation,
            policy: snapshot.participation_policy,
        })
    })
    .await
    .map_err(|_| AppError::internal())?
}

pub(super) fn decode(document: &str, run: &Run) -> Result<Snapshot, AppError> {
    if document.len() > MAX_EXECUTION_BYTES {
        return Err(AppError::internal());
    }
    let mut snapshot: Snapshot =
        serde_json::from_str(document).map_err(|_| AppError::internal())?;
    if snapshot.schema_version != 1
        || snapshot.source_profile != IMPORT_EXECUTION_PROFILE
        || snapshot.study_id != run.study_id
        || snapshot.revision != run.revision
        || snapshot.native_profile != run.profile
        || ![PAGES_PROFILE, DEFAULTS_PROFILE].contains(&run.profile.as_str())
        || run.navigation != Some(snapshot.navigation)
        || run.participation_policy != snapshot.participation_policy
        || !digest(&snapshot.source_sha256)
        || !digest(&snapshot.context_sha256)
        || uuid::Uuid::try_parse(&snapshot.context_id).is_err()
        || snapshot.facts.schema_version != 1
    {
        return Err(AppError::internal());
    }
    snapshot
        .definition
        .upgrade()
        .map_err(|_| AppError::internal())?;
    Ok(snapshot)
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) async fn definition(state: &WorkspaceState, run: &Run) -> Result<Definition, AppError> {
    if !run.has_execution {
        return Ok(history::load(state, &run.study_id, run.revision)
            .await?
            .definition);
    }
    let permit = state.imports.clone().try_acquire_owned().map_err(|_| {
        AppError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "collection_busy",
            "Lecture de définition occupée. Réessayez.",
        )
    })?;
    guarded_definition(state, run, Arc::new(permit)).await
}

pub(super) async fn guarded_definition(
    state: &WorkspaceState,
    run: &Run,
    permit: Arc<OwnedSemaphorePermit>,
) -> Result<Definition, AppError> {
    if !run.has_execution {
        return Ok(history::load(state, &run.study_id, run.revision)
            .await?
            .definition);
    }
    let row = state
        .database
        .query_one(
            &Query::select()
                .column("execution_document")
                .from("runs")
                .and_where(Expr::col("id").eq(&run.id))
                .to_owned(),
        )
        .await?
        .ok_or_else(AppError::internal)?;
    let document: String = row.try_get("", "execution_document")?;
    let run = run.clone();
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        Ok(decode(&document, &run)?.definition)
    })
    .await
    .map_err(|_| AppError::internal())?
}
