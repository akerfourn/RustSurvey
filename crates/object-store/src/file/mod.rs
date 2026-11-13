//! Private local objects, published atomically after their content and metadata are durable.
mod filesystem;
mod pending;

use crate::{
    ByteRange, ObjectKey, ObjectMetadata, ObjectRead, ObjectStore, StoreError, StoreFuture,
};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicU64},
};
use tokio::sync::Semaphore;

pub struct FileObjectStore {
    root: Arc<PathBuf>,
    serial: Arc<AtomicU64>,
    operations: Arc<Semaphore>,
}

/// Create a private directory without modifying permissions on existing directories.
pub async fn prepare_directory(path: impl Into<PathBuf>) -> Result<PathBuf, StoreError> {
    let path = path.into();
    tokio::task::spawn_blocking(move || filesystem::prepare_directory(&path))
        .await
        .map_err(|_| StoreError::Unavailable)?
}

impl FileObjectStore {
    pub async fn open(path: impl Into<PathBuf>) -> Result<Self, StoreError> {
        Ok(Self {
            root: Arc::new(prepare_directory(path).await?),
            serial: Arc::new(AtomicU64::new(0)),
            operations: Arc::new(Semaphore::new(4)),
        })
    }

    async fn run<T: Send + 'static>(
        &self,
        operation: impl FnOnce(&Path) -> Result<T, StoreError> + Send + 'static,
    ) -> Result<T, StoreError> {
        let permit = self
            .operations
            .clone()
            .try_acquire_owned()
            .map_err(|_| StoreError::Unavailable)?;
        let root = self.root.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            filesystem::check_directory(&root).map_err(|_| StoreError::Unavailable)?;
            operation(&root)
        })
        .await
        .map_err(|_| StoreError::Unavailable)?
    }
}

impl ObjectStore for FileObjectStore {
    fn put<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ObjectMetadata> {
        Box::pin(async move {
            let key = key.clone();
            let serial = self.serial.clone();
            self.run(move |root| {
                let metadata = ObjectMetadata::for_bytes(&bytes)?;
                let destination = root.join(key.as_str());
                filesystem::require_absent(&destination)?;
                let pending = pending::PendingDirectory::create(root, &key, &serial)?;
                pending.write("content", &bytes)?;
                pending.write("metadata", &filesystem::encode_metadata(&metadata))?;
                pending.publish(&destination)?;
                Ok(metadata)
            })
            .await
        })
    }
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ObjectMetadata> {
        Box::pin(async move {
            let key = key.clone();
            self.run(move |root| filesystem::metadata(root, &key)).await
        })
    }
    fn get<'a>(
        &'a self,
        key: &'a ObjectKey,
        range: Option<ByteRange>,
    ) -> StoreFuture<'a, ObjectRead> {
        Box::pin(async move {
            let key = key.clone();
            self.run(move |root| filesystem::read(root, &key, range))
                .await
        })
    }
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            let key = key.clone();
            self.run(move |root| filesystem::delete(root, &key)).await
        })
    }
}
