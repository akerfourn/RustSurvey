use super::{
    AppError, Checkpoint, Document, Run, WorkspaceState, access, closed, conflict,
    handlers::{Start, Update},
    participation,
    repository::{self, ResponseAccess, StoredResponse},
};
use axum::http::StatusCode;
use sea_orm::{
    ConnectionTrait, TransactionTrait,
    sea_query::{Expr, ExprTrait, Query},
};
use serde_json::{Value, json};

const RECEIPT_SECONDS: i64 = 24 * 60 * 60;
const MAX_RECEIPTS: usize = 1024;

pub(super) async fn lock_run(
    connection: &impl ConnectionTrait,
    run_id: &str,
    require_open: bool,
) -> Result<(), AppError> {
    // A real update serializes closure with writes on all three SQL engines.
    let mut update = Query::update();
    update
        .table("runs")
        .value("write_version", Expr::col("write_version").add(1_i64))
        .and_where(Expr::col("id").eq(run_id));
    if require_open {
        update.and_where(Expr::col("is_open").eq(true));
    }
    let result = connection.execute(&update).await?;
    if result.rows_affected() != 1 {
        return Err(closed());
    }
    Ok(())
}

pub(super) fn start_hash(run_id: &str, input: &Start) -> Result<String, AppError> {
    let bytes = serde_json::to_vec(&(run_id, input)).map_err(|_| AppError::internal())?;
    Ok(access::digest(&bytes))
}

fn same_start(
    stored: &ResponseAccess,
    run_id: &str,
    hash: &str,
    request_hash: &str,
) -> Result<(), AppError> {
    if stored.run_id != run_id || !access::same_hash(&stored.access_hash, hash) {
        return Err(super::missing());
    }
    if stored.start_hash != request_hash {
        return Err(conflict());
    }
    Ok(())
}

pub(super) async fn start(
    state: &WorkspaceState,
    run: &Run,
    input: &Start,
    hash: &str,
    group_id: Option<String>,
    invitation_hash: Option<&str>,
    browser_completed: bool,
) -> Result<String, AppError> {
    let run_id = &run.id;
    let request_hash = start_hash(run_id, input)?;
    if let Some(stored) = repository::access(&state.database, &input.response_id).await? {
        same_start(&stored, run_id, hash, &request_hash)?;
        return Ok(input.response_id.clone());
    }
    let transaction = state.database.begin().await?;
    lock_run(&transaction, run_id, true).await?;
    if let Some(stored) = repository::access(&transaction, &input.response_id).await? {
        same_start(&stored, run_id, hash, &request_hash)?;
        return Ok(input.response_id.clone());
    }
    let invitation = participation::admit_start(
        &transaction,
        run,
        invitation_hash,
        hash,
        browser_completed,
        state.runtime.now()?,
    )
    .await?;
    if run.participation_policy.persists_response()
        && let Some(id) = invitation
            .as_ref()
            .and_then(|invitation| invitation.response_id.as_ref())
    {
        return Ok(id.clone());
    }
    let document = Document {
        answers: Default::default(),
        checkpoint: Checkpoint {
            language: input.language.clone(),
            group_id,
        },
    };
    let document = serde_json::to_string(&document).map_err(|_| AppError::internal())?;
    let now = state.runtime.now()?;
    transaction
        .execute(
            Query::insert()
                .into_table("responses")
                .columns([
                    "id",
                    "run_id",
                    "access_hash",
                    "start_hash",
                    "version",
                    "status",
                    "document",
                    "created_at",
                    "updated_at",
                ])
                .values([
                    input.response_id.clone().into(),
                    run_id.clone().into(),
                    hash.into(),
                    request_hash.into(),
                    0_i64.into(),
                    "draft".into(),
                    document.into(),
                    now.into(),
                    now.into(),
                ])?,
        )
        .await?;
    if run.participation_policy.persists_response() {
        let invitation = invitation.ok_or_else(AppError::internal)?;
        transaction
            .execute(
                &Query::update()
                    .table("invitations")
                    .value("response_id", input.response_id.clone())
                    .and_where(Expr::col("id").eq(invitation.id))
                    .to_owned(),
            )
            .await?;
    }
    transaction.commit().await?;
    Ok(input.response_id.clone())
}

