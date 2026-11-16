use super::AppError;
use axum::http::StatusCode;
use rustsurvey_object_store::ObjectMetadata;
use sea_orm::{
    ConnectionTrait, QueryResult,
    sea_query::{Expr, ExprTrait, Order, Query},
};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Media {
    pub id: String,
    pub study_id: String,
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub sha256: String,
    pub state: String,
    pub created_at: i64,
    #[serde(skip)]
    pub object_key: String,
}

const COLUMNS: [&str; 9] = [
    "id",
    "study_id",
    "filename",
    "content_type",
    "size",
    "sha256",
    "state",
    "created_at",
    "object_key",
];
fn decode(row: QueryResult) -> Result<Media, AppError> {
    Ok(Media {
        id: row.try_get("", "id")?,
        study_id: row.try_get("", "study_id")?,
        filename: row.try_get("", "filename")?,
        content_type: row.try_get("", "content_type")?,
        size: row.try_get("", "size")?,
        sha256: row.try_get("", "sha256")?,
        state: row.try_get("", "state")?,
        created_at: row.try_get("", "created_at")?,
        object_key: row.try_get("", "object_key")?,
    })
}
pub(super) async fn get(
    connection: &impl ConnectionTrait,
    study: &str,
    id: &str,
) -> Result<Option<Media>, AppError> {
    connection
        .query_one(
            &Query::select()
                .columns(COLUMNS)
                .from("media")
                .and_where(Expr::col("study_id").eq(study))
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await?
        .map(decode)
        .transpose()
}
pub(super) async fn list(
    connection: &impl ConnectionTrait,
    study: &str,
    offset: u64,
) -> Result<Vec<Media>, AppError> {
    let rows = connection
        .query_all(
            &Query::select()
                .columns(COLUMNS)
                .from("media")
                .and_where(Expr::col("study_id").eq(study))
                .order_by("created_at", Order::Desc)
                .order_by("id", Order::Asc)
                .limit(51)
                .offset(offset)
                .to_owned(),
        )
        .await?;
    rows.into_iter().map(decode).collect()
}
pub(super) async fn begin(
    connection: &impl ConnectionTrait,
    media: &Media,
) -> Result<(), AppError> {
    let query = Query::insert()
        .into_table("media")
        .columns([
            "id",
            "study_id",
            "filename",
            "content_type",
            "size",
            "sha256",
            "state",
            "created_at",
            "object_key",
        ])
        .values([
            media.id.clone().into(),
            media.study_id.clone().into(),
            media.filename.clone().into(),
            media.content_type.clone().into(),
            media.size.into(),
            media.sha256.clone().into(),
            media.state.clone().into(),
            media.created_at.into(),
            media.object_key.clone().into(),
        ])?
        .to_owned();
    if let Err(error) = connection.execute(&query).await {
        if matches!(
            error.sql_err(),
            Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
        ) {
            return Err(conflict());
        }
        return Err(error.into());
    }
    Ok(())
}
pub(super) fn conflict() -> AppError {
    AppError::new(
        StatusCode::CONFLICT,
        "media_conflict",
        "Cet identifiant de média correspond à un autre contenu ou à un envoi concurrent. Réessayez avec le même fichier.",
    )
}

pub(super) async fn ready(connection: &impl ConnectionTrait, id: &str) -> Result<(), AppError> {
    connection
        .execute(
            &Query::update()
                .table("media")
                .value("state", "ready")
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await?;
    Ok(())
}
pub(super) async fn is_public(
    connection: &impl ConnectionTrait,
    study: &str,
    media: &str,
    revision: Option<i64>,
) -> Result<bool, AppError> {
    let mut query = Query::select();
    query
        .column(("media_references", "media_id"))
        .from("media_references")
        .inner_join(
            "study_publications",
            Expr::col(("study_publications", "study_id"))
                .equals(("media_references", "study_id"))
                .and(
                    Expr::col(("study_publications", "revision"))
                        .equals(("media_references", "revision")),
                ),
        )
        .and_where(Expr::col(("media_references", "study_id")).eq(study))
        .and_where(Expr::col("media_id").eq(media));
    if let Some(revision) = revision {
        query.and_where(Expr::col(("study_publications", "revision")).eq(revision));
    } else {
        query.and_where(
            Expr::col(("study_publications", "revision"))
                .eq(super::publications::latest(Expr::val(study))),
        );
    }
    Ok(connection.query_one(&query).await?.is_some())
}
pub(super) fn matches(
    media: &Media,
    filename: &str,
    content_type: &str,
    metadata: &ObjectMetadata,
) -> bool {
    media.filename == filename
        && media.content_type == content_type
        && media.size == metadata.size as i64
        && media.sha256 == metadata.sha256
}
