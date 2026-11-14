use crate::{ObjectKey, ObjectMetadata, StoreFuture};

/// A durable binding from an opaque key to one physical S3 content path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectBinding {
    pub metadata: ObjectMetadata,
    pub location: String,
}

pub trait S3ObjectIndex: Send + Sync {
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ObjectBinding>;
    /// Atomically publish a new key; return Conflict if it already exists.
    fn insert<'a>(&'a self, key: &'a ObjectKey, binding: &'a ObjectBinding) -> StoreFuture<'a, ()>;
    /// Remove only the expected binding.
    fn delete<'a>(&'a self, key: &'a ObjectKey, binding: &'a ObjectBinding) -> StoreFuture<'a, ()>;
}
