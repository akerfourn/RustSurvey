use rustsurvey_object_store::{
    ByteRange, MAX_OBJECT_BYTES, ObjectKey, ObjectMetadata, ObjectStore, StoreError,
};
pub async fn contract(store: &dyn ObjectStore) {
    let key = ObjectKey::parse(&"a".repeat(64)).unwrap();
    let missing = ObjectKey::parse(&"b".repeat(64)).unwrap();
    assert_eq!(
        store.head(&missing).await.unwrap_err(),
        StoreError::NotFound
    );
    let bytes: Vec<_> = (0..70000).map(|index| (index % 256) as u8).collect();
    let expected = ObjectMetadata {
        size: 70000,
        sha256: "0c6c96cc20d3f906e54f1f1296e8878c1ac39262fb587cd56235c3aa9103d837".to_owned(),
    };
    assert_eq!(store.put(&key, bytes.clone()).await.unwrap(), expected);
    assert_eq!(store.head(&key).await.unwrap(), expected);
    assert_eq!(store.get(&key, None).await.unwrap().bytes, bytes);
    assert_eq!(
        store.put(&key, b"replacement".to_vec()).await.unwrap_err(),
        StoreError::Conflict
    );
    let range = ByteRange {
        start: 130,
        end: 260,
    };
    let read = store.get(&key, Some(range)).await.unwrap();
    assert_eq!(read.bytes, bytes[130..260]);
    assert_eq!(read.metadata, expected);
    assert_eq!(read.range, range);
    assert_eq!(
        store
            .get(
                &key,
                Some(ByteRange {
                    start: 0,
                    end: 70001
                })
            )
            .await
            .unwrap_err(),
        StoreError::InvalidRange
    );
    assert_eq!(
        store
            .put(&missing, vec![0; MAX_OBJECT_BYTES + 1])
            .await
            .unwrap_err(),
        StoreError::TooLarge
    );
    assert_eq!(
        store.head(&missing).await.unwrap_err(),
        StoreError::NotFound
    );
    store.delete(&key).await.unwrap();
    store.delete(&key).await.unwrap();
    assert_eq!(store.head(&key).await.unwrap_err(), StoreError::NotFound);
}
