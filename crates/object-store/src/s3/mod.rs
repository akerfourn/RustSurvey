//! Private S3 content published under immutable keys through a durable index.
mod config;
mod index;
mod signing;

use crate::{
    ByteRange, MAX_OBJECT_BYTES, ObjectKey, ObjectMetadata, ObjectRead, ObjectStore, StoreError,
    StoreFuture,
};
pub use config::{ConfigurationError, S3Configuration, S3Credentials};
pub use index::{ObjectBinding, S3ObjectIndex};
use reqwest::{Client, Method, Response, StatusCode, header::HeaderMap};
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const DIGEST_HEADER: &str = "x-amz-meta-rustsurvey-sha256";
const FORMAT_HEADER: &str = "x-amz-meta-rustsurvey-format";

pub struct S3ObjectStore {
    configuration: S3Configuration,
    client: Client,
    operations: Arc<Semaphore>,
    index: Arc<dyn S3ObjectIndex>,
    location: String,
}

impl S3ObjectStore {
    pub fn new(
        configuration: S3Configuration,
        index: Arc<dyn S3ObjectIndex>,
    ) -> Result<Self, StoreError> {
        // The same ring provider is already used by the SQL TLS drivers.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .no_proxy()
            .connect_timeout(Duration::from_secs(5))
            .read_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(4)
            .pool_idle_timeout(Duration::from_secs(30))
            .http1_only()
            .build()
            .map_err(|_| StoreError::Unavailable)?;
        let location = ObjectMetadata::for_bytes(
            format!(
                "s3-v1\0{}\0{}\0{}",
                configuration.endpoint, configuration.bucket, configuration.prefix
            )
            .as_bytes(),
        )?
        .sha256;
        Ok(Self {
            configuration,
            client,
            operations: Arc::new(Semaphore::new(4)),
            index,
            location,
        })
    }

    fn permit(&self) -> Result<Arc<OwnedSemaphorePermit>, StoreError> {
        self.operations
            .clone()
            .try_acquire_owned()
            .map(Arc::new)
            .map_err(|_| StoreError::Unavailable)
    }

    async fn request(
        &self,
        method: Method,
        key: &ObjectKey,
        metadata: &ObjectMetadata,
        body: Option<Vec<u8>>,
    ) -> Result<Response, StoreError> {
        let mut url = self.configuration.endpoint.clone();
        url.set_path(&format!(
            "/{}/{}/{}/{}",
            self.configuration.bucket,
            self.configuration.prefix,
            key.as_str(),
            metadata.sha256
        ));
        let mut builder = self.client.request(method, url);
        let mut digest = EMPTY_SHA256.to_owned();
        if let Some(bytes) = body {
            digest = metadata.sha256.clone();
            builder = builder
                .header("if-none-match", "*")
                .header(FORMAT_HEADER, "1")
                .header(DIGEST_HEADER, &metadata.sha256)
                .header("content-type", "application/octet-stream")
                .body(bytes);
        }
        let mut request = builder.build().map_err(|_| StoreError::Unavailable)?;
        signing::apply(&mut request, &self.configuration, digest, SystemTime::now())?;
        self.client
            .execute(request)
            .await
            .map_err(|_| StoreError::Unavailable)
    }

    /// The caller supplies a fresh random key, never a key from an existing SQL reference.
    /// Verifies the configured index and transport without touching an existing binding.
    pub async fn verify_immutable_writes(&self, key: &ObjectKey) -> Result<(), StoreError> {
        let original = b"rustsurvey-s3-conditional-probe-v1".to_vec();
        self.put(key, original.clone()).await?;
        let check = async {
            if self.put(key, b"must-not-replace".to_vec()).await != Err(StoreError::Conflict) {
                return Err(StoreError::Unavailable);
            }
            if self.get(key, None).await?.bytes != original {
                return Err(StoreError::Unavailable);
            }
            Ok(())
        }
        .await;
        let cleanup = self.delete(key).await;
        check.and(cleanup)
    }

    async fn binding(&self, key: &ObjectKey) -> Result<ObjectBinding, StoreError> {
        let binding = self.index.head(key).await?;
        if binding.location != self.location
            || binding.metadata.size > MAX_OBJECT_BYTES as u64
            || ObjectKey::parse(&binding.metadata.sha256).is_err()
        {
            return Err(StoreError::Unavailable);
        }
        Ok(binding)
    }

