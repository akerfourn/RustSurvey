use super::super::{AppError, WorkspaceState, runtime};
use rustsurvey_limesurvey::{
    GROUP_SETTINGS_PROFILE, GroupSettingsReport, LssDocument, MAX_SETTINGS_CONTEXT_BYTES,
    SettingsContext,
};
use sea_orm::{
    ConnectionTrait, QueryResult, TransactionTrait,
    sea_query::{Expr, ExprTrait, Order, Query},
};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub(super) const MAX_CONTEXTS: i64 = 32;
const COLUMNS: [&str; 10] = [
    "id",
    "study_id",
    "definition_revision",
    "source_sha256",
    "sha256",
    "size",
    "profile",
    "is_complete",
    "created_at",
    "document",
];
const METADATA: [&str; 9] = [
    "id",
    "study_id",
    "definition_revision",
    "source_sha256",
    "sha256",
    "size",
    "profile",
    "is_complete",
    "created_at",
];

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::workspace) struct Metadata {
    pub id: String,
    pub study_id: String,
    pub revision: i64,
    pub source_sha256: String,
    pub sha256: String,
    pub size: i64,
    pub profile: String,
    pub complete: bool,
    pub created_at: i64,
}

pub(in crate::workspace) struct StoredContext {
    pub metadata: Metadata,
    pub document: String,
    pub report: String,
}

impl StoredContext {
    pub fn verify_bytes(&self) -> Result<(), AppError> {
        if self.document.len() > MAX_SETTINGS_CONTEXT_BYTES
            || self.document.len() as i64 != self.metadata.size
            || runtime::hex(&Sha256::digest(self.document.as_bytes())) != self.metadata.sha256
        {
            return Err(AppError::internal());
        }
        Ok(())
    }

    pub fn resolve(
        &self,
        document: &LssDocument,
        source_sha256: &str,
    ) -> Result<GroupSettingsReport, AppError> {
        self.verify_bytes()?;
        if self.metadata.profile != GROUP_SETTINGS_PROFILE
            || self.metadata.source_sha256 != source_sha256
        {
            return Err(AppError::new(
                axum::http::StatusCode::CONFLICT,
                "settings_context_profile_changed",
                "Le contexte ne correspond pas au profil ou à l'original source de ce bilan.",
            ));
        }
        let context: SettingsContext =
            serde_json::from_str(&self.document).map_err(|_| AppError::internal())?;
        let resolved = document
            .group_settings(Some(&context))
            .map_err(|_| AppError::internal())?;
        if self.report.len() > MAX_SETTINGS_CONTEXT_BYTES {
            return Err(AppError::internal());
        }
        let saved: serde_json::Value =
            serde_json::from_str(&self.report).map_err(|_| AppError::internal())?;
        if serde_json::to_value(&resolved).map_err(|_| AppError::internal())? != saved
            || resolved.complete != self.metadata.complete
        {
            return Err(AppError::new(
                axum::http::StatusCode::CONFLICT,
                "settings_context_resolution_changed",
                "La résolution diffère du bilan enregistré. Une nouvelle qualification explicite est nécessaire.",
            ));
        }
        Ok(resolved)
    }
}

fn metadata(row: &QueryResult) -> Result<Metadata, AppError> {
    Ok(Metadata {
        id: row.try_get("", "id")?,
        study_id: row.try_get("", "study_id")?,
        revision: row.try_get("", "definition_revision")?,
        source_sha256: row.try_get("", "source_sha256")?,
        sha256: row.try_get("", "sha256")?,
        size: row.try_get("", "size")?,
        profile: row.try_get("", "profile")?,
        complete: row.try_get("", "is_complete")?,
        created_at: row.try_get("", "created_at")?,
    })
}

