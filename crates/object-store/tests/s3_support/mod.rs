use axum::{
    Router,
    body::{Body, to_bytes},
    extract::State,
    http::{Request, Response, StatusCode},
};
use rustsurvey_object_store::{
    MAX_OBJECT_BYTES, ObjectKey, StoreError, StoreFuture,
    s3::{ObjectBinding, S3Configuration, S3Credentials, S3ObjectIndex, S3ObjectStore},
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

#[derive(Clone, Copy)]
pub enum Mode {
    Normal,
    IgnoreCondition,
    Corrupt,
    Oversize,
    Redirect,
    Denied,
    Hold,
    Missing,
}
#[derive(Clone)]
struct Object {
    bytes: Vec<u8>,
    sha256: String,
}
struct Server {
    mode: Mutex<Mode>,
    objects: Mutex<BTreeMap<String, Object>>,
    requests: AtomicUsize,
    entered: tokio::sync::Semaphore,
    release: tokio::sync::Notify,
}
pub struct Fixture {
    pub store: S3ObjectStore,
    pub index: Arc<MemoryIndex>,
    state: Arc<Server>,
    task: tokio::task::JoinHandle<()>,
}
impl Fixture {
    pub async fn start(mode: Mode) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let state = Arc::new(Server {
            mode: Mutex::new(mode),
            objects: Mutex::default(),
            requests: AtomicUsize::new(0),
            entered: tokio::sync::Semaphore::new(0),
            release: tokio::sync::Notify::new(),
        });
        let configuration = S3Configuration::new(
            &format!("http://{}", listener.local_addr().unwrap()),
            "test-bucket",
            "objects/v1",
            "test-region",
            S3Credentials {
                access_key_id: "synthetic-access".into(),
                secret_access_key: "synthetic-secret".into(),
                session_token: Some("synthetic-session-token".into()),
            },
            true,
        )
        .unwrap();
        let router = Router::new().fallback(handle).with_state(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let index = Arc::new(MemoryIndex::default());
        Self {
            store: S3ObjectStore::new(configuration, index.clone()).unwrap(),
            index,
            state,
            task,
        }
    }
    pub fn requests(&self) -> usize {
        self.state.requests.load(Ordering::SeqCst)
    }
    pub fn set_mode(&self, mode: Mode) {
        *self.state.mode.lock().unwrap() = mode;
    }
    pub async fn wait_reads(&self, count: u32) {
        self.state
            .entered
            .acquire_many(count)
            .await
            .unwrap()
            .forget();
    }
    pub fn release_reads(&self) {
        self.state.release.notify_waiters();
    }
}

#[derive(Default)]
pub struct MemoryIndex {
    bindings: Mutex<BTreeMap<ObjectKey, ObjectBinding>>,
    pub fail_next_insert: AtomicBool,
}
impl S3ObjectIndex for MemoryIndex {
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ObjectBinding> {
        Box::pin(async move {
            self.bindings
                .lock()
                .unwrap()
                .get(key)
                .cloned()
                .ok_or(StoreError::NotFound)
        })
    }
    fn insert<'a>(&'a self, key: &'a ObjectKey, binding: &'a ObjectBinding) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            if self.fail_next_insert.swap(false, Ordering::SeqCst) {
                return Err(StoreError::Unavailable);
            }
            let mut bindings = self.bindings.lock().unwrap();
            if bindings.contains_key(key) {
                return Err(StoreError::Conflict);
            }
            bindings.insert(key.clone(), binding.clone());
            Ok(())
        })
    }
    fn delete<'a>(&'a self, key: &'a ObjectKey, binding: &'a ObjectBinding) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            let mut bindings = self.bindings.lock().unwrap();
            if bindings.get(key) == Some(binding) {
                bindings.remove(key);
            }
            Ok(())
        })
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn handle(State(state): State<Arc<Server>>, request: Request<Body>) -> Response<Body> {
    state.requests.fetch_add(1, Ordering::SeqCst);
    let mode = *state.mode.lock().unwrap();
    let headers = request.headers();
    let auth = headers.get("authorization").unwrap().to_str().unwrap();
    assert!(auth.starts_with("AWS4-HMAC-SHA256 Credential=synthetic-access/"));
    assert_eq!(headers["x-amz-security-token"], "synthetic-session-token");
    assert!(auth.contains("x-amz-security-token"));
    if matches!(mode, Mode::Redirect) {
        return Response::builder()
            .status(302)
            .header("location", "/must-not-follow")
            .body(Body::empty())
            .unwrap();
    }
    if matches!(mode, Mode::Denied) {
        return status(StatusCode::FORBIDDEN);
    }
    if matches!(mode, Mode::Missing) {
        return status(StatusCode::NOT_FOUND);
    }
    if matches!(mode, Mode::Hold) && request.method() == "GET" {
        let release = state.release.notified();
        tokio::pin!(release);
        release.as_mut().enable();
        state.entered.add_permits(1);
        release.await;
    }
    let path = request.uri().path().to_owned();
    assert!(path.starts_with("/test-bucket/objects/v1/"));
    match request.method().as_str() {
        "PUT" => {
            assert_eq!(headers["if-none-match"], "*");
            assert!(auth.contains("if-none-match"));
            assert_eq!(headers["x-amz-meta-rustsurvey-format"], "1");
            let sha256 = headers["x-amz-meta-rustsurvey-sha256"]
                .to_str()
                .unwrap()
                .to_owned();
            assert_eq!(headers["x-amz-content-sha256"], sha256);
            let bytes = to_bytes(request.into_body(), MAX_OBJECT_BYTES)
                .await
                .unwrap()
                .to_vec();
            let mut objects = state.objects.lock().unwrap();
            if objects.contains_key(&path) && !matches!(mode, Mode::IgnoreCondition) {
                return status(StatusCode::PRECONDITION_FAILED);
            }
            objects.insert(path, Object { bytes, sha256 });
            status(StatusCode::OK)
        }
        "GET" | "HEAD" => {
            let objects = state.objects.lock().unwrap();
            let Some(object) = objects.get(&path) else {
                return status(StatusCode::NOT_FOUND);
            };
            let mut bytes = object.bytes.clone();
            if matches!(mode, Mode::Corrupt) && !bytes.is_empty() {
                bytes[0] ^= 1;
            }
            let mut size = bytes.len();
            if matches!(mode, Mode::Oversize) {
                size = MAX_OBJECT_BYTES + 1;
            }
            let body = if request.method() == "HEAD" {
                Body::empty()
            } else {
                Body::from(bytes)
            };
            Response::builder()
                .status(200)
                .header("content-length", size)
                .header("x-amz-meta-rustsurvey-format", "1")
                .header("x-amz-meta-rustsurvey-sha256", &object.sha256)
                .body(body)
                .unwrap()
        }
        "DELETE" => {
            state.objects.lock().unwrap().remove(&path);
            status(StatusCode::NO_CONTENT)
        }
        _ => status(StatusCode::METHOD_NOT_ALLOWED),
    }
}
fn status(status: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(Body::empty())
        .unwrap()
}
