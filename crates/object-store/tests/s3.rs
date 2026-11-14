mod common;
mod s3_support;

use rustsurvey_object_store::{
    ObjectKey, ObjectStore, StoreError,
    s3::{S3Configuration, S3Credentials, S3ObjectStore},
};
use s3_support::{Fixture, MemoryIndex, Mode};
use std::sync::{Arc, atomic::Ordering};

fn credentials() -> S3Credentials {
    S3Credentials {
        access_key_id: "synthetic-access".into(),
        secret_access_key: "synthetic-secret".into(),
        session_token: None,
    }
}

#[test]
fn test_s3_configuration_should_reject_unsafe_endpoints_and_paths_without_disclosing_credentials() {
    for endpoint in [
        "http://example.invalid",
        "https://user:private@example.invalid",
        "https://example.invalid/path",
        "https://example.invalid/?secret=private",
        "https://example.invalid/#private",
        "file:///tmp/private",
    ] {
        let error = S3Configuration::new(
            endpoint,
            "test-bucket",
            "objects/v1",
            "test-region",
            credentials(),
            true,
        )
        .err()
        .unwrap();
        assert!(error.to_string().contains("HTTPS origin"));
        assert!(!error.to_string().contains("private"));
    }
    assert!(
        S3Configuration::new(
            "http://127.0.0.1:9000",
            "test-bucket",
            "objects/v1",
            "test-region",
            credentials(),
            false
        )
        .is_err()
    );
    assert!(
        S3Configuration::new(
            "http://127.0.0.1:9000",
            "test-bucket",
            "objects/v1",
            "test-region",
            credentials(),
            true
        )
        .is_ok()
    );
    for prefix in [
        "",
        "../private",
        "objects//v1",
        "objects/",
        "/objects",
        "objects?q=private",
    ] {
        assert!(
            S3Configuration::new(
                "https://example.invalid",
                "test-bucket",
                prefix,
                "test-region",
                credentials(),
                false
            )
            .is_err()
        );
    }
    for bucket in ["ab", "UPPER", "../private", "bucket..name", "127.0.0.1"] {
        assert!(
            S3Configuration::new(
                "https://example.invalid",
                bucket,
                "objects/v1",
                "test-region",
                credentials(),
                false
            )
            .is_err()
        );
    }
}

#[tokio::test]
async fn test_s3_should_respect_the_shared_contract_and_verify_its_immutable_write_capability() {
    let fixture = Fixture::start(Mode::Normal).await;
    common::contract(&fixture.store).await;
    let key = ObjectKey::parse(&"c".repeat(64)).unwrap();
    fixture.store.verify_immutable_writes(&key).await.unwrap();
    assert_eq!(fixture.store.head(&key).await, Err(StoreError::NotFound));
    let empty = ObjectKey::parse(&"d".repeat(64)).unwrap();
    fixture.store.put(&empty, vec![]).await.unwrap();
    assert!(
        fixture
            .store
            .get(&empty, None)
            .await
            .unwrap()
            .bytes
            .is_empty()
    );
}

#[tokio::test]
async fn test_s3_should_preserve_immutability_when_the_provider_ignores_conditional_writes() {
    let fixture = Fixture::start(Mode::IgnoreCondition).await;
    let key = ObjectKey::parse(&"c".repeat(64)).unwrap();
    fixture.store.verify_immutable_writes(&key).await.unwrap();
    assert_eq!(fixture.store.head(&key).await, Err(StoreError::NotFound));
}

#[tokio::test]
async fn test_s3_should_preserve_the_winner_when_two_writers_create_the_same_key() {
    let fixture = Fixture::start(Mode::Normal).await;
    let key = ObjectKey::parse(&"c".repeat(64)).unwrap();
    let (first, second) = tokio::join!(
        fixture.store.put(&key, b"first".to_vec()),
        fixture.store.put(&key, b"second".to_vec())
    );
    let winner = match (first, second) {
        (Ok(_), Err(StoreError::Conflict)) => b"first".as_slice(),
        (Err(StoreError::Conflict), Ok(_)) => b"second".as_slice(),
        _ => panic!("exactly one immutable writer must succeed"),
    };
    assert_eq!(fixture.store.get(&key, None).await.unwrap().bytes, winner);
}

