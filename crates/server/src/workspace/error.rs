use axum::{
    Json,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: &'static str,
    issues: Vec<rustsurvey_domain::ValidationIssue>,
}
impl AppError {
    pub(super) fn json(error: axum::extract::rejection::JsonRejection) -> Self {
        let status = error.status();
        let (code, message) = match status {
            StatusCode::PAYLOAD_TOO_LARGE => (
                "body_too_large",
                "Le document dépasse la taille maximale autorisée pour cette route.",
            ),
            StatusCode::UNSUPPORTED_MEDIA_TYPE => (
                "json_required",
                "Envoyez le document avec le type application/json.",
            ),
            StatusCode::UNPROCESSABLE_ENTITY => (
                "invalid_definition_json",
                "Définition JSON incompatible : champ inconnu, type incorrect ou propriété requise absente.",
            ),
            _ => (
                "invalid_json",
                "Le corps de la requête n’est pas un JSON valide.",
            ),
        };
        Self::new(status, code, message)
    }
    pub fn new(status: StatusCode, code: &'static str, message: &'static str) -> Self {
        Self {
            status,
            code,
            message,
            issues: Vec::new(),
        }
    }
    pub fn invalid_definition(issues: Vec<rustsurvey_domain::ValidationIssue>) -> Self {
        let mut error = Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_definition",
            "Corrigez la définition du questionnaire.",
        );
        error.issues = issues;
        error
    }
    pub(super) fn with_issues(mut self, issues: Vec<rustsurvey_domain::ValidationIssue>) -> Self {
        self.issues = issues;
        self
    }
    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "L’opération n’a pas pu être terminée.",
        )
    }
    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "authentication_required",
            "Une connexion administrateur est requise.",
        )
    }
    pub fn not_found() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            "study_not_found",
            "Questionnaire introuvable.",
        )
    }
}
impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message)
    }
}
impl std::error::Error for AppError {}
impl From<sea_orm::DbErr> for AppError {
    fn from(_: sea_orm::DbErr) -> Self {
        Self::internal()
    }
}
impl From<sea_orm::sea_query::error::Error> for AppError {
    fn from(_: sea_orm::sea_query::error::Error) -> Self {
        Self::internal()
    }
}
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let mut response = (
            self.status,
            Json(json!({"error": {"code": self.code, "message": self.message, "issues": self.issues}})),
        )
            .into_response();
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-store"),
        );
        response
    }
}
