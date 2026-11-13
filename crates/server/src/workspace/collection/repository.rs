use super::{AppError, Document, Run, WorkspaceState, access, missing};
use sea_orm::{
    ConnectionTrait, QueryResult,
    sea_query::{Expr, ExprTrait, Order, Query},
};
use serde::Serialize;

const RESPONSE_COLUMNS: [&str; 7] = [
    "id",
    "run_id",
    "version",
    "status",
    "document",
    "created_at",
    "updated_at",
];

pub(super) struct StoredResponse {
    pub id: String,
    pub run_id: String,
    pub version: i64,
    pub status: String,
    pub document: Document,
    pub created_at: i64,
    pub updated_at: i64,
}

pub(super) struct ResponseAccess {
    pub run_id: String,
    pub access_hash: String,
    pub start_hash: String,
}

pub(super) async fn access(
    connection: &impl ConnectionTrait,
    id: &str,
) -> Result<Option<ResponseAccess>, AppError> {
    let row = connection
        .query_one(
            &Query::select()
                .columns(["run_id", "access_hash", "start_hash"])
                .from("responses")
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await?;
    row.map(|row| {
        Ok(ResponseAccess {
            run_id: row.try_get("", "run_id")?,
            access_hash: row.try_get("", "access_hash")?,
            start_hash: row.try_get("", "start_hash")?,
        })
    })
    .transpose()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Summary {
    id: String,
    version: i64,
    status: String,
    created_at: i64,
    updated_at: i64,
}

fn run_row(row: &QueryResult) -> Result<Run, AppError> {
    Ok(Run {
        id: row.try_get("", "id")?,
        study_id: row.try_get("", "study_id")?,
        revision: row.try_get("", "definition_revision")?,
        profile: row.try_get("", "profile")?,
        is_open: row.try_get("", "is_open")?,
        created_at: row.try_get("", "created_at")?,
        participation_policy: serde_json::from_str(
            &row.try_get::<String>("", "participation_policy")?,
        )
        .map_err(|_| AppError::internal())?,
    })
}

pub(super) async fn run(connection: &impl ConnectionTrait, id: &str) -> Result<Run, AppError> {
    access::id(id)?;
    let row = connection
        .query_one(
            &Query::select()
                .columns([
                    "id",
                    "study_id",
                    "definition_revision",
                    "profile",
                    "is_open",
                    "created_at",
                    "participation_policy",
                ])
                .from("runs")
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await?
        .ok_or_else(missing)?;
    run_row(&row)
}

pub(super) async fn runs(state: &WorkspaceState, id: &str) -> Result<Vec<Run>, AppError> {
    let rows = state
        .database
        .query_all(
            &Query::select()
                .columns([
                    "id",
                    "study_id",
                    "definition_revision",
                    "profile",
                    "is_open",
                    "created_at",
                    "participation_policy",
                ])
                .from("runs")
                .and_where(Expr::col("study_id").eq(id))
                .order_by("created_at", Order::Desc)
                .order_by("id", Order::Desc)
                .limit(50)
                .to_owned(),
        )
        .await?;
    rows.iter().map(run_row).collect()
}

pub(super) async fn find(
    connection: &impl ConnectionTrait,
    id: &str,
) -> Result<Option<StoredResponse>, AppError> {
    let row = connection
        .query_one(
            &Query::select()
                .columns(RESPONSE_COLUMNS)
                .from("responses")
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await?;
    row.map(|row| {
        let document: String = row.try_get("", "document")?;
        Ok(StoredResponse {
            id: row.try_get("", "id")?,
            run_id: row.try_get("", "run_id")?,
            version: row.try_get("", "version")?,
            status: row.try_get("", "status")?,
            document: serde_json::from_str(&document).map_err(|_| AppError::internal())?,
            created_at: row.try_get("", "created_at")?,
            updated_at: row.try_get("", "updated_at")?,
        })
    })
    .transpose()
}

pub(super) async fn authorized(
    state: &WorkspaceState,
    id: &str,
    hash: &str,
) -> Result<StoredResponse, AppError> {
    access::id(id)?;
    let response = access(&state.database, id).await?.ok_or_else(missing)?;
    if !access::same_hash(&response.access_hash, hash) {
        return Err(missing());
    }
    super::participation::authorize_response(&state.database, id, state.runtime.now()?).await?;
    find(&state.database, id).await?.ok_or_else(missing)
}

pub(super) async fn summaries(
    state: &WorkspaceState,
    run: &str,
    offset: u64,
) -> Result<Vec<Summary>, AppError> {
    let rows = state
        .database
        .query_all(
            &Query::select()
                .columns(["id", "version", "status", "created_at", "updated_at"])
                .from("responses")
                .and_where(Expr::col("run_id").eq(run))
                .order_by("created_at", Order::Desc)
                .order_by("id", Order::Asc)
                .offset(offset)
                .limit(21)
                .to_owned(),
        )
        .await?;
    rows.iter()
        .map(|row| {
            Ok(Summary {
                id: row.try_get("", "id")?,
                version: row.try_get("", "version")?,
                status: row.try_get("", "status")?,
                created_at: row.try_get("", "created_at")?,
                updated_at: row.try_get("", "updated_at")?,
            })
        })
        .collect()
}