    async fn read_content(
        &self,
        key: &ObjectKey,
        expected: &ObjectMetadata,
        range: Option<ByteRange>,
        permit: Arc<OwnedSemaphorePermit>,
    ) -> Result<ObjectRead, StoreError> {
        let mut response = self.request(Method::GET, key, expected, None).await?;
        successful(&response)?;
        if metadata(response.headers())? != *expected {
            return Err(StoreError::Unavailable);
        }
        let range = ByteRange::resolve(range, expected.size)?;
        let mut bytes = Vec::with_capacity(expected.size as usize);
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| StoreError::Unavailable)?
        {
            if chunk.len() > (expected.size as usize).saturating_sub(bytes.len()) {
                return Err(StoreError::Unavailable);
            }
            bytes.extend_from_slice(&chunk);
        }
        let (mut bytes, actual) = checksum(bytes, permit).await?;
        if actual != *expected {
            return Err(StoreError::Unavailable);
        }
        bytes.truncate(range.end as usize);
        bytes.drain(..range.start as usize);
        Ok(ObjectRead {
            metadata: expected.clone(),
            range,
            bytes,
        })
    }
}

impl ObjectStore for S3ObjectStore {
    fn put<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ObjectMetadata> {
        Box::pin(async move {
            if bytes.len() > MAX_OBJECT_BYTES {
                return Err(StoreError::TooLarge);
            }
            let permit = self.permit()?;
            match self.index.head(key).await {
                Ok(_) => return Err(StoreError::Conflict),
                Err(StoreError::NotFound) => {}
                Err(error) => return Err(error),
            }
            let (bytes, metadata) = checksum(bytes, permit.clone()).await?;
            let response = self
                .request(Method::PUT, key, &metadata, Some(bytes))
                .await?;
            match response.status() {
                StatusCode::OK => {}
                StatusCode::PRECONDITION_FAILED => {
                    self.read_content(key, &metadata, None, permit.clone())
                        .await?;
                }
                _ => return Err(StoreError::Unavailable),
            }
            self.index
                .insert(
                    key,
                    &ObjectBinding {
                        metadata: metadata.clone(),
                        location: self.location.clone(),
                    },
                )
                .await?;
            Ok(metadata)
        })
    }

    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ObjectMetadata> {
        Box::pin(async move {
            let _permit = self.permit()?;
            let binding = self.binding(key).await?;
            let response = self
                .request(Method::HEAD, key, &binding.metadata, None)
                .await?;
            successful(&response)?;
            if metadata(response.headers())? != binding.metadata {
                return Err(StoreError::Unavailable);
            }
            Ok(binding.metadata)
        })
    }

    fn get<'a>(
        &'a self,
        key: &'a ObjectKey,
        range: Option<ByteRange>,
    ) -> StoreFuture<'a, ObjectRead> {
        Box::pin(async move {
            let permit = self.permit()?;
            let binding = self.binding(key).await?;
            self.read_content(key, &binding.metadata, range, permit)
                .await
        })
    }

    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            let _permit = self.permit()?;
            let binding = match self.binding(key).await {
                Err(StoreError::NotFound) => return Ok(()),
                result => result?,
            };
            let response = self
                .request(Method::DELETE, key, &binding.metadata, None)
                .await?;
            match response.status() {
                StatusCode::NO_CONTENT | StatusCode::OK | StatusCode::NOT_FOUND => {
                    self.index.delete(key, &binding).await
                }
                _ => Err(StoreError::Unavailable),
            }
        })
    }
}

async fn checksum(
    bytes: Vec<u8>,
    permit: Arc<OwnedSemaphorePermit>,
) -> Result<(Vec<u8>, ObjectMetadata), StoreError> {
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let metadata = ObjectMetadata::for_bytes(&bytes)?;
        Ok((bytes, metadata))
    })
    .await
    .map_err(|_| StoreError::Unavailable)?
}

fn successful(response: &Response) -> Result<(), StoreError> {
    match response.status() {
        StatusCode::OK => Ok(()),
        _ => Err(StoreError::Unavailable),
    }
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Result<&'a str, StoreError> {
    let mut values = headers.get_all(name).iter();
    let value = values.next().ok_or(StoreError::Unavailable)?;
    if values.next().is_some() {
        return Err(StoreError::Unavailable);
    }
    value.to_str().map_err(|_| StoreError::Unavailable)
}

fn metadata(headers: &HeaderMap) -> Result<ObjectMetadata, StoreError> {
    let size = header(headers, "content-length")?
        .parse::<u64>()
        .map_err(|_| StoreError::Unavailable)?;
    let sha256 = header(headers, DIGEST_HEADER)?;
    if size > MAX_OBJECT_BYTES as u64
        || ObjectKey::parse(sha256).is_err()
        || header(headers, FORMAT_HEADER)? != "1"
    {
        return Err(StoreError::Unavailable);
    }
    Ok(ObjectMetadata {
        size,
        sha256: sha256.into(),
    })
}
