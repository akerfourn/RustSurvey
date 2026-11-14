use super::super::{AppError, WorkspaceState, auth, history, repository as studies, runtime};
use super::{prepare::Schema, reads::attachment, repository};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Response,
};
use rustsurvey_domain::{Definition, ValidationIssue};
use rustsurvey_limesurvey::{Limits, LssDocument, ResponseProjector, ResponseRow};
use rustsurvey_object_store::StoreError;
use serde_json::json;
use sha2::{Digest, Sha256};

pub(super) async fn get(
    State(state): State<WorkspaceState>,
    Path((id, ordinal)): Path<(String, i64)>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    auth::authorize_read(&state, &headers).await?;
    super::handlers::valid_id(&id)?;
    let download = state
        .object_reads
        .clone()
        .try_acquire_owned()
        .map_err(|_| super::handlers::unavailable(StoreError::Capacity))?;
    let permit = state.imports.clone().try_acquire_owned().map_err(|_| {
        AppError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "response_projection_busy",
            "Deux préparations sont déjà en cours. Réessayez.",
        )
    })?;
    let batch = repository::get(&state, &id).await?;
    if ordinal < 0 || ordinal >= batch.loaded_rows {
        return Err(AppError::not_found());
    }
    let definition = history::load(&state, &batch.study_id, batch.revision)
        .await?
        .definition;
    let original = studies::original(&state.database, &batch.study_id).await?;
    let record = repository::record(&state, &id, ordinal).await?;
    let bytes = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        prepare(&definition, &original, &batch, &record, ordinal)
    })
    .await
    .map_err(|_| AppError::internal())??;
    attachment(
        format!("import-{id}-row-{ordinal}-native-cells.json"),
        "application/json; charset=utf-8",
        bytes,
        download,
    )
}

fn unavailable(issues: Vec<ValidationIssue>) -> AppError {
    AppError::new(StatusCode::UNPROCESSABLE_ENTITY, "response_projection_unavailable",
        "La conversion de cette ligne demande des correspondances ou des capacités supplémentaires.").with_issues(issues)
}

fn prepare(
    definition: &Definition,
    original: &str,
    batch: &repository::Batch,
    record: &repository::StoredRecord,
    ordinal: i64,
) -> Result<Vec<u8>, AppError> {
    if runtime::hex(&Sha256::digest(original.as_bytes())) != batch.source_sha256
        || runtime::hex(&Sha256::digest(record.document.as_bytes())) != record.sha256
    {
        return Err(AppError::internal());
    }
    let schema: Schema = serde_json::from_str(&batch.schema).map_err(|_| AppError::internal())?;
    let document = LssDocument::parse(original.as_bytes(), Limits::default())
        .map_err(|_| AppError::internal())?;
    let source = document.source_model().ok_or_else(AppError::internal)?;
    let projector =
        ResponseProjector::compile(definition, source, &schema.columns).map_err(unavailable)?;
    let mapping = projector.bindings();
    if schema.schema_version != 1
        || mapping.survey_id != schema.source_survey_id
        || mapping.fields != schema.bindings
        || mapping.ignored_columns != schema.ignored_columns
    {
        return Err(unavailable(vec![ValidationIssue { path: "mapping".into(),
            message: "Les correspondances enregistrées ne sont plus identiques à cette révision et son original.".into() }]));
    }
    let row: ResponseRow =
        serde_json::from_str(&record.document).map_err(|_| AppError::internal())?;
    let projected = projector.project(&row).map_err(unavailable)?;
    let payload = json!({"schemaVersion":1,"importId":batch.id,"revision":batch.revision,"ordinal":ordinal,
        "sourceSha256":batch.source_sha256,"recordSha256":record.sha256,"projection":projected,
        "execution":"blocked","continuity":"unavailable"});
    let bytes = serde_json::to_vec(&payload).map_err(|_| AppError::internal())?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(unavailable(vec![ValidationIssue {
            path: "projection".into(),
            message: "Conversion limitée à 16 Mio par ligne.".into(),
        }]));
    }
    Ok(bytes)
}
