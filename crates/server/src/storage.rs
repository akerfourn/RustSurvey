//! Deployment choices for SQL and binary storage; questionnaire code only sees ObjectStore.
mod s3;
use crate::{
    object_store::{SqlObjectStore, SqlS3ObjectIndex},
    settings::Settings,
};
use rustsurvey_object_store::{
    ByteRange, MAX_OBJECT_BYTES, ObjectKey, ObjectMetadata, ObjectRead, ObjectStore, StoreError,
    StoreFuture,
    file::{FileObjectStore, prepare_directory},
    s3::{S3Configuration, S3ObjectStore},
};
use sea_orm::DatabaseConnection;
use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

pub const DEFAULT_DATABASE_URL: &str = "sqlite://.data/rustsurvey.db?mode=rwc";
const LEGACY_DATABASE_URL: &str = "sqlite://rustsurvey.db?mode=rwc";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Backend {
    Sql,
    File,
    S3,
}

pub struct StorageConfiguration {
    database_url: Option<String>,
    backend: Backend,
    directory: PathBuf,
    s3: Option<S3Configuration>,
}
impl StorageConfiguration {
    pub fn read(settings: &Settings) -> io::Result<Self> {
        let backend = match settings.value_or("OBJECT_STORE_BACKEND", "sql")?.as_str() {
            "sql" => Backend::Sql,
            "file" => Backend::File,
            "s3" => Backend::S3,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "OBJECT_STORE_BACKEND must be sql or file or s3",
                ));
            }
        };
        let directory = settings.value_or("OBJECT_STORE_DIRECTORY", ".data/media")?;
        if backend == Backend::File && directory.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "OBJECT_STORE_DIRECTORY must not be empty in file mode",
            ));
        }
        let database_url = settings.get("DATABASE_URL")?;
        if database_url.as_deref() == Some("") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "DATABASE_URL must not be empty",
            ));
        }
        Ok(Self {
            database_url,
            backend,
            directory: PathBuf::from(directory),
            s3: if backend == Backend::S3 {
                Some(s3::configuration(settings)?)
            } else {
                None
            },
        })
    }

    pub async fn database_url(&self, working_directory: &Path) -> io::Result<String> {
        let url = if let Some(url) = &self.database_url {
            url.clone()
        } else {
            let root = working_directory.to_path_buf();
            tokio::task::spawn_blocking(move || choose_default_database(&root))
                .await
                .map_err(|_| io::Error::other("cannot inspect local database paths"))??
        };
        if url == DEFAULT_DATABASE_URL {
            prepare_directory(working_directory.join(".data"))
                .await
                .map_err(|_| io::Error::other("cannot prepare .data directory"))?;
        }
        Ok(url)
    }

    pub async fn object_store(
        self,
        database: DatabaseConnection,
    ) -> io::Result<Arc<dyn ObjectStore>> {
        let sql = SqlObjectStore::new(database.clone());
        let primary: Arc<dyn ObjectStore> = match self.backend {
            Backend::Sql => return Ok(Arc::new(sql)),
            Backend::File => {
                Arc::new(FileObjectStore::open(&self.directory).await.map_err(|_| {
                    io::Error::other(
                        "cannot open local object storage; check OBJECT_STORE_DIRECTORY",
                    )
                })?)
            }
            Backend::S3 => {
                let configuration = self
                    .s3
                    .ok_or_else(|| io::Error::other("missing S3 configuration"))?;
                let store =
                    S3ObjectStore::new(configuration, Arc::new(SqlS3ObjectIndex::new(database)))
                        .map_err(|_| io::Error::other("cannot initialize S3 transport"))?;
                s3::verify(&store).await?;
                Arc::new(store)
            }
        };
        Ok(Arc::new(ExternalWithSqlHistory { primary, sql }))
    }
}

fn choose_default_database(root: &Path) -> io::Result<String> {
    let legacy = root.join("rustsurvey.db").try_exists()?;
    let current = root.join(".data/rustsurvey.db").try_exists()?;
    match (legacy, current) {
        (true, true) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "two local databases exist; select one explicitly with DATABASE_URL",
        )),
        (true, false) => Ok(LEGACY_DATABASE_URL.to_owned()),
        _ => Ok(DEFAULT_DATABASE_URL.to_owned()),
    }
}

/// New writes go to the configured backend; old SQL keys remain readable without moving their data.
struct ExternalWithSqlHistory {
    primary: Arc<dyn ObjectStore>,
    sql: SqlObjectStore,
}
impl ObjectStore for ExternalWithSqlHistory {
    fn put<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ObjectMetadata> {
        Box::pin(async move {
            if bytes.len() > MAX_OBJECT_BYTES {
                return Err(StoreError::TooLarge);
            }
            match self.sql.head(key).await {
                Ok(_) => return Err(StoreError::Conflict),
                Err(StoreError::NotFound) => {}
                Err(error) => return Err(error),
            }
            self.primary.put(key, bytes).await
        })
    }
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ObjectMetadata> {
        Box::pin(async move {
            match self.primary.head(key).await {
                Err(StoreError::NotFound) => self.sql.head(key).await,
                result => result,
            }
        })
    }
    fn get<'a>(
        &'a self,
        key: &'a ObjectKey,
        range: Option<ByteRange>,
    ) -> StoreFuture<'a, ObjectRead> {
        Box::pin(async move {
            match self.primary.get(key, range).await {
                Err(StoreError::NotFound) => self.sql.get(key, range).await,
                result => result,
            }
        })
    }
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            self.primary.delete(key).await?;
            self.sql.delete(key).await
        })
    }
}
