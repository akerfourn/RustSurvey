mod common;
use rustsurvey_object_store::{ObjectKey, ObjectStore, StoreError, memory::MemoryObjectStore};

#[tokio::test]
async fn test_memory_objects_should_respect_the_shared_storage_contract() {
    common::contract(&MemoryObjectStore::new(200000, 10)).await;
}

#[tokio::test]
async fn test_memory_capacity_should_bound_bytes_and_empty_objects_and_recover_after_delete() {
    let store = MemoryObjectStore::new(10, 1);
    let first = ObjectKey::parse(&"a".repeat(64)).unwrap();
    let second = ObjectKey::parse(&"b".repeat(64)).unwrap();
    store.put(&first, vec![]).await.unwrap();
    assert_eq!(
        store.put(&second, vec![]).await.unwrap_err(),
        StoreError::Capacity
    );
    store.delete(&first).await.unwrap();
    assert_eq!(
        store.put(&second, vec![0; 11]).await.unwrap_err(),
        StoreError::Capacity
    );
    store.put(&second, vec![0; 10]).await.unwrap();
    assert_eq!(store.head(&second).await.unwrap().size, 10);
}

#[test]
fn test_object_keys_should_reject_filenames_paths_and_noncanonical_keys() {
    for key in [
        "../secret",
        "/tmp/object",
        "https://bucket/key",
        &"A".repeat(64),
        &"a".repeat(65),
    ] {
        assert_eq!(ObjectKey::parse(key).unwrap_err(), StoreError::InvalidKey);
    }
}
