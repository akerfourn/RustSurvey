use super::*;
use rustsurvey_object_store::{
    ObjectKey, ObjectMetadata, ObjectStore, StoreError,
    s3::{ObjectBinding, S3ObjectIndex},
};
use rustsurvey_server::{
    object_store::SqlS3ObjectIndex, settings::Settings, storage::StorageConfiguration,
};
use std::{ffi::OsString, io};

pub(super) async fn index_roundtrip(fixture: &Fixture) {
    let first = SqlS3ObjectIndex::new(fixture.database.clone());
    let second = SqlS3ObjectIndex::new(fixture.database.clone());
    let key = ObjectKey::parse(&"e".repeat(64)).unwrap();
    let left = ObjectBinding {
        metadata: ObjectMetadata::for_bytes(b"first").unwrap(),
        location: "0".repeat(64),
    };
    let right = ObjectBinding {
        metadata: ObjectMetadata::for_bytes(b"second").unwrap(),
        location: "0".repeat(64),
    };
    let (one, two) = tokio::join!(first.insert(&key, &left), second.insert(&key, &right));
    let winner = match (one, two) {
        (Ok(()), Err(StoreError::Conflict)) => &left,
        (Err(StoreError::Conflict), Ok(())) => &right,
        _ => panic!("exactly one SQL publication must succeed"),
    };
    assert_eq!(first.head(&key).await.unwrap(), *winner);
    let reopened = SqlS3ObjectIndex::new(fixture.database.clone());
    assert_eq!(reopened.head(&key).await.unwrap(), *winner);
    let mut other = winner.clone();
    other.location = "1".repeat(64);
    reopened.delete(&key, &other).await.unwrap();
    assert_eq!(reopened.head(&key).await.unwrap(), *winner);
    reopened.delete(&key, winner).await.unwrap();
    reopened.delete(&key, winner).await.unwrap();
    assert_eq!(reopened.head(&key).await, Err(StoreError::NotFound));
}

#[tokio::test]
async fn test_s3_index_should_publish_one_immutable_binding_and_preserve_it_across_reopening() {
    index_roundtrip(&fixture().await).await;
}

#[tokio::test]
async fn test_s3_should_refuse_a_binding_from_another_location_before_contacting_the_endpoint() {
    use rustsurvey_object_store::s3::{S3Configuration, S3Credentials, S3ObjectStore};
    let fixture = fixture().await;
    let index = Arc::new(SqlS3ObjectIndex::new(fixture.database.clone()));
    let key = ObjectKey::parse(&"a".repeat(64)).unwrap();
    index
        .insert(
            &key,
            &ObjectBinding {
                metadata: ObjectMetadata::for_bytes(b"private").unwrap(),
                location: "0".repeat(64),
            },
        )
        .await
        .unwrap();
    let configuration = S3Configuration::new(
        "https://example.invalid",
        "test-bucket",
        "another/v1",
        "test-region",
        S3Credentials {
            access_key_id: "synthetic-access".into(),
            secret_access_key: "synthetic-secret".into(),
            session_token: None,
        },
        false,
    )
    .unwrap();
    let store = S3ObjectStore::new(configuration, index).unwrap();
    assert_eq!(store.head(&key).await, Err(StoreError::Unavailable));
    assert_eq!(
        store.get(&key, None).await.unwrap_err(),
        StoreError::Unavailable
    );
    assert_eq!(store.delete(&key).await, Err(StoreError::Unavailable));
}

#[test]
fn test_s3_configuration_should_require_explicit_credentials_without_echoing_private_values() {
    let settings = Settings::from_reader(
        io::empty(),
        [
            (OsString::from("OBJECT_STORE_BACKEND"), OsString::from("s3")),
            (
                OsString::from("S3_ENDPOINT"),
                OsString::from("https://example.invalid"),
            ),
        ],
    )
    .unwrap();
    let error = StorageConfiguration::read(&settings).err().unwrap();
    assert!(error.to_string().contains("S3_BUCKET"));
    assert!(!error.to_string().contains("example.invalid"));
}

