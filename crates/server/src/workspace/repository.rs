use super::AppError;
use rustsurvey_domain::Definition;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, QueryResult, TransactionTrait,
    sea_query::{Expr, ExprTrait, Order, Query},
};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportInfo {
    pub sha256: String,
    pub report: Value,
}
pub struct ImportedSource {
    pub xml: String,
    pub sha256: String,
    pub report: Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudySummary {
    pub id: String,
    pub title: String,
    pub revision: i64,
    pub published_revision: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredStudy {
    pub id: String,
    pub revision: i64,
    pub published_revision: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub definition: Definition,
    pub source: Option<ImportInfo>,
}

pub(super) fn summary(row: &QueryResult) -> Result<StudySummary, AppError> {
    Ok(StudySummary {
        id: row.try_get("", "id")?,
        title: row.try_get("", "title")?,
        revision: row.try_get("", "revision")?,
        published_revision: row.try_get("", "published_revision")?,
        created_at: row.try_get("", "created_at")?,
        updated_at: row.try_get("", "updated_at")?,
    })
}
pub async fn list(
    database: &DatabaseConnection,
    offset: u64,
) -> Result<Vec<StudySummary>, AppError> {
    let rows = database
        .query_all(
            &Query::select()
                .columns(["id", "title", "revision", "created_at", "updated_at"])
                .expr_as(
                    super::publications::latest(Expr::col(("studies", "id"))),
                    "published_revision",
                )
                .from("studies")
                .order_by("created_at", Order::Desc)
                .order_by("id", Order::Asc)
                .limit(51)
                .offset(offset)
                .to_owned(),
        )
        .await?;
    rows.iter().map(summary).collect()
}
pub async fn get(database: &DatabaseConnection, id: &str) -> Result<StoredStudy, AppError> {
    let row = database
        .query_one(
            &Query::select()
                .columns(["id", "title", "revision", "created_at", "updated_at"])
                .expr_as(
                    super::publications::latest(Expr::col(("studies", "id"))),
                    "published_revision",
                )
                .from("studies")
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await?
        .ok_or_else(AppError::not_found)?;
    let metadata = summary(&row)?;
    let revision = database
        .query_one(
            &Query::select()
                .column("document")
                .from("definition_revisions")
                .and_where(Expr::col("study_id").eq(id))
                .and_where(Expr::col("revision").eq(metadata.revision))
                .to_owned(),
        )
        .await?
        .ok_or_else(AppError::internal)?;
    let document: String = revision.try_get("", "document")?;
    let definition = decode_definition(&document)?;
    let source = import_info(database, id).await?;
    Ok(StoredStudy {
        id: metadata.id,
        revision: metadata.revision,
        published_revision: metadata.published_revision,
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
        definition,
        source,
    })
}
pub(super) fn decode_definition(document: &str) -> Result<Definition, AppError> {
    let mut definition: Definition =
        serde_json::from_str(document).map_err(|_| AppError::internal())?;
    definition.upgrade().map_err(|_| AppError::internal())?;
    Ok(definition)
}
async fn insert_revision(
    connection: &impl ConnectionTrait,
    id: &str,
    revision: i64,
    document: String,
    now: i64,
    definition: &Definition,
) -> Result<(), AppError> {
    connection
        .execute(
            Query::insert()
                .into_table("definition_revisions")
                .columns(["study_id", "revision", "document", "created_at"])
                .values([id.into(), revision.into(), document.into(), now.into()])?,
        )
        .await?;
    super::media::reference(connection, id, revision, definition).await?;
    Ok(())
}
pub async fn create(
    database: &DatabaseConnection,
    id: &str,
    definition: &Definition,
    document: String,
    now: i64,
    source: Option<ImportedSource>,
) -> Result<StoredStudy, AppError> {
    let transaction = database.begin().await?;
    transaction
        .execute(
            Query::insert()
                .into_table("studies")
                .columns(["id", "title", "revision", "created_at", "updated_at"])
                .values([
                    id.into(),
                    definition.title.clone().into(),
                    1_i64.into(),
                    now.into(),
                    now.into(),
                ])?,
        )
        .await?;
    insert_revision(&transaction, id, 1, document, now, definition).await?;
    let source = if let Some(source) = source {
        transaction
            .execute(
                Query::insert()
                    .into_table("import_sources")
                    .columns(["study_id", "xml", "sha256", "report"])
                    .values([
                        id.into(),
                        source.xml.into(),
                        source.sha256.clone().into(),
                        source.report.to_string().into(),
                    ])?,
            )
            .await?;
        Some(ImportInfo {
            sha256: source.sha256,
            report: source.report,
        })
    } else {
        None
    };
    transaction.commit().await?;
    Ok(StoredStudy {
        id: id.to_owned(),
        revision: 1,
        published_revision: None,
        created_at: now,
        updated_at: now,
        definition: definition.clone(),
        source,
    })
}
pub async fn update(
    database: &DatabaseConnection,
    id: &str,
    expected: i64,
    definition: &Definition,
    document: String,
    now: i64,
) -> Result<StoredStudy, AppError> {
    let transaction = database.begin().await?;
    let result = transaction
        .execute(
            &Query::update()
                .table("studies")
                .value("revision", expected + 1)
                .value("title", definition.title.clone())
                .value("updated_at", now)
                .and_where(Expr::col("id").eq(id))
                .and_where(Expr::col("revision").eq(expected))
                .to_owned(),
        )
        .await?;
    if result.rows_affected() != 1 {
        transaction.rollback().await?;
        return Err(AppError::new(
            axum::http::StatusCode::CONFLICT,
            "revision_conflict",
            "Le questionnaire a été modifié. Rechargez sa version avant de sauvegarder.",
        ));
    }
    insert_revision(&transaction, id, expected + 1, document, now, definition).await?;
    let row = transaction
        .query_one(
            &Query::select()
                .column("created_at")
                .from("studies")
                .and_where(Expr::col("id").eq(id))
                .to_owned(),
        )
        .await?
        .ok_or_else(AppError::internal)?;
    let created_at = row.try_get("", "created_at")?;
    transaction.commit().await?;
    let source = import_info(database, id).await?;
    Ok(StoredStudy {
        id: id.to_owned(),
        revision: expected + 1,
        published_revision: super::publications::current(database, id).await?,
        created_at,
        updated_at: now,
        definition: definition.clone(),
        source,
    })
}

pub async fn import_info(
    database: &DatabaseConnection,
    id: &str,
) -> Result<Option<ImportInfo>, AppError> {
    let row = database
        .query_one(
            &Query::select()
                .columns(["sha256", "report"])
                .from("import_sources")
                .and_where(Expr::col("study_id").eq(id))
                .to_owned(),
        )
        .await?;
    row.map(|row| {
        let report: String = row.try_get("", "report")?;
        Ok(ImportInfo {
            sha256: row.try_get("", "sha256")?,
            report: serde_json::from_str(&report).map_err(|_| AppError::internal())?,
        })
    })
    .transpose()
}
pub async fn original(database: &DatabaseConnection, id: &str) -> Result<String, AppError> {
    let row = database
        .query_one(
            &Query::select()
                .column("xml")
                .from("import_sources")
                .and_where(Expr::col("study_id").eq(id))
                .to_owned(),
        )
        .await?
        .ok_or_else(AppError::not_found)?;
    Ok(row.try_get("", "xml")?)
}