pub(in crate::workspace) async fn selected(
    state: &WorkspaceState,
    study: &str,
    revision: i64,
    id: &str,
) -> Result<StoredContext, AppError> {
    super::valid_id(id)?;
    let row = state
        .database
        .query_one(
            &Query::select()
                .columns(COLUMNS)
                .column("report")
                .from("source_settings_contexts")
                .and_where(Expr::col("id").eq(id))
                .and_where(Expr::col("study_id").eq(study))
                .and_where(Expr::col("definition_revision").eq(revision))
                .to_owned(),
        )
        .await?
        .ok_or_else(AppError::not_found)?;
    Ok(StoredContext {
        metadata: metadata(&row)?,
        document: row.try_get("", "document")?,
        report: row.try_get("", "report")?,
    })
}

pub(super) async fn get(state: &WorkspaceState, id: &str) -> Result<Metadata, AppError> {
    let row = state
        .database
        .query_one(
            &Query::select()
                .columns(METADATA)
                .from("source_settings_contexts")
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await?
        .ok_or_else(AppError::not_found)?;
    metadata(&row)
}

pub(super) async fn list(
    state: &WorkspaceState,
    study: &str,
    revision: i64,
) -> Result<Vec<Metadata>, AppError> {
    let rows = state
        .database
        .query_all(
            &Query::select()
                .columns(METADATA)
                .from("source_settings_contexts")
                .and_where(Expr::col("study_id").eq(study))
                .and_where(Expr::col("definition_revision").eq(revision))
                .order_by("created_at", Order::Desc)
                .order_by("id", Order::Desc)
                .limit(MAX_CONTEXTS as u64)
                .to_owned(),
        )
        .await?;
    rows.iter().map(metadata).collect()
}

pub(super) async fn insert(
    state: &WorkspaceState,
    context: StoredContext,
) -> Result<(Metadata, bool), AppError> {
    let value = &context.metadata;
    let transaction = state.database.begin().await?;
    // Serialize the bounded count and insertion on the study, without changing its revision.
    transaction
        .execute(
            &Query::update()
                .table("studies")
                .value("revision", Expr::col("revision"))
                .and_where(Expr::col("id").eq(&value.study_id))
                .to_owned(),
        )
        .await?;
    let existing = transaction
        .query_one(
            &Query::select()
                .columns(METADATA)
                .from("source_settings_contexts")
                .and_where(Expr::col("study_id").eq(&value.study_id))
                .and_where(Expr::col("definition_revision").eq(value.revision))
                .and_where(Expr::col("profile").eq(&value.profile))
                .and_where(Expr::col("sha256").eq(&value.sha256))
                .to_owned(),
        )
        .await?;
    if let Some(row) = existing {
        let found = metadata(&row)?;
        transaction.commit().await?;
        return Ok((found, false));
    }
    let row = transaction
        .query_one(
            &Query::select()
                .expr_as(Expr::col("id").count(), "count")
                .from("source_settings_contexts")
                .and_where(Expr::col("study_id").eq(&value.study_id))
                .and_where(Expr::col("definition_revision").eq(value.revision))
                .to_owned(),
        )
        .await?
        .ok_or_else(AppError::internal)?;
    if row.try_get::<i64>("", "count")? >= MAX_CONTEXTS {
        return Err(AppError::new(
            axum::http::StatusCode::CONFLICT,
            "settings_context_limit",
            "Cette révision possède déjà 32 contextes. Les contextes existants restent accessibles.",
        ));
    }
    transaction
        .execute(
            Query::insert()
                .into_table("source_settings_contexts")
                .columns([
                    "id",
                    "study_id",
                    "definition_revision",
                    "source_sha256",
                    "sha256",
                    "size",
                    "profile",
                    "is_complete",
                    "created_at",
                    "document",
                    "report",
                ])
                .values([
                    value.id.clone().into(),
                    value.study_id.clone().into(),
                    value.revision.into(),
                    value.source_sha256.clone().into(),
                    value.sha256.clone().into(),
                    value.size.into(),
                    value.profile.clone().into(),
                    value.complete.into(),
                    value.created_at.into(),
                    context.document.into(),
                    context.report.into(),
                ])?,
        )
        .await?;
    transaction.commit().await?;
    Ok((context.metadata, true))
}
