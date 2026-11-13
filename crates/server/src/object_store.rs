//! SQL adapter for the object port. Engine-specific binary types belong to the migration.
use rustsurvey_object_store::{
    ByteRange, MAX_OBJECT_BYTES, ObjectKey, ObjectMetadata, ObjectRead, ObjectStore, StoreError,
    StoreFuture,
};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, QueryResult, SqlErr,
    sea_query::{Expr, ExprTrait, Query},
};

pub struct SqlObjectStore {
    database: DatabaseConnection,
}
impl SqlObjectStore {
    pub fn new(database: DatabaseConnection) -> Self {
        Self { database }
    }
}

fn unavailable(_: impl std::fmt::Debug) -> StoreError {
    StoreError::Unavailable
}
fn metadata(row: &QueryResult) -> Result<ObjectMetadata, StoreError> {
    let size: i64 = row.try_get("", "size").map_err(unavailable)?;
    let sha256: String = row.try_get("", "sha256").map_err(unavailable)?;
    if size < 0 || size as u64 > MAX_OBJECT_BYTES as u64 || sha256.len() != 64 {
        return Err(StoreError::Unavailable);
    }
    Ok(ObjectMetadata {
        size: size as u64,
        sha256,
    })
}

impl ObjectStore for SqlObjectStore {
    fn put<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ObjectMetadata> {
        Box::pin(async move {
            let metadata = ObjectMetadata::for_bytes(&bytes)?;
            let insert = Query::insert()
                .into_table("objects")
                .columns(["object_key", "body", "size", "sha256"])
                .values([
                    key.as_str().into(),
                    bytes.into(),
                    (metadata.size as i64).into(),
                    metadata.sha256.clone().into(),
                ])
                .map_err(unavailable)?
                .to_owned();
            if let Err(error) = self.database.execute(&insert).await {
                if matches!(error.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) {
                    return Err(StoreError::Conflict);
                }
                return Err(StoreError::Unavailable);
            }
            Ok(metadata)
        })
    }
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ObjectMetadata> {
        Box::pin(async move {
            let row = self
                .database
                .query_one(
                    &Query::select()
                        .columns(["size", "sha256"])
                        .from("objects")
                        .and_where(Expr::col("object_key").eq(key.as_str()))
                        .to_owned(),
                )
                .await
                .map_err(unavailable)?
                .ok_or(StoreError::NotFound)?;
            metadata(&row)
        })
    }
    fn get<'a>(
        &'a self,
        key: &'a ObjectKey,
        range: Option<ByteRange>,
    ) -> StoreFuture<'a, ObjectRead> {
        Box::pin(async move {
            let row = self
                .database
                .query_one(
                    &Query::select()
                        .columns(["size", "sha256", "body"])
                        .from("objects")
                        .and_where(Expr::col("object_key").eq(key.as_str()))
                        .to_owned(),
                )
                .await
                .map_err(unavailable)?
                .ok_or(StoreError::NotFound)?;
            let metadata = metadata(&row)?;
            let mut bytes: Vec<u8> = row.try_get("", "body").map_err(unavailable)?;
            if ObjectMetadata::for_bytes(&bytes)? != metadata {
                return Err(StoreError::Unavailable);
            }
            let range = ByteRange::resolve(range, metadata.size)?;
            bytes.drain(..range.start as usize);
            bytes.truncate((range.end - range.start) as usize);
            Ok(ObjectRead {
                metadata,
                range,
                bytes,
            })
        })
    }
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            self.database
                .execute(
                    &Query::delete()
                        .from_table("objects")
                        .and_where(Expr::col("object_key").eq(key.as_str()))
                        .to_owned(),
                )
                .await
                .map_err(unavailable)?;
            Ok(())
        })
    }
}
