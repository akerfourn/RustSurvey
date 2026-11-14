use super::{
    super::{AppError, WorkspaceState},
    prepare,
};
use sea_orm::{
    ConnectionTrait, QueryResult, TransactionTrait,
    sea_query::{Expr, ExprTrait, Order, Query},
};
use serde_json::{Value, json};

pub(super) struct Batch {
    pub id: String,
    pub study_id: String,
    pub revision: i64,
    pub source_sha256: String,
    pub sha256: String,
    pub object_key: String,
    pub size: i64,
    pub state: String,
    pub row_count: i64,
    pub loaded_rows: i64,
    pub schema: String,
    pub report: String,
    pub created_at: i64,
    pub updated_at: i64,
}

const COLUMNS: [&str; 15] = [
    "id",
    "study_id",
    "definition_revision",
    "source_sha256",
    "sha256",
    "object_key",
    "size",
    "state",
    "row_count",
    "loaded_rows",
    "schema_document",
    "report_document",
    "created_at",
    "updated_at",
    "write_version",
];

impl Batch {
    pub fn summary(&self) -> Result<Value, AppError> {
        let mut report: Value =
            serde_json::from_str(&self.report).map_err(|_| AppError::internal())?;
        if self.state == "pending" {
            report["conservation"] = json!("pending");
        }
        Ok(
            json!({"id":self.id,"studyId":self.study_id,"revision":self.revision,"sourceSha256":self.source_sha256,"sha256":self.sha256,"size":self.size,"state":self.state,"rowCount":self.row_count,"loadedRows":self.loaded_rows,"report":report,"createdAt":self.created_at,"updatedAt":self.updated_at}),
        )
    }
}

fn decode(row: QueryResult, with_schema: bool) -> Result<Batch, AppError> {
    let batch = Batch {
        id: row.try_get("", "id")?,
        study_id: row.try_get("", "study_id")?,
        revision: row.try_get("", "definition_revision")?,
        source_sha256: row.try_get("", "source_sha256")?,
        sha256: row.try_get("", "sha256")?,
        object_key: row.try_get("", "object_key")?,
        size: row.try_get("", "size")?,
        state: row.try_get("", "state")?,
        row_count: row.try_get("", "row_count")?,
        loaded_rows: row.try_get("", "loaded_rows")?,
        schema: if with_schema {
            row.try_get("", "schema_document")?
        } else {
            String::new()
        },
        report: row.try_get("", "report_document")?,
        created_at: row.try_get("", "created_at")?,
        updated_at: row.try_get("", "updated_at")?,
    };
    if batch.loaded_rows < 0
        || batch.loaded_rows > batch.row_count
        || batch.row_count > 100_000
        || !["pending", "loading", "prepared"].contains(&batch.state.as_str())
    {
        return Err(AppError::internal());
    }
    Ok(batch)
}

