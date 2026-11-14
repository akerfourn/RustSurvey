//! Private, revision-specific evidence for future imported collection profiles.
use super::{AppError, WorkspaceState, auth, history, repository, runtime, source_contexts};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use rustsurvey_domain::{Definition, ValidationIssue, collection::Questionnaire};
use rustsurvey_limesurvey::{
    GroupSettingsReport, Limits, LssDocument, ProjectionGaps, source_bindings,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CollectionCheck {
    schema_version: u32,
    revision: i64,
    source_sha256: String,
    execution: &'static str,
    continuity: &'static str,
    runtime: &'static str,
    mapping: &'static str,
    mapped_cells: usize,
    projection: ProjectionGaps,
    group_settings: GroupSettingsReport,
    source_context: Option<source_contexts::Metadata>,
    issue_count: usize,
    pub(super) issues: Vec<ValidationIssue>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Target {
    context_id: Option<String>,
}

impl CollectionCheck {
    fn append(&mut self, prefix: &str, count: usize, issues: Vec<ValidationIssue>) {
        self.issue_count += count;
        for issue in issues {
            if self.issues.len() == 128 {
                break;
            }
            self.issues.push(ValidationIssue {
                path: format!("{prefix}{}", issue.path),
                message: issue.message,
            });
        }
    }
}

pub(super) async fn get(
    State(state): State<WorkspaceState>,
    Path((id, revision)): Path<(String, i64)>,
    headers: HeaderMap,
    Query(target): Query<Target>,
) -> Result<Json<CollectionCheck>, AppError> {
    auth::authorize_read(&state, &headers).await?;
    Ok(Json(
        inspect_with_context(&state, &id, revision, target.context_id.as_deref()).await?,
    ))
}

pub(super) async fn inspect(
    state: &WorkspaceState,
    id: &str,
    revision: i64,
) -> Result<CollectionCheck, AppError> {
    inspect_with_context(state, id, revision, None).await
}

async fn inspect_with_context(
    state: &WorkspaceState,
    id: &str,
    revision: i64,
    context_id: Option<&str>,
) -> Result<CollectionCheck, AppError> {
    let permit = state.imports.clone().try_acquire_owned().map_err(|_| {
        AppError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "collection_check_busy",
            "Deux préparations sont déjà en cours. Réessayez.",
        )
    })?;
    let saved = history::load(state, id, revision).await?;
    let source = repository::import_info(&state.database, id)
        .await?
        .ok_or_else(AppError::not_found)?;
    let original = repository::original(&state.database, id).await?;
    let context = match context_id {
        Some(context_id) => Some(source_contexts::selected(state, id, revision, context_id).await?),
        None => None,
    };
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        analyze(
            &saved.definition,
            &original,
            source.sha256,
            revision,
            context,
        )
    })
    .await
    .map_err(|_| AppError::internal())?
}

fn analyze(
    definition: &Definition,
    original: &str,
    sha256: String,
    revision: i64,
    context: Option<source_contexts::StoredContext>,
) -> Result<CollectionCheck, AppError> {
    if runtime::hex(&Sha256::digest(original.as_bytes())) != sha256 {
        return Err(AppError::internal());
    }
    let document = LssDocument::parse(original.as_bytes(), Limits::default())
        .map_err(|_| AppError::internal())?;
    let projection = document.projection_gaps();
    let group_settings = match &context {
        Some(context) => context.resolve(&document, &sha256)?,
        None => document
            .group_settings(None)
            .map_err(|_| AppError::internal())?,
    };
    let model = document.source_model().ok_or_else(AppError::internal)?;
    let bindings = source_bindings(definition, model).map_err(|_| AppError::internal())?;
    let has_defaults = definition
        .groups
        .iter()
        .flat_map(|group| &group.questions)
        .any(|question| question.defaults.is_some());
    let runtime_result = if has_defaults {
        Questionnaire::compile_defaults(definition)
    } else {
        Questionnaire::compile_forms(definition)
    };
    let runtime_issues = runtime_result.err().unwrap_or_default();
    let runtime = if runtime_issues.is_empty() {
        "supported"
    } else {
        "blocked"
    };
    let mapping = if bindings.issue_count == 0 {
        "complete"
    } else {
        "partial"
    };
    let mut result = CollectionCheck {
        schema_version: 1,
        revision,
        source_sha256: sha256,
        execution: "blocked",
        continuity: "unavailable",
        runtime,
        mapping,
        mapped_cells: bindings.fields.len(),
        issue_count: projection.issue_count,
        issues: Vec::new(),
        projection,
        group_settings,
        source_context: context.map(|context| context.metadata),
    };
    result.append("", 2, qualification_gates());
    result.append("runtime.", runtime_issues.len(), runtime_issues);
    result.append("mapping.", bindings.issue_count, bindings.issues);
    let projected_issues = std::mem::take(&mut result.projection.issues);
    // Its count was already included above; keep one bounded combined list in the API.
    result.append("", 0, projected_issues);
    Ok(result)
}

fn qualification_gates() -> Vec<ValidationIssue> {
    // Keep these first even when source details fill the bounded diagnostic list.
    vec![
        ValidationIssue {
            path: "execution.reference".into(),
            message: "Le LSS seul ne qualifie pas l'exécution. Un contexte et des faits source explicites sont requis pour une nouvelle campagne.".into(),
        },
        ValidationIssue {
            path: "continuity".into(),
            message: "Le transfert des anciennes réponses, des participants et des droits de reprise reste indisponible.".into(),
        },
    ]
}
