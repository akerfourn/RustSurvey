use super::AppError;
use sea_orm::{
    ConnectionTrait, DatabaseConnection,
    sea_query::{Expr, ExprTrait, Query},
};

pub(super) struct Account {
    pub username: String,
    pub password_hash: String,
    pub failures: i32,
    pub blocked_until: i64,
}
pub(super) async fn account(database: &DatabaseConnection) -> Result<Option<Account>, AppError> {
    let row = database
        .query_one(
            &Query::select()
                .columns(["username", "password_hash", "failures", "blocked_until"])
                .from("administrator")
                .and_where(Expr::col("id").eq(1))
                .to_owned(),
        )
        .await?;
    row.map(|row| {
        Ok(Account {
            username: row.try_get("", "username")?,
            password_hash: row.try_get("", "password_hash")?,
            failures: row.try_get("", "failures")?,
            blocked_until: row.try_get("", "blocked_until")?,
        })
    })
    .transpose()
}
pub(super) async fn insert_account(
    database: &DatabaseConnection,
    username: String,
    hash: String,
) -> Result<bool, AppError> {
    let result = database
        .execute(
            Query::insert()
                .into_table("administrator")
                .columns([
                    "id",
                    "username",
                    "password_hash",
                    "failures",
                    "blocked_until",
                ])
                .values([
                    1.into(),
                    username.into(),
                    hash.into(),
                    0.into(),
                    0_i64.into(),
                ])?,
        )
        .await;
    match result {
        Ok(_) => Ok(true),
        Err(_) if account(database).await?.is_some() => Ok(false),
        Err(_) => Err(AppError::internal()),
    }
}
pub(super) async fn reset_failures(database: &DatabaseConnection) -> Result<(), AppError> {
    database
        .execute(
            &Query::update()
                .table("administrator")
                .value("failures", 0)
                .value("blocked_until", 0_i64)
                .and_where(Expr::col("id").eq(1))
                .to_owned(),
        )
        .await?;
    Ok(())
}
pub(super) async fn record_failure(
    database: &DatabaseConnection,
    now: i64,
) -> Result<(), AppError> {
    for _ in 0..3 {
        let current = account(database)
            .await?
            .ok_or_else(AppError::unauthorized)?;
        if current.blocked_until > now {
            return Ok(());
        }
        let next = std::cmp::min(current.failures.saturating_add(1), 5);
        let blocked_until = if next >= 5 { now + 60 } else { 0 };
        let result = database
            .execute(
                &Query::update()
                    .table("administrator")
                    .value("failures", next)
                    .value("blocked_until", blocked_until)
                    .and_where(Expr::col("id").eq(1))
                    .and_where(Expr::col("failures").eq(current.failures))
                    .and_where(Expr::col("blocked_until").eq(current.blocked_until))
                    .to_owned(),
            )
            .await?;
        if result.rows_affected() == 1 {
            return Ok(());
        }
    }
    Err(AppError::internal())
}
pub(super) struct Session {
    pub csrf_token: String,
}
pub(super) async fn session(
    database: &DatabaseConnection,
    token_hash: &str,
    now: i64,
) -> Result<Option<Session>, AppError> {
    let row = database
        .query_one(
            &Query::select()
                .column("csrf_token")
                .from("administrator_sessions")
                .and_where(Expr::col("token_hash").eq(token_hash))
                .and_where(Expr::col("expires_at").gt(now))
                .to_owned(),
        )
        .await?;
    row.map(|row| {
        Ok(Session {
            csrf_token: row.try_get("", "csrf_token")?,
        })
    })
    .transpose()
}
pub(super) async fn insert_session(
    database: &DatabaseConnection,
    token_hash: String,
    csrf: String,
    now: i64,
    expires: i64,
) -> Result<(), AppError> {
    database
        .execute(
            &Query::delete()
                .from_table("administrator_sessions")
                .and_where(Expr::col("expires_at").lte(now))
                .to_owned(),
        )
        .await?;
    database
        .execute(
            Query::insert()
                .into_table("administrator_sessions")
                .columns(["token_hash", "administrator_id", "csrf_token", "expires_at"])
                .values([token_hash.into(), 1.into(), csrf.into(), expires.into()])?,
        )
        .await?;
    Ok(())
}
pub(super) async fn delete_session(
    database: &DatabaseConnection,
    token_hash: &str,
) -> Result<(), AppError> {
    database
        .execute(
            &Query::delete()
                .from_table("administrator_sessions")
                .and_where(Expr::col("token_hash").eq(token_hash))
                .to_owned(),
        )
        .await?;
    Ok(())
}
