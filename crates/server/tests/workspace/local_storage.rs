use super::*;
use rustsurvey_object_store::{ObjectKey, ObjectStore, StoreError};
use rustsurvey_server::{
    settings::Settings,
    storage::{DEFAULT_DATABASE_URL, StorageConfiguration},
};
use std::{ffi::OsString, fs, io, path::Path};
use test_directory::Directory;

fn settings(values: &[(&str, &str)]) -> Settings {
    Settings::from_reader(
        io::empty(),
        values
            .iter()
            .map(|(name, value)| (OsString::from(name), OsString::from(value))),
    )
    .unwrap()
}
async fn local_store(database: DatabaseConnection, directory: &Path) -> Arc<dyn ObjectStore> {
    let settings = Settings::from_reader(
        io::empty(),
        [
            (
                OsString::from("OBJECT_STORE_BACKEND"),
                OsString::from("file"),
            ),
            (
                OsString::from("OBJECT_STORE_DIRECTORY"),
                directory.as_os_str().to_owned(),
            ),
        ],
    )
    .unwrap();
    StorageConfiguration::read(&settings)
        .unwrap()
        .object_store(database)
        .await
        .unwrap()
}

#[tokio::test]
async fn test_default_sqlite_should_prepare_private_data_and_preserve_an_existing_legacy_database()
{
    let directory = Directory::create();
    let configuration = StorageConfiguration::read(&settings(&[])).unwrap();
    assert_eq!(
        configuration.database_url(directory.path()).await.unwrap(),
        DEFAULT_DATABASE_URL
    );
    assert!(directory.path().join(".data").is_dir());
    fs::write(directory.path().join("rustsurvey.db"), b"legacy marker").unwrap();
    assert_eq!(
        configuration.database_url(directory.path()).await.unwrap(),
        "sqlite://rustsurvey.db?mode=rwc"
    );
    assert_eq!(
        fs::read(directory.path().join("rustsurvey.db")).unwrap(),
        b"legacy marker"
    );
    fs::write(
        directory.path().join(".data/rustsurvey.db"),
        b"second marker",
    )
    .unwrap();
    let error = configuration
        .database_url(directory.path())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("select one explicitly"));
    let explicit =
        StorageConfiguration::read(&settings(&[("DATABASE_URL", DEFAULT_DATABASE_URL)])).unwrap();
    assert_eq!(
        explicit.database_url(directory.path()).await.unwrap(),
        DEFAULT_DATABASE_URL
    );
}

#[tokio::test]
async fn test_explicit_databases_should_not_create_a_local_directory_and_invalid_storage_should_fail_clearly()
 {
    let directory = Directory::create();
    for url in ["sqlite::memory:", "postgres://synthetic-local-database"] {
        let configuration =
            StorageConfiguration::read(&settings(&[("DATABASE_URL", url)])).unwrap();
        assert_eq!(
            configuration.database_url(directory.path()).await.unwrap(),
            url
        );
        assert!(!directory.path().join(".data").exists());
    }
    for (name, value, expected) in [
        (
            "OBJECT_STORE_BACKEND",
            "unknown-private-value",
            "must be sql or file",
        ),
        ("DATABASE_URL", "", "must not be empty"),
    ] {
        let error = StorageConfiguration::read(&settings(&[(name, value)]))
            .err()
            .unwrap();
        assert!(error.to_string().contains(expected));
        assert!(!error.to_string().contains("unknown-private-value"));
    }
}

#[tokio::test]
async fn test_file_mode_should_respect_the_shared_contract_and_reopen_both_legacy_sql_and_new_file_objects()
 {
    let fixture = fixture().await;
    let directory = Directory::create();
    let store = local_store(fixture.database.clone(), directory.path()).await;
    object_contract::contract(store.as_ref()).await;
    let sql = rustsurvey_server::object_store::SqlObjectStore::new(fixture.database.clone());
    let legacy = ObjectKey::parse(&"c".repeat(64)).unwrap();
    let local = ObjectKey::parse(&"d".repeat(64)).unwrap();
    sql.put(&legacy, b"legacy SQL image".to_vec())
        .await
        .unwrap();
    assert_eq!(
        store.get(&legacy, None).await.unwrap().bytes,
        b"legacy SQL image"
    );
    assert_eq!(
        store
            .put(&legacy, b"must not shadow".to_vec())
            .await
            .unwrap_err(),
        StoreError::Conflict
    );
    store.put(&local, b"local image".to_vec()).await.unwrap();
    assert_eq!(
        sql.head(&local).await.unwrap_err(),
        StoreError::NotFound,
        "new binary content must be stored in files, not duplicated in SQL"
    );
    assert_eq!(
        fs::read(directory.path().join(local.as_str()).join("content")).unwrap(),
        b"local image"
    );
    drop(store);
    let reopened = local_store(fixture.database.clone(), directory.path()).await;
    assert_eq!(
        reopened.get(&legacy, None).await.unwrap().bytes,
        b"legacy SQL image"
    );
    assert_eq!(
        reopened.get(&local, None).await.unwrap().bytes,
        b"local image"
    );
    fs::write(
        directory.path().join(local.as_str()).join("content"),
        b"corrupted!!",
    )
    .unwrap();
    assert_eq!(
        reopened.get(&local, None).await.unwrap_err(),
        StoreError::Unavailable
    );
}

#[tokio::test]
async fn test_file_media_should_keep_access_controls_public_references_and_historical_restoration()
{
    let mut fixture = fixture().await;
    let directory = Directory::create();
    fixture.app = routes(
        WorkspaceState::with_runtime(fixture.database.clone(), false, fixture.runtime.clone())
            .with_object_store(local_store(fixture.database.clone(), directory.path()).await),
    );
    media_tests::roundtrip(&fixture).await;
    let objects = fixture
        .database
        .query_all(
            &Query::select()
                .column("object_key")
                .from("objects")
                .to_owned(),
        )
        .await
        .unwrap();
    assert!(
        objects.is_empty(),
        "file uploads must not leave their binary content in SQL"
    );
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}