fn external_settings() -> Settings {
    let endpoint = std::env::var("RUSTSURVEY_TEST_S3_ENDPOINT").expect("fixture endpoint");
    assert!(
        endpoint.starts_with("http://127.0.0.1:"),
        "only disposable loopback S3 fixtures are allowed"
    );
    let values = [
        ("OBJECT_STORE_BACKEND", "s3".into()),
        ("S3_ENDPOINT", endpoint),
        (
            "S3_ACCESS_KEY_ID",
            std::env::var("RUSTSURVEY_TEST_S3_ACCESS_KEY_ID").expect("fixture access key"),
        ),
        (
            "S3_SECRET_ACCESS_KEY",
            std::env::var("RUSTSURVEY_TEST_S3_SECRET_ACCESS_KEY").expect("fixture secret key"),
        ),
        ("S3_BUCKET", "rustsurvey-test".into()),
        ("S3_REGION", "garage".into()),
        ("S3_PREFIX", "workspace/v1".into()),
        ("S3_ALLOW_HTTP_LOOPBACK", "true".into()),
    ];
    Settings::from_reader(
        io::empty(),
        values
            .into_iter()
            .map(|(name, value)| (OsString::from(name), OsString::from(value))),
    )
    .unwrap()
}

#[tokio::test]
#[ignore = "requires an isolated local S3 fixture from npm run test:s3"]
async fn test_external_s3_should_preserve_sql_history_media_access_and_response_imports() {
    let mut fixture = fixture().await;
    let sql = rustsurvey_server::object_store::SqlObjectStore::new(fixture.database.clone());
    let legacy = ObjectKey::parse(&"f".repeat(64)).unwrap();
    sql.put(&legacy, b"legacy SQL".to_vec()).await.unwrap();
    let store = StorageConfiguration::read(&external_settings())
        .unwrap()
        .object_store(fixture.database.clone())
        .await
        .unwrap();
    assert_eq!(store.get(&legacy, None).await.unwrap().bytes, b"legacy SQL");
    assert_eq!(
        store.put(&legacy, b"no shadow".to_vec()).await,
        Err(StoreError::Conflict)
    );
    object_contract::contract(store.as_ref()).await;
    fixture.app = routes(
        WorkspaceState::with_runtime(fixture.database.clone(), false, fixture.runtime.clone())
            .with_object_store(store),
    );
    media_tests::roundtrip(&fixture).await;
    response_import_tests::roundtrip(&fixture).await;
    let reopened = StorageConfiguration::read(&external_settings())
        .unwrap()
        .object_store(fixture.database.clone())
        .await
        .unwrap();
    fixture.app = routes(
        WorkspaceState::with_runtime(fixture.database.clone(), false, fixture.runtime.clone())
            .with_object_store(reopened),
    );
    let (cookie, _) = login(&fixture).await;
    let batches = fixture
        .database
        .query_all(
            &Query::select()
                .columns(["id", "sha256"])
                .from("response_imports")
                .to_owned(),
        )
        .await
        .unwrap();
    assert!(!batches.is_empty());
    for batch in batches {
        let id: String = batch.try_get("", "id").unwrap();
        let sha256: String = batch.try_get("", "sha256").unwrap();
        let response = call(
            &fixture.app,
            "GET",
            &format!("/api/admin/response-imports/{id}/original.lsr"),
            None,
            Some(&cookie),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
            .await
            .unwrap();
        assert_eq!(ObjectMetadata::for_bytes(&bytes).unwrap().sha256, sha256);
    }
    let row = fixture
        .database
        .query_one(
            &Query::select()
                .expr(Expr::col("object_key").count())
                .from("objects")
                .to_owned(),
        )
        .await
        .unwrap()
        .unwrap();
    let count: i64 = row.try_get_by_index(0).unwrap();
    assert_eq!(
        count, 1,
        "only the legacy object may keep its binary content in SQL"
    );
}
