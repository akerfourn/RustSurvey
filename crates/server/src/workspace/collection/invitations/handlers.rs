use super::{Invitation, repository};
use crate::workspace::{
    AppError, WorkspaceState, auth,
    collection::{access, participation, repository as runs, writes},
};
use axum::{
    Json,
    extract::{Path, Query as PageQuery, State},
    http::{HeaderMap, StatusCode},
};
use rustsurvey_domain::participation::InvitationLimits;
use sea_orm::{
    ConnectionTrait, TransactionTrait,
    sea_query::{Expr, ExprTrait, Query},
};
use serde::Deserialize;
use serde_json::{Value, json};

fn one_use() -> Option<u32> {
    Some(1)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in super::super) struct Create {
    id: String,
    token: String,
    #[serde(default = "one_use")]
    max_uses: Option<u32>,
    valid_from: Option<i64>,
    valid_until: Option<i64>,
}

#[derive(Deserialize)]
pub(in super::super) struct Page {
    #[serde(default)]
    offset: u64,
}

fn same_creation(
    stored: &Invitation,
    run_id: &str,
    hash: &str,
    limits: &InvitationLimits,
) -> Result<(), AppError> {
    if stored.run_id != run_id
        || !access::same_hash(&stored.secret_hash, hash)
        || &stored.limits != limits
    {
        return Err(AppError::new(
            StatusCode::CONFLICT,
            "invitation_conflict",
            "Cet identifiant d'invitation est déjà utilisé.",
        ));
    }
    Ok(())
}

pub(in super::super) async fn create(
    State(state): State<WorkspaceState>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
    body: Result<Json<Create>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    auth::authorize_write(&state, &headers).await?;
    let Json(input) = body.map_err(AppError::json)?;
    access::id(&input.id)?;
    let hash = access::secret_hash(&input.token)
        .map_err(|_| participation::invalid_policy("Secret d'invitation invalide."))?;
    let run = runs::run(&state.database, &run_id).await?;
    let limits = InvitationLimits {
        max_uses: input.max_uses,
        valid_from: input.valid_from,
        valid_until: input.valid_until,
    };
    limits
        .validate(&run.participation_policy)
        .map_err(participation::invalid_policy)?;
    if let Some(stored) = repository::by_id(&state.database, &input.id).await? {
        same_creation(&stored, &run_id, &hash, &limits)?;
        return Ok((StatusCode::CREATED, Json(json!(stored))));
    }
    let transaction = state.database.begin().await?;
    writes::lock_run(&transaction, &run_id, true).await?;
    if let Some(stored) = repository::by_id(&transaction, &input.id).await? {
        same_creation(&stored, &run_id, &hash, &limits)?;
        return Ok((StatusCode::CREATED, Json(json!(stored))));
    }
    if repository::by_secret(&transaction, &run_id, &hash)
        .await?
        .is_some()
    {
        return Err(AppError::new(
            StatusCode::CONFLICT,
            "invitation_conflict",
            "Ce secret est déjà utilisé pour cette campagne.",
        ));
    }
    if repository::count(&transaction, &run_id).await? >= 10_000 {
        return Err(AppError::new(
            StatusCode::CONFLICT,
            "invitation_limit",
            "Cette campagne atteint la limite de 10 000 invitations.",
        ));
    }
    transaction
        .execute(
            Query::insert()
                .into_table("invitations")
                .columns([
                    "id",
                    "run_id",
                    "secret_hash",
                    "max_uses",
                    "valid_from",
                    "valid_until",
                    "created_at",
                ])
                .values([
                    input.id.clone().into(),
                    run_id.into(),
                    hash.into(),
                    limits.max_uses.map(i64::from).into(),
                    limits.valid_from.into(),
                    limits.valid_until.into(),
                    state.runtime.now()?.into(),
                ])?,
        )
        .await?;
    transaction.commit().await?;
    let stored = repository::by_id(&state.database, &input.id)
        .await?
        .ok_or_else(AppError::internal)?;
    Ok((StatusCode::CREATED, Json(json!(stored))))
}

pub(in super::super) async fn list(
    State(state): State<WorkspaceState>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
    PageQuery(page): PageQuery<Page>,
) -> Result<Json<Value>, AppError> {
    auth::authorize_read(&state, &headers).await?;
    runs::run(&state.database, &run_id).await?;
    if page.offset > 10_000 {
        return Err(participation::invalid_policy("Pagination invalide."));
    }
    let mut items = repository::list(&state.database, &run_id, page.offset).await?;
    let has_more = items.len() > 50;
    items.truncate(50);
    Ok(Json(json!({"items": items, "hasMore": has_more})))
}

pub(in super::super) async fn revoke(
    State(state): State<WorkspaceState>,
    Path((run_id, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    auth::authorize_write(&state, &headers).await?;
    access::id(&id)?;
    runs::run(&state.database, &run_id).await?;
    let transaction = state.database.begin().await?;
    writes::lock_run(&transaction, &run_id, false).await?;
    let stored = repository::by_id(&transaction, &id)
        .await?
        .ok_or_else(participation::denied)?;
    if stored.run_id != run_id {
        return Err(participation::denied());
    }
    if stored.revoked {
        return Ok(Json(json!(stored)));
    }
    let updated = transaction
        .execute(
            &Query::update()
                .table("invitations")
                .value("revoked", true)
                .and_where(Expr::col("id").eq(&id))
                .and_where(Expr::col("run_id").eq(&run_id))
                .to_owned(),
        )
        .await?;
    if updated.rows_affected() != 1 {
        return Err(participation::denied());
    }
    transaction.commit().await?;
    Ok(Json(json!(
        repository::by_id(&state.database, &id)
            .await?
            .ok_or_else(AppError::internal)?
    )))
}
