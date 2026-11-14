use super::{AppError, WorkspaceState, auth, history, repository};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use rustsurvey_domain::{Definition, navigation::NavigationSettings};
use rustsurvey_limesurvey::{Error, Limits, LssDocument, RelevanceEdit, RelevanceTarget, SourceId};
use serde::Deserialize;

fn default_survey_id() -> u64 {
    1
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct NativeOptions {
    #[serde(default = "default_survey_id")]
    pub(super) survey_id: u64,
}

pub(super) async fn native(
    State(state): State<WorkspaceState>,
    Path((id, revision)): Path<(String, i64)>,
    headers: HeaderMap,
    Query(options): Query<NativeOptions>,
) -> Result<Response, AppError> {
    auth::authorize_read(&state, &headers).await?;
    let saved = history::load(&state, &id, revision).await?;
    if repository::import_info(&state.database, &id)
        .await?
        .is_some()
    {
        return Err(blocked());
    }
    native_response(
        state,
        saved.definition,
        options,
        NavigationSettings::default(),
        format!("survey-{id}-r{revision}.lss"),
    )
    .await
}

pub(super) async fn native_response(
    state: WorkspaceState,
    definition: Definition,
    options: NativeOptions,
    navigation: NavigationSettings,
    filename: String,
) -> Result<Response, AppError> {
    let permit = state.imports.clone().try_acquire_owned().map_err(|_| {
        AppError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "export_busy",
            "Deux traitements d'import/export sont déjà en cours. Réessayez.",
        )
    })?;
    let xml = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        rustsurvey_limesurvey::export_native_with_navigation(
            &definition,
            SourceId(options.survey_id),
            navigation,
            Limits::default(),
        )
        .map(|exported| exported.xml)
        .map_err(native_error)
    })
    .await
    .map_err(|_| AppError::internal())??;
    Ok((
        [
            (header::CONTENT_TYPE, "application/xml; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{filename}\""),
            ),
        ],
        xml,
    )
        .into_response())
}

fn native_error(error: rustsurvey_limesurvey::NativeExportError) -> AppError {
    use rustsurvey_limesurvey::NativeExportError;
    match error {
        NativeExportError::Unsupported(issues) => AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "native_lss_export_blocked",
            "Certains éléments ne peuvent pas être exportés sans perte dans ce LSS.",
        )
        .with_issues(issues),
        NativeExportError::Format(Error::LimitExceeded { .. }) => AppError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "export_too_large",
            "L'export LSS dépasse les limites XML prises en charge.",
        ),
        NativeExportError::Format(Error::InvalidXmlCharacter) => AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_xml_character",
            "La définition contient un caractère non représentable en XML 1.0.",
        ),
        _ => blocked(),
    }
}

fn blocked() -> AppError {
    AppError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "lss_export_blocked",
        "Le réexport modifié est bloqué : correspondance source absente ou structure non prise en charge.",
    )
}
fn alias(source: Option<&rustsurvey_domain::SourceReference>) -> Result<SourceId, AppError> {
    let id = source
        .ok_or_else(blocked)?
        .id
        .parse()
        .map_err(|_| blocked())?;
    Ok(SourceId(id))
}

pub(super) async fn export(
    State(state): State<WorkspaceState>,
    Path((id, revision)): Path<(String, i64)>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    auth::authorize_read(&state, &headers).await?;
    let saved = history::load(&state, &id, revision).await?;
    let original = repository::original(&state.database, &id).await?;
    let permit = state.imports.clone().try_acquire_owned().map_err(|_| {
        AppError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "export_busy",
            "Deux traitements LSS sont déjà en cours. Réessayez.",
        )
    })?;
    let xml = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let source = LssDocument::parse(original.as_bytes(), Limits::default()).map_err(|_| blocked())?;
        let mut edits = Vec::new();
        for group in &saved.definition.groups {
            edits.push(RelevanceEdit { target: RelevanceTarget::Group, id: alias(group.source.as_ref())?, expression: group.relevance.clone() });
            for question in &group.questions { edits.push(RelevanceEdit { target: RelevanceTarget::Question, id: alias(question.source.as_ref())?, expression: question.relevance.clone() }); }
        }
        let updated = source.with_relevances(&edits).map_err(|error| match error {
            Error::LegacyConditionsPreventEdit => AppError::new(StatusCode::UNPROCESSABLE_ENTITY, "legacy_conditions_block_export", "Les anciennes conditions LimeSurvey empêchent le réexport modifié. L’original reste disponible."),
            Error::LimitExceeded { .. } => AppError::new(StatusCode::PAYLOAD_TOO_LARGE, "export_too_large", "Le LSS modifié dépasse les limites du format pris en charge."),
            _ => blocked(),
        })?;
        Ok::<_, AppError>(updated.original_xml().to_vec())
    }).await.map_err(|_| AppError::internal())??;
    Ok((
        [
            (header::CONTENT_TYPE, "application/xml; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"modified-{id}-r{revision}.lss\""),
            ),
        ],
        xml,
    )
        .into_response())
}
