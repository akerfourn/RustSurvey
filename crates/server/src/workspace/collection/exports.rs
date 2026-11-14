use super::super::auth;
use super::{
    AppError, WorkspaceState,
    export_body::{ExportBody, Message},
    export_time::utc_datetime,
    repository,
};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use rustsurvey_limesurvey::{
    Limits, LssDocument, NativeExportError, SourceId, VvEncoder, native_bindings, source_bindings,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{OwnedSemaphorePermit, mpsc};

struct Schema {
    encoder: VvEncoder,
    keys: BTreeSet<String>,
    legacy_language: Option<String>,
}

fn blocked() -> AppError {
    AppError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "vv_export_blocked",
        "La définition ne permet pas un échange VV non ambigu avec LimeSurvey.",
    )
}

impl Schema {
    fn new(definition: &rustsurvey_domain::Definition) -> Result<Self, AppError> {
        let bindings = native_bindings(definition, SourceId(1)).map_err(|error| match error {
            NativeExportError::Unsupported(issues) => blocked().with_issues(issues),
            _ => blocked(),
        })?;
        let keys: BTreeSet<_> = bindings
            .into_iter()
            .map(|binding| binding.runtime_key)
            .collect();
        Self::from_keys(definition, keys, true)
    }

    fn imported(
        definition: &rustsurvey_domain::Definition,
        original: &str,
        sha256: &str,
    ) -> Result<Self, AppError> {
        if super::access::digest(original.as_bytes()) != sha256 {
            return Err(AppError::internal());
        }
        let source =
            LssDocument::parse(original.as_bytes(), Limits::default()).map_err(|_| blocked())?;
        let mapping = source_bindings(definition, source.source_model().ok_or_else(blocked)?)
            .map_err(|_| blocked())?;
        if mapping.issue_count != 0 {
            return Err(blocked().with_issues(mapping.issues));
        }
        let keys = mapping
            .fields
            .into_iter()
            .map(|field| field.runtime_key)
            .collect();
        Self::from_keys(definition, keys, false)
    }

    fn from_keys(
        definition: &rustsurvey_domain::Definition,
        keys: BTreeSet<String>,
        legacy_fallback: bool,
    ) -> Result<Self, AppError> {
        let mut columns = vec!["submitdate".to_string(), "startlanguage".to_string()];
        columns.extend(keys.iter().cloned());
        let encoder =
            VvEncoder::new(columns, &BTreeMap::new(), Limits::default()).map_err(|_| blocked())?;
        let languages: BTreeSet<_> = std::iter::once(&definition.language)
            .chain(&definition.languages)
            .collect();
        let legacy_language = if legacy_fallback && languages.len() == 1 {
            Some(definition.language.clone())
        } else {
            None
        };
        Ok(Self {
            encoder,
            keys,
            legacy_language,
        })
    }

    fn row(&self, stored: repository::ExportRecord) -> Result<Vec<u8>, AppError> {
        if stored.document.len() > rustsurvey_domain::MAX_DOCUMENT_BYTES {
            return Err(AppError::internal());
        }
        let document: super::Document =
            serde_json::from_str(&stored.document).map_err(|_| AppError::internal())?;
        if document.answers.keys().any(|key| !self.keys.contains(key)) {
            return Err(AppError::internal());
        }
        let submitted = match stored.status.as_str() {
            "submitted" => Some(utc_datetime(stored.updated_at)?),
            "draft" => None,
            _ => return Err(AppError::internal()),
        };
        let mut row: BTreeMap<_, _> = document
            .answers
            .into_iter()
            .map(|(key, value)| (key, Some(value)))
            .collect();
        row.insert("submitdate".into(), submitted);
        row.insert(
            "startlanguage".into(),
            Some(
                stored
                    .initial_language
                    .or_else(|| self.legacy_language.clone())
                    .ok_or_else(AppError::internal)?,
            ),
        );
        self.encoder.row(&row).map_err(|_| AppError::internal())
    }
}

pub(super) async fn vv(
    State(state): State<WorkspaceState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    auth::authorize_read(&state, &headers).await?;
    let run = repository::run(&state.database, &id).await?;
    if run.is_open {
        return Err(AppError::new(
            StatusCode::CONFLICT,
            "export_requires_closed_run",
            "Fermez la campagne avant d'exporter un ensemble de réponses cohérent.",
        ));
    }
    let permit = Arc::new(state.imports.clone().try_acquire_owned().map_err(|_| {
        AppError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "export_busy",
            "Deux traitements sont déjà en cours. Réessayez.",
        )
    })?);
    let definition =
        super::source_execution::guarded_definition(&state, &run, permit.clone()).await?;
    let source = if run.has_execution {
        let metadata = super::super::repository::import_info(&state.database, &run.study_id)
            .await?
            .ok_or_else(AppError::internal)?;
        Some((
            super::super::repository::original(&state.database, &run.study_id).await?,
            metadata.sha256,
        ))
    } else {
        None
    };
    let guard = permit.clone();
    let schema = Arc::new(
        tokio::task::spawn_blocking(move || {
            let _guard = guard;
            match source {
                Some((original, sha256)) => Schema::imported(&definition, &original, &sha256),
                None => Schema::new(&definition),
            }
        })
        .await
        .map_err(|_| AppError::internal())??,
    );
    if schema.legacy_language.is_none() && repository::missing_initial_language(&state, &id).await?
    {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "initial_language_unavailable",
            "La langue initiale de réponses anciennes n'est pas connue. Leur JSON natif reste disponible ; aucune langue n'est devinée pour l'export.",
        ));
    }
    let (sender, receiver) = mpsc::channel(1);
    let guard = permit.clone();
    let filename = format!("attachment; filename=\"responses-{id}.vv\"");
    let task = tokio::spawn(async move {
        match tokio::time::timeout(
            Duration::from_secs(300),
            produce(state, id, schema, sender, guard),
        )
        .await
        {
            Ok(Ok(())) => {}
            _ => tracing::warn!(
                event = "response_export_interrupted",
                "Response download did not complete"
            ),
        }
    });
    Ok((
        [
            (
                header::CONTENT_TYPE,
                "text/tab-separated-values; charset=utf-8",
            ),
            (header::CONTENT_DISPOSITION, filename.as_str()),
        ],
        Body::new(ExportBody::new(receiver, task, permit)),
    )
        .into_response())
}

async fn produce(
    state: WorkspaceState,
    id: String,
    schema: Arc<Schema>,
    sender: mpsc::Sender<Message>,
    permit: Arc<OwnedSemaphorePermit>,
) -> Result<(), AppError> {
    sender
        .send(Message::Data(schema.encoder.header().to_vec()))
        .await
        .map_err(|_| AppError::internal())?;
    let mut after = String::new();
    loop {
        let ids = repository::export_ids(&state, &id, &after).await?;
        if ids.is_empty() {
            break;
        }
        for response_id in ids {
            // Reserve queue capacity before reading or encoding the next private document.
            let slot = sender.reserve().await.map_err(|_| AppError::internal())?;
            let stored = repository::export_record(&state, &response_id).await?;
            if stored.run_id != id {
                return Err(AppError::internal());
            }
            after = response_id;
            let schema = schema.clone();
            let guard = permit.clone();
            let bytes = tokio::task::spawn_blocking(move || {
                let _guard = guard;
                schema.row(stored)
            })
            .await
            .map_err(|_| AppError::internal())??;
            slot.send(Message::Data(bytes));
        }
    }
    sender
        .send(Message::Finished)
        .await
        .map_err(|_| AppError::internal())?;
    Ok(())
}
