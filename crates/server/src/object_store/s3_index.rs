use rustsurvey_object_store::{
    MAX_OBJECT_BYTES, ObjectKey, ObjectMetadata, StoreError, StoreFuture,
    s3::{ObjectBinding, S3ObjectIndex},
};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, SqlErr,
    sea_query::{Expr, ExprTrait, Query},
};

pub struct SqlS3ObjectIndex {
    database: DatabaseConnection,
}
impl SqlS3ObjectIndex {
    pub fn new(database: DatabaseConnection) -> Self {
        Self { database }
    }
}

impl S3ObjectIndex for SqlS3ObjectIndex {
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ObjectBinding> {
        Box::pin(async move {
            let row = self
                .database
                .query_one(
                    &Query::select()
                        .columns(["location", "size", "sha256"])
                        .from("s3_objects")
                        .and_where(Expr::col("object_key").eq(key.as_str()))
                        .to_owned(),
                )
                .await
                .map_err(|_| StoreError::Unavailable)?
                .ok_or(StoreError::NotFound)?;
            let size: i64 = row
                .try_get("", "size")
                .map_err(|_| StoreError::Unavailable)?;
            let binding = ObjectBinding {
                metadata: ObjectMetadata {
                    size: size as u64,
                    sha256: row
                        .try_get("", "sha256")
                        .map_err(|_| StoreError::Unavailable)?,
                },
                location: row
                    .try_get("", "location")
                    .map_err(|_| StoreError::Unavailable)?,
            };
            validate(&binding)?;
            Ok(binding)
        })
    }

    fn insert<'a>(&'a self, key: &'a ObjectKey, binding: &'a ObjectBinding) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            validate(binding)?;
            let insert = Query::insert()
                .into_table("s3_objects")
                .columns(["object_key", "location", "size", "sha256"])
                .values([
                    key.as_str().into(),
                    binding.location.clone().into(),
                    (binding.metadata.size as i64).into(),
                    binding.metadata.sha256.clone().into(),
                ])
                .map_err(|_| StoreError::Unavailable)?
                .to_owned();
            if let Err(error) = self.database.execute(&insert).await {
                if matches!(error.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) {
                    return Err(StoreError::Conflict);
                }
                return Err(StoreError::Unavailable);
            }
            Ok(())
        })
    }

    fn delete<'a>(&'a self, key: &'a ObjectKey, binding: &'a ObjectBinding) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            self.database
                .execute(
                    &Query::delete()
                        .from_table("s3_objects")
                        .and_where(Expr::col("object_key").eq(key.as_str()))
                        .and_where(Expr::col("location").eq(&binding.location))
                        .and_where(Expr::col("sha256").eq(&binding.metadata.sha256))
                        .and_where(Expr::col("size").eq(binding.metadata.size as i64))
                        .to_owned(),
                )
                .await
                .map_err(|_| StoreError::Unavailable)?;
            Ok(())
        })
    }
}

fn validate(binding: &ObjectBinding) -> Result<(), StoreError> {
    if binding.metadata.size > MAX_OBJECT_BYTES as u64
        || ObjectKey::parse(&binding.metadata.sha256).is_err()
        || ObjectKey::parse(&binding.location).is_err()
    {
        return Err(StoreError::Unavailable);
    }
    Ok(())
}