#[tokio::test]
async fn test_s3_should_detect_corrupt_content_and_reject_unbounded_metadata() {
    for mode in [Mode::Corrupt, Mode::Oversize] {
        let fixture = Fixture::start(mode).await;
        let key = ObjectKey::parse(&"c".repeat(64)).unwrap();
        fixture.store.put(&key, b"original".to_vec()).await.unwrap();
        assert_eq!(
            fixture.store.get(&key, None).await.unwrap_err(),
            StoreError::Unavailable
        );
    }
}

#[tokio::test]
async fn test_s3_should_not_follow_redirects_or_disclose_remote_error_messages() {
    for mode in [Mode::Redirect, Mode::Denied, Mode::Missing] {
        let fixture = Fixture::start(Mode::Normal).await;
        let key = ObjectKey::parse(&"c".repeat(64)).unwrap();
        fixture.store.put(&key, b"original".to_vec()).await.unwrap();
        fixture.set_mode(mode);
        let previous = fixture.requests();
        let error = fixture.store.get(&key, None).await.unwrap_err();
        assert_eq!(error, StoreError::Unavailable);
        assert!(!error.to_string().contains("private"));
        assert_eq!(fixture.requests() - previous, 1);
    }
}

#[tokio::test]
#[ignore = "requires an isolated local S3 fixture from npm run test:s3"]
async fn test_external_s3_should_verify_signatures_and_respect_the_shared_immutable_contract() {
    let endpoint = std::env::var("RUSTSURVEY_TEST_S3_ENDPOINT").expect("fixture endpoint");
    let access = std::env::var("RUSTSURVEY_TEST_S3_ACCESS_KEY_ID").expect("fixture access key");
    let secret = std::env::var("RUSTSURVEY_TEST_S3_SECRET_ACCESS_KEY").expect("fixture secret key");
    let create = |secret: String| {
        S3Configuration::new(
            &endpoint,
            "rustsurvey-test",
            "contract/v1",
            "garage",
            S3Credentials {
                access_key_id: access.clone(),
                secret_access_key: secret,
                session_token: None,
            },
            true,
        )
        .unwrap()
    };
    assert!(
        endpoint.starts_with("http://127.0.0.1:"),
        "only disposable loopback S3 fixtures are allowed"
    );
    let index = Arc::new(MemoryIndex::default());
    let store = S3ObjectStore::new(create(secret), index.clone()).unwrap();
    common::contract(&store).await;
    store
        .verify_immutable_writes(&ObjectKey::parse(&"c".repeat(64)).unwrap())
        .await
        .unwrap();
    let existing = ObjectKey::parse(&"d".repeat(64)).unwrap();
    store
        .put(&existing, b"signature-check".to_vec())
        .await
        .unwrap();
    let wrong = S3ObjectStore::new(create("incorrect-synthetic-secret".into()), index).unwrap();
    assert_eq!(wrong.head(&existing).await, Err(StoreError::Unavailable));
}

#[tokio::test]
async fn test_s3_should_recover_a_completed_upload_after_sql_publication_fails() {
    let fixture = Fixture::start(Mode::Normal).await;
    let key = ObjectKey::parse(&"c".repeat(64)).unwrap();
    fixture.index.fail_next_insert.store(true, Ordering::SeqCst);
    assert_eq!(
        fixture.store.put(&key, b"retry".to_vec()).await,
        Err(StoreError::Unavailable)
    );
    assert_eq!(fixture.store.head(&key).await, Err(StoreError::NotFound));
    fixture.store.put(&key, b"retry".to_vec()).await.unwrap();
    assert_eq!(fixture.store.get(&key, None).await.unwrap().bytes, b"retry");
}

#[tokio::test]
async fn test_s3_should_bound_active_network_reads_without_queuing_more_objects() {
    let fixture = Fixture::start(Mode::Normal).await;
    let key = ObjectKey::parse(&"c".repeat(64)).unwrap();
    fixture.store.put(&key, b"bounded".to_vec()).await.unwrap();
    fixture.set_mode(Mode::Hold);
    let reads = async {
        tokio::join!(
            fixture.store.get(&key, None),
            fixture.store.get(&key, None),
            fixture.store.get(&key, None),
            fixture.store.get(&key, None)
        )
    };
    let saturated = async {
        fixture.wait_reads(4).await;
        assert_eq!(fixture.store.head(&key).await, Err(StoreError::Unavailable));
        fixture.release_reads();
    };
    let ((one, two, three, four), ()) = tokio::join!(reads, saturated);
    for result in [one, two, three, four] {
        assert_eq!(result.unwrap().bytes, b"bounded");
    }
    assert!(fixture.store.head(&key).await.is_ok());
}
