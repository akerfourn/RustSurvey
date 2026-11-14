#[path = "../../object-store/tests/common/directory.rs"]
mod directory;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    routing::get,
};
use directory::Directory;
use rustsurvey_server::{
    settings::Settings,
    web::{WebAssets, configured_assets},
};
use std::{fs, io};
use tower::ServiceExt;

const HTML: &str = "<!doctype html><script type=\"module\" src=\"/assets/app-12345678.js\"></script><link rel=\"stylesheet\" href=\"/assets/app-12345678.css\">";

fn build() -> Directory {
    let directory = Directory::create();
    fs::create_dir(directory.path().join("assets")).unwrap();
    fs::write(directory.path().join("index.html"), HTML).unwrap();
    fs::write(
        directory.path().join("assets/app-12345678.js"),
        b"export const value = 42;",
    )
    .unwrap();
    fs::write(
        directory.path().join("assets/app-12345678.css"),
        b"body { color: black; }",
    )
    .unwrap();
    directory
}

async fn call(
    router: &Router,
    method: &str,
    path: &str,
    etag: Option<&str>,
) -> axum::response::Response {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(etag) = etag {
        request = request.header(header::IF_NONE_MATCH, etag);
    }
    router
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn test_web_build_should_serve_the_same_immutable_bytes_with_head_and_conditional_requests() {
    let directory = build();
    let router = WebAssets::load(directory.path()).await.unwrap().routes();
    fs::write(
        directory.path().join("assets/app-12345678.js"),
        b"changed on disk",
    )
    .unwrap();
    let response = call(&router, "GET", "/", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-cache");
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "text/html; charset=utf-8"
    );
    assert_eq!(to_bytes(response.into_body(), 1024).await.unwrap(), HTML);
    let response = call(&router, "GET", "/assets/app-12345678.js", None).await;
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    let etag = response.headers()[header::ETAG]
        .to_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap(),
        "export const value = 42;"
    );
    let head = call(&router, "HEAD", "/assets/app-12345678.js", None).await;
    assert_eq!(head.headers()[header::CONTENT_LENGTH], "24");
    assert!(to_bytes(head.into_body(), 1024).await.unwrap().is_empty());
    let unchanged = call(
        &router,
        "GET",
        "/assets/app-12345678.js",
        Some(&format!("\"another\", W/{etag}")),
    )
    .await;
    assert_eq!(unchanged.status(), StatusCode::NOT_MODIFIED);
    assert!(
        unchanged.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("frame-ancestors 'none'")
    );
    assert!(
        to_bytes(unchanged.into_body(), 1024)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        call(&router, "GET", "/assets/app-12345678.js", Some("\"old\""))
            .await
            .status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn test_web_build_should_never_serve_private_files_source_maps_or_traversal_paths() {
    let directory = build();
    fs::write(directory.path().join(".env"), b"private marker").unwrap();
    fs::write(
        directory.path().join("assets/app-12345678.js.map"),
        b"private source map",
    )
    .unwrap();
    let router = WebAssets::load(directory.path()).await.unwrap().routes();
    for path in [
        "/.env",
        "/.git/config",
        "/.data/rustsurvey.db",
        "/assets/../.env",
        "/assets/%2e%2e/.env",
        "/assets/%2F..%2F.env",
        "/assets/app-12345678.js.map",
        "/api/missing",
    ] {
        let response = call(&router, "GET", path, None).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert!(
            to_bytes(response.into_body(), 1024)
                .await
                .unwrap()
                .is_empty()
        );
    }
}

#[tokio::test]
async fn test_web_build_should_keep_api_routes_and_refuse_nonread_methods_on_assets() {
    let directory = build();
    let router = Router::new()
        .route(
            "/api/admin/session",
            get(|| async { StatusCode::UNAUTHORIZED }),
        )
        .merge(WebAssets::load(directory.path()).await.unwrap().routes());
    assert_eq!(
        call(&router, "GET", "/api/admin/session", None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = call(&router, "POST", "/", None).await;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(response.headers()[header::ALLOW], "GET, HEAD");
}

#[tokio::test]
async fn test_web_build_should_refuse_incomplete_development_and_oversized_bundles() {
    let directory = build();
    fs::remove_file(directory.path().join("assets/app-12345678.js")).unwrap();
    assert!(WebAssets::load(directory.path()).await.is_err());
    fs::write(
        directory.path().join("assets/app-12345678.js"),
        vec![b'x'; 4 * 1024 * 1024 + 1],
    )
    .unwrap();
    assert!(WebAssets::load(directory.path()).await.is_err());
    fs::write(
        directory.path().join("index.html"),
        "<script src=\"/src/main.tsx\"></script>",
    )
    .unwrap();
    let error = WebAssets::load(directory.path()).await.err().unwrap();
    assert!(error.to_string().contains("STATIC_ASSETS_DIRECTORY"));
    assert!(
        !error
            .to_string()
            .contains(&directory.path().to_string_lossy().to_string())
    );
}

#[tokio::test]
async fn test_web_configuration_should_keep_api_only_mode_and_reject_an_explicit_empty_directory() {
    assert!(
        configured_assets(&Settings::from_reader(io::empty(), []).unwrap())
            .await
            .unwrap()
            .is_none()
    );
    let settings = Settings::from_reader("STATIC_ASSETS_DIRECTORY=\n".as_bytes(), []).unwrap();
    assert!(configured_assets(&settings).await.is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn test_web_build_should_refuse_symbolic_links_in_public_assets() {
    use std::os::unix::fs::symlink;
    let directory = build();
    let script = directory.path().join("assets/app-12345678.js");
    fs::remove_file(&script).unwrap();
    symlink(directory.path().join("index.html"), script).unwrap();
    assert!(WebAssets::load(directory.path()).await.is_err());
}