pub(super) async fn get(state: &WorkspaceState, id: &str) -> Result<Batch, AppError> {
    state
        .database
        .query_one(
            &Query::select()
                .columns(COLUMNS)
                .from("response_imports")
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await?
        .ok_or_else(AppError::not_found)
        .and_then(|row| decode(row, true))
}

pub(super) async fn metadata(state: &WorkspaceState, id: &str) -> Result<Batch, AppError> {
    let columns: Vec<_> = COLUMNS
        .into_iter()
        .filter(|column| *column != "schema_document")
        .collect();
    state
        .database
        .query_one(
            &Query::select()
                .columns(columns)
                .from("response_imports")
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await?
        .ok_or_else(AppError::not_found)
        .and_then(|row| decode(row, false))
}

pub(super) async fn find(
    state: &WorkspaceState,
    study: &str,
    revision: i64,
    hash: &str,
) -> Result<Option<Batch>, AppError> {
    state
        .database
        .query_one(
            &Query::select()
                .columns(COLUMNS)
                .from("response_imports")
                .and_where(Expr::col("study_id").eq(study))
                .and_where(Expr::col("definition_revision").eq(revision))
                .and_where(Expr::col("sha256").eq(hash))
                .to_owned(),
        )
        .await?
        .map(|row| decode(row, true))
        .transpose()
}

pub(super) async fn create(state: &WorkspaceState, batch: Batch) -> Result<Batch, AppError> {
    let result = state
        .database
        .execute(
            Query::insert()
                .into_table("response_imports")
                .columns([
                    "id",
                    "study_id",
                    "definition_revision",
                    "source_sha256",
                    "sha256",
                    "object_key",
                    "size",
                    "state",
                    "row_count",
                    "schema_document",
                    "report_document",
                    "created_at",
                    "updated_at",
                ])
                .values([
                    batch.id.clone().into(),
                    batch.study_id.clone().into(),
                    batch.revision.into(),
                    batch.source_sha256.clone().into(),
                    batch.sha256.clone().into(),
                    batch.object_key.clone().into(),
                    batch.size.into(),
                    "pending".into(),
                    batch.row_count.into(),
                    batch.schema.clone().into(),
                    batch.report.clone().into(),
                    batch.created_at.into(),
                    batch.updated_at.into(),
                ])?,
        )
        .await;
    if result.is_err() {
        if let Some(existing) = find(state, &batch.study_id, batch.revision, &batch.sha256).await? {
            return Ok(existing);
        }
        return Err(AppError::internal());
    }
    Ok(batch)
}

pub(super) async fn claim(
    state: &WorkspaceState,
    id: &str,
    token: &str,
    now: i64,
) -> Result<Option<i64>, AppError> {
    let transaction = state.database.begin().await?;
    let result = transaction
        .execute(
            &Query::update()
                .table("response_imports")
                .value("writer_token", token)
                .value("state", "loading")
                .value("updated_at", now)
                .value("write_version", Expr::col("write_version").add(1))
                .and_where(Expr::col("id").eq(id))
                .and_where(Expr::col("state").ne("prepared"))
                .and_where(Expr::col("write_version").lt(1_000_000_000_i64))
                .to_owned(),
        )
        .await?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    let row = transaction
        .query_one(
            &Query::select()
                .column("loaded_rows")
                .from("response_imports")
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await?
        .ok_or_else(AppError::internal)?;
    let loaded = row.try_get("", "loaded_rows")?;
    transaction.commit().await?;
    Ok(Some(loaded))
}

fn conflict() -> AppError {
    AppError::new(
        axum::http::StatusCode::CONFLICT,
        "response_import_superseded",
        "Une autre tentative a repris cet import. Actualisez son état.",
    )
}

pub(super) async fn append(
    state: &WorkspaceState,
    id: &str,
    token: &str,
    rows: &[prepare::Row],
    now: i64,
) -> Result<(), AppError> {
    let Some(first) = rows.first() else {
        return Ok(());
    };
    let transaction = state.database.begin().await?;
    let changed = transaction
        .execute(
            &Query::update()
                .table("response_imports")
                .value("write_version", Expr::col("write_version").add(1))
                .and_where(Expr::col("id").eq(id))
                .and_where(Expr::col("writer_token").eq(token))
                .and_where(Expr::col("loaded_rows").eq(first.ordinal))
                .and_where(Expr::col("state").eq("loading"))
                .to_owned(),
        )
        .await?;
    if changed.rows_affected() != 1 {
        return Err(conflict());
    }
    for row in rows {
        transaction
            .execute(
                Query::insert()
                    .into_table("response_import_records")
                    .columns([
                        "batch_id",
                        "ordinal",
                        "source_id",
                        "status",
                        "sha256",
                        "document",
                    ])
                    .values([
                        id.into(),
                        row.ordinal.into(),
                        row.source_id.clone().into(),
                        row.status.into(),
                        row.sha256.clone().into(),
                        row.document.clone().into(),
                    ])?,
            )
            .await?;
    }
    transaction
        .execute(
            &Query::update()
                .table("response_imports")
                .value("loaded_rows", first.ordinal + rows.len() as i64)
                .value("updated_at", now)
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await?;
    transaction.commit().await?;
    Ok(())
}

pub(super) async fn finish(
    state: &WorkspaceState,
    id: &str,
    token: &str,
    now: i64,
) -> Result<(), AppError> {
    let result = state
        .database
        .execute(
            &Query::update()
                .table("response_imports")
                .value("state", "prepared")
                .value("writer_token", Option::<String>::None)
                .value("updated_at", now)
                .value("write_version", Expr::col("write_version").add(1))
                .and_where(Expr::col("id").eq(id))
                .and_where(Expr::col("writer_token").eq(token))
                .and_where(Expr::col("loaded_rows").eq(Expr::col("row_count")))
                .to_owned(),
        )
        .await?;
    if result.rows_affected() != 1 {
        return Err(conflict());
    }
    Ok(())
}

pub(super) async fn list(state: &WorkspaceState, study: &str) -> Result<Vec<Value>, AppError> {
    let rows = state
        .database
        .query_all(
            &Query::select()
                .columns([
                    "id",
                    "definition_revision",
                    "state",
                    "row_count",
                    "loaded_rows",
                    "sha256",
                    "created_at",
                ])
                .from("response_imports")
                .and_where(Expr::col("study_id").eq(study))
                .order_by("created_at", Order::Desc)
                .order_by("id", Order::Desc)
                .limit(50)
                .to_owned(),
        )
        .await?;
    rows.into_iter().map(|row| Ok(json!({"id":row.try_get::<String>("", "id")?,"revision":row.try_get::<i64>("", "definition_revision")?,"state":row.try_get::<String>("", "state")?,"rowCount":row.try_get::<i64>("", "row_count")?,"loadedRows":row.try_get::<i64>("", "loaded_rows")?,"sha256":row.try_get::<String>("", "sha256")?,"createdAt":row.try_get::<i64>("", "created_at")?}))).collect()
}

pub(super) async fn records(
    state: &WorkspaceState,
    id: &str,
    from: i64,
) -> Result<Vec<Value>, AppError> {
    let rows = state
        .database
        .query_all(
            &Query::select()
                .columns(["ordinal", "source_id", "status", "sha256"])
                .from("response_import_records")
                .and_where(Expr::col("batch_id").eq(id))
                .and_where(Expr::col("ordinal").gte(from))
                .order_by("ordinal", Order::Asc)
                .limit(21)
                .to_owned(),
        )
        .await?;
    rows.into_iter().map(|row| Ok(json!({"ordinal":row.try_get::<i64>("", "ordinal")?,"sourceId":row.try_get::<Option<String>>("", "source_id")?,"status":row.try_get::<String>("", "status")?,"sha256":row.try_get::<String>("", "sha256")?}))).collect()
}

pub(super) async fn record(
    state: &WorkspaceState,
    id: &str,
    ordinal: i64,
) -> Result<StoredRecord, AppError> {
    let row = state
        .database
        .query_one(
            &Query::select()
                .columns(["document", "sha256"])
                .from("response_import_records")
                .and_where(Expr::col("batch_id").eq(id))
                .and_where(Expr::col("ordinal").eq(ordinal))
                .to_owned(),
        )
        .await?
        .ok_or_else(AppError::not_found)?;
    Ok(StoredRecord {
        document: row.try_get("", "document")?,
        sha256: row.try_get("", "sha256")?,
    })
}

pub(super) struct StoredRecord {
    pub document: String,
    pub sha256: String,
}
