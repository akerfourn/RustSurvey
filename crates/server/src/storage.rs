//! Deployment choices for SQL and binary storage; questionnaire code only sees ObjectStore.
use crate::{object_store::SqlObjectStore, settings::Settings};
use rustsurvey_object_store::{
    ByteRange, MAX_OBJECT_BYTES, ObjectKey, ObjectMetadata, ObjectRead, ObjectStore, StoreError,
    StoreFuture,
    file::{FileObjectStore, prepare_directory},
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
}

pub struct StorageConfiguration {
    database_url: Option<String>,
    backend: Backend,
    directory: PathBuf,
}
impl StorageConfiguration {
    pub fn read(settings: &Settings) -> io::Result<Self> {
        let backend = match settings.value_or("OBJECT_STORE_BACKEND", "sql")?.as_str() {
            "sql" => Backend::Sql,
            "file" => Backend::File,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "OBJECT_STORE_BACKEND must be sql or file",
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
        &self,
        database: DatabaseConnection,
    ) -> io::Result<Arc<dyn ObjectStore>> {
        let sql = SqlObjectStore::new(database);
        if self.backend == Backend::Sql {
            return Ok(Arc::new(sql));
        }
        let file = FileObjectStore::open(&self.directory).await.map_err(|_| {
            io::Error::other("cannot open local object storage; check OBJECT_STORE_DIRECTORY")
        })?;
        Ok(Arc::new(FileWithSqlHistory { file, sql }))
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

/// New writes go to files; old SQL keys remain readable without moving or deleting their data.
struct FileWithSqlHistory {
    file: FileObjectStore,
    sql: SqlObjectStore,
}
impl ObjectStore for FileWithSqlHistory {
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
            self.file.put(key, bytes).await
        })
    }
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ObjectMetadata> {
        Box::pin(async move {
            match self.file.head(key).await {
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
            match self.file.get(key, range).await {
                Err(StoreError::NotFound) => self.sql.get(key, range).await,
                result => result,
            }
        })
    }
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            self.file.delete(key).await?;
            self.sql.delete(key).await
        })
    }
}