pub(super) async fn receipt(
    connection: &impl ConnectionTrait,
    id: &str,
    operation: &str,
    hash: &str,
    now: i64,
) -> Result<Option<Value>, AppError> {
    let row = connection
        .query_one(
            &Query::select()
                .columns(["request_hash", "result"])
                .from("operation_receipts")
                .and_where(Expr::col("response_id").eq(id))
                .and_where(Expr::col("operation_id").eq(operation))
                .and_where(Expr::col("expires_at").gt(now))
                .to_owned(),
        )
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row.try_get::<String>("", "request_hash")? != hash {
        return Err(conflict());
    }
    let result: String = row.try_get("", "result")?;
    Ok(Some(
        serde_json::from_str(&result).map_err(|_| AppError::internal())?,
    ))
}

pub(super) async fn commit(
    state: &WorkspaceState,
    stored: &StoredResponse,
    input: &Update,
    document: Document,
    warnings: Vec<rustsurvey_domain::ValidationIssue>,
    invitation_hash: Option<&str>,
    browser_completed: bool,
) -> Result<Value, AppError> {
    let now = state.runtime.now()?;
    let request_hash = input.hash()?;
    let transaction = state.database.begin().await?;
    lock_run(&transaction, &stored.run_id, true).await?;
    if let Some(result) = receipt(
        &transaction,
        &stored.id,
        &input.operation_id,
        &request_hash,
        now,
    )
    .await?
    {
        return Ok(result);
    }
    let run = repository::run(&transaction, &stored.run_id).await?;
    if stored.status != "draft" && !run.participation_policy.allows_edit() {
        return Err(conflict());
    }
    if input.submit {
        participation::check_browser(browser_completed)?;
    }
    participation::admit_write(
        &transaction,
        &run,
        &stored.id,
        invitation_hash,
        input.submit,
        now,
    )
    .await?;
    transaction
        .execute(
            &Query::delete()
                .from_table("operation_receipts")
                .and_where(Expr::col("response_id").eq(&stored.id))
                .and_where(Expr::col("expires_at").lte(now))
                .to_owned(),
        )
        .await?;
    let count = transaction
        .query_all(
            &Query::select()
                .column("operation_id")
                .from("operation_receipts")
                .and_where(Expr::col("response_id").eq(&stored.id))
                .limit(MAX_RECEIPTS as u64)
                .to_owned(),
        )
        .await?
        .len();
    if count >= MAX_RECEIPTS {
        return Err(AppError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "response_operation_limit",
            "La limite de sauvegardes sur 24 heures est atteinte pour cette participation.",
        ));
    }
    let status = if input.submit { "submitted" } else { "draft" };
    let result = json!({"id": stored.id, "version": input.expected_version + 1, "status": status, "checkpoint": document.checkpoint, "updatedAt": now, "warnings": warnings});
    let document = serde_json::to_string(&document).map_err(|_| AppError::internal())?;
    let updated = transaction
        .execute(
            &Query::update()
                .table("responses")
                .value("version", input.expected_version + 1)
                .value("status", status)
                .value("document", document)
                .value("updated_at", now)
                .and_where(Expr::col("id").eq(&stored.id))
                .and_where(Expr::col("version").eq(input.expected_version))
                .and_where(Expr::col("status").eq(&stored.status))
                .to_owned(),
        )
        .await?;
    if updated.rows_affected() != 1 {
        return Err(conflict());
    }
    transaction
        .execute(
            Query::insert()
                .into_table("operation_receipts")
                .columns([
                    "response_id",
                    "operation_id",
                    "request_hash",
                    "result",
                    "expires_at",
                ])
                .values([
                    stored.id.clone().into(),
                    input.operation_id.clone().into(),
                    request_hash.into(),
                    result.to_string().into(),
                    (now + RECEIPT_SECONDS).into(),
                ])?,
        )
        .await?;
    transaction.commit().await?;
    Ok(result)
}
