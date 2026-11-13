use super::{AppError, WorkspaceState, missing};
use axum::http::{HeaderMap, StatusCode, header};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub(super) fn digest(bytes: &[u8]) -> String {
    super::super::runtime::hex(&Sha256::digest(bytes))
}

pub(super) fn token_hash(headers: &HeaderMap) -> Result<String, AppError> {
    let mut authorization = headers.get_all(header::AUTHORIZATION).iter();
    let token = authorization
        .next()
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "))
        .ok_or_else(missing)?;
    if authorization.next().is_some()
        || token.len() != 64
        || !token.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(missing());
    }
    Ok(digest(token.as_bytes()))
}

pub(super) fn same_hash(left: &str, right: &str) -> bool {
    bool::from(left.as_bytes().ct_eq(right.as_bytes()))
}

pub(super) fn client(headers: &HeaderMap) -> Result<(), AppError> {
    if headers
        .get("X-Requested-With")
        .is_none_or(|value| value != "RustSurvey")
    {
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            "client_header_required",
            "En-tête de client requis.",
        ));
    }
    Ok(())
}

pub(super) fn id(id: &str) -> Result<(), AppError> {
    if !uuid::Uuid::try_parse(id).is_ok_and(|parsed| parsed.to_string() == id) {
        return Err(missing());
    }
    Ok(())
}

pub(super) fn write_slot(
    state: &WorkspaceState,
) -> Result<tokio::sync::SemaphorePermit<'_>, AppError> {
    state.writes.try_acquire().map_err(|_| {
        AppError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "write_busy",
            "Deux enregistrements sont déjà en cours. Réessayez.",
        )
    })
}
