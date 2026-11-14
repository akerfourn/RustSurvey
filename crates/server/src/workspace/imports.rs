use super::{AppError, WorkspaceState, auth, lss_projection, repository, runtime};
use axum::{
    Json,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use rustsurvey_limesurvey::{Limits, LssDocument};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::error::Error as _;

pub const MAX_IMPORTED_DOCUMENT_BYTES: usize = rustsurvey_domain::MAX_DOCUMENT_BYTES;

pub(super) async fn import(
    State(state): State<WorkspaceState>,
    request: Request,
) -> Result<Response, AppError> {
    auth::authorize_write(&state, request.headers()).await?;
    let content_type = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|item| item.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim();
    if !["application/xml", "text/xml", "application/octet-stream"].contains(&content_type) {
        return Err(AppError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "invalid_media_type",
            "Envoyez le fichier LSS comme corps XML.",
        ));
    }
    let permit = state.imports.clone().try_acquire_owned().map_err(|_| {
        AppError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "import_busy",
            "Deux imports sont déjà en cours.",
        )
    })?;
    let bytes = to_bytes(request.into_body(), MAX_IMPORTED_DOCUMENT_BYTES)
        .await
        .map_err(|error| {
            if !error
                .source()
                .is_some_and(|source| source.is::<http_body_util::LengthLimitError>())
            {
                return AppError::new(
                    StatusCode::BAD_REQUEST,
                    "invalid_body",
                    "Le fichier LSS n’a pas pu être reçu.",
                );
            }
            AppError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "import_too_large",
                "Import LSS limité à 8 Mio.",
            )
        })?;
    let runtime = state.runtime.clone();
    let (definition, document, source) = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        prepare_import(&bytes, runtime.as_ref())
    })
    .await
    .map_err(|_| AppError::internal())??;
    let id = runtime::identifier(state.runtime.as_ref())?;
    let stored = repository::create(
        &state.database,
        &id,
        &definition,
        document,
        state.runtime.now()?,
        Some(source),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(stored)).into_response())
}

fn prepare_import(
    bytes: &[u8],
    runtime: &dyn super::Runtime,
) -> Result<
    (
        rustsurvey_domain::Definition,
        String,
        repository::ImportedSource,
    ),
    AppError,
> {
    let source = LssDocument::parse(bytes, Limits::default()).map_err(|_| {
        AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_lss",
            "Fichier LSS illisible ou limites XML dépassées.",
        )
    })?;
    if source.report().has_errors()
        || source.report().source_schema != rustsurvey_limesurvey::TARGET_SCHEMA
    {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_lss_structure",
            "L’import nécessite une structure LSS valide du schéma 623. Consultez l’inspecteur LSS pour le détail.",
        ));
    }
    let model = source.source_model().ok_or_else(|| {
        AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_lss_structure",
            "La structure LSS ne possède pas de projection exploitable.",
        )
    })?;
    let projection = lss_projection::project(model, runtime)?;
    let definition = projection.definition;
    let issues = definition.validate();
    if !issues.is_empty() {
        return Err(AppError::invalid_definition(issues));
    }
    let document = serde_json::to_string(&definition).map_err(|_| AppError::internal())?;
    if document.len() > MAX_IMPORTED_DOCUMENT_BYTES {
        return Err(AppError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "definition_too_large",
            "Définition importée limitée à 8 Mio.",
        ));
    }
    let report = source.report();
    let report = json!({
        "sourceSchema":report.source_schema, "targetSchema":report.target_schema,
        "errors":report.errors, "warnings":report.warnings, "omittedDiagnostics":report.omitted_diagnostics,
        "checkedSections":report.checked_sections, "sections":report.sections, "diagnostics":report.diagnostics,
        "defaultProjection":projection.defaults
    });
    let xml =
        String::from_utf8(source.original_xml().to_vec()).map_err(|_| AppError::internal())?;
    let imported = repository::ImportedSource {
        xml,
        sha256: runtime::hex(&Sha256::digest(source.original_xml())),
        report,
    };
    Ok((definition, document, imported))
}

pub(super) async fn original(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    auth::authorize_read(&state, &headers).await?;
    if uuid::Uuid::try_parse(&id).is_err() {
        return Err(AppError::not_found());
    }
    let xml = repository::original(&state.database, &id).await?;
    let mut response = (
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    )
        .into_response();
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"original-{id}.lss\"")
            .parse()
            .map_err(|_| AppError::internal())?,
    );
    Ok(response)
}
