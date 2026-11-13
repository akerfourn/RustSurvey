//! HTTP admission checks; authoritative counters are changed inside the run lock.
use super::{AppError, Run, invitations};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use rustsurvey_domain::participation::ParticipationPolicy;
use sea_orm::{
    ConnectionTrait,
    sea_query::{Expr, ExprTrait, Query},
};

pub(super) fn invalid_policy(message: &'static str) -> AppError {
    AppError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_participation_policy",
        message,
    )
}

pub(super) fn denied() -> AppError {
    AppError::new(
        StatusCode::FORBIDDEN,
        "invitation_unavailable",
        "Invitation invalide, révoquée ou hors de sa période de validité.",
    )
}

pub(super) fn exhausted() -> AppError {
    AppError::new(
        StatusCode::CONFLICT,
        "invitation_exhausted",
        "Le nombre de soumissions autorisées pour cette invitation est atteint.",
    )
}

fn cookie_name(run: &Run) -> String {
    format!("rs_completed_{}", run.id.replace('-', ""))
}

pub(super) fn browser_completed(run: &Run, headers: &HeaderMap) -> bool {
    if run.participation_policy != ParticipationPolicy::BrowserOnce {
        return false;
    }
    let name = cookie_name(run);
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|entry| entry.trim().split_once('='))
        .any(|(key, value)| key == name && value == "complete")
}

pub(super) fn completion_cookie(run: &Run, secure: bool) -> Result<Option<HeaderValue>, AppError> {
    if run.participation_policy != ParticipationPolicy::BrowserOnce {
        return Ok(None);
    }
    let mut value = format!(
        "{}=complete; Path=/api; HttpOnly; SameSite=Lax; Max-Age=31536000",
        cookie_name(run)
    );
    if secure {
        value.push_str("; Secure");
    }
    HeaderValue::from_str(&value)
        .map(Some)
        .map_err(|_| AppError::internal())
}

pub(super) fn check_browser(completed: bool) -> Result<(), AppError> {
    if completed {
        return Err(AppError::new(
            StatusCode::CONFLICT,
            "browser_already_completed",
            "Ce navigateur a déjà envoyé une réponse à cette campagne.",
        ));
    }
    Ok(())
}

pub(super) async fn authorize_response(
    connection: &impl ConnectionTrait,
    id: &str,
    now: i64,
) -> Result<(), AppError> {
    if let Some(invitation) = invitations::by_response(connection, id).await? {
        invitation.check_active(now)?;
    }
    Ok(())
}

pub(super) async fn admit_start(
    connection: &impl ConnectionTrait,
    run: &Run,
    invitation_hash: Option<&str>,
    access_hash: &str,
    completed: bool,
    now: i64,
) -> Result<Option<invitations::Invitation>, AppError> {
    check_browser(completed)?;
    if !run.participation_policy.requires_invitation() {
        return Ok(None);
    }
    let invitation =
        invitations::by_secret(connection, &run.id, invitation_hash.ok_or_else(denied)?)
            .await?
            .ok_or_else(denied)?;
    invitation.check_active(now)?;
    if run.participation_policy.persists_response() {
        if !super::access::same_hash(&invitation.secret_hash, access_hash) {
            return Err(denied());
        }
        if invitation.response_id.is_some() {
            return Ok(Some(invitation));
        }
    }
    invitation.check_remaining()?;
    Ok(Some(invitation))
}

pub(super) async fn admit_write(
    connection: &impl ConnectionTrait,
    run: &Run,
    id: &str,
    invitation_hash: Option<&str>,
    submit: bool,
    now: i64,
) -> Result<(), AppError> {
    if !run.participation_policy.requires_invitation() {
        return Ok(());
    }
    let invitation = if run.participation_policy.persists_response() {
        invitations::by_response(connection, id).await?
    } else {
        invitations::by_secret(connection, &run.id, invitation_hash.ok_or_else(denied)?).await?
    }
    .ok_or_else(denied)?;
    invitation.check_active(now)?;
    // A correction of the same linked response never consumes a second use.
    let already_submitted = run.participation_policy.persists_response() && invitation.uses > 0;
    if !already_submitted {
        invitation.check_remaining()?;
        if submit {
            connection
                .execute(
                    &Query::update()
                        .table("invitations")
                        .value("uses", Expr::col("uses").add(1_i64))
                        .and_where(Expr::col("id").eq(&invitation.id))
                        .to_owned(),
                )
                .await?;
        }
    }
    Ok(())
}
