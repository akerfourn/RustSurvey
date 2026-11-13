use super::*;
use rustsurvey_object_store::{
    ByteRange, ObjectKey, ObjectMetadata, ObjectRead, ObjectStore, StoreError, StoreFuture,
    memory::MemoryObjectStore,
};
use std::sync::atomic::AtomicBool;

const MEDIA_ID: &str = "00000000-0000-4000-8000-000000000010";
const PIXEL: &[u8] = include_bytes!("../fixtures/pixel.png");

async fn upload(
    fixture: &Fixture,
    study: &str,
    bytes: Vec<u8>,
    cookie: Option<&str>,
    csrf: Option<&str>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method("PUT")
        .uri(format!(
            "/api/admin/studies/{study}/media/{MEDIA_ID}?name=pixel.png"
        ))
        .header(header::CONTENT_TYPE, "application/octet-stream");
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    if let Some(csrf) = csrf {
        request = request.header("x-csrf-token", csrf);
    }
    fixture
        .app
        .clone()
        .oneshot(request.body(Body::from(bytes)).unwrap())
        .await
        .unwrap()
}

pub(super) async fn roundtrip(fixture: &Fixture) {
    let (cookie, csrf) = login(fixture).await;
    let study = create(fixture, &cookie, &csrf, definition()).await;
    let id = study["id"].as_str().unwrap();
    assert_eq!(
        upload(fixture, id, PIXEL.to_vec(), None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        upload(fixture, id, PIXEL.to_vec(), Some(&cookie), None)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let saved = json(upload(fixture, id, PIXEL.to_vec(), Some(&cookie), Some(&csrf)).await).await;
    assert_eq!(saved["state"], "ready");
    assert_eq!(saved["contentType"], "image/png");
    assert!(
        saved.get("objectKey").is_none(),
        "storage keys are private implementation details"
    );
    assert_eq!(
        json(upload(fixture, id, PIXEL.to_vec(), Some(&cookie), Some(&csrf)).await).await,
        saved,
        "a repeated upload must acknowledge the same object"
    );
    let private = format!("/api/admin/studies/{id}/media/{MEDIA_ID}");
    let public = format!("/api/studies/{id}/media/{MEDIA_ID}");
    assert_eq!(
        call(&fixture.app, "GET", &private, None, None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&fixture.app, "GET", &public, None, None, None)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    let private_read = call(&fixture.app, "GET", &private, None, Some(&cookie), None).await;
    assert_eq!(
        private_read.status(),
        StatusCode::OK,
        "read-only media access must not require CSRF"
    );
    assert_eq!(
        to_bytes(private_read.into_body(), 1024)
            .await
            .unwrap()
            .as_ref(),
        PIXEL
    );
    let other = create(fixture, &cookie, &csrf, definition()).await;
    let mut referenced = definition();
    referenced["groups"][0]["questions"][0]["media"] =
        json!([{"id":MEDIA_ID,"alt":"Pixel synthétique","language":null}]);
    let wrong_owner = call(
        &fixture.app,
        "PUT",
        &format!("/api/admin/studies/{}", other["id"].as_str().unwrap()),
        Some(json!({"expectedRevision":1,"definition":referenced})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(wrong_owner.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let update = format!("/api/admin/studies/{id}");
    let saved = call(
        &fixture.app,
        "PUT",
        &update,
        Some(json!({"expectedRevision":1,"definition":referenced})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(saved.status(), StatusCode::OK);
    let request = Request::builder()
        .uri(&public)
        .header(header::RANGE, "bytes=0-7")
        .body(Body::empty())
        .unwrap();
    let partial = fixture.app.clone().oneshot(request).await.unwrap();
    assert_eq!(partial.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        partial.headers()[header::CONTENT_RANGE],
        format!("bytes 0-7/{}", PIXEL.len())
    );
    assert_eq!(
        to_bytes(partial.into_body(), 1024).await.unwrap().as_ref(),
        &PIXEL[..8]
    );
    let head = call(&fixture.app, "HEAD", &public, None, None, None).await;
    assert_eq!(head.status(), StatusCode::OK);
    assert_eq!(
        head.headers()[header::CONTENT_LENGTH],
        PIXEL.len().to_string()
    );
    assert!(to_bytes(head.into_body(), 1024).await.unwrap().is_empty());
    let detached = call(
        &fixture.app,
        "PUT",
        &update,
        Some(json!({"expectedRevision":2,"definition":definition()})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(detached.status(), StatusCode::OK);
    assert_eq!(
        call(&fixture.app, "GET", &public, None, None, None)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    let restored = call(
        &fixture.app,
        "POST",
        &format!("/api/admin/studies/{id}/restore"),
        Some(json!({"expectedRevision":3,"revision":2})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(restored.status(), StatusCode::OK);
    assert_eq!(
        call(&fixture.app, "GET", &public, None, None, None)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        call(
            &fixture.app,
            "GET",
            &format!(
                "/api/studies/{}/media/{MEDIA_ID}",
                other["id"].as_str().unwrap()
            ),
            None,
            None,
            None
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn test_media_should_remain_private_until_referenced_and_survive_historical_restoration() {
    roundtrip(&fixture().await).await;
}

#[tokio::test]
async fn test_media_upload_should_refuse_oversized_and_nonimage_bodies() {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let study = create(&fixture, &cookie, &csrf, definition()).await;
    let id = study["id"].as_str().unwrap();
    assert_eq!(
        upload(
            &fixture,
            id,
            b"<svg onload='bad()'></svg>".to_vec(),
            Some(&cookie),
            Some(&csrf)
        )
        .await
        .status(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    assert_eq!(
        upload(
            &fixture,
            id,
            vec![0; 8 * 1024 * 1024 + 1],
            Some(&cookie),
            Some(&csrf)
        )
        .await
        .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let list = json(
        call(
            &fixture.app,
            "GET",
            &format!("/api/admin/studies/{id}/media"),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert!(list["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_media_reads_should_hold_bounded_slots_until_the_body_is_consumed_or_dropped() {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let stored = create(&fixture, &cookie, &csrf, definition()).await;
    let id = stored["id"].as_str().unwrap();
    assert_eq!(
        upload(&fixture, id, PIXEL.to_vec(), Some(&cookie), Some(&csrf))
            .await
            .status(),
        StatusCode::OK
    );
    let path = format!("/api/admin/studies/{id}/media/{MEDIA_ID}");
    let mut responses = Vec::new();
    for _ in 0..4 {
        let response = call(&fixture.app, "GET", &path, None, Some(&cookie), None).await;
        assert_eq!(response.status(), StatusCode::OK);
        responses.push(response);
    }
    assert_eq!(
        call(&fixture.app, "GET", &path, None, Some(&cookie), None)
            .await
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    drop(responses.pop());
    let resumed = call(&fixture.app, "GET", &path, None, Some(&cookie), None).await;
    assert_eq!(resumed.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(resumed.into_body(), 1024).await.unwrap().as_ref(),
        PIXEL
    );
    assert_eq!(
        call(&fixture.app, "GET", &path, None, Some(&cookie), None)
            .await
            .status(),
        StatusCode::OK
    );
}

struct LostAcknowledgement {
    memory: MemoryObjectStore,
    fail: AtomicBool,
}
impl ObjectStore for LostAcknowledgement {
    fn put<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ObjectMetadata> {
        Box::pin(async move {
            let metadata = self.memory.put(key, bytes).await?;
            if self.fail.swap(false, Ordering::SeqCst) {
                return Err(StoreError::Unavailable);
            }
            Ok(metadata)
        })
    }
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ObjectMetadata> {
        self.memory.head(key)
    }
    fn get<'a>(
        &'a self,
        key: &'a ObjectKey,
        range: Option<ByteRange>,
    ) -> StoreFuture<'a, ObjectRead> {
        self.memory.get(key, range)
    }
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()> {
        self.memory.delete(key)
    }
}
#[tokio::test]
async fn test_upload_retry_should_reconcile_an_object_written_before_a_lost_acknowledgement() {
    let mut fixture = fixture().await;
    let objects = Arc::new(LostAcknowledgement {
        memory: MemoryObjectStore::new(10000, 10),
        fail: AtomicBool::new(true),
    });
    fixture.app = routes(
        WorkspaceState::with_runtime(fixture.database.clone(), false, fixture.runtime.clone())
            .with_object_store(objects.clone()),
    );
    let (cookie, csrf) = login(&fixture).await;
    let study = create(&fixture, &cookie, &csrf, definition()).await;
    let id = study["id"].as_str().unwrap();
    assert_eq!(
        upload(&fixture, id, PIXEL.to_vec(), Some(&cookie), Some(&csrf))
            .await
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let list = json(
        call(
            &fixture.app,
            "GET",
            &format!("/api/admin/studies/{id}/media"),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(list["items"][0]["state"], "pending");
    fixture.app = routes(
        WorkspaceState::with_runtime(fixture.database.clone(), false, fixture.runtime.clone())
            .with_object_store(objects),
    );
    let recovered =
        json(upload(&fixture, id, PIXEL.to_vec(), Some(&cookie), Some(&csrf)).await).await;
    assert_eq!(recovered["state"], "ready");
    let list = json(
        call(
            &fixture.app,
            "GET",
            &format!("/api/admin/studies/{id}/media"),
            None,
            Some(&cookie),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(
        list["items"].as_array().unwrap().len(),
        1,
        "retries must not create duplicate objects"
    );
}
