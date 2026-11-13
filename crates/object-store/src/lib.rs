//! Bounded immutable objects. No filesystem path, SQL connection or HTTP type crosses this port.
use sha2::{Digest, Sha256};
use std::{future::Future, pin::Pin};

pub mod memory;
pub const MAX_OBJECT_BYTES: usize = 8 * 1024 * 1024;
pub type StoreFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, StoreError>> + Send + 'a>>;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ObjectKey(String);

impl ObjectKey {
    pub fn parse(key: &str) -> Result<Self, StoreError> {
        if key.len() != 64
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(StoreError::InvalidKey);
        }
        Ok(Self(key.to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectMetadata {
    pub size: u64,
    pub sha256: String,
}
impl ObjectMetadata {
    pub fn for_bytes(bytes: &[u8]) -> Result<Self, StoreError> {
        if bytes.len() > MAX_OBJECT_BYTES {
            return Err(StoreError::TooLarge);
        }
        let digest = Sha256::digest(bytes);
        let mut sha256 = String::with_capacity(64);
        for byte in digest {
            use std::fmt::Write;
            write!(sha256, "{byte:02x}").expect("writing to a string");
        }
        Ok(Self {
            size: bytes.len() as u64,
            sha256,
        })
    }
}

/// An exclusive upper bound. A missing range reads the whole bounded object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteRange {
    pub start: u64,
    pub end: u64,
}
impl ByteRange {
    pub fn resolve(range: Option<Self>, size: u64) -> Result<Self, StoreError> {
        let range = range.unwrap_or(Self {
            start: 0,
            end: size,
        });
        if range.start > range.end || range.end > size || (range.start == range.end && size != 0) {
            return Err(StoreError::InvalidRange);
        }
        Ok(range)
    }
}

#[derive(Debug)]
pub struct ObjectRead {
    pub metadata: ObjectMetadata,
    pub range: ByteRange,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreError {
    InvalidKey,
    NotFound,
    Conflict,
    TooLarge,
    InvalidRange,
    Capacity,
    Unavailable,
}
impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "object store: {self:?}")
    }
}
impl std::error::Error for StoreError {}

pub trait ObjectStore: Send + Sync {
    /// Never overwrite an existing key, even if its content is identical.
    fn put<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ObjectMetadata>;
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ObjectMetadata>;
    fn get<'a>(
        &'a self,
        key: &'a ObjectKey,
        range: Option<ByteRange>,
    ) -> StoreFuture<'a, ObjectRead>;
    /// Idempotent deletion; the application owns reference and retention checks.
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()>;
}
