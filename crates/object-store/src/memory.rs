use crate::{
    ByteRange, ObjectKey, ObjectMetadata, ObjectRead, ObjectStore, StoreError, StoreFuture,
};
use std::{collections::BTreeMap, sync::Mutex};

struct Entry {
    bytes: Vec<u8>,
    metadata: ObjectMetadata,
}
#[derive(Default)]
struct Objects {
    entries: BTreeMap<ObjectKey, Entry>,
    size: usize,
}

/// Useful for contract tests and ephemeral workspaces; capacity is explicit.
pub struct MemoryObjectStore {
    objects: Mutex<Objects>,
    capacity: usize,
    maximum_objects: usize,
}
impl MemoryObjectStore {
    pub fn new(capacity: usize, maximum_objects: usize) -> Self {
        Self {
            objects: Mutex::new(Objects::default()),
            capacity,
            maximum_objects,
        }
    }
}
impl ObjectStore for MemoryObjectStore {
    fn put<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ObjectMetadata> {
        Box::pin(async move {
            let metadata = ObjectMetadata::for_bytes(&bytes)?;
            let mut objects = self.objects.lock().map_err(|_| StoreError::Unavailable)?;
            if objects.entries.contains_key(key) {
                return Err(StoreError::Conflict);
            }
            if bytes.len() > self.capacity.saturating_sub(objects.size)
                || objects.entries.len() >= self.maximum_objects
            {
                return Err(StoreError::Capacity);
            }
            objects.size += bytes.len();
            objects.entries.insert(
                key.clone(),
                Entry {
                    bytes,
                    metadata: metadata.clone(),
                },
            );
            Ok(metadata)
        })
    }
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ObjectMetadata> {
        Box::pin(async move {
            let objects = self.objects.lock().map_err(|_| StoreError::Unavailable)?;
            Ok(objects
                .entries
                .get(key)
                .ok_or(StoreError::NotFound)?
                .metadata
                .clone())
        })
    }
    fn get<'a>(
        &'a self,
        key: &'a ObjectKey,
        range: Option<ByteRange>,
    ) -> StoreFuture<'a, ObjectRead> {
        Box::pin(async move {
            let objects = self.objects.lock().map_err(|_| StoreError::Unavailable)?;
            let entry = objects.entries.get(key).ok_or(StoreError::NotFound)?;
            let range = ByteRange::resolve(range, entry.metadata.size)?;
            Ok(ObjectRead {
                metadata: entry.metadata.clone(),
                range,
                bytes: entry.bytes[range.start as usize..range.end as usize].to_vec(),
            })
        })
    }
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            let mut objects = self.objects.lock().map_err(|_| StoreError::Unavailable)?;
            if let Some(entry) = objects.entries.remove(key) {
                objects.size -= entry.bytes.len();
            }
            Ok(())
        })
    }
}
