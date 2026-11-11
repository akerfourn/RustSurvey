use super::{AppError, WorkspaceState, auth_repository as repository, runtime::hex};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

const COOKIE: &str = "rustsurvey_administrator";
const SESSION_SECONDS: i64 = 12 * 60 * 60;

fn digest(token: &str) -> String {
    hex(&Sha256::digest(token.as_bytes()))
}
fn cookie_token(headers: &HeaderMap) -> Option<String> {
    let mut found = None;
    for header in headers.get_all(header::COOKIE) {
        for part in header.to_str().ok()?.split(';') {
            let Some((name, value)) = part.trim().split_once('=') else {
                continue;
            };
            if name != COOKIE {
                continue;
            }
            if found.is_some()
                || value.len() != 64
                || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return None;
            }
            found = Some(value.to_owned());
        }
    }
    found
}
fn cookie(state: &WorkspaceState, token: &str, seconds: i64) -> String {
    let mut value =
        format!("{COOKIE}={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age={seconds}");
    if state.secure_cookie {
        value.push_str("; Secure");
    }
    value
}
fn response(
    state: &WorkspaceState,
    value: serde_json::Value,
    session_cookie: Option<(&str, i64)>,
) -> Result<Response, AppError> {
    let mut response = ([(header::CACHE_CONTROL, "no-store")], Json(value)).into_response();
    if let Some((token, seconds)) = session_cookie {
        response.headers_mut().insert(
            header::SET_COOKIE,
            cookie(state, token, seconds)
                .parse()
                .map_err(|_| AppError::internal())?,
        );
    }
    Ok(response)
}
fn client_header(headers: &HeaderMap) -> Result<(), AppError> {
    if headers
        .get("x-requested-with")
        .and_then(|value| value.to_str().ok())
        != Some("RustSurvey")
    {
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            "invalid_request",
            "En-tête de requête requis.",
        ));
    }
    Ok(())
}
async fn authenticated(
    state: &WorkspaceState,
    headers: &HeaderMap,
) -> Result<Option<repository::Session>, AppError> {
    let Some(token) = cookie_token(headers) else {
        return Ok(None);
    };
    repository::session(&state.database, &digest(&token), state.runtime.now()?).await
}
pub(super) async fn authorize_write(
    state: &WorkspaceState,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    let session = authenticated(state, headers)
        .await?
        .ok_or_else(AppError::unauthorized)?;
    let supplied = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if !bool::from(session.csrf_token.as_bytes().ct_eq(supplied.as_bytes())) {
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            "invalid_csrf_token",
            "Jeton de formulaire invalide. Rechargez la session.",
        ));
    }
    Ok(())
}
pub(super) async fn session(
    State(state): State<WorkspaceState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let account = repository::account(&state.database).await?;
    let current = authenticated(&state, &headers).await?;
    if let (Some(account), Some(session)) = (&account, current) {
        return response(
            &state,
            json!({"authenticated":true, "configured":true, "username":account.username, "csrfToken":session.csrf_token}),
            None,
        );
    }
    response(
        &state,
        json!({"authenticated":false, "configured":account.is_some(), "username":null, "csrfToken":null}),
        Some(("", 0)),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Login {
    username: String,
    password: String,
}
pub(super) async fn login(
    State(state): State<WorkspaceState>,
    headers: HeaderMap,
    Json(login): Json<Login>,
) -> Result<Response, AppError> {
    client_header(&headers)?;
    if login.username.len() > 64 || login.password.len() > 1024 {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "invalid_credentials",
            "Identifiants invalides.",
        ));
    }
    let permit = state.login_slots.clone().try_acquire_owned().map_err(|_| {
        AppError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "login_busy",
            "Une connexion est déjà en cours. Réessayez.",
        )
    })?;
    let account = repository::account(&state.database)
        .await?
        .ok_or_else(AppError::unauthorized)?;
    let now = state.runtime.now()?;
    if account.blocked_until > now {
        return Err(AppError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "login_throttled",
            "Trop de tentatives. Réessayez dans une minute.",
        ));
    }
    if account.blocked_until != 0 {
        repository::reset_failures(&state.database).await?;
    }
    let hash = account.password_hash;
    let correct_username = account.username == login.username.trim().to_ascii_lowercase();
    let verified = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let parsed = PasswordHash::new(&hash).map_err(|_| AppError::internal())?;
        Ok::<bool, AppError>(
            Argon2::default()
                .verify_password(login.password.as_bytes(), &parsed)
                .is_ok(),
        )
    })
    .await
    .map_err(|_| AppError::internal())??;
    if !verified || !correct_username {
        repository::record_failure(&state.database, now).await?;
        return Err(AppError::new(
            StatusCode::UNAUTHORIZED,
            "invalid_credentials",
            "Identifiants invalides.",
        ));
    }
    repository::reset_failures(&state.database).await?;
    let token = hex(&state.runtime.random()?);
    let csrf = hex(&state.runtime.random()?);
    if let Some(previous) = cookie_token(&headers) {
        repository::delete_session(&state.database, &digest(&previous)).await?;
    }
    repository::insert_session(
        &state.database,
        digest(&token),
        csrf.clone(),
        now,
        now + SESSION_SECONDS,
    )
    .await?;
    response(
        &state,
        json!({"authenticated":true, "configured":true, "username":account.username, "csrfToken":csrf}),
        Some((&token, SESSION_SECONDS)),
    )
}
pub(super) async fn logout(
    State(state): State<WorkspaceState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    client_header(&headers)?;
    if authenticated(&state, &headers).await?.is_some() {
        authorize_write(&state, &headers).await?;
    }
    if let Some(token) = cookie_token(&headers) {
        repository::delete_session(&state.database, &digest(&token)).await?;
    }
    response(
        &state,
        json!({"authenticated":false, "configured":true, "username":null, "csrfToken":null}),
        Some(("", 0)),
    )
}
pub(super) async fn bootstrap(
    state: &WorkspaceState,
    username: &str,
    password: &str,
) -> Result<bool, AppError> {
    if repository::account(&state.database).await?.is_some() {
        return Ok(false);
    }
    let username = username.trim().to_ascii_lowercase();
    if username.is_empty()
        || username.len() > 64
        || !username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        || (password.chars().count() < 12 || password.len() > 1024)
    {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "invalid_administrator",
            "Identifiant ASCII requis ; mot de passe d’au moins 12 caractères et au plus 1024 octets.",
        ));
    }
    let salt = state.runtime.random()?;
    let password = password.to_owned();
    let hash = tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password_with_salt(password.as_bytes(), &salt[..16])
            .map(|value| value.to_string())
            .map_err(|_| AppError::internal())
    })
    .await
    .map_err(|_| AppError::internal())??;
    repository::insert_account(&state.database, username, hash).await
}
