use super::WebAssets;
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri, header},
    response::Response,
};

const CONTENT_SECURITY_POLICY: &str = "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'; object-src 'none'";

pub(super) async fn serve(
    State(assets): State<WebAssets>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    let path = match uri.path() {
        "/" => "/index.html",
        path => path,
    };
    let Some(asset) = assets.files.get(path) else {
        return empty(StatusCode::NOT_FOUND);
    };
    if method != Method::GET && method != Method::HEAD {
        let mut response = empty(StatusCode::METHOD_NOT_ALLOWED);
        response
            .headers_mut()
            .insert(header::ALLOW, "GET, HEAD".parse().unwrap());
        return response;
    }
    let unchanged = headers
        .get_all(header::IF_NONE_MATCH)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .any(|value| {
            value
                .split(',')
                .map(str::trim)
                .any(|tag| tag == "*" || tag.strip_prefix("W/").unwrap_or(tag) == asset.etag)
        });
    let mut builder = Response::builder()
        .header(header::CONTENT_TYPE, asset.content_type)
        .header(header::CACHE_CONTROL, asset.cache_control)
        .header(header::ETAG, &asset.etag);
    let response = if unchanged {
        builder
            .status(StatusCode::NOT_MODIFIED)
            .body(Body::empty())
            .unwrap()
    } else {
        builder = builder.header(header::CONTENT_LENGTH, asset.body.len());
        let body = if method == Method::HEAD {
            Body::empty()
        } else {
            Body::from(asset.body.clone())
        };
        builder.body(body).unwrap()
    };
    secure(response)
}

fn empty(status: StatusCode) -> Response {
    secure(
        Response::builder()
            .status(status)
            .header(header::CACHE_CONTROL, "no-store")
            .body(Body::empty())
            .unwrap(),
    )
}

fn secure(mut response: Response) -> Response {
    let headers = response.headers_mut();
    for (name, value) in [
        ("content-security-policy", CONTENT_SECURITY_POLICY),
        ("x-content-type-options", "nosniff"),
        ("x-frame-options", "DENY"),
        ("referrer-policy", "no-referrer"),
        ("cross-origin-resource-policy", "same-origin"),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
    ] {
        headers.insert(
            header::HeaderName::from_static(name),
            header::HeaderValue::from_static(value),
        );
    }
    response
}
