use super::{AppError, Invitation};
use rustsurvey_domain::participation::InvitationLimits;
use sea_orm::{
    ConnectionTrait, DbBackend, QueryResult,
    sea_query::{Expr, ExprTrait, Order, Query, SimpleExpr},
};

const COLUMNS: [&str; 10] = [
    "id",
    "run_id",
    "secret_hash",
    "max_uses",
    "uses",
    "valid_from",
    "valid_until",
    "revoked",
    "response_id",
    "created_at",
];

pub(super) async fn count(
    connection: &impl ConnectionTrait,
    run_id: &str,
) -> Result<i64, AppError> {
    let row = connection
        .query_one(
            &Query::select()
                .expr(Expr::col("id").count())
                .from("invitations")
                .and_where(Expr::col("run_id").eq(run_id))
                .to_owned(),
        )
        .await?
        .ok_or_else(AppError::internal)?;
    if connection.get_database_backend() == DbBackend::MySql {
        return row
            .try_get_by_index::<u64>(0)?
            .try_into()
            .map_err(|_| AppError::internal());
    }
    Ok(row.try_get_by_index::<i64>(0)?)
}

fn decode(row: QueryResult) -> Result<Invitation, AppError> {
    let maximum: Option<i64> = row.try_get("", "max_uses")?;
    let max_uses = maximum
        .map(u32::try_from)
        .transpose()
        .map_err(|_| AppError::internal())?;
    Ok(Invitation {
        id: row.try_get("", "id")?,
        run_id: row.try_get("", "run_id")?,
        secret_hash: row.try_get("", "secret_hash")?,
        limits: InvitationLimits {
            max_uses,
            valid_from: row.try_get("", "valid_from")?,
            valid_until: row.try_get("", "valid_until")?,
        },
        uses: row.try_get("", "uses")?,
        revoked: row.try_get("", "revoked")?,
        response_id: row.try_get("", "response_id")?,
        created_at: row.try_get("", "created_at")?,
    })
}

async fn find(
    connection: &impl ConnectionTrait,
    condition: SimpleExpr,
) -> Result<Option<Invitation>, AppError> {
    connection
        .query_one(
            &Query::select()
                .columns(COLUMNS)
                .from("invitations")
                .and_where(condition)
                .to_owned(),
        )
        .await?
        .map(decode)
        .transpose()
}

pub(in super::super) async fn by_id(
    connection: &impl ConnectionTrait,
    id: &str,
) -> Result<Option<Invitation>, AppError> {
    find(connection, Expr::col("id").eq(id)).await
}

pub(in super::super) async fn by_secret(
    connection: &impl ConnectionTrait,
    run_id: &str,
    hash: &str,
) -> Result<Option<Invitation>, AppError> {
    find(
        connection,
        Expr::col("run_id")
            .eq(run_id)
            .and(Expr::col("secret_hash").eq(hash)),
    )
    .await
}

pub(in super::super) async fn by_response(
    connection: &impl ConnectionTrait,
    id: &str,
) -> Result<Option<Invitation>, AppError> {
    find(connection, Expr::col("response_id").eq(id)).await
}

pub(super) async fn list(
    connection: &impl ConnectionTrait,
    run_id: &str,
    offset: u64,
) -> Result<Vec<Invitation>, AppError> {
    connection
        .query_all(
            &Query::select()
                .columns(COLUMNS)
                .from("invitations")
                .and_where(Expr::col("run_id").eq(run_id))
                .order_by("created_at", Order::Desc)
                .order_by("id", Order::Asc)
                .offset(offset)
                .limit(51)
                .to_owned(),
        )
        .await?
        .into_iter()
        .map(decode)
        .collect()
}
