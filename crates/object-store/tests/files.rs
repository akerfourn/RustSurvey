mod common;
#[path = "common/directory.rs"]
mod directory;
use directory::Directory;
use rustsurvey_object_store::{ObjectKey, ObjectStore, StoreError, file::FileObjectStore};
use std::fs;

#[tokio::test]
async fn test_local_files_should_respect_the_same_contract_as_memory_and_sql() {
    let directory = Directory::create();
    common::contract(&FileObjectStore::open(directory.path()).await.unwrap()).await;
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn test_reopening_should_preserve_raw_content_and_detect_corruption_without_loading_content_for_head()
 {
    let directory = Directory::create();
    let key = ObjectKey::parse(&"a".repeat(64)).unwrap();
    let store = FileObjectStore::open(directory.path()).await.unwrap();
    let metadata = store.put(&key, b"original".to_vec()).await.unwrap();
    drop(store);
    let reopened = FileObjectStore::open(directory.path()).await.unwrap();
    assert_eq!(reopened.get(&key, None).await.unwrap().bytes, b"original");
    let path = directory.path().join(key.as_str()).join("content");
    assert_eq!(fs::read(&path).unwrap(), b"original");
    fs::write(&path, b"tampered").unwrap();
    assert_eq!(
        reopened.head(&key).await.unwrap(),
        metadata,
        "head reads metadata and size, not the full content"
    );
    assert_eq!(
        reopened.get(&key, None).await.unwrap_err(),
        StoreError::Unavailable
    );
}

#[tokio::test]
async fn test_concurrent_writers_should_publish_exactly_one_complete_object_without_leaving_staging_files()
 {
    let directory = Directory::create();
    let first = FileObjectStore::open(directory.path()).await.unwrap();
    let second = FileObjectStore::open(directory.path()).await.unwrap();
    let key = ObjectKey::parse(&"a".repeat(64)).unwrap();
    let (a, b) = tokio::join!(
        first.put(&key, b"first".to_vec()),
        second.put(&key, b"second".to_vec())
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(a == Err(StoreError::Conflict) || b == Err(StoreError::Conflict));
    let expected = if a.is_ok() {
        b"first".as_slice()
    } else {
        b"second".as_slice()
    };
    assert_eq!(first.get(&key, None).await.unwrap().bytes, expected);
    assert_eq!(
        fs::read_dir(directory.path()).unwrap().count(),
        1,
        "only the final object directory should remain"
    );
}

#[tokio::test]
async fn test_existing_or_incomplete_destinations_should_never_be_overwritten() {
    let directory = Directory::create();
    let store = FileObjectStore::open(directory.path()).await.unwrap();
    let key = ObjectKey::parse(&"a".repeat(64)).unwrap();
    let target = directory.path().join(key.as_str());
    fs::create_dir(&target).unwrap();
    assert_eq!(
        store.put(&key, b"new".to_vec()).await.unwrap_err(),
        StoreError::Conflict
    );
    assert_eq!(store.head(&key).await.unwrap_err(), StoreError::Unavailable);
    assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
}

#[tokio::test]
async fn test_empty_objects_and_missing_storage_roots_should_have_distinct_results() {
    let directory = Directory::create();
    let root = directory.path().join("objects");
    let store = FileObjectStore::open(&root).await.unwrap();
    let key = ObjectKey::parse(&"a".repeat(64)).unwrap();
    store.put(&key, vec![]).await.unwrap();
    assert_eq!(store.get(&key, None).await.unwrap().bytes, Vec::<u8>::new());
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(store.head(&key).await.unwrap_err(), StoreError::Unavailable);
    assert_eq!(
        store.put(&key, b"new".to_vec()).await.unwrap_err(),
        StoreError::Unavailable
    );
}

#[cfg(unix)]
#[tokio::test]
async fn test_symlinks_should_not_redirect_object_reads_writes_or_deletions() {
    use std::os::unix::fs::symlink;
    let directory = Directory::create();
    let outside = Directory::create();
    fs::write(outside.path().join("protected"), b"kept").unwrap();
    let linked_root = directory.path().join("linked");
    symlink(outside.path(), &linked_root).unwrap();
    assert!(matches!(
        FileObjectStore::open(&linked_root).await,
        Err(StoreError::Unavailable)
    ));
    let store = FileObjectStore::open(directory.path()).await.unwrap();
    let key = ObjectKey::parse(&"a".repeat(64)).unwrap();
    symlink(outside.path(), directory.path().join(key.as_str())).unwrap();
    assert_eq!(
        store.get(&key, None).await.unwrap_err(),
        StoreError::Unavailable
    );
    assert_eq!(
        store.delete(&key).await.unwrap_err(),
        StoreError::Unavailable
    );
    assert_eq!(
        store.put(&key, b"replacement".to_vec()).await.unwrap_err(),
        StoreError::Conflict
    );
    fs::remove_file(directory.path().join(key.as_str())).unwrap();
    store.put(&key, b"data".to_vec()).await.unwrap();
    let content = directory.path().join(key.as_str()).join("content");
    fs::remove_file(&content).unwrap();
    symlink(outside.path().join("protected"), &content).unwrap();
    assert_eq!(store.head(&key).await.unwrap_err(), StoreError::Unavailable);
    assert_eq!(
        store.delete(&key).await.unwrap_err(),
        StoreError::Unavailable
    );
    assert_eq!(fs::read(outside.path().join("protected")).unwrap(), b"kept");
}

#[cfg(unix)]
#[tokio::test]
async fn test_new_objects_should_use_private_directory_and_file_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let directory = Directory::create();
    let root = directory.path().join("objects");
    let store = FileObjectStore::open(&root).await.unwrap();
    let key = ObjectKey::parse(&"a".repeat(64)).unwrap();
    store.put(&key, b"private".to_vec()).await.unwrap();
    for path in [&root, &root.join(key.as_str())] {
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
    for name in ["content", "metadata"] {
        assert_eq!(
            fs::metadata(root.join(key.as_str()).join(name))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}
